//! 故事 CRUD 路由
//!
//! /api/v1/stories - 故事项目的创建、查询、更新、删除

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, patch, delete},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::middleware::AuthUser;
use crate::services;
use crate::AppState;
use sp_common::error::SpError;

/// 故事请求 DTO
#[derive(Debug, Deserialize)]
pub struct CreateStoryRequest {
    pub title: String,
    pub description: Option<String>,
    pub genre: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStoryRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub genre: Option<Vec<String>>,
    pub world_rules: Option<Value>,
    pub status: Option<String>,
}

/// 注册故事路由
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_story).get(list_stories))
        .route("/:story_id", get(get_story).patch(update_story).delete(delete_story))
        .route("/:story_id/dag", get(get_story_dag))
        // 嵌套节点和边的路由
        .nest("/:story_id/nodes", super::nodes::router())
        .nest("/:story_id/edges", super::edges::router())
}

/// POST /api/v1/stories - 创建故事
async fn create_story(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<CreateStoryRequest>,
) -> Result<(StatusCode, Json<Value>), SpError> {
    let story_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    sqlx::query!(
        r#"
        INSERT INTO stories (id, title, description, author_id, genre, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        story_id,
        req.title,
        req.description,
        auth.0,
        req.genre.as_deref().unwrap_or(&[]),
        now,
        now,
    )
    .execute(&state.db)
    .await
    .map_err(SpError::Database)?;

    tracing::info!("故事已创建: {} - {}", story_id, req.title);

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": story_id,
            "title": req.title,
            "description": req.description,
            "created_at": now.to_rfc3339()
        })),
    ))
}

/// GET /api/v1/stories - 列出故事
async fn list_stories(
    State(state): State<AppState>,
) -> Result<Json<Value>, SpError> {
    let stories = sqlx::query!(
        r#"SELECT id, title, description, genre, status, created_at, updated_at FROM stories ORDER BY created_at DESC LIMIT 50"#
    )
    .fetch_all(&state.db)
    .await
    .map_err(SpError::Database)?;

    let items: Vec<Value> = stories
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "title": s.title,
                "description": s.description,
                "genre": s.genre,
                "status": s.status,
                "created_at": s.created_at.map(|t| t.to_rfc3339()),
                "updated_at": s.updated_at.map(|t| t.to_rfc3339()),
            })
        })
        .collect();

    Ok(Json(json!({ "stories": items, "total": items.len() })))
}

/// GET /api/v1/stories/:story_id - 获取故事详情
async fn get_story(
    State(state): State<AppState>,
    Path(story_id): Path<Uuid>,
) -> Result<Json<Value>, SpError> {
    let story = sqlx::query!(
        r#"SELECT id, title, description, genre, world_rules, mdp_config, status, version, created_at, updated_at FROM stories WHERE id = $1"#,
        story_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(SpError::Database)?;

    match story {
        Some(s) => Ok(Json(json!({
            "id": s.id,
            "title": s.title,
            "description": s.description,
            "genre": s.genre,
            "world_rules": s.world_rules,
            "mdp_config": s.mdp_config,
            "status": s.status,
            "version": s.version,
            "created_at": s.created_at.map(|t| t.to_rfc3339()),
            "updated_at": s.updated_at.map(|t| t.to_rfc3339()),
        }))),
        None => Err(SpError::StoryNotFound(story_id)),
    }
}

/// PATCH /api/v1/stories/:story_id - 更新故事
async fn update_story(
    State(state): State<AppState>,
    Path(story_id): Path<Uuid>,
    Json(req): Json<UpdateStoryRequest>,
) -> Result<Json<Value>, SpError> {
    // 简化实现: 使用 sqlx::query 动态构建
    let now = chrono::Utc::now();

    sqlx::query!(
        r#"
        UPDATE stories 
        SET title = COALESCE($2, title),
            description = COALESCE($3, description),
            genre = COALESCE($4, genre),
            world_rules = COALESCE($5, world_rules),
            status = COALESCE($6, status),
            updated_at = $7
        WHERE id = $1
        "#,
        story_id,
        req.title,
        req.description,
        req.genre.as_deref(),
        req.world_rules,
        req.status,
        now,
    )
    .execute(&state.db)
    .await
    .map_err(SpError::Database)?;

    Ok(Json(json!({ "id": story_id, "updated_at": now.to_rfc3339() })))
}

/// DELETE /api/v1/stories/:story_id - 删除故事
async fn delete_story(
    State(state): State<AppState>,
    Path(story_id): Path<Uuid>,
) -> Result<StatusCode, SpError> {
    sqlx::query!("DELETE FROM stories WHERE id = $1", story_id)
        .execute(&state.db)
        .await
        .map_err(SpError::Database)?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/stories/:story_id/dag - 获取故事的完整 DAG 结构
async fn get_story_dag(
    State(state): State<AppState>,
    Path(story_id): Path<Uuid>,
) -> Result<Json<Value>, SpError> {
    // 查询所有节点
    let nodes = sqlx::query!(
        r#"SELECT id, node_type, title, content, position_x, position_y 
           FROM story_nodes WHERE story_id = $1 ORDER BY created_at"#,
        story_id
    )
    .fetch_all(&state.db)
    .await
    .map_err(SpError::Database)?;

    // 查询所有边
    let edges = sqlx::query!(
        r#"SELECT id, source_node_id, target_node_id, edge_type, probability, observer_weight
           FROM narrative_edges WHERE story_id = $1"#,
        story_id
    )
    .fetch_all(&state.db)
    .await
    .map_err(SpError::Database)?;

    let node_items: Vec<Value> = nodes
        .iter()
        .map(|n| {
            json!({
                "id": n.id,
                "node_type": n.node_type,
                "title": n.title,
                "content": n.content,
                "position_x": n.position_x,
                "position_y": n.position_y,
            })
        })
        .collect();

    let edge_items: Vec<Value> = edges
        .iter()
        .map(|e| {
            json!({
                "id": e.id,
                "source": e.source_node_id,
                "target": e.target_node_id,
                "edge_type": e.edge_type,
                "probability": e.probability,
                "observer_weight": e.observer_weight,
            })
        })
        .collect();

    let dag = services::load_story_dag(&state.db, story_id).await?;
    let topo = dag.topological_sort().ok();

    Ok(Json(json!({
        "story_id": story_id,
        "nodes": node_items,
        "edges": edge_items,
        "node_count": node_items.len(),
        "edge_count": edge_items.len(),
        "topological_order": topo,
        "root_nodes": dag.root_nodes(),
        "leaf_nodes": dag.leaf_nodes(),
    })))
}