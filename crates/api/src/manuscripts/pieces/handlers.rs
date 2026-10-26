use axum::{
    extract::{
        rejection::{JsonRejection, QueryRejection},
        Path, Query, State,
    },
    http::StatusCode,
    Json,
};
use sqlx::{sqlite::SqlitePool, SqliteConnection};

use super::models::{
    CreateChild, CreatePiece, LoosePieces, Piece, PieceDetail, Storage, UpdatePiece, EMPTY_DELTA,
};
use crate::chapter_revisions::{load, Revision};
use crate::error::Error;
use crate::manuscripts::{
    chapters::models::{MissedDeltas, SubmitDelta},
    json_input, leaf_sync, trigger_rejection,
    universes::handlers::existing_universe,
};

pub(super) async fn piece(connection: &mut SqliteConnection, id: i64) -> Result<Piece, Error> {
    sqlx::query_as!(
        Piece,
        r#"SELECT id AS "id!", universe_id, container_id, parent_id, title, position, label, format, shape FROM pieces WHERE id = ?"#,
        id
    )
    .fetch_optional(connection)
    .await?
    .ok_or(Error::NotFound)
}

pub(super) async fn detail(
    connection: &mut SqliteConnection,
    piece: Piece,
) -> Result<PieceDetail, Error> {
    if piece.shape == "group" {
        let children = sqlx::query_as!(
            Piece,
            r#"SELECT id AS "id!", universe_id, container_id, parent_id, title, position, label, format, shape FROM pieces WHERE parent_id = ? ORDER BY position, id"#,
            piece.id
        )
        .fetch_all(connection)
        .await?;
        return Ok(PieceDetail {
            piece,
            content: None,
            head_rev: None,
            children: Some(children),
        });
    }
    let head = load(connection, piece.id)
        .await
        .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
    Ok(PieceDetail {
        piece,
        content: Some(head.delta),
        head_rev: Some(head.head_rev),
        children: None,
    })
}

async fn insert_root(
    connection: &mut SqliteConnection,
    universe_id: i64,
    container_id: Option<i64>,
    input: CreatePiece,
) -> Result<Piece, Error> {
    let format = input.format();
    let leaf = input.shape == "leaf";
    let content = leaf.then_some(EMPTY_DELTA);
    let revision = leaf.then_some(0_i64);
    sqlx::query_as!(
        Piece,
        r#"INSERT INTO pieces (universe_id, container_id, title, position, label, format, shape, content, head_rev, snapshot_rev) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id AS "id!", universe_id, container_id, parent_id, title, position, label, format, shape"#,
        universe_id,
        container_id,
        input.title,
        input.position,
        input.label,
        format,
        input.shape,
        content,
        revision,
        revision,
    )
    .fetch_one(connection)
    .await
    .map_err(trigger_rejection)
}

