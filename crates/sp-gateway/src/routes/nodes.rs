//! 叙事节点 CRUD 路由
//!
//! /api/v1/stories/:story_id/nodes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, patch, delete},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

use crate::AppState;
use crate::error::SpErrorWrapper;
use sp_common::error::SpError;
use sp_common::types::parse_node_type;

#[derive(Debug, Deserialize)]
pub struct CreateNodeRequest {
    pub node_type: String,
    pub title: Option<String>,
    pub content: Option<String>,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNodeRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
    pub metadata: Option<Value>,
}

/// 注册节点路由 (嵌套在 stories 下)
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_node).get(list_nodes))
        .route("/:node_id", get(get_node).patch(update_node).delete(delete_node))
}

/// POST / - 创建叙事节点
async fn create_node(
    State(state): State<AppState>,
    Path(story_id): Path<Uuid>,
    Json(req): Json<CreateNodeRequest>,
) -> Result<(StatusCode, Json<Value>), SpErrorWrapper> {
    let node_type = parse_node_type(&req.node_type)?;
    let node_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    sqlx::query(
        r#"
        INSERT INTO story_nodes (id, story_id, node_type, title, content, position_x, position_y, metadata, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#
    )
    .bind(node_id)
    .bind(story_id)
    .bind(node_type.as_str())
    .bind(&req.title)
    .bind(&req.content)
    .bind(req.position_x.unwrap_or(0.0))
    .bind(req.position_y.unwrap_or(0.0))
    .bind(req.metadata.unwrap_or_else(|| json!({})))
    .bind(now)
    .bind(now)
    .execute(&state.db)
    .await
    .map_err(SpError::Database)?;

    tracing::info!("叙事节点已创建: {} (故事: {})", node_id, story_id);

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": node_id,
            "story_id": story_id,
            "node_type": node_type.as_str(),
            "title": req.title,
            "created_at": now.to_rfc3339()
        })),
    ))
}

/// GET / - 列出故事的所有节点
async fn list_nodes(
    State(state): State<AppState>,
    Path(story_id): Path<Uuid>,
) -> Result<Json<Value>, SpErrorWrapper> {
    let nodes = sqlx::query(
        r#"SELECT id, node_type, title, content, position_x, position_y, version, created_at, updated_at
           FROM story_nodes WHERE story_id = $1 ORDER BY created_at"#
    )
    .bind(story_id)
    .fetch_all(&state.db)
    .await
    .map_err(SpError::Database)?;

    let items: Vec<Value> = nodes
        .iter()
        .map(|n| {
            let id: Uuid = n.get("id");
            let node_type: String = n.get("node_type");
            let title: Option<String> = n.get("title");
            let content: Option<String> = n.get("content");
            let position_x: Option<f64> = n.get("position_x");
            let position_y: Option<f64> = n.get("position_y");
            let version: Option<i32> = n.get("version");
            let created_at: Option<chrono::DateTime<chrono::Utc>> = n.get("created_at");
            let updated_at: Option<chrono::DateTime<chrono::Utc>> = n.get("updated_at");
            json!({
                "id": id,
                "node_type": node_type,
                "title": title,
                "content": content,
                "position_x": position_x,
                "position_y": position_y,
                "version": version,
                "created_at": created_at.map(|t| t.to_rfc3339()),
                "updated_at": updated_at.map(|t| t.to_rfc3339()),
            })
        })
        .collect();

    Ok(Json(json!({ "nodes": items, "total": items.len() })))
}

/// GET /:node_id - 获取单个节点
async fn get_node(
    State(state): State<AppState>,
    Path((story_id, node_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>, SpErrorWrapper> {
    let node = sqlx::query(
        r#"SELECT id, node_type, title, content, position_x, position_y, metadata, world_snapshot,
                  llm_provider, llm_prompt, llm_tokens_used, version, created_at, updated_at
           FROM story_nodes WHERE id = $1 AND story_id = $2"#
    )
    .bind(node_id)
    .bind(story_id)
    .fetch_optional(&state.db)
    .await
    .map_err(SpError::Database)?;

    match node {
        Some(n) => {
            let id: Uuid = n.get("id");
            let node_type: String = n.get("node_type");
            let title: Option<String> = n.get("title");
            let content: Option<String> = n.get("content");
            let position_x: Option<f64> = n.get("position_x");
            let position_y: Option<f64> = n.get("position_y");
            let metadata: Option<serde_json::Value> = n.get("metadata");
            let world_snapshot: Option<serde_json::Value> = n.get("world_snapshot");
            let llm_provider: Option<String> = n.get("llm_provider");
            let llm_tokens_used: Option<i32> = n.get("llm_tokens_used");
            let version: Option<i32> = n.get("version");
            let created_at: Option<chrono::DateTime<chrono::Utc>> = n.get("created_at");
            let updated_at: Option<chrono::DateTime<chrono::Utc>> = n.get("updated_at");
            Ok(Json(json!({
                "id": id,
                "story_id": story_id,
                "node_type": node_type,
                "title": title,
                "content": content,
                "position_x": position_x,
                "position_y": position_y,
                "metadata": metadata,
                "world_snapshot": world_snapshot,
                "llm_provider": llm_provider,
                "llm_tokens_used": llm_tokens_used,
                "version": version,
                "created_at": created_at.map(|t| t.to_rfc3339()),
                "updated_at": updated_at.map(|t| t.to_rfc3339()),
            })))
        },
        None => Err(SpError::NodeNotFound(node_id).into()),
    }
}

/// PATCH /:node_id - 更新节点
async fn update_node(
    State(state): State<AppState>,
    Path((story_id, node_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateNodeRequest>,
) -> Result<Json<Value>, SpErrorWrapper> {
    let now = chrono::Utc::now();

    sqlx::query(
        r#"
        UPDATE story_nodes 
        SET title = COALESCE($3, title),
            content = COALESCE($4, content),
            position_x = COALESCE($5, position_x),
            position_y = COALESCE($6, position_y),
            updated_at = $7,
            version = version + 1
        WHERE id = $1 AND story_id = $2
        "#
    )
    .bind(node_id)
    .bind(story_id)
    .bind(&req.title)
    .bind(&req.content)
    .bind(req.position_x)
    .bind(req.position_y)
    .bind(now)
    .execute(&state.db)
    .await
    .map_err(SpError::Database)?;

    Ok(Json(json!({ "id": node_id, "updated_at": now.to_rfc3339() })))
}

/// DELETE /:node_id - 删除节点 (级联删除关联边)
async fn delete_node(
    State(state): State<AppState>,
    Path((story_id, node_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, SpErrorWrapper> {
    // 先删除关联边
    sqlx::query(
        "DELETE FROM narrative_edges WHERE story_id = $1 AND (source_node_id = $2 OR target_node_id = $2)"
    )
    .bind(story_id)
    .bind(node_id)
    .execute(&state.db)
    .await
    .map_err(SpError::Database)?;

    // 再删除节点
    sqlx::query("DELETE FROM story_nodes WHERE id = $1 AND story_id = $2")
        .bind(node_id)
        .bind(story_id)
        .execute(&state.db)
        .await
        .map_err(SpError::Database)?;

    Ok(StatusCode::NO_CONTENT)
}