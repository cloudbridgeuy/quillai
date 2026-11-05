use std::collections::HashMap;

use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use sqlx::SqlitePool;

use crate::{
    error::Error,
    manuscripts::{
        books::models::Book, chapters::models::ChapterSummary, containers::models::Container,
        pieces::models::Piece, series::models::Series,
    },
};

#[derive(Serialize)]
struct Tree {
    series: Vec<SeriesTree>,
    universes: Vec<UniverseTree>,
}

#[derive(Serialize)]
struct SeriesTree {
    #[serde(flatten)]
    series: Series,
    books: Vec<BookTree>,
}

#[derive(Serialize)]
struct BookTree {
    #[serde(flatten)]
    book: Book,
    chapters: Vec<ChapterSummary>,
}

#[derive(Serialize)]
struct UniverseTree {
    #[serde(flatten)]
    universe: Series,
    containers: Vec<ContainerTree>,
    loose: Vec<PieceTree>,
}

#[derive(Serialize)]
struct ContainerTree {
    #[serde(flatten)]
    container: Container,
    pieces: Vec<PieceTree>,
}

#[derive(Serialize)]
struct PieceTree {
    #[serde(flatten)]
    piece: Piece,
    children: Vec<Piece>,
}

struct ChapterWithBook {
    book_id: i64,
    id: i64,
    title: String,
    position: i64,
    kind: String,
    head_rev: i64,
}

fn assemble(
    series: Vec<Series>,
    books: Vec<Book>,
    chapters: Vec<ChapterWithBook>,
    universes: Vec<Series>,
    containers: Vec<Container>,
    pieces: Vec<Piece>,
) -> Tree {
    let mut chapters_by_book: HashMap<i64, Vec<ChapterSummary>> = HashMap::new();
    for chapter in chapters {
        chapters_by_book
            .entry(chapter.book_id)
            .or_default()
            .push(ChapterSummary {
                id: chapter.id,
                title: chapter.title,
                position: chapter.position,
                kind: chapter.kind,
                head_rev: chapter.head_rev,
            });
    }
    let mut books_by_series: HashMap<i64, Vec<BookTree>> = HashMap::new();
    for book in books {
        let chapters = chapters_by_book.remove(&book.id).unwrap_or_default();
        books_by_series
            .entry(book.series_id)
            .or_default()
            .push(BookTree { book, chapters });
    }
    let series = series
        .into_iter()
        .map(|series| {
            let books = books_by_series.remove(&series.id).unwrap_or_default();
            SeriesTree { series, books }
        })
        .collect();
    let mut children: HashMap<i64, Vec<Piece>> = HashMap::new();
    let mut roots = Vec::new();
    for piece in pieces {
        if let Some(parent_id) = piece.parent_id {
            children.entry(parent_id).or_default().push(piece);
        } else {
            roots.push(piece);
        }
    }
    let mut by_container: HashMap<i64, Vec<PieceTree>> = HashMap::new();
    let mut loose: HashMap<i64, Vec<PieceTree>> = HashMap::new();
    for piece in roots {
        let descendants = children.remove(&piece.id).unwrap_or_default();
        let container_id = piece.container_id;
        let universe_id = piece.universe_id;
        let tree = PieceTree {
            piece,
            children: descendants,
        };
        if let Some(id) = container_id {
            by_container.entry(id).or_default().push(tree);
        } else {
            loose.entry(universe_id).or_default().push(tree);
        }
    }
    let mut by_universe: HashMap<i64, Vec<ContainerTree>> = HashMap::new();
    for container in containers {
        let pieces = by_container.remove(&container.id).unwrap_or_default();
        by_universe
            .entry(container.universe_id)
            .or_default()
            .push(ContainerTree { container, pieces });
    }
    let universes = universes
        .into_iter()
        .map(|universe| {
            let containers = by_universe.remove(&universe.id).unwrap_or_default();
            let loose = loose.remove(&universe.id).unwrap_or_default();
            UniverseTree {
                universe,
                containers,
                loose,
            }
        })
        .collect();
    Tree { series, universes }
}

