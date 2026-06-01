//! 叙事边 CRUD 路由
//!
//! /api/v1/stories/:story_id/edges

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, patch, delete},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::services::{self, normalized_probability_for_source};
use crate::AppState;
use sp_common::error::SpError;
use sp_common::types::parse_edge_type;

#[derive(Debug, Deserialize)]
pub struct CreateEdgeRequest {
    pub source_node_id: Uuid,
    pub target_node_id: Uuid,
    pub edge_type: String,
    pub probability: Option<f64>,
    pub reward_signal: Option<f64>,
    pub conditions: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEdgeRequest {
    pub probability: Option<f64>,
    pub reward_signal: Option<f64>,
    pub observer_weight: Option<f64>,
    pub conditions: Option<Value>,
}

/// 注册边路由 (嵌套在 stories 下)
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_edge).get(list_edges))
        .route("/:edge_id", get(get_edge).patch(update_edge).delete(delete_edge))
}

/// POST / - 创建叙事边
async fn create_edge(
    State(state): State<AppState>,
    Path(story_id): Path<Uuid>,
    Json(req): Json<CreateEdgeRequest>,
) -> Result<(StatusCode, Json<Value>), SpError> {
    let edge_type = parse_edge_type(&req.edge_type)?;

    services::validate_new_edge(
        &state.db,
        story_id,
        req.source_node_id,
        req.target_node_id,
    )
    .await?;

    let existing = sqlx::query!(
        r#"SELECT id, probability FROM narrative_edges
           WHERE story_id = $1 AND source_node_id = $2"#,
        story_id,
        req.source_node_id
    )
    .fetch_all(&state.db)
    .await
    .map_err(SpError::Database)?;

    let existing_probs: Vec<f64> = existing
        .iter()
        .map(|e| e.probability.unwrap_or(0.0) as f64)
        .collect();
    let requested = req.probability.unwrap_or(1.0);
    let (renormalized_old, new_prob) =
        normalized_probability_for_source(&existing_probs, requested);

    for (row, prob) in existing.iter().zip(renormalized_old.iter()) {
        sqlx::query!(
            r#"UPDATE narrative_edges SET probability = $2 WHERE id = $1"#,
            row.id,
            *prob
        )
        .execute(&state.db)
        .await
        .map_err(SpError::Database)?;
    }

    let edge_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    sqlx::query!(
        r#"
        INSERT INTO narrative_edges (id, story_id, source_node_id, target_node_id, edge_type, probability, reward_signal, conditions, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
        edge_id,
        story_id,
        req.source_node_id,
        req.target_node_id,
        edge_type.as_str(),
        new_prob,
        req.reward_signal.unwrap_or(0.0),
        req.conditions.unwrap_or_else(|| json!([])),
        now,
    )
    .execute(&state.db)
    .await
    .map_err(SpError::Database)?;

    tracing::info!("叙事边已创建: {} → {}", req.source_node_id, req.target_node_id);

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": edge_id,
            "story_id": story_id,
            "source": req.source_node_id,
            "target": req.target_node_id,
            "edge_type": edge_type.as_str(),
            "probability": new_prob,
        })),
    ))
}

/// GET / - 列出故事的所有边
async fn list_edges(
    State(state): State<AppState>,
    Path(story_id): Path<Uuid>,
) -> Result<Json<Value>, SpError> {
    let edges = sqlx::query!(
        r#"SELECT id, source_node_id, target_node_id, edge_type, probability, reward_signal, observer_weight, collapse_count
           FROM narrative_edges WHERE story_id = $1"#,
        story_id
    )
    .fetch_all(&state.db)
    .await
    .map_err(SpError::Database)?;

    let items: Vec<Value> = edges
        .iter()
        .map(|e| {
            json!({
                "id": e.id,
                "source": e.source_node_id,
                "target": e.target_node_id,
                "edge_type": e.edge_type,
                "probability": e.probability,
                "reward_signal": e.reward_signal,
                "observer_weight": e.observer_weight,
                "collapse_count": e.collapse_count,
            })
        })
        .collect();

    Ok(Json(json!({ "edges": items, "total": items.len() })))
}

