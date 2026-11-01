use sqlx::{types::Json as SqlJson, SqliteConnection};

use super::models::{LoreInput, UniverseLoreEntry};
use crate::error::Error;

pub async fn list(
    connection: &mut SqliteConnection,
    universe_id: i64,
) -> Result<Vec<UniverseLoreEntry>, Error> {
    Ok(sqlx::query_as!(
        UniverseLoreEntry,
        r#"SELECT id AS "id!", universe_id, kind, name, keywords AS "keywords: SqlJson<Vec<String>>", aliases AS "aliases: SqlJson<Vec<String>>", visible_text, hidden_notes, pinned AS "pinned: bool", level FROM lore_entries WHERE universe_id = ? AND deleted_at IS NULL ORDER BY name, id"#,
        universe_id,
    )
    .fetch_all(connection)
    .await?)
}

pub async fn create(
    connection: &mut SqliteConnection,
    universe_id: i64,
    input: LoreInput,
) -> Result<UniverseLoreEntry, Error> {
    let keywords = input.keywords.map(SqlJson);
    let aliases = input.aliases.map(SqlJson);
    Ok(sqlx::query_as!(
        UniverseLoreEntry,
        r#"INSERT INTO lore_entries (universe_id, kind, name, keywords, aliases, visible_text, hidden_notes, pinned, level) VALUES (?, ?, ?, COALESCE(?, '[]'), COALESCE(?, '[]'), COALESCE(?, ''), COALESCE(?, ''), COALESCE(?, 0), COALESCE(?, 'include')) RETURNING id AS "id!", universe_id, kind, name, keywords AS "keywords: SqlJson<Vec<String>>", aliases AS "aliases: SqlJson<Vec<String>>", visible_text, hidden_notes, pinned AS "pinned: bool", level"#,
        universe_id,
        input.kind,
        input.name,
        keywords,
        aliases,
        input.visible_text,
        input.hidden_notes,
        input.pinned,
        input.level,
    )
    .fetch_one(connection)
    .await?)
}

pub async fn get(
    connection: &mut SqliteConnection,
    universe_id: i64,
    lore_id: i64,
) -> Result<UniverseLoreEntry, Error> {
    sqlx::query_as!(
        UniverseLoreEntry,
        r#"SELECT id, universe_id, kind, name, keywords AS "keywords: SqlJson<Vec<String>>", aliases AS "aliases: SqlJson<Vec<String>>", visible_text, hidden_notes, pinned AS "pinned: bool", level FROM lore_entries WHERE universe_id = ? AND id = ? AND deleted_at IS NULL"#,
        universe_id,
        lore_id,
    )
    .fetch_optional(connection)
    .await?
    .ok_or(Error::NotFound)
}

pub async fn update(
    connection: &mut SqliteConnection,
    universe_id: i64,
    lore_id: i64,
    input: LoreInput,
) -> Result<UniverseLoreEntry, Error> {
    let keywords = input.keywords.map(SqlJson);
    let aliases = input.aliases.map(SqlJson);
    sqlx::query_as!(
        UniverseLoreEntry,
        r#"UPDATE lore_entries SET kind = COALESCE(?, kind), name = COALESCE(?, name), keywords = COALESCE(?, keywords), aliases = COALESCE(?, aliases), visible_text = COALESCE(?, visible_text), hidden_notes = COALESCE(?, hidden_notes), pinned = COALESCE(?, pinned), level = COALESCE(?, level) WHERE universe_id = ? AND id = ? AND deleted_at IS NULL RETURNING id, universe_id, kind, name, keywords AS "keywords: SqlJson<Vec<String>>", aliases AS "aliases: SqlJson<Vec<String>>", visible_text, hidden_notes, pinned AS "pinned: bool", level"#,
        input.kind,
        input.name,
        keywords,
        aliases,
        input.visible_text,
        input.hidden_notes,
        input.pinned,
        input.level,
        universe_id,
        lore_id,
    )
    .fetch_optional(connection)
    .await?
    .ok_or(Error::NotFound)
}

pub async fn delete(
    connection: &mut SqliteConnection,
    universe_id: i64,
    lore_id: i64,
) -> Result<(), Error> {
    let result = sqlx::query!(
        "UPDATE lore_entries SET deleted_at = CURRENT_TIMESTAMP WHERE universe_id = ? AND id = ? AND deleted_at IS NULL",
        universe_id,
        lore_id,
    )
    .execute(connection)
    .await?;
    if result.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    Ok(())
}

pub async fn restore(
    connection: &mut SqliteConnection,
    universe_id: i64,
    lore_id: i64,
) -> Result<UniverseLoreEntry, Error> {
    let result = sqlx::query!(
        "UPDATE lore_entries SET deleted_at = NULL WHERE universe_id = ? AND id = ? AND deleted_at IS NOT NULL",
        universe_id,
        lore_id,
    )
    .execute(&mut *connection)
    .await?;
    if result.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    get(connection, universe_id, lore_id).await
}

pub async fn permanent_delete(
    connection: &mut SqliteConnection,
    universe_id: i64,
    lore_id: i64,
) -> Result<(), Error> {
    let result = sqlx::query!(
        "DELETE FROM lore_entries WHERE universe_id = ? AND id = ?",
        universe_id,
        lore_id,
    )
    .execute(connection)
    .await?;
    if result.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    Ok(())
}
