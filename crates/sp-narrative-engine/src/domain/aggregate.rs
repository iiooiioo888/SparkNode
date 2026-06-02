//! CQRS 聚合根 — NarrativeGraph
//!
//! `NarrativeGraph` 是叙事引擎的聚合根（Aggregate Root），
//! 封装了 DAG 图结构 + MDP 概率矩阵的内存状态。
//!
//! 所有状态变更必须通过 `handle_command` 方法：
//! 1. 接收 Command
//! 2. 执行业务验证（环路检测、概率校验等）
//! 3. 产出不可变的 Event 列表
//!
//! 状态回放通过 `apply_event` / `replay` 方法实现。

use std::collections::HashMap;
use uuid::Uuid;

use crate::dag::graph::{DagEdge, DagNode, DirectedAcyclicGraph};
use super::commands::*;
use super::events::*;

/// 叙事图聚合根
///
/// 包装现有 `DirectedAcyclicGraph` 内存结构，
/// 提供 CQRS Command → Event → State 的完整流程。
#[derive(Debug, Clone)]
pub struct NarrativeGraph {
    /// 所属故事 ID
    pub story_id: Uuid,
    /// 内部 DAG 图结构（复用现有实现）
    pub dag: DirectedAcyclicGraph,
    /// MDP 概率矩阵缓存: edge_id → probability
    pub probability_cache: HashMap<Uuid, f64>,
    /// 当前版本号（每次成功处理 Command 递增）
    pub version: u64,
}

impl NarrativeGraph {
    /// 创建空的叙事图
    pub fn new(story_id: Uuid) -> Self {
        Self {
            story_id,
            dag: DirectedAcyclicGraph::new(),
            probability_cache: HashMap::new(),
            version: 0,
        }
    }

    /// 从事件流重建聚合根
    ///
    /// 按时间顺序回放所有 Event，重建到最新状态。
    /// 用于 Event Sourcing 场景下的状态恢复。
    pub fn replay(story_id: Uuid, events: &[NarrativeEvent]) -> Self {
        let mut graph = Self::new(story_id);
        for event in events {
            graph.apply_event(event);
        }
        graph
    }

    // ═══════════════════════════════════════════════
    //  Command Handler（写入端核心）
    // ═══════════════════════════════════════════════

    /// 处理命令，产出事件
    ///
    /// 这是聚合根的唯一入口。所有外部修改请求
    /// 必须封装为 `NarrativeCommand` 并通过此方法下达。
    ///
    /// # 返回
    /// - `Ok(events)`: 产出的不可变事件列表（可能为空）
    /// - `Err(e)`: 业务验证失败
    pub fn handle_command(
        &self,
        command: NarrativeCommand,
        actor_id: Uuid,
    ) -> Result<Vec<NarrativeEvent>, AggregateError> {
        match command {
            NarrativeCommand::CreateNode(cmd) => self.handle_create_node(cmd, actor_id),
            NarrativeCommand::UpdateNode(cmd) => self.handle_update_node(cmd, actor_id),
            NarrativeCommand::DeleteNode(cmd) => self.handle_delete_node(cmd, actor_id),
            NarrativeCommand::LinkEdge(cmd) => self.handle_link_edge(cmd, actor_id),
            NarrativeCommand::UpdateEdge(cmd) => self.handle_update_edge(cmd, actor_id),
            NarrativeCommand::DeleteEdge(cmd) => self.handle_delete_edge(cmd, actor_id),
            NarrativeCommand::CollapseBranch(cmd) => self.handle_collapse(cmd, actor_id),
            NarrativeCommand::CreateCheckpoint(cmd) => self.handle_checkpoint(cmd, actor_id),
            NarrativeCommand::Rollback(cmd) => self.handle_rollback(cmd, actor_id),
        }
    }

    // ── 节点命令处理 ──

