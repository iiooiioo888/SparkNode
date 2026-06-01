//! 记忆存储与检索

use super::qdrant::{QdrantClient, QdrantConfig};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 记忆检索结果
#[derive(Debug, Clone)]
pub struct MemoryRecallResult {
    pub memory_id: Uuid,
    pub content: String,
    pub strength: f64,
    pub relevance_score: f64,
    pub emotional_valence: f64,
}

/// 海马体记忆管理器
pub struct Hippocampus {
    pub npc_id: Uuid,
    qdrant: QdrantClient,
}

impl Hippocampus {
    pub fn new(npc_id: Uuid) -> Self {
        Self {
            npc_id,
            qdrant: QdrantClient::new(QdrantConfig::default()),
        }
    }

    /// 语义检索 (基于向量相似度)
    pub async fn recall(&self, _query: &str, top_k: usize) -> Vec<MemoryRecallResult> {
        // 佔位向量：正式環境應替換為 Embedding API 輸出
        let query_vector = vec![0.0_f32; 384];
        match self.qdrant.search(&query_vector, top_k).await {
            Ok(hits) => hits
                .into_iter()
                .map(|(id, score, payload)| MemoryRecallResult {
                    memory_id: id,
                    content: payload
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    strength: payload
                        .get("strength")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(1.0),
                    relevance_score: score,
                    emotional_valence: payload
                        .get("emotional_valence")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0),
                })
                .collect(),
            Err(e) => {
                tracing::warn!("Qdrant 檢索失敗: {}", e);
                Vec::new()
            }
        }
    }

    /// 编码新记忆 (写入向量数据库)
    pub async fn encode(&self, content: &str, memory_type: &str) -> Uuid {
        let id = Uuid::new_v4();
        let vector = vec![0.0_f32; 384];
        let payload = serde_json::json!({
            "npc_id": self.npc_id,
            "content": content,
            "memory_type": memory_type,
            "strength": 1.0,
            "emotional_valence": 0.0,
        });
        if let Err(e) = self.qdrant.upsert(id, &vector, payload).await {
            tracing::warn!("Qdrant 寫入失敗: {}", e);
        }
        id
    }
}