async fn get_tree(State(pool): State<SqlitePool>) -> Result<Json<Tree>, Error> {
    let mut tx = pool.begin().await?;
    let series = sqlx::query_as!(Series, r#"SELECT id AS "id!", title AS "title!", position AS "position!" FROM legacy_series ORDER BY position, id"#).fetch_all(&mut *tx).await?;
    let books = sqlx::query_as!(Book, r#"SELECT id AS "id!", series_id AS "series_id!", title AS "title!", position AS "position!" FROM legacy_books ORDER BY position, id"#).fetch_all(&mut *tx).await?;
    let chapters = sqlx::query_as!(ChapterWithBook, r#"SELECT book_id AS "book_id!", id AS "id!", title AS "title!", position AS "position!", kind AS "kind!", head_rev AS "head_rev!" FROM legacy_chapters ORDER BY position, id"#).fetch_all(&mut *tx).await?;
    let universes = sqlx::query_as!(
        Series,
        "SELECT id, title, position FROM universes WHERE deleted_at IS NULL ORDER BY position, id"
    )
    .fetch_all(&mut *tx)
    .await?;
    let containers = sqlx::query_as!(Container, r#"SELECT c.id AS "id!", c.universe_id, c.title, c.position, c.kind FROM containers c JOIN universes u ON u.id = c.universe_id WHERE c.deleted_at IS NULL AND u.deleted_at IS NULL ORDER BY c.position, c.id"#).fetch_all(&mut *tx).await?;
    let pieces = sqlx::query_as!(Piece, r#"SELECT id AS "id!", universe_id, container_id, parent_id, title, position, label, format, shape, timeline_point_id FROM active_pieces ORDER BY position, id"#).fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(assemble(
        series, books, chapters, universes, containers, pieces,
    )))
}

pub fn routes(pool: SqlitePool) -> Router {
    Router::new().route("/tree", get(get_tree)).with_state(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_tree() {
        let tree = assemble(vec![], vec![], vec![], vec![], vec![], vec![]);
        assert_eq!(
            serde_json::to_value(tree).unwrap(),
            serde_json::json!({"series": [], "universes": []})
        );
    }
    fn piece(id: i64, container_id: Option<i64>, parent_id: Option<i64>, shape: &str) -> Piece {
        Piece {
            id,
            universe_id: 1,
            container_id,
            parent_id,
            title: id.to_string(),
            position: id,
            label: "piece".into(),
            format: "prose".into(),
            shape: shape.into(),
            timeline_point_id: None,
        }
    }

    #[test]
    fn groups_roots_and_children_in_input_order() {
        let universe = Series {
            id: 1,
            title: "World".into(),
            position: 0,
        };
        let container = Container {
            id: 2,
            universe_id: 1,
            title: "Collection".into(),
            position: 0,
            kind: "collection".into(),
        };
        let tree = assemble(
            vec![],
            vec![],
            vec![],
            vec![universe],
            vec![container],
            vec![
                piece(3, None, Some(5), "leaf"),
                piece(4, Some(2), None, "leaf"),
                piece(5, Some(2), None, "group"),
                piece(6, None, Some(5), "leaf"),
                piece(7, None, None, "leaf"),
            ],
        );
        let container = &tree.universes[0].containers[0];
        assert_eq!(
            container
                .pieces
                .iter()
                .map(|p| p.piece.id)
                .collect::<Vec<_>>(),
            [4, 5]
        );
        assert!(container.pieces[0].children.is_empty());
        assert_eq!(
            container.pieces[1]
                .children
                .iter()
                .map(|p| p.id)
                .collect::<Vec<_>>(),
            [3, 6]
        );
        assert_eq!(tree.universes[0].loose[0].piece.id, 7);
        assert!(tree.universes[0].loose[0].children.is_empty());
    }
}
