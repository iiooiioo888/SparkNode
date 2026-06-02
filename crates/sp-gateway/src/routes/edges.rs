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
use sqlx::Row;
use uuid::Uuid;

use crate::services::{self, normalized_probability_for_source};
use crate::AppState;
use crate::error::SpErrorWrapper;
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
) -> Result<(StatusCode, Json<Value>), SpErrorWrapper> {
    let edge_type = parse_edge_type(&req.edge_type)?;

    services::validate_new_edge(
        &state.db,
        story_id,
        req.source_node_id,
        req.target_node_id,
    )
    .await?;

    let existing = sqlx::query(
        r#"SELECT id, probability FROM narrative_edges
           WHERE story_id = $1 AND source_node_id = $2"#
    )
    .bind(story_id)
    .bind(req.source_node_id)
    .fetch_all(&state.db)
    .await
    .map_err(SpError::Database)?;

    let existing_probs: Vec<f64> = existing
        .iter()
        .map(|e| {
            let prob: Option<f64> = e.get("probability");
            prob.unwrap_or(0.0)
        })
        .collect();
    let requested = req.probability.unwrap_or(1.0);
    let (renormalized_old, new_prob) =
        normalized_probability_for_source(&existing_probs, requested);

    for (row, prob) in existing.iter().zip(renormalized_old.iter()) {
        let row_id: Uuid = row.get("id");
        sqlx::query(
            r#"UPDATE narrative_edges SET probability = $2 WHERE id = $1"#
        )
        .bind(row_id)
        .bind(*prob)
        .execute(&state.db)
        .await
        .map_err(SpError::Database)?;
    }

    let edge_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    sqlx::query(
        r#"
        INSERT INTO narrative_edges (id, story_id, source_node_id, target_node_id, edge_type, probability, reward_signal, conditions, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#
    )
    .bind(edge_id)
    .bind(story_id)
    .bind(req.source_node_id)
    .bind(req.target_node_id)
    .bind(edge_type.as_str())
    .bind(new_prob)
    .bind(req.reward_signal.unwrap_or(0.0))
    .bind(req.conditions.unwrap_or_else(|| json!([])))
    .bind(now)
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
) -> Result<Json<Value>, SpErrorWrapper> {
    let edges = sqlx::query(
        r#"SELECT id, source_node_id, target_node_id, edge_type, probability, reward_signal, observer_weight, collapse_count
           FROM narrative_edges WHERE story_id = $1"#
    )
    .bind(story_id)
    .fetch_all(&state.db)
    .await
    .map_err(SpError::Database)?;

    let items: Vec<Value> = edges
        .iter()
        .map(|e| {
            let id: Uuid = e.get("id");
            let source_node_id: Uuid = e.get("source_node_id");
            let target_node_id: Uuid = e.get("target_node_id");
            let edge_type: String = e.get("edge_type");
            let probability: Option<f64> = e.get("probability");
            let reward_signal: Option<f64> = e.get("reward_signal");
            let observer_weight: Option<f64> = e.get("observer_weight");
            let collapse_count: Option<i32> = e.get("collapse_count");
            json!({
                "id": id,
                "source": source_node_id,
                "target": target_node_id,
                "edge_type": edge_type,
                "probability": probability,
                "reward_signal": reward_signal,
                "observer_weight": observer_weight,
                "collapse_count": collapse_count,
            })
        })
        .collect();

    Ok(Json(json!({ "edges": items, "total": items.len() })))
}

/// GET /:edge_id - 获取单条边
async fn get_edge(
    State(state): State<AppState>,
    Path((story_id, edge_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>, SpErrorWrapper> {
    let edge = sqlx::query(
        r#"SELECT id, source_node_id, target_node_id, edge_type, probability, reward_signal, observer_weight, collapse_count, conditions, created_at
           FROM narrative_edges WHERE id = $1 AND story_id = $2"#
    )
    .bind(edge_id)
    .bind(story_id)
    .fetch_optional(&state.db)
    .await
    .map_err(SpError::Database)?;

    match edge {
        Some(e) => {
            let id: Uuid = e.get("id");
            let source_node_id: Uuid = e.get("source_node_id");
            let target_node_id: Uuid = e.get("target_node_id");
            let edge_type: String = e.get("edge_type");
            let probability: Option<f64> = e.get("probability");
            let reward_signal: Option<f64> = e.get("reward_signal");
            let observer_weight: Option<f64> = e.get("observer_weight");
            let collapse_count: Option<i32> = e.get("collapse_count");
            let conditions: Option<serde_json::Value> = e.get("conditions");
            let created_at: Option<chrono::DateTime<chrono::Utc>> = e.get("created_at");
            Ok(Json(json!({
                "id": id,
                "story_id": story_id,
                "source": source_node_id,
                "target": target_node_id,
                "edge_type": edge_type,
                "probability": probability,
                "reward_signal": reward_signal,
                "observer_weight": observer_weight,
                "collapse_count": collapse_count,
                "conditions": conditions,
                "created_at": created_at.map(|t| t.to_rfc3339()),
            })))
        },
        None => Err(SpError::EdgeNotFound(edge_id).into()),
    }
}

/// PATCH /:edge_id - 更新边
async fn update_edge(
    State(state): State<AppState>,
    Path((story_id, edge_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateEdgeRequest>,
) -> Result<Json<Value>, SpErrorWrapper> {
    if let Some(new_prob) = req.probability {
        let edge = sqlx::query(
            r#"SELECT source_node_id FROM narrative_edges WHERE id = $1 AND story_id = $2"#
        )
        .bind(edge_id)
        .bind(story_id)
        .fetch_optional(&state.db)
        .await
        .map_err(SpError::Database)?;

        if let Some(e) = edge {
            let source_node_id: Uuid = e.get("source_node_id");
            let siblings = sqlx::query(
                r#"SELECT id, probability FROM narrative_edges
                   WHERE story_id = $1 AND source_node_id = $2"#
            )
            .bind(story_id)
            .bind(source_node_id)
            .fetch_all(&state.db)
            .await
            .map_err(SpError::Database)?;

            let others: Vec<f64> = siblings
                .iter()
                .filter(|s| {
                    let s_id: Uuid = s.get("id");
                    s_id != edge_id
                })
                .map(|s| {
                    let prob: Option<f64> = s.get("probability");
                    prob.unwrap_or(0.0)
                })
                .collect();
            let (renorm_old, normalized_new) =
                normalized_probability_for_source(&others, new_prob);

            for (s, prob) in siblings
                .iter()
                .filter(|s| {
                    let s_id: Uuid = s.get("id");
                    s_id != edge_id
                })
                .zip(renorm_old.iter())
            {
                let s_id: Uuid = s.get("id");
                sqlx::query(
                    r#"UPDATE narrative_edges SET probability = $2 WHERE id = $1"#
                )
                .bind(s_id)
                .bind(*prob)
                .execute(&state.db)
                .await
                .map_err(SpError::Database)?;
            }

            sqlx::query(
                r#"UPDATE narrative_edges SET probability = $2 WHERE id = $1"#
            )
            .bind(edge_id)
            .bind(normalized_new)
            .execute(&state.db)
            .await
            .map_err(SpError::Database)?;
        }
    }

    sqlx::query(
        r#"
        UPDATE narrative_edges 
        SET reward_signal = COALESCE($3, reward_signal),
            observer_weight = COALESCE($4, observer_weight)
        WHERE id = $1 AND story_id = $2
        "#
    )
    .bind(edge_id)
    .bind(story_id)
    .bind(req.reward_signal)
    .bind(req.observer_weight)
    .execute(&state.db)
    .await
    .map_err(SpError::Database)?;

    Ok(Json(json!({ "id": edge_id })))
}

/// DELETE /:edge_id - 删除边
async fn delete_edge(
    State(state): State<AppState>,
    Path((story_id, edge_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, SpErrorWrapper> {
    sqlx::query("DELETE FROM narrative_edges WHERE id = $1 AND story_id = $2")
        .bind(edge_id)
        .bind(story_id)
        .execute(&state.db)
        .await
        .map_err(SpError::Database)?;

    Ok(StatusCode::NO_CONTENT)
}