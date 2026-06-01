//! 世界状态快照

use uuid::Uuid;
use serde_json::Value;

/// 快照创建参数
pub struct CheckpointParams {
    pub story_id: Uuid,
    pub node_id: Option<Uuid>,
    pub checkpoint_type: String,
    pub world_state: Value,
}

/// 创建世界状态快照
pub fn create_checkpoint(params: CheckpointParams) -> Value {
    serde_json::json!({
        "id": Uuid::new_v4(),
        "story_id": params.story_id,
        "node_id": params.node_id,
        "checkpoint_type": params.checkpoint_type,
        "world_state": params.world_state,
        "created_at": chrono::Utc::now().to_rfc3339()
    })
}