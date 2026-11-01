use std::collections::BTreeMap;

use axum::{
    extract::{rejection::QueryRejection, Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePool, SqliteConnection};

use super::handlers::{detail, piece};
use super::models::{Piece, PieceDetail};
use crate::chapter_revisions::{missed, Revision, RevisionError};
use crate::error::Error;

#[derive(Debug, Serialize)]
pub struct ScriptGroup {
    #[serde(flatten)]
    pub piece: Piece,
    pub scenes: Vec<PieceDetail>,
}

#[derive(Debug, Deserialize)]
pub struct ScenePoll {
    pub since: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SceneDeltas {
    Revisions(Vec<Revision>),
    Conflict(&'static str),
}

#[derive(Debug, Serialize)]
pub struct MissedScenes {
    pub children: Vec<i64>,
    pub scenes: BTreeMap<i64, SceneDeltas>,
}

fn parse_since(since: &str) -> Result<BTreeMap<i64, i64>, Error> {
    let invalid = || Error::Validation("since must be <sceneId>:<rev>,...".into());
    let mut scenes = BTreeMap::new();
    for pair in since.split(',').filter(|pair| !pair.is_empty()) {
        let (id, rev) = pair.split_once(':').ok_or_else(invalid)?;
        let id = id.parse().map_err(|_| invalid())?;
        let rev = rev.parse().map_err(|_| invalid())?;
        if scenes.insert(id, rev).is_some() {
            return Err(Error::Validation(format!("scene {id} appears twice")));
        }
    }
    Ok(scenes)
}

async fn script_scenes(
    connection: &mut SqliteConnection,
    group_id: i64,
) -> Result<(Piece, Vec<Piece>), Error> {
    let group = piece(connection, group_id).await?;
    if group.shape != "group" || group.format != "script" {
        return Err(Error::Validation("scenes belong to a script group".into()));
    }
    let group = detail(connection, group).await?;
    Ok((group.piece, group.children.unwrap_or_default()))
}

async fn scene_deltas(
    connection: &mut SqliteConnection,
    scene_id: i64,
    since: i64,
) -> Result<SceneDeltas, Error> {
    let head = sqlx::query_scalar!(
        r#"SELECT head_rev AS "head_rev!" FROM active_pieces WHERE id = ? AND shape = 'leaf'"#,
        scene_id,
    )
    .fetch_optional(&mut *connection)
    .await?
    .ok_or(Error::NotFound)?;
    match missed(connection, scene_id, since, head).await {
        Ok(revisions) => Ok(SceneDeltas::Revisions(revisions)),
        Err(RevisionError::StaleHead) => Ok(SceneDeltas::Conflict("future")),
        Err(RevisionError::IncompleteHistory) => Ok(SceneDeltas::Conflict("unavailable")),
        Err(error) => Err(error.into()),
    }
}

pub async fn get_scenes(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<Json<ScriptGroup>, Error> {
    let mut transaction = pool.begin().await?;
    let (group, children) = script_scenes(&mut transaction, id).await?;
    let mut scenes = Vec::with_capacity(children.len());
    for child in children {
        scenes.push(detail(&mut transaction, child).await?);
    }
    transaction.commit().await?;
    Ok(Json(ScriptGroup {
        piece: group,
        scenes,
    }))
}

pub async fn get_scene_deltas(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    query: Result<Query<ScenePoll>, QueryRejection>,
) -> Result<Json<MissedScenes>, Error> {
    let Query(query) = query.map_err(|error| Error::Validation(error.body_text()))?;
    let requested = parse_since(&query.since)?;
    let mut transaction = pool.begin().await?;
    let (_, children) = script_scenes(&mut transaction, id).await?;
    let children: Vec<i64> = children.iter().map(|child| child.id).collect();
    if let Some(stranger) = requested.keys().find(|id| !children.contains(id)) {
        return Err(Error::Validation(format!(
            "scene {stranger} is not in group {id}"
        )));
    }
    let mut scenes = BTreeMap::new();
    for (scene_id, since) in requested {
        scenes.insert(
            scene_id,
            scene_deltas(&mut transaction, scene_id, since).await?,
        );
    }
    transaction.commit().await?;
    Ok(Json(MissedScenes { children, scenes }))
}
