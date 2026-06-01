//! LLM 生成路由 (灵犀矩阵核心)
//!
//! /api/v1/generate - 多模型并行流式生成

use axum::{
    extract::State,
    response::sse::{Event, Sse},
    routing::post,
    Json, Router,
};
use futures::stream::{self, Stream};
use serde::Deserialize;
use serde_json::{json, Value};
use std::convert::Infallible;
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct GenerateRequest {
    pub story_id: Uuid,
    pub node_id: Option<Uuid>,
    pub prompt: String,
    pub target_providers: Option<Vec<String>>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i32>,
}

/// 注册生成路由
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/stream", post(generate_stream))
        .route("/compare", post(generate_compare))
}

fn llm_client() -> reqwest::Client {
    reqwest::Client::new()
}

/// POST /stream - 多模型并行流式生成 (SSE)
async fn generate_stream(
    State(state): State<AppState>,
    Json(req): Json<GenerateRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let providers = req
        .target_providers
        .clone()
        .unwrap_or_else(|| vec!["openai".to_string(), "anthropic".to_string()]);
    let llm_url = state.config.llm_router_url.clone();
    let story_id = req.story_id;
    let node_id = req.node_id;
    let prompt = req.prompt.clone();
    let temperature = req.temperature.unwrap_or(0.7);
    let max_tokens = req.max_tokens.unwrap_or(2048);

    let stream = stream::iter(providers)
        .then(move |provider| {
            let llm_url = llm_url.clone();
            let prompt = prompt.clone();
            async move {
                let payload = json!({
                    "story_id": story_id,
                    "node_id": node_id,
                    "prompt": prompt,
                    "provider": provider,
                    "temperature": temperature,
                    "max_tokens": max_tokens,
                });

                match llm_client()
                    .post(format!("{}/generate/stream", llm_url))
                    .json(&payload)
                    .send()
                    .await
                {
                    Ok(response) => {
                        let mut byte_stream = response.bytes_stream();
                        let mut events: Vec<Result<Event, Infallible>> = Vec::new();
                        while let Some(chunk) = byte_stream.next().await {
                            if let Ok(bytes) = chunk {
                                let text = String::from_utf8_lossy(&bytes);
                                for line in text.lines() {
                                    let chunk_text = line
                                        .strip_prefix("data: ")
                                        .unwrap_or(line)
                                        .trim();
                                    if chunk_text.is_empty() || chunk_text == "[DONE]" {
                                        continue;
                                    }
                                    events.push(Ok(Event::default().json_data(json!({
                                        "provider": provider,
                                        "text": chunk_text,
                                        "is_final": false,
                                    })).unwrap()));
                                }
                            }
                        }
                        events.push(Ok(Event::default().json_data(json!({
                            "provider": provider,
                            "text": "",
                            "is_final": true,
                        })).unwrap()));
                        events
                    }
                    Err(e) => {
                        vec![Ok(Event::default().json_data(json!({
                            "provider": provider,
                            "error": e.to_string(),
                            "is_final": true,
                        })).unwrap())]
                    }
                }
            }
        })
        .flat_map(stream::iter);

    Sse::new(stream)
}

/// POST /compare - 灵犀矩阵对比生成（轉發 Python /generate/compare）
async fn generate_compare(
    State(state): State<AppState>,
    Json(req): Json<GenerateRequest>,
) -> Result<Json<Value>, sp_common::error::SpError> {
    let providers = req.target_providers.unwrap_or_else(|| {
        vec![
            "openai".to_string(),
            "anthropic".to_string(),
            "local".to_string(),
        ]
    });

    let payload = json!({
        "story_id": req.story_id.to_string(),
        "prompt": req.prompt,
        "providers": providers,
        "temperature": req.temperature.unwrap_or(0.7),
        "max_tokens": req.max_tokens.unwrap_or(2048),
    });

    let response = llm_client()
        .post(format!("{}/generate/compare", state.config.llm_router_url))
        .json(&payload)
        .send()
        .await
        .map_err(|e| sp_common::error::SpError::Http(e))?;

    let body: Value = response
        .json()
        .await
        .map_err(|e| sp_common::error::SpError::Internal(e.to_string()))?;

    Ok(Json(body))
}
