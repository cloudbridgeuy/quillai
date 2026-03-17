use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::sqlite::SqlitePool;

use super::models::{CreateUser, User};
use crate::error::Error;

pub async fn list_users(State(pool): State<SqlitePool>) -> Result<Json<Vec<User>>, Error> {
    let users = sqlx::query_as!(User, "SELECT id, name, email FROM users")
        .fetch_all(&pool)
        .await?;
    Ok(Json(users))
}

pub async fn create_user(
    State(pool): State<SqlitePool>,
    Json(input): Json<CreateUser>,
) -> Result<(StatusCode, Json<User>), Error> {
    input.validate()?;
    let result = sqlx::query!(
        "INSERT INTO users (name, email) VALUES (?, ?)",
        input.name,
        input.email,
    )
    .execute(&pool)
    .await?;

    let user = User {
        id: result.last_insert_rowid(),
        name: input.name,
        email: input.email,
    };
    Ok((StatusCode::CREATED, Json(user)))
}

pub async fn get_user(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<Json<User>, Error> {
    let user = sqlx::query_as!(User, "SELECT id, name, email FROM users WHERE id = ?", id)
        .fetch_optional(&pool)
        .await?
        .ok_or(Error::NotFound)?;
    Ok(Json(user))
}

pub async fn delete_user(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<StatusCode, Error> {
    let result = sqlx::query!("DELETE FROM users WHERE id = ?", id)
        .execute(&pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