    fn handle_create_node(
        &self,
        cmd: CreateNodeCommand,
        actor_id: Uuid,
    ) -> Result<Vec<NarrativeEvent>, AggregateError> {
        // 业务验证
        if cmd.story_id != self.story_id {
            return Err(AggregateError::StoryMismatch {
                expected: self.story_id,
                got: cmd.story_id,
            });
        }

        if cmd.node_type.is_empty() {
            return Err(AggregateError::ValidationError("node_type 不能为空".into()));
        }

        // 节点类型合法性校验
        let valid_types = ["scene", "dialogue", "decision", "combat", "transition", "system_event"];
        if !valid_types.contains(&cmd.node_type.as_str()) {
            return Err(AggregateError::ValidationError(format!(
                "无效的 node_type: {}，合法值: {:?}",
                cmd.node_type, valid_types
            )));
        }

        let node_id = Uuid::new_v4();
        let meta = EventMetadata::new(self.story_id, actor_id);

        Ok(vec![NarrativeEvent::NodeCreated {
            meta,
            node_id,
            node_type: cmd.node_type,
            title: cmd.title,
            content: cmd.content,
            position_x: cmd.position_x,
            position_y: cmd.position_y,
            metadata: cmd.metadata.unwrap_or_default(),
        }])
    }

    fn handle_update_node(
        &self,
        cmd: UpdateNodeCommand,
        actor_id: Uuid,
    ) -> Result<Vec<NarrativeEvent>, AggregateError> {
        // 验证节点存在
        if !self.dag.nodes.contains_key(&cmd.node_id) {
            return Err(AggregateError::NodeNotFound(cmd.node_id));
        }

        let meta = EventMetadata::new(self.story_id, actor_id);
        Ok(vec![NarrativeEvent::NodeUpdated {
            meta,
            node_id: cmd.node_id,
            title: cmd.title,
            content: cmd.content,
            position_x: cmd.position_x,
            position_y: cmd.position_y,
            metadata: cmd.metadata,
        }])
    }

    fn handle_delete_node(
        &self,
        cmd: DeleteNodeCommand,
        actor_id: Uuid,
    ) -> Result<Vec<NarrativeEvent>, AggregateError> {
        // 验证节点存在
        if !self.dag.nodes.contains_key(&cmd.node_id) {
            return Err(AggregateError::NodeNotFound(cmd.node_id));
        }

        // 收集级联删除的边 ID
        let mut cascaded_edge_ids = Vec::new();
        if let Some(out_edges) = self.dag.out_edges.get(&cmd.node_id) {
            cascaded_edge_ids.extend(out_edges);
        }
        if let Some(in_edges) = self.dag.in_edges.get(&cmd.node_id) {
            cascaded_edge_ids.extend(in_edges);
        }

        let meta = EventMetadata::new(self.story_id, actor_id);
        Ok(vec![NarrativeEvent::NodeDeleted {
            meta,
            node_id: cmd.node_id,
            cascaded_edge_ids,
        }])
    }

    // ── 边命令处理 ──

    fn handle_link_edge(
        &self,
        cmd: LinkEdgeCommand,
        actor_id: Uuid,
    ) -> Result<Vec<NarrativeEvent>, AggregateError> {
        // 验证源节点和目标节点存在
        if !self.dag.nodes.contains_key(&cmd.source_node_id) {
            return Err(AggregateError::NodeNotFound(cmd.source_node_id));
        }
        if !self.dag.nodes.contains_key(&cmd.target_node_id) {
            return Err(AggregateError::NodeNotFound(cmd.target_node_id));
        }

        // 环路检测：添加此边是否会导致 DAG 变成有环图
        if self.dag.would_create_cycle(cmd.source_node_id, cmd.target_node_id) {
            return Err(AggregateError::CycleDetected(
                cmd.source_node_id.to_string(),
                cmd.target_node_id.to_string(),
            ));
        }

        // 概率校验：[0, 1] 范围
        if cmd.probability < 0.0 || cmd.probability > 1.0 {
            return Err(AggregateError::InvalidProbability(cmd.probability));
        }

        // 边类型合法性校验
        let valid_edge_types = ["causal", "branch", "parallel", "temporal", "conditional"];
        if !valid_edge_types.contains(&cmd.edge_type.as_str()) {
            return Err(AggregateError::ValidationError(format!(
                "无效的 edge_type: {}，合法值: {:?}",
                cmd.edge_type, valid_edge_types
            )));
        }

        let edge_id = Uuid::new_v4();
        let meta = EventMetadata::new(self.story_id, actor_id);

        Ok(vec![NarrativeEvent::EdgeLinked {
            meta,
            edge_id,
            source_node_id: cmd.source_node_id,
            target_node_id: cmd.target_node_id,
            edge_type: cmd.edge_type,
            probability: cmd.probability,
            reward_signal: cmd.reward_signal.unwrap_or(0.0),
            conditions: cmd.conditions.unwrap_or_default(),
        }])
    }

