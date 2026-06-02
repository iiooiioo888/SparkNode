//! 觀察者坍縮 API

use axum::{
    extract::{Path, State},
    routing::post,
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::services;
use crate::AppState;
use crate::error::SpErrorWrapper;
use sp_common::error::SpError;
use sp_common::types::ObserverSignal;

#[derive(Debug, Deserialize)]
pub struct CollapseRequest {
    pub source_node_id: Uuid,
    pub signal: ObserverSignal,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/:story_id/collapse", post(observer_collapse))
}

async fn observer_collapse(
    State(state): State<AppState>,
    Path(story_id): Path<Uuid>,
    Json(req): Json<CollapseRequest>,
) -> Result<Json<Value>, SpErrorWrapper> {
    let shifts = services::apply_collapse(
        &state.db,
        story_id,
        req.source_node_id,
        &req.signal,
    )
    .await?;

    let items: Vec<Value> = shifts
        .iter()
        .map(|(edge_id, prob)| {
            json!({
                "edge_id": edge_id,
                "probability_after": prob,
            })
        })
        .collect();

    Ok(Json(json!({
        "story_id": story_id,
        "source_node_id": req.source_node_id,
        "success": true,
        "shifts": items,
    })))
}
