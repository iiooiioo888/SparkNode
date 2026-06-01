//! 健康检查路由

use axum::Json;
use serde_json::{json, Value};

/// GET /api/v1/health
pub async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "online",
        "service": "sp-gateway",
        "version": "0.1.0",
        "engine": "灵犀节点 (SparkNode) 高维演算引擎",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}