    fn handle_update_edge(
        &self,
        cmd: UpdateEdgeCommand,
        actor_id: Uuid,
    ) -> Result<Vec<NarrativeEvent>, AggregateError> {
        // 验证边存在
        if !self.dag.edges.contains_key(&cmd.edge_id) {
            return Err(AggregateError::EdgeNotFound(cmd.edge_id));
        }

        // 概率校验
        if let Some(p) = cmd.probability {
            if p < 0.0 || p > 1.0 {
                return Err(AggregateError::InvalidProbability(p));
            }
        }

        let meta = EventMetadata::new(self.story_id, actor_id);
        Ok(vec![NarrativeEvent::EdgeUpdated {
            meta,
            edge_id: cmd.edge_id,
            probability: cmd.probability,
            reward_signal: cmd.reward_signal,
            observer_weight: cmd.observer_weight,
        }])
    }

    fn handle_delete_edge(
        &self,
        cmd: DeleteEdgeCommand,
        actor_id: Uuid,
    ) -> Result<Vec<NarrativeEvent>, AggregateError> {
        if !self.dag.edges.contains_key(&cmd.edge_id) {
            return Err(AggregateError::EdgeNotFound(cmd.edge_id));
        }

        let meta = EventMetadata::new(self.story_id, actor_id);
        Ok(vec![NarrativeEvent::EdgeDeleted {
            meta,
            edge_id: cmd.edge_id,
        }])
    }

    // ── 观察者坍缩处理 ──

    fn handle_collapse(
        &self,
        cmd: CollapseBranchCommand,
        actor_id: Uuid,
    ) -> Result<Vec<NarrativeEvent>, AggregateError> {
        // 验证节点存在
        if !self.dag.nodes.contains_key(&cmd.node_id) {
            return Err(AggregateError::NodeNotFound(cmd.node_id));
        }

        // 获取从该节点出发的所有出边
        let out_edge_ids = self.dag.out_edges.get(&cmd.node_id)
            .cloned()
            .unwrap_or_default();

        if out_edge_ids.is_empty() {
            return Err(AggregateError::ValidationError(
                format!("节点 {} 没有出边，无法坍缩", cmd.node_id)
            ));
        }

        // 计算观察者信号权重 α
        let alpha = cmd.signal.arousal * (1.0 + cmd.signal.valence.abs());

        // 计算坍缩后的概率偏移
        let mut shifts = Vec::new();
        let mut best_edge_id = None;
        let mut best_score = f64::NEG_INFINITY;

        for edge_id in &out_edge_ids {
            if let Some(edge) = self.dag.edges.get(edge_id) {
                let p_before = edge.probability;
                let p_after = p_before + alpha * edge.observer_weight;

                shifts.push(ProbabilityShift {
                    edge_id: *edge_id,
                    probability_before: p_before,
                    probability_after: p_after,
                    reason: format!(
                        "观察者坍缩: α={:.4}, valence={:.2}, arousal={:.2}",
                        alpha, cmd.signal.valence, cmd.signal.arousal
                    ),
                });

                if p_after > best_score {
                    best_score = p_after;
                    best_edge_id = Some(*edge_id);
                }
            }
        }

        let collapsed_path_id = best_edge_id
            .ok_or(AggregateError::ValidationError("无法确定坍缩路径".into()))?;

        let meta = EventMetadata::new(self.story_id, actor_id);
        Ok(vec![NarrativeEvent::BranchCollapsed {
            meta,
            node_id: cmd.node_id,
            reader_id: cmd.reader_id,
            signal: ObserverSignalEvent {
                valence: cmd.signal.valence,
                arousal: cmd.signal.arousal,
                dominance: cmd.signal.dominance,
                gaze_duration_ms: cmd.signal.gaze_duration_ms,
                focused_entity_id: cmd.signal.focused_entity_id,
            },
            shifts,
            collapsed_path_id,
        }])
    }

