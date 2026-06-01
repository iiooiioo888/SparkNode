//! API 字串與領域枚舉轉換

use crate::error::SpError;
use crate::types::{EdgeType, NodeType};

/// 解析節點類型字串
pub fn parse_node_type(value: &str) -> Result<NodeType, SpError> {
    match value {
        "scene" => Ok(NodeType::Scene),
        "dialogue" => Ok(NodeType::Dialogue),
        "decision" => Ok(NodeType::Decision),
        "combat" => Ok(NodeType::Combat),
        "transition" => Ok(NodeType::Transition),
        "system_event" => Ok(NodeType::SystemEvent),
        other => Err(SpError::Internal(format!("無效的 node_type: {}", other))),
    }
}

/// 解析邊類型字串
pub fn parse_edge_type(value: &str) -> Result<EdgeType, SpError> {
    match value {
        "causal" => Ok(EdgeType::Causal),
        "branch" => Ok(EdgeType::Branch),
        "parallel" => Ok(EdgeType::Parallel),
        "temporal" => Ok(EdgeType::Temporal),
        "conditional" => Ok(EdgeType::Conditional),
        other => Err(SpError::Internal(format!("無效的 edge_type: {}", other))),
    }
}
