//! 时间轴管理
//!
//! 维护故事的时间树结构，支持多分支并行。

use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 时间轴节点 (快照在时间树中的位置)
#[derive(Debug, Clone)]
pub struct TimelineNode {
    pub checkpoint_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub children: Vec<Uuid>,
    pub depth: usize,
    pub created_at: DateTime<Utc>,
}

/// 时间轴管理器
pub struct Timeline {
    pub story_id: Uuid,
    pub nodes: HashMap<Uuid, TimelineNode>,
    pub current_id: Option<Uuid>,
    pub root_id: Option<Uuid>,
}

impl Timeline {
    pub fn new(story_id: Uuid) -> Self {
        Self {
            story_id,
            nodes: HashMap::new(),
            current_id: None,
            root_id: None,
        }
    }

    /// 添加快照到时间轴
    pub fn add_checkpoint(&mut self, checkpoint_id: Uuid, parent_id: Option<Uuid>) {
        let depth = parent_id
            .and_then(|pid| self.nodes.get(&pid))
            .map(|p| p.depth + 1)
            .unwrap_or(0);

        let node = TimelineNode {
            checkpoint_id,
            parent_id,
            children: Vec::new(),
            depth,
            created_at: Utc::now(),
        };

        // 更新父节点的子列表
        if let Some(pid) = parent_id {
            if let Some(parent) = self.nodes.get_mut(&pid) {
                parent.children.push(checkpoint_id);
            }
        }

        if self.root_id.is_none() {
            self.root_id = Some(checkpoint_id);
        }

        self.nodes.insert(checkpoint_id, node);
        self.current_id = Some(checkpoint_id);
    }

    /// 获取当前深度
    pub fn current_depth(&self) -> usize {
        self.current_id
            .and_then(|id| self.nodes.get(&id))
            .map(|n| n.depth)
            .unwrap_or(0)
    }
}