    // ── Chronos 时间轴处理 ──

    fn handle_checkpoint(
        &self,
        cmd: CreateCheckpointCommand,
        actor_id: Uuid,
    ) -> Result<Vec<NarrativeEvent>, AggregateError> {
        if !self.dag.nodes.contains_key(&cmd.node_id) {
            return Err(AggregateError::NodeNotFound(cmd.node_id));
        }

        let checkpoint_id = Uuid::new_v4();
        let meta = EventMetadata::new(self.story_id, actor_id);

        Ok(vec![NarrativeEvent::CheckpointCreated {
            meta,
            checkpoint_id,
            node_id: cmd.node_id,
            checkpoint_type: cmd.checkpoint_type,
            world_state: cmd.world_state,
        }])
    }

    fn handle_rollback(
        &self,
        cmd: RollbackCommand,
        actor_id: Uuid,
    ) -> Result<Vec<NarrativeEvent>, AggregateError> {
        let meta = EventMetadata::new(self.story_id, actor_id);

        Ok(vec![NarrativeEvent::BranchRolledBack {
            meta,
            checkpoint_id: cmd.checkpoint_id,
            rollback_from: Uuid::nil(), // TODO: 从当前状态获取
            timeline_depth: 0,          // TODO: 从 checkpoint 计算
        }])
    }

    // ═══════════════════════════════════════════════
    //  Event Applier（状态回放）
    // ═══════════════════════════════════════════════

