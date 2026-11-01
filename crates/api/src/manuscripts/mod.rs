pub mod books;
pub mod chapters;
pub mod containers;
pub mod leaf_sync;
pub mod lore;
pub mod pieces;
pub mod series;
pub mod trash;
pub mod universes;

use axum::{extract::rejection::JsonRejection, Json, Router};
use sqlx::sqlite::SqlitePool;

use crate::error::Error;

pub fn routes(pool: SqlitePool) -> Router {
    series::routes(pool.clone())
        .merge(books::routes(pool.clone()))
        .merge(lore::routes(pool.clone()))
        .merge(chapters::routes(pool.clone()))
        .merge(universes::routes(pool.clone()))
        .merge(containers::routes(pool.clone()))
        .merge(pieces::routes(pool.clone()))
        .merge(trash::routes(pool))
}

pub(crate) fn json_input<T>(input: Result<Json<T>, JsonRejection>) -> Result<T, Error> {
    let Json(input) = input.map_err(|rejection| match rejection {
        JsonRejection::JsonDataError(error) => Error::Validation(error.body_text()),
        rejection => Error::from_json_rejection(rejection),
    })?;
    Ok(input)
}

pub(crate) fn reject_keys(
    extra: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Result<(), Error> {
    match keys.iter().find(|key| extra.contains_key(**key)) {
        Some(key) => Err(Error::Validation(format!(
            "{key} cannot be updated through PUT"
        ))),
        None => Ok(()),
    }
}

pub(crate) fn trigger_rejection(error: sqlx::Error) -> Error {
    match error.as_database_error() {
        Some(database) if database.code().as_deref() == Some("1811") => {
            Error::Validation(database.message().to_owned())
        }
        _ => Error::Sqlx(error),
    }
}
