//! Qdrant REST 客戶端（向量記憶存儲）

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Qdrant 連線設定
#[derive(Debug, Clone)]
pub struct QdrantConfig {
    pub base_url: String,
    pub collection: String,
}

impl Default for QdrantConfig {
    fn default() -> Self {
        Self {
            base_url: std::env::var("QDRANT_URL")
                .unwrap_or_else(|_| "http://localhost:6333".to_string()),
            collection: std::env::var("QDRANT_COLLECTION")
                .unwrap_or_else(|_| "npc_memories".to_string()),
        }
    }
}

#[derive(Debug, Serialize)]
struct UpsertPoint<'a> {
    id: Uuid,
    vector: &'a [f32],
    payload: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    result: Vec<SearchHit>,
}

#[derive(Debug, Deserialize)]
struct SearchHit {
    id: Uuid,
    score: f64,
    payload: Option<serde_json::Value>,
}

/// Qdrant HTTP 客戶端
pub struct QdrantClient {
    config: QdrantConfig,
    http: reqwest::Client,
}

impl QdrantClient {
    pub fn new(config: QdrantConfig) -> Self {
        Self {
            config,
            http: reqwest::Client::new(),
        }
    }

    /// 寫入向量點
    pub async fn upsert(
        &self,
        id: Uuid,
        vector: &[f32],
        payload: serde_json::Value,
    ) -> Result<(), String> {
        let url = format!(
            "{}/collections/{}/points",
            self.config.base_url, self.config.collection
        );
        let body = serde_json::json!({
            "points": [UpsertPoint { id, vector, payload }]
        });
        self.http
            .put(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 向量相似度搜尋
    pub async fn search(
        &self,
        vector: &[f32],
        limit: usize,
    ) -> Result<Vec<(Uuid, f64, serde_json::Value)>, String> {
        let url = format!(
            "{}/collections/{}/points/search",
            self.config.base_url, self.config.collection
        );
        let body = serde_json::json!({
            "vector": vector,
            "limit": limit,
            "with_payload": true,
        });
        let resp: SearchResponse = self
            .http
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;

        Ok(resp
            .result
            .into_iter()
            .map(|h| (h.id, h.score, h.payload.unwrap_or(serde_json::json!({}))))
            .collect())
    }
}
