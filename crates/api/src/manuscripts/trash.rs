use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use sqlx::SqlitePool;

use crate::error::Error;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Item {
    #[serde(rename = "type")]
    kind: &'static str,
    id: i64,
    title: String,
    deleted_at: String,
    location: Location,
    child_counts: Counts,
    deleted_parents: Vec<String>,
    restore_ancestors: Vec<String>,
    resource_path: String,
}

#[derive(Default, Serialize)]
struct Location {
    universe: Option<String>,
    series: Option<String>,
    book: Option<String>,
}

#[derive(Default, Serialize)]
struct Counts {
    chapters: usize,
    pieces: usize,
    lore: usize,
}

struct Universe {
    id: i64,
    title: String,
    deleted_at: Option<String>,
}
struct Container {
    id: i64,
    universe_id: i64,
    title: String,
    kind: String,
    deleted_at: Option<String>,
}
struct Piece {
    id: i64,
    universe_id: i64,
    container_id: Option<i64>,
    parent_id: Option<i64>,
    title: String,
    shape: String,
    label: String,
    format: String,
    legacy_book_id: Option<i64>,
    legacy_chapter_id: Option<i64>,
    deleted_at: Option<String>,
}
struct Lore {
    id: i64,
    universe_id: i64,
    name: String,
    deleted_at: Option<String>,
}

pub fn routes(pool: SqlitePool) -> Router {
    Router::new().route("/trash", get(list)).with_state(pool)
}

fn piece_kind(
    piece: &Piece,
    parent: Option<&Piece>,
    container: Option<&Container>,
) -> &'static str {
    if piece.legacy_book_id.is_some()
        && piece.shape == "group"
        && piece.label == "novel"
        && piece.format == "prose"
        && container.is_some_and(|c| c.kind == "book_series")
    {
        "book"
    } else if piece.legacy_chapter_id.is_some()
        && piece.shape == "leaf"
        && matches!(piece.label.as_str(), "prologue" | "chapter" | "epilogue")
        && parent.is_some_and(|p| piece_kind(p, None, container) == "book")
    {
        "chapter"
    } else {
        "piece"
    }
}

