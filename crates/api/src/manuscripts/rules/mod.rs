pub mod catalogue;

use axum::{
    extract::{rejection::JsonRejection, Path, State},
    routing::get,
    Json, Router,
};
use serde_json::{json, Value};
use sqlx::sqlite::SqlitePool;

use crate::error::Error;

pub fn routes(pool: SqlitePool) -> Router {
    Router::new()
        .route("/series/{id}/rules", get(get_rules).put(put_rules))
        .with_state(pool)
}

async fn get_rules(
    State(pool): State<SqlitePool>,
    Path(series_id): Path<i64>,
) -> Result<Json<Value>, Error> {
    let row = sqlx::query!(
        "SELECT language, rules_config FROM containers WHERE id IN (SELECT id FROM legacy_series WHERE id = ?)",
        series_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(Error::NotFound)?;
    let rules: Value = serde_json::from_str(&row.rules_config)
        .map_err(|error| Error::Internal(format!("stored rules_config: {error}")))?;
    Ok(Json(json!({"language": row.language, "rules": rules})))
}

async fn put_rules(
    State(pool): State<SqlitePool>,
    Path(series_id): Path<i64>,
    body: Result<Json<Value>, JsonRejection>,
) -> Result<Json<Value>, Error> {
    let Json(body) = body.map_err(Error::from_json_rejection)?;
    let update = catalogue::parse_update(body, catalogue::CATALOGUE)?;
    let rules = update.rules.map(|rules| Value::Object(rules).to_string());
    let updated = sqlx::query!(
        "UPDATE containers SET language = COALESCE(?, language), rules_config = COALESCE(?, rules_config) WHERE id IN (SELECT id FROM legacy_series WHERE id = ?)",
        update.language,
        rules,
        series_id
    )
    .execute(&pool)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    get_rules(State(pool), Path(series_id)).await
}