/// GET /:edge_id - 获取单条边
async fn get_edge(
    State(state): State<AppState>,
    Path((story_id, edge_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>, SpError> {
    let edge = sqlx::query!(
        r#"SELECT id, source_node_id, target_node_id, edge_type, probability, reward_signal, observer_weight, collapse_count, conditions, created_at
           FROM narrative_edges WHERE id = $1 AND story_id = $2"#,
        edge_id,
        story_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(SpError::Database)?;

    match edge {
        Some(e) => Ok(Json(json!({
            "id": e.id,
            "story_id": story_id,
            "source": e.source_node_id,
            "target": e.target_node_id,
            "edge_type": e.edge_type,
            "probability": e.probability,
            "reward_signal": e.reward_signal,
            "observer_weight": e.observer_weight,
            "collapse_count": e.collapse_count,
            "conditions": e.conditions,
            "created_at": e.created_at.map(|t| t.to_rfc3339()),
        }))),
        None => Err(SpError::EdgeNotFound(edge_id)),
    }
}

/// PATCH /:edge_id - 更新边
async fn update_edge(
    State(state): State<AppState>,
    Path((story_id, edge_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateEdgeRequest>,
) -> Result<Json<Value>, SpError> {
    if let Some(new_prob) = req.probability {
        let edge = sqlx::query!(
            r#"SELECT source_node_id FROM narrative_edges WHERE id = $1 AND story_id = $2"#,
            edge_id,
            story_id
        )
        .fetch_optional(&state.db)
        .await
        .map_err(SpError::Database)?;

        if let Some(e) = edge {
            let siblings = sqlx::query!(
                r#"SELECT id, probability FROM narrative_edges
                   WHERE story_id = $1 AND source_node_id = $2"#,
                story_id,
                e.source_node_id
            )
            .fetch_all(&state.db)
            .await
            .map_err(SpError::Database)?;

            let others: Vec<f64> = siblings
                .iter()
                .filter(|s| s.id != edge_id)
                .map(|s| s.probability.unwrap_or(0.0) as f64)
                .collect();
            let (renorm_old, normalized_new) =
                normalized_probability_for_source(&others, new_prob);

            for (s, prob) in siblings
                .iter()
                .filter(|s| s.id != edge_id)
                .zip(renorm_old.iter())
            {
                sqlx::query!(
                    r#"UPDATE narrative_edges SET probability = $2 WHERE id = $1"#,
                    s.id,
                    *prob
                )
                .execute(&state.db)
                .await
                .map_err(SpError::Database)?;
            }

            sqlx::query!(
                r#"UPDATE narrative_edges SET probability = $2 WHERE id = $1"#,
                edge_id,
                normalized_new
            )
            .execute(&state.db)
            .await
            .map_err(SpError::Database)?;
        }
    }

    sqlx::query!(
        r#"
        UPDATE narrative_edges 
        SET reward_signal = COALESCE($3, reward_signal),
            observer_weight = COALESCE($4, observer_weight)
        WHERE id = $1 AND story_id = $2
        "#,
        edge_id,
        story_id,
        req.reward_signal,
        req.observer_weight,
    )
    .execute(&state.db)
    .await
    .map_err(SpError::Database)?;

    Ok(Json(json!({ "id": edge_id })))
}

/// DELETE /:edge_id - 删除边
async fn delete_edge(
    State(state): State<AppState>,
    Path((story_id, edge_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, SpError> {
    sqlx::query!("DELETE FROM narrative_edges WHERE id = $1 AND story_id = $2", edge_id, story_id)
        .execute(&state.db)
        .await
        .map_err(SpError::Database)?;

    Ok(StatusCode::NO_CONTENT)
}