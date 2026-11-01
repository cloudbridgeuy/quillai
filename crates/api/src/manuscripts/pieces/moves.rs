use axum::{
    extract::rejection::JsonRejection, extract::Path, extract::State, http::StatusCode, Json,
};
use sqlx::sqlite::SqlitePool;

use super::handlers::{container_universe, detail, piece};
use super::models::{Attach, Piece, PieceDetail, Placement, Split};
use crate::error::Error;
use crate::manuscripts::{json_input, series::models::validate_position, trigger_rejection};

pub async fn place(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    input: Result<Json<Placement>, JsonRejection>,
) -> Result<Json<PieceDetail>, Error> {
    let input = json_input(input)?;
    validate_position(input.position)?;
    let mut transaction = pool.begin().await?;
    let current = piece(&mut transaction, id).await?;
    if current.parent_id.is_some() && input.container_id.is_some() {
        return Err(Error::Validation("a child detaches to storage only".into()));
    }
    if let Some(container_id) = input.container_id {
        match container_universe(&mut transaction, container_id).await {
            Err(Error::NotFound)
                if sqlx::query_scalar::<_, i64>("SELECT id FROM containers WHERE id = ?")
                    .bind(container_id)
                    .fetch_optional(&mut *transaction)
                    .await?
                    .is_none() =>
            {
                return Err(Error::Validation(
                    "placement container does not exist".into(),
                ));
            }
            result => result?,
        };
    }
    let moved = sqlx::query_as!(
        Piece,
        r#"UPDATE pieces SET container_id = ?, parent_id = NULL, position = ? WHERE id = ? RETURNING id AS "id!", universe_id, container_id, parent_id, title, position, label, format, shape"#,
        input.container_id,
        input.position,
        id,
    )
    .fetch_one(&mut *transaction)
    .await
    .map_err(trigger_rejection)?;
    let detail = detail(&mut transaction, moved).await?;
    transaction.commit().await?;
    Ok(Json(detail))
}

pub async fn split(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    input: Result<Json<Split>, JsonRejection>,
) -> Result<(StatusCode, Json<PieceDetail>), Error> {
    let input = json_input(input)?;
    input.validate()?;
    let mut transaction = pool.begin().await?;
    let leaf = piece(&mut transaction, id).await?;
    if leaf.shape != "leaf" || leaf.parent_id.is_some() {
        return Err(Error::Validation("only a root leaf can be split".into()));
    }
    let group = sqlx::query_as!(
        Piece,
        r#"INSERT INTO pieces (universe_id, container_id, title, position, label, format, shape) VALUES (?, ?, ?, ?, ?, ?, 'group') RETURNING id AS "id!", universe_id, container_id, parent_id, title, position, label, format, shape"#,
        leaf.universe_id,
        leaf.container_id,
        leaf.title,
        leaf.position,
        leaf.label,
        leaf.format,
    )
    .fetch_one(&mut *transaction)
    .await
    .map_err(trigger_rejection)?;
    let label = input.label(&leaf.format);
    sqlx::query!(
        "UPDATE pieces SET container_id = NULL, parent_id = ?, position = 0, label = ? WHERE id = ?",
        group.id,
        label,
        leaf.id,
    )
    .execute(&mut *transaction)
    .await
    .map_err(trigger_rejection)?;
    let detail = detail(&mut transaction, group).await?;
    transaction.commit().await?;
    Ok((StatusCode::CREATED, Json(detail)))
}

pub async fn attach(
    State(pool): State<SqlitePool>,
    Path(group_id): Path<i64>,
    input: Result<Json<Attach>, JsonRejection>,
) -> Result<Json<PieceDetail>, Error> {
    let input = json_input(input)?;
    validate_position(input.position)?;
    let mut transaction = pool.begin().await?;
    let group = piece(&mut transaction, group_id).await?;
    if group.shape != "group" {
        return Err(Error::Validation("attach target must be a group".into()));
    }
    let source = match piece(&mut transaction, input.piece_id).await {
        Err(Error::NotFound)
            if sqlx::query_scalar::<_, i64>("SELECT id FROM pieces WHERE id = ?")
                .bind(input.piece_id)
                .fetch_optional(&mut *transaction)
                .await?
                .is_none() =>
        {
            return Err(Error::Validation("attach source does not exist".into()));
        }
        result => result?,
    };
    if source.shape != "leaf" || source.parent_id.is_some() {
        return Err(Error::Validation(
            "attach source must be a root leaf".into(),
        ));
    }
    sqlx::query!(
        "UPDATE pieces SET container_id = NULL, parent_id = ?, position = ? WHERE id = ?",
        group.id,
        input.position,
        source.id,
    )
    .execute(&mut *transaction)
    .await
    .map_err(trigger_rejection)?;
    let detail = detail(&mut transaction, group).await?;
    transaction.commit().await?;
    Ok(Json(detail))
}