pub async fn list_loose_pieces(
    State(pool): State<SqlitePool>,
    Path(universe_id): Path<i64>,
    query: Result<Query<LoosePieces>, QueryRejection>,
) -> Result<Json<Vec<Piece>>, Error> {
    let Query(LoosePieces {
        container: Storage::None,
    }) = query.map_err(|error| Error::Validation(error.body_text()))?;
    let mut transaction = pool.begin().await?;
    existing_universe(&mut transaction, universe_id).await?;
    let pieces = sqlx::query_as!(
        Piece,
        r#"SELECT id AS "id!", universe_id, container_id, parent_id, title, position, label, format, shape FROM pieces WHERE universe_id = ? AND container_id IS NULL AND parent_id IS NULL ORDER BY position, id"#,
        universe_id
    )
    .fetch_all(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(Json(pieces))
}

pub async fn create_loose_piece(
    State(pool): State<SqlitePool>,
    Path(universe_id): Path<i64>,
    input: Result<Json<CreatePiece>, JsonRejection>,
) -> Result<(StatusCode, Json<Piece>), Error> {
    let input = json_input(input)?;
    input.validate()?;
    let mut transaction = pool.begin().await?;
    existing_universe(&mut transaction, universe_id).await?;
    let piece = insert_root(&mut transaction, universe_id, None, input).await?;
    transaction.commit().await?;
    Ok((StatusCode::CREATED, Json(piece)))
}

async fn container_universe(
    connection: &mut SqliteConnection,
    container_id: i64,
) -> Result<i64, Error> {
    sqlx::query_scalar!(
        "SELECT universe_id FROM containers WHERE id = ?",
        container_id
    )
    .fetch_optional(connection)
    .await?
    .ok_or(Error::NotFound)
}

pub async fn list_container_pieces(
    State(pool): State<SqlitePool>,
    Path(container_id): Path<i64>,
) -> Result<Json<Vec<Piece>>, Error> {
    let mut transaction = pool.begin().await?;
    container_universe(&mut transaction, container_id).await?;
    let pieces = sqlx::query_as!(
        Piece,
        r#"SELECT id AS "id!", universe_id, container_id, parent_id, title, position, label, format, shape FROM pieces WHERE container_id = ? ORDER BY position, id"#,
        container_id
    )
    .fetch_all(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(Json(pieces))
}

pub async fn create_container_piece(
    State(pool): State<SqlitePool>,
    Path(container_id): Path<i64>,
    input: Result<Json<CreatePiece>, JsonRejection>,
) -> Result<(StatusCode, Json<Piece>), Error> {
    let input = json_input(input)?;
    input.validate()?;
    let mut transaction = pool.begin().await?;
    let universe_id = container_universe(&mut transaction, container_id).await?;
    let piece = insert_root(&mut transaction, universe_id, Some(container_id), input).await?;
    transaction.commit().await?;
    Ok((StatusCode::CREATED, Json(piece)))
}

pub async fn create_child(
    State(pool): State<SqlitePool>,
    Path(group_id): Path<i64>,
    input: Result<Json<CreateChild>, JsonRejection>,
) -> Result<(StatusCode, Json<Piece>), Error> {
    let input = json_input(input)?;
    input.validate()?;
    let mut transaction = pool.begin().await?;
    let group = piece(&mut transaction, group_id).await?;
    if group.shape != "group" {
        return Err(Error::Validation("children belong to a group".into()));
    }
    let label = input.label(&group.format);
    let child = sqlx::query_as!(
        Piece,
        r#"INSERT INTO pieces (universe_id, parent_id, title, position, label, format, shape, content, head_rev, snapshot_rev) VALUES (?, ?, ?, ?, ?, ?, 'leaf', ?, 0, 0) RETURNING id AS "id!", universe_id, container_id, parent_id, title, position, label, format, shape"#,
        group.universe_id,
        group.id,
        input.title,
        input.position,
        label,
        group.format,
        EMPTY_DELTA,
    )
    .fetch_one(&mut *transaction)
    .await
    .map_err(trigger_rejection)?;
    transaction.commit().await?;
    Ok((StatusCode::CREATED, Json(child)))
}

pub async fn get_piece(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<Json<PieceDetail>, Error> {
    let mut transaction = pool.begin().await?;
    let piece = piece(&mut transaction, id).await?;
    let detail = detail(&mut transaction, piece).await?;
    transaction.commit().await?;
    Ok(Json(detail))
}

pub async fn update_piece(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    input: Result<Json<UpdatePiece>, JsonRejection>,
) -> Result<Json<PieceDetail>, Error> {
    let input = json_input(input)?;
    input.validate()?;
    let mut transaction = pool.begin().await?;
    let piece = sqlx::query_as!(
        Piece,
        r#"UPDATE pieces SET title = COALESCE(?, title), position = COALESCE(?, position), label = COALESCE(?, label), format = COALESCE(?, format) WHERE id = ? RETURNING id AS "id!", universe_id, container_id, parent_id, title, position, label, format, shape"#,
        input.title,
        input.position,
        input.label,
        input.format,
        id,
    )
    .fetch_optional(&mut *transaction)
    .await
    .map_err(trigger_rejection)?
    .ok_or(Error::NotFound)?;
    let detail = detail(&mut transaction, piece).await?;
    transaction.commit().await?;
    Ok(Json(detail))
}

pub async fn delete_piece(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<StatusCode, Error> {
    let result = sqlx::query!("DELETE FROM pieces WHERE id = ?", id)
        .execute(&pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_deltas(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    query: Result<Query<MissedDeltas>, QueryRejection>,
) -> Result<Json<Vec<Revision>>, Error> {
    let Query(query) = query.map_err(|error| Error::Validation(error.body_text()))?;
    let (_, revisions) = leaf_sync::history(&pool, id, query.since).await?;
    Ok(Json(revisions))
}

pub async fn post_delta(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    input: Result<Json<SubmitDelta>, JsonRejection>,
) -> Result<Json<Revision>, Error> {
    let Json(input) = input.map_err(|error| Error::Validation(error.body_text()))?;
    let revision = leaf_sync::submit(&pool, id, input.base_rev, &input.delta).await?;
    Ok(Json(revision))
}
