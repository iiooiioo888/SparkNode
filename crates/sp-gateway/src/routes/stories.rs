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
use sqlx::Row;
use uuid::Uuid;

use crate::middleware::AuthUser;
use crate::services;
use crate::AppState;
use crate::error::SpErrorWrapper;
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
) -> Result<(StatusCode, Json<Value>), SpErrorWrapper> {
    let story_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    sqlx::query(
        r#"
        INSERT INTO stories (id, title, description, author_id, genre, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#
    )
    .bind(story_id)
    .bind(&req.title)
    .bind(&req.description)
    .bind(auth.0)
    .bind(req.genre.as_deref().unwrap_or(&[]))
    .bind(now)
    .bind(now)
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
) -> Result<Json<Value>, SpErrorWrapper> {
    let stories = sqlx::query(
        r#"SELECT id, title, description, genre, status, created_at, updated_at FROM stories ORDER BY created_at DESC LIMIT 50"#
    )
    .fetch_all(&state.db)
    .await
    .map_err(SpError::Database)?;

    let items: Vec<Value> = stories
        .iter()
        .map(|s| {
            let id: Uuid = s.get("id");
            let title: String = s.get("title");
            let description: Option<String> = s.get("description");
            let genre: Option<Vec<String>> = s.get("genre");
            let status: Option<String> = s.get("status");
            let created_at: Option<chrono::DateTime<chrono::Utc>> = s.get("created_at");
            let updated_at: Option<chrono::DateTime<chrono::Utc>> = s.get("updated_at");
            json!({
                "id": id,
                "title": title,
                "description": description,
                "genre": genre,
                "status": status,
                "created_at": created_at.map(|t| t.to_rfc3339()),
                "updated_at": updated_at.map(|t| t.to_rfc3339()),
            })
        })
        .collect();

    Ok(Json(json!({ "stories": items, "total": items.len() })))
}

/// GET /api/v1/stories/:story_id - 获取故事详情
async fn get_story(
    State(state): State<AppState>,
    Path(story_id): Path<Uuid>,
) -> Result<Json<Value>, SpErrorWrapper> {
    let story = sqlx::query(
        r#"SELECT id, title, description, genre, world_rules, mdp_config, status, version, created_at, updated_at FROM stories WHERE id = $1"#
    )
    .bind(story_id)
    .fetch_optional(&state.db)
    .await
    .map_err(SpError::Database)?;

    match story {
        Some(s) => {
            let id: Uuid = s.get("id");
            let title: String = s.get("title");
            let description: Option<String> = s.get("description");
            let genre: Option<Vec<String>> = s.get("genre");
            let world_rules: Option<serde_json::Value> = s.get("world_rules");
            let mdp_config: Option<serde_json::Value> = s.get("mdp_config");
            let status: Option<String> = s.get("status");
            let version: Option<i32> = s.get("version");
            let created_at: Option<chrono::DateTime<chrono::Utc>> = s.get("created_at");
            let updated_at: Option<chrono::DateTime<chrono::Utc>> = s.get("updated_at");
            Ok(Json(json!({
                "id": id,
                "title": title,
                "description": description,
                "genre": genre,
                "world_rules": world_rules,
                "mdp_config": mdp_config,
                "status": status,
                "version": version,
                "created_at": created_at.map(|t| t.to_rfc3339()),
                "updated_at": updated_at.map(|t| t.to_rfc3339()),
            })))
        },
        None => Err(SpError::StoryNotFound(story_id).into()),
    }
}

/// PATCH /api/v1/stories/:story_id - 更新故事
async fn update_story(
    State(state): State<AppState>,
    Path(story_id): Path<Uuid>,
    Json(req): Json<UpdateStoryRequest>,
) -> Result<Json<Value>, SpErrorWrapper> {
    // 简化实现: 使用 sqlx::query 动态构建
    let now = chrono::Utc::now();

    sqlx::query(
        r#"
        UPDATE stories 
        SET title = COALESCE($2, title),
            description = COALESCE($3, description),
            genre = COALESCE($4, genre),
            world_rules = COALESCE($5, world_rules),
            status = COALESCE($6, status),
            updated_at = $7
        WHERE id = $1
        "#
    )
    .bind(story_id)
    .bind(&req.title)
    .bind(&req.description)
    .bind(req.genre.as_deref())
    .bind(&req.world_rules)
    .bind(&req.status)
    .bind(now)
    .execute(&state.db)
    .await
    .map_err(SpError::Database)?;

    Ok(Json(json!({ "id": story_id, "updated_at": now.to_rfc3339() })))
}

/// DELETE /api/v1/stories/:story_id - 删除故事
async fn delete_story(
    State(state): State<AppState>,
    Path(story_id): Path<Uuid>,
) -> Result<StatusCode, SpErrorWrapper> {
    sqlx::query("DELETE FROM stories WHERE id = $1")
        .bind(story_id)
        .execute(&state.db)
        .await
        .map_err(SpError::Database)?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/stories/:story_id/dag - 获取故事的完整 DAG 结构
async fn get_story_dag(
    State(state): State<AppState>,
    Path(story_id): Path<Uuid>,
) -> Result<Json<Value>, SpErrorWrapper> {
    // 查询所有节点
    let nodes = sqlx::query(
        r#"SELECT id, node_type, title, content, position_x, position_y 
           FROM story_nodes WHERE story_id = $1 ORDER BY created_at"#
    )
    .bind(story_id)
    .fetch_all(&state.db)
    .await
    .map_err(SpError::Database)?;

    // 查询所有边
    let edges = sqlx::query(
        r#"SELECT id, source_node_id, target_node_id, edge_type, probability, observer_weight
           FROM narrative_edges WHERE story_id = $1"#
    )
    .bind(story_id)
    .fetch_all(&state.db)
    .await
    .map_err(SpError::Database)?;

    let node_items: Vec<Value> = nodes
        .iter()
        .map(|n| {
            let id: Uuid = n.get("id");
            let node_type: String = n.get("node_type");
            let title: Option<String> = n.get("title");
            let content: Option<String> = n.get("content");
            let position_x: Option<f64> = n.get("position_x");
            let position_y: Option<f64> = n.get("position_y");
            json!({
                "id": id,
                "node_type": node_type,
                "title": title,
                "content": content,
                "position_x": position_x,
                "position_y": position_y,
            })
        })
        .collect();

    let edge_items: Vec<Value> = edges
        .iter()
        .map(|e| {
            let id: Uuid = e.get("id");
            let source_node_id: Uuid = e.get("source_node_id");
            let target_node_id: Uuid = e.get("target_node_id");
            let edge_type: String = e.get("edge_type");
            let probability: Option<f64> = e.get("probability");
            let observer_weight: Option<f64> = e.get("observer_weight");
            json!({
                "id": id,
                "source": source_node_id,
                "target": target_node_id,
                "edge_type": edge_type,
                "probability": probability,
                "observer_weight": observer_weight,
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