    /// 将单个 Event 应用到内存状态
    ///
    /// 此方法是幂等的：同一个 Event 应用两次结果相同。
    pub fn apply_event(&mut self, event: &NarrativeEvent) {
        match event {
            NarrativeEvent::NodeCreated {
                node_id, node_type, title, position_x, position_y, ..
            } => {
                self.dag.nodes.insert(*node_id, DagNode {
                    id: *node_id,
                    node_type: node_type.clone(),
                    title: title.clone(),
                });
                // 更新入度表
                self.dag.in_degree.entry(*node_id).or_insert(0);
            }

            NarrativeEvent::NodeDeleted { node_id, cascaded_edge_ids, .. } => {
                // 删除所有级联边
                for edge_id in cascaded_edge_ids {
                    if let Some(edge) = self.dag.edges.remove(edge_id) {
                        if let Some(out) = self.dag.out_edges.get_mut(&edge.source) {
                            out.retain(|id| id != edge_id);
                        }
                        if let Some(inc) = self.dag.in_edges.get_mut(&edge.target) {
                            inc.retain(|id| id != edge_id);
                        }
                        self.dag.in_degree.entry(edge.target)
                            .and_modify(|d| *d = d.saturating_sub(1));
                        self.probability_cache.remove(edge_id);
                    }
                }
                // 删除节点
                self.dag.nodes.remove(node_id);
                self.dag.out_edges.remove(node_id);
                self.dag.in_edges.remove(node_id);
                self.dag.in_degree.remove(node_id);
            }

            NarrativeEvent::NodeUpdated { node_id, title, position_x: _, position_y: _, .. } => {
                if let Some(node) = self.dag.nodes.get_mut(node_id) {
                    if let Some(t) = title {
                        node.title = Some(t.clone());
                    }
                }
            }

            NarrativeEvent::EdgeLinked {
                edge_id, source_node_id, target_node_id,
                edge_type, probability, ..
            } => {
                let edge = DagEdge {
                    id: *edge_id,
                    source: *source_node_id,
                    target: *target_node_id,
                    edge_type: edge_type.clone(),
                    probability: *probability,
                    observer_weight: 0.0,
                };
                self.dag.edges.insert(*edge_id, edge);
                self.dag.out_edges.entry(*source_node_id)
                    .or_default().push(*edge_id);
                self.dag.in_edges.entry(*target_node_id)
                    .or_default().push(*edge_id);
                *self.dag.in_degree.entry(*target_node_id).or_insert(0) += 1;
                self.probability_cache.insert(*edge_id, *probability);
            }

            NarrativeEvent::EdgeUpdated { edge_id, probability, observer_weight, .. } => {
                if let Some(edge) = self.dag.edges.get_mut(edge_id) {
                    if let Some(p) = probability {
                        edge.probability = *p;
                        self.probability_cache.insert(*edge_id, *p);
                    }
                    if let Some(ow) = observer_weight {
                        edge.observer_weight = *ow;
                    }
                }
            }

            NarrativeEvent::EdgeDeleted { edge_id, .. } => {
                if let Some(edge) = self.dag.edges.remove(edge_id) {
                    if let Some(out) = self.dag.out_edges.get_mut(&edge.source) {
                        out.retain(|id| id != edge_id);
                    }
                    if let Some(inc) = self.dag.in_edges.get_mut(&edge.target) {
                        inc.retain(|id| id != edge_id);
                    }
                    self.dag.in_degree.entry(edge.target)
                        .and_modify(|d| *d = d.saturating_sub(1));
                    self.probability_cache.remove(edge_id);
                }
            }

            NarrativeEvent::BranchCollapsed { shifts, .. } => {
                // 应用坍缩后的概率偏移
                for shift in shifts {
                    if let Some(edge) = self.dag.edges.get_mut(&shift.edge_id) {
                        edge.probability = shift.probability_after;
                        edge.observer_weight = 0.0; // 重置观察者权重
                        self.probability_cache.insert(shift.edge_id, shift.probability_after);
                    }
                }
            }

            // Chronos 事件不修改 DAG 内存状态（持久化层处理）
            NarrativeEvent::CheckpointCreated { .. }
            | NarrativeEvent::BranchRolledBack { .. } => {}
        }

        self.version += 1;
    }
}

/// 聚合根错误类型
#[derive(Debug, thiserror::Error)]
pub enum AggregateError {
    #[error("故事 ID 不匹配: 期望 {expected}，实际 {got}")]
    StoryMismatch { expected: Uuid, got: Uuid },

    #[error("节点 {0} 不存在")]
    NodeNotFound(Uuid),

    #[error("边 {0} 不存在")]
    EdgeNotFound(Uuid),

    // 注意: 字段不能命名为 source，否则 thiserror 会将其当作 error source
    #[error("添加边 {0} → {1} 将形成环路")]
    CycleDetected(String, String),

    #[error("概率值 {0} 不在 [0, 1] 范围内")]
    InvalidProbability(f64),

