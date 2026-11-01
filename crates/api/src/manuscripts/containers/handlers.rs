use axum::{
    extract::{rejection::JsonRejection, Path, Query, State},
    http::StatusCode,
    Json,
};
use sqlx::sqlite::SqlitePool;

use super::models::{Container, CreateContainer, UpdateContainer};
use crate::error::Error;
use crate::manuscripts::{json_input, universes::handlers::existing_universe};

pub async fn list_containers(
    State(pool): State<SqlitePool>,
    Path(universe_id): Path<i64>,
) -> Result<Json<Vec<Container>>, Error> {
    let mut transaction = pool.begin().await?;
    existing_universe(&mut transaction, universe_id).await?;
    let containers = sqlx::query_as!(
        Container,
        r#"SELECT id AS "id!", universe_id, title, position, kind FROM containers WHERE universe_id = ? AND deleted_at IS NULL ORDER BY position, id"#,
        universe_id
    )
    .fetch_all(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(Json(containers))
}

pub async fn create_container(
    State(pool): State<SqlitePool>,
    Path(universe_id): Path<i64>,
    input: Result<Json<CreateContainer>, JsonRejection>,
) -> Result<(StatusCode, Json<Container>), Error> {
    let input = json_input(input)?;
    input.validate()?;
    let mut transaction = pool.begin().await?;
    existing_universe(&mut transaction, universe_id).await?;
    let container = sqlx::query_as!(
        Container,
        r#"INSERT INTO containers (universe_id, title, position, kind) VALUES (?, ?, ?, ?) RETURNING id AS "id!", universe_id, title, position, kind"#,
        universe_id,
        input.title,
        input.position,
        input.kind,
    )
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok((StatusCode::CREATED, Json(container)))
}

pub async fn get_container(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<Json<Container>, Error> {
    let container = sqlx::query_as!(
        Container,
        "SELECT id, universe_id, title, position, kind FROM containers WHERE id = ? AND deleted_at IS NULL AND universe_id IN (SELECT id FROM universes WHERE deleted_at IS NULL)",
        id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(Error::NotFound)?;
    Ok(Json(container))
}

pub async fn update_container(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    input: Result<Json<UpdateContainer>, JsonRejection>,
) -> Result<Json<Container>, Error> {
    let input = json_input(input)?;
    input.validate()?;
    let container = sqlx::query_as!(
        Container,
        "UPDATE containers SET title = COALESCE(?, title), position = COALESCE(?, position) WHERE id = ? AND deleted_at IS NULL AND universe_id IN (SELECT id FROM universes WHERE deleted_at IS NULL) RETURNING id, universe_id, title, position, kind",
        input.title,
        input.position,
        id,
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(Error::NotFound)?;
    Ok(Json(container))
}

pub async fn delete_container(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Query(query): Query<crate::manuscripts::lore::handlers::DeleteQuery>,
) -> Result<StatusCode, Error> {
    let sql = if query.permanent {
        "DELETE FROM containers WHERE id = ?"
    } else {
        "UPDATE containers SET deleted_at = CURRENT_TIMESTAMP WHERE id = ? AND deleted_at IS NULL AND universe_id IN (SELECT id FROM universes WHERE deleted_at IS NULL)"
    };
    if sqlx::query(sql)
        .bind(id)
        .execute(&pool)
        .await?
        .rows_affected()
        == 0
    {
        return Err(Error::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn restore_container(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<Json<Container>, Error> {
    let mut transaction = pool.begin().await?;
    let universe_id = sqlx::query_scalar::<_, i64>("SELECT universe_id FROM containers WHERE id = ? AND (deleted_at IS NOT NULL OR universe_id IN (SELECT id FROM universes WHERE deleted_at IS NOT NULL))").bind(id).fetch_optional(&mut *transaction).await?.ok_or(Error::NotFound)?;
    crate::manuscripts::pieces::trash::restore_universe(&mut transaction, universe_id).await?;
    crate::manuscripts::pieces::trash::restore_container(&mut transaction, id).await?;
    transaction.commit().await?;
    get_container(State(pool), Path(id)).await
}
