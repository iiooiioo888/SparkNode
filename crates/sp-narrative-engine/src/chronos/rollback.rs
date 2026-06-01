//! 时间回溯逻辑

use uuid::Uuid;

/// 回溯结果
#[derive(Debug, Clone)]
pub struct RollbackResult {
    pub checkpoint_id: Uuid,
    pub rolled_back_nodes: Vec<Uuid>,
    pub timeline_depth_change: i32,
}

/// 回溯到指定快照
pub fn rollback_to(checkpoint_id: Uuid, current_depth: usize, target_depth: usize) -> RollbackResult {
    RollbackResult {
        checkpoint_id,
        rolled_back_nodes: Vec::new(),
        timeline_depth_change: target_depth as i32 - current_depth as i32,
    }
}