    #[error("验证失败: {0}")]
    ValidationError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_node_command() {
        let story_id = Uuid::new_v4();
        let graph = NarrativeGraph::new(story_id);
        let actor_id = Uuid::new_v4();

        let cmd = NarrativeCommand::CreateNode(CreateNodeCommand {
            story_id,
            node_type: "scene".to_string(),
            title: Some("开场".to_string()),
            content: None,
            position_x: 100.0,
            position_y: 200.0,
            metadata: None,
        });

        let events = graph.handle_command(cmd, actor_id).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type_label(), "node_created");
    }

    #[test]
    fn test_cycle_detection() {
        let story_id = Uuid::new_v4();
        let actor_id = Uuid::new_v4();
        let node_a = Uuid::new_v4();
        let node_b = Uuid::new_v4();

        // 手动构建有两个节点和一条边 A→B 的图
        let mut graph = NarrativeGraph::new(story_id);
        graph.dag.nodes.insert(node_a, DagNode {
            id: node_a,
            node_type: "scene".to_string(),
            title: Some("A".to_string()),
        });
        graph.dag.nodes.insert(node_b, DagNode {
            id: node_b,
            node_type: "scene".to_string(),
            title: Some("B".to_string()),
        });
        graph.dag.in_degree.insert(node_a, 0);
        graph.dag.in_degree.insert(node_b, 0);

        // 先合法添加 A→B
        let edge_cmd = NarrativeCommand::LinkEdge(LinkEdgeCommand {
            story_id,
            source_node_id: node_a,
            target_node_id: node_b,
            edge_type: "causal".to_string(),
            probability: 1.0,
            reward_signal: None,
            conditions: None,
        });
        let events = graph.handle_command(edge_cmd, actor_id).unwrap();
        graph.apply_event(&events[0]);

        // 尝试添加 B→A（应触发环路检测）
        let cycle_cmd = NarrativeCommand::LinkEdge(LinkEdgeCommand {
            story_id,
            source_node_id: node_b,
            target_node_id: node_a,
            edge_type: "causal".to_string(),
            probability: 1.0,
            reward_signal: None,
            conditions: None,
        });
        let result = graph.handle_command(cycle_cmd, actor_id);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AggregateError::CycleDetected { .. }));
    }

    #[test]
    fn test_replay_idempotency() {
        let story_id = Uuid::new_v4();
        let actor_id = Uuid::new_v4();

        let cmd = NarrativeCommand::CreateNode(CreateNodeCommand {
            story_id,
            node_type: "scene".to_string(),
            title: Some("测试节点".to_string()),
            content: None,
            position_x: 0.0,
            position_y: 0.0,
            metadata: None,
        });

        let graph = NarrativeGraph::new(story_id);
        let events = graph.handle_command(cmd, actor_id).unwrap();

        // 回放一次
        let replayed1 = NarrativeGraph::replay(story_id, &events);
        assert_eq!(replayed1.dag.nodes.len(), 1);
        assert_eq!(replayed1.version, 1);

        // 回放两次（幂等性）
        let replayed2 = NarrativeGraph::replay(story_id, &events);
        assert_eq!(replayed2.dag.nodes.len(), 1);
        assert_eq!(replayed2.version, 1);
    }

    #[test]
    fn test_story_mismatch() {
        let story_id = Uuid::new_v4();
        let wrong_story = Uuid::new_v4();
        let graph = NarrativeGraph::new(story_id);
        let actor_id = Uuid::new_v4();

        let cmd = NarrativeCommand::CreateNode(CreateNodeCommand {
            story_id: wrong_story,
            node_type: "scene".to_string(),
            title: None,
            content: None,
            position_x: 0.0,
            position_y: 0.0,
            metadata: None,
        });

        let result = graph.handle_command(cmd, actor_id);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AggregateError::StoryMismatch { .. }));
    }

    #[test]
    fn test_invalid_probability() {
        let story_id = Uuid::new_v4();
        let actor_id = Uuid::new_v4();
        let node_a = Uuid::new_v4();
        let node_b = Uuid::new_v4();

        let mut graph = NarrativeGraph::new(story_id);
        graph.dag.nodes.insert(node_a, DagNode {
            id: node_a,
            node_type: "scene".to_string(),
            title: Some("A".to_string()),
        });
        graph.dag.nodes.insert(node_b, DagNode {
            id: node_b,
            node_type: "scene".to_string(),
            title: Some("B".to_string()),
        });
        graph.dag.in_degree.insert(node_a, 0);
        graph.dag.in_degree.insert(node_b, 0);

        let cmd = NarrativeCommand::LinkEdge(LinkEdgeCommand {
            story_id,
            source_node_id: node_a,
            target_node_id: node_b,
            edge_type: "branch".to_string(),
            probability: 1.5, // 超出范围
            reward_signal: None,
            conditions: None,
        });

        let result = graph.handle_command(cmd, actor_id);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AggregateError::InvalidProbability(1.5)));
    }
}