use sqlx::{SqliteConnection, SqlitePool};

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

pub(crate) async fn restore_universe(
    connection: &mut SqliteConnection,
    id: i64,
) -> Result<(), Error> {
    sqlx::query("UPDATE universes SET deleted_at = NULL, position = COALESCE((SELECT MAX(position) + 1 FROM universes WHERE deleted_at IS NULL), 0) WHERE id = ? AND deleted_at IS NOT NULL")
        .bind(id).execute(connection).await?;
    Ok(())
}

pub(crate) async fn restore_container(
    connection: &mut SqliteConnection,
    id: i64,
) -> Result<(), Error> {
    sqlx::query("UPDATE containers SET deleted_at = NULL, position = COALESCE((SELECT MAX(position) + 1 FROM containers s WHERE s.universe_id = containers.universe_id AND s.deleted_at IS NULL), 0) WHERE id = ? AND deleted_at IS NOT NULL")
        .bind(id).execute(connection).await?;
    Ok(())
}

pub async fn restore(pool: &SqlitePool, id: i64) -> Result<(), Error> {
    let mut transaction = pool.begin().await?;
    let rows = sqlx::query_as::<_, (i64, i64, Option<i64>)>("WITH RECURSIVE ancestors(id, universe_id, container_id, parent_id, depth) AS (SELECT id, universe_id, container_id, parent_id, 0 FROM pieces WHERE id = ? AND id NOT IN (SELECT id FROM active_pieces) UNION ALL SELECT p.id, p.universe_id, p.container_id, p.parent_id, a.depth + 1 FROM pieces p JOIN ancestors a ON p.id = a.parent_id) SELECT id, universe_id, container_id FROM ancestors ORDER BY depth DESC")
        .bind(id).fetch_all(&mut *transaction).await?;
    let Some((_, universe_id, _)) = rows.first() else {
        return Err(Error::NotFound);
    };
    restore_universe(&mut transaction, *universe_id).await?;
    for (piece_id, _, container_id) in rows {
        if let Some(container_id) = container_id {
            restore_container(&mut transaction, container_id).await?;
        }
        sqlx::query("UPDATE pieces SET deleted_at = NULL, position = COALESCE((SELECT MAX(s.position) + 1 FROM active_pieces s WHERE s.universe_id = pieces.universe_id AND s.parent_id IS pieces.parent_id AND s.container_id IS pieces.container_id), 0) WHERE id = ? AND deleted_at IS NOT NULL")
            .bind(piece_id).execute(&mut *transaction).await?;
    }
    transaction.commit().await?;
    Ok(())
}
