use sqlx::SqliteConnection;

use super::models::{CreatePoint, TimelinePoint, UpdatePoint};
use crate::error::Error;

pub async fn list(
    connection: &mut SqliteConnection,
    universe_id: i64,
) -> Result<Vec<TimelinePoint>, Error> {
    Ok(sqlx::query_as!(
        TimelinePoint,
        r#"SELECT id AS "id!", universe_id, name, position FROM timeline_points WHERE universe_id = ? AND deleted_at IS NULL ORDER BY position, id"#,
        universe_id,
    )
    .fetch_all(connection)
    .await?)
}

pub async fn create(
    connection: &mut SqliteConnection,
    universe_id: i64,
    input: CreatePoint,
) -> Result<TimelinePoint, Error> {
    Ok(sqlx::query_as!(
        TimelinePoint,
        r#"INSERT INTO timeline_points (universe_id, name, position) VALUES (?, ?, ?) RETURNING id AS "id!", universe_id, name, position"#,
        universe_id,
        input.name,
        input.position,
    )
    .fetch_one(connection)
    .await?)
}

pub async fn get(connection: &mut SqliteConnection, id: i64) -> Result<TimelinePoint, Error> {
    sqlx::query_as!(
        TimelinePoint,
        r#"SELECT id AS "id!", universe_id, name, position FROM timeline_points WHERE id = ? AND deleted_at IS NULL AND universe_id IN (SELECT id FROM universes WHERE deleted_at IS NULL)"#,
        id,
    )
    .fetch_optional(connection)
    .await?
    .ok_or(Error::NotFound)
}

pub async fn update(
    connection: &mut SqliteConnection,
    id: i64,
    input: UpdatePoint,
) -> Result<TimelinePoint, Error> {
    get(connection, id).await?;
    Ok(sqlx::query_as!(
        TimelinePoint,
        r#"UPDATE timeline_points SET name = COALESCE(?, name), position = COALESCE(?, position) WHERE id = ? RETURNING id AS "id!", universe_id, name, position"#,
        input.name,
        input.position,
        id,
    )
    .fetch_one(connection)
    .await?)
}

pub async fn delete(connection: &mut SqliteConnection, id: i64) -> Result<(), Error> {
    get(connection, id).await?;
    sqlx::query!(
        "UPDATE timeline_points SET deleted_at = CURRENT_TIMESTAMP WHERE id = ?",
        id,
    )
    .execute(connection)
    .await?;
    Ok(())
}

pub async fn restore(connection: &mut SqliteConnection, id: i64) -> Result<TimelinePoint, Error> {
    let result = sqlx::query!(
        "UPDATE timeline_points SET deleted_at = NULL WHERE id = ? AND deleted_at IS NOT NULL AND universe_id IN (SELECT id FROM universes WHERE deleted_at IS NULL)",
        id,
    )
    .execute(&mut *connection)
    .await?;
    if result.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    get(connection, id).await
}

/// A piece may only point at an active point of its own universe.
pub async fn check_for_piece(
    connection: &mut SqliteConnection,
    point_id: i64,
    piece_universe_id: i64,
) -> Result<(), Error> {
    let point = sqlx::query_scalar!(
        "SELECT universe_id FROM timeline_points WHERE id = ? AND deleted_at IS NULL",
        point_id,
    )
    .fetch_optional(connection)
    .await?;
    match point {
        Some(universe_id) if universe_id == piece_universe_id => Ok(()),
        Some(_) => Err(Error::Validation(
            "timeline point belongs to another universe".into(),
        )),
        None => Err(Error::Validation("timeline point does not exist".into())),
    }
}
