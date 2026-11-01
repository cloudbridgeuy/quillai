use sqlx::SqlitePool;

use crate::error::Error;

pub async fn delete(pool: &SqlitePool, id: i64, permanent: bool) -> Result<(), Error> {
    let sql = if permanent {
        "DELETE FROM pieces WHERE id = ?"
    } else {
        "UPDATE pieces SET deleted_at = CURRENT_TIMESTAMP WHERE id = ? AND id IN (SELECT id FROM active_pieces)"
    };
    if sqlx::query(sql)
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected()
        == 0
    {
        return Err(Error::NotFound);
    }
    Ok(())
}

pub async fn restore(pool: &SqlitePool, id: i64) -> Result<(), Error> {
    let result = sqlx::query("UPDATE pieces SET deleted_at = NULL, position = COALESCE((SELECT MAX(s.position) + 1 FROM active_pieces s WHERE s.universe_id = pieces.universe_id AND s.parent_id IS pieces.parent_id AND s.container_id IS pieces.container_id), 0) WHERE id = ? AND deleted_at IS NOT NULL AND (parent_id IS NULL OR parent_id IN (SELECT id FROM active_pieces))")
        .bind(id).execute(pool).await?;
    if result.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    Ok(())
}