async fn list(State(pool): State<SqlitePool>) -> Result<Json<Vec<Item>>, Error> {
    let mut transaction = pool.begin().await?;
    let universes = sqlx::query_as!(Universe, "SELECT id, title, deleted_at FROM universes")
        .fetch_all(&mut *transaction)
        .await?;
    let containers = sqlx::query_as!(
        Container,
        "SELECT id, universe_id, title, kind, deleted_at FROM containers"
    )
    .fetch_all(&mut *transaction)
    .await?;
    let pieces = sqlx::query_as!(Piece, "SELECT id, universe_id, container_id, parent_id, title, shape, label, format, legacy_book_id, legacy_chapter_id, deleted_at FROM pieces").fetch_all(&mut *transaction).await?;
    let lore = sqlx::query_as!(
        Lore,
        "SELECT id, universe_id, name, deleted_at FROM lore_entries"
    )
    .fetch_all(&mut *transaction)
    .await?;
    transaction.commit().await?;

    let mut items = Vec::new();
    for u in &universes {
        if let Some(deleted_at) = &u.deleted_at {
            let descendants = pieces.iter().filter(|p| p.universe_id == u.id);
            items.push(Item {
                kind: "universe",
                id: u.id,
                title: u.title.clone(),
                deleted_at: deleted_at.clone(),
                location: Location::default(),
                child_counts: counts(
                    descendants,
                    &pieces,
                    &containers,
                    lore.iter().filter(|l| l.universe_id == u.id).count(),
                ),
                deleted_parents: vec![],
                restore_ancestors: vec![],
                resource_path: format!("/universes/{}", u.id),
            });
        }
    }
    for c in &containers {
        if let Some(deleted_at) = &c.deleted_at {
            let u = universes.iter().find(|u| u.id == c.universe_id);
            let kind = if c.kind == "collection" {
                "collection"
            } else {
                "series"
            };
            let belongs = |p: &Piece| root_container(p, &pieces) == Some(c.id);
            let descendants = pieces.iter().filter(|p| belongs(p));
            let orphan = c.kind == "book_series"
                && !containers
                    .iter()
                    .any(|other| other.universe_id == c.universe_id && other.id != c.id)
                && !pieces
                    .iter()
                    .any(|p| p.universe_id == c.universe_id && !belongs(p));
            let lore_count = if orphan {
                lore.iter()
                    .filter(|l| l.universe_id == c.universe_id)
                    .count()
            } else {
                0
            };
            items.push(Item {
                kind,
                id: c.id,
                title: c.title.clone(),
                deleted_at: deleted_at.clone(),
                location: Location {
                    universe: u.map(|u| u.title.clone()),
                    ..Default::default()
                },
                child_counts: counts(descendants, &pieces, &containers, lore_count),
                deleted_parents: u
                    .filter(|u| u.deleted_at.is_some())
                    .map(|u| vec![u.title.clone()])
                    .unwrap_or_default(),
                restore_ancestors: vec![],
                resource_path: format!(
                    "/{}/{}",
                    if c.kind == "book_series" {
                        "series"
                    } else {
                        "containers"
                    },
                    c.id
                ),
            });
        }
    }
    for p in &pieces {
        if let Some(deleted_at) = &p.deleted_at {
            let parent = p
                .parent_id
                .and_then(|id| pieces.iter().find(|p| p.id == id));
            let c = p
                .container_id
                .or_else(|| parent.and_then(|p| p.container_id))
                .and_then(|id| containers.iter().find(|c| c.id == id));
            let u = universes.iter().find(|u| u.id == p.universe_id);
            let kind = piece_kind(p, parent, c);
            let mut deleted_parents = Vec::new();
            if let Some(u) = u.filter(|u| u.deleted_at.is_some()) {
                deleted_parents.push(u.title.clone());
            }
            if let Some(c) = c.filter(|c| c.deleted_at.is_some()) {
                deleted_parents.push(c.title.clone());
            }
            if let Some(parent) = parent.filter(|p| p.deleted_at.is_some()) {
                deleted_parents.push(parent.title.clone());
            }
            items.push(Item {
                kind,
                id: p.id,
                title: p.title.clone(),
                deleted_at: deleted_at.clone(),
                location: Location {
                    universe: u.map(|u| u.title.clone()),
                    series: c
                        .filter(|c| c.kind != "collection")
                        .map(|c| c.title.clone()),
                    book: parent
                        .filter(|p| piece_kind(p, None, c) == "book")
                        .map(|p| p.title.clone()),
                },
                child_counts: counts(
                    pieces.iter().filter(|child| child.parent_id == Some(p.id)),
                    &pieces,
                    &containers,
                    0,
                ),
                deleted_parents,
                restore_ancestors: vec![],
                resource_path: format!("/pieces/{}", p.id),
            });
        }
    }
    for l in &lore {
        if let Some(deleted_at) = &l.deleted_at {
            let u = universes.iter().find(|u| u.id == l.universe_id);
            items.push(Item {
                kind: "lore",
                id: l.id,
                title: l.name.clone(),
                deleted_at: deleted_at.clone(),
                location: Location {
                    universe: u.map(|u| u.title.clone()),
                    ..Default::default()
                },
                child_counts: Counts::default(),
                deleted_parents: u
                    .filter(|u| u.deleted_at.is_some())
                    .map(|u| vec![u.title.clone()])
                    .unwrap_or_default(),
                restore_ancestors: u
                    .filter(|u| u.deleted_at.is_some())
                    .map(|u| vec![format!("/universes/{}", u.id)])
                    .unwrap_or_default(),
                resource_path: format!("/universes/{}/lore/{}", l.universe_id, l.id),
            });
        }
    }
    items.sort_by(|a, b| {
        b.deleted_at
            .cmp(&a.deleted_at)
            .then_with(|| a.kind.cmp(b.kind))
            .then_with(|| a.id.cmp(&b.id))
    });
    Ok(Json(items))
}

fn counts<'a>(
    descendants: impl Iterator<Item = &'a Piece>,
    pieces: &[Piece],
    containers: &[Container],
    lore: usize,
) -> Counts {
    let mut counts = Counts {
        lore,
        ..Default::default()
    };
    for p in descendants {
        let parent = p
            .parent_id
            .and_then(|id| pieces.iter().find(|p| p.id == id));
        let c = p
            .container_id
            .or_else(|| parent.and_then(|p| p.container_id))
            .and_then(|id| containers.iter().find(|c| c.id == id));
        if piece_kind(p, parent, c) == "chapter" {
            counts.chapters += 1;
        } else {
            counts.pieces += 1;
        }
    }
    counts
}

fn root_container(p: &Piece, pieces: &[Piece]) -> Option<i64> {
    p.container_id.or_else(|| {
        p.parent_id
            .and_then(|id| pieces.iter().find(|parent| parent.id == id)?.container_id)
    })
}
