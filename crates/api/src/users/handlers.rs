use axum::{extract::State, http::StatusCode, Json};
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
