//! Memgraph Graph Projection（Read Model）
//!
//! 将 PostgreSQL Event Store 中的领域事件投影到 Memgraph 图数据库，
//! 供 Chronos 引擎进行高效的图遍历查询。
//!
//! 架构职责：
//! - 监听新产生的 Event（增量同步）
//! - 将 Event 转换为 Cypher 语句写入 Memgraph
//! - 支持全量重建（从零投影）

use neo4rs::Graph;
use uuid::Uuid;

use crate::domain::events::NarrativeEvent;

/// Graph Projection 错误类型
#[derive(Debug, thiserror::Error)]
pub enum ProjectionError {
    #[error("Memgraph 连接错误: {0}")]
    Connection(String),

    #[error("Cypher 执行错误: {0}")]
    CypherExecution(String),

    #[error("事件处理错误: {0}")]
    EventProcessing(String),
}

/// Memgraph Graph Projection
///
/// 负责将领域事件转换为图数据库操作。
/// `neo4rs` crate 完全兼容 Memgraph 的 Bolt 协议。
pub struct GraphProjection {
    graph: Graph,
}

impl GraphProjection {
    /// 从已有的 neo4rs::Graph 连接创建 Projection
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// 连接 Memgraph 实例
    ///
    /// # 参数
    /// - `uri`: Memgraph Bolt 协议地址（如 `bolt://localhost:7687`）
    /// - `user`: 用户名（默认 `""`）
    /// - `password`: 密码（默认 `""`）
    pub async fn connect(uri: &str, user: &str, password: &str) -> Result<Self, ProjectionError> {
        let graph = Graph::new(uri, user, password)
            .map_err(|e| ProjectionError::Connection(format!("Memgraph 连接失败: {}", e)))?;

        tracing::info!("✓ Memgraph Graph Projection 已连接: {}", uri);
        Ok(Self { graph })
    }

    /// 将单个领域事件投影到 Memgraph
    ///
    /// 根据事件类型生成对应的 Cypher 语句并执行。
    pub async fn project_event(&self, event: &NarrativeEvent) -> Result<(), ProjectionError> {
        match event {
            NarrativeEvent::NodeCreated {
                node_id,
                node_type,
                title,
                content,
                position_x,
                position_y,
                ..
            } => {
                let query = neo4rs::query(
                    "MERGE (n:StoryNode {id: $id}) \
                     SET n.node_type = $node_type, \
                         n.title = $title, \
                         n.content = $content, \
                         n.position_x = $position_x, \
                         n.position_y = $position_y, \
                         n.updated_at = timestamp()"
                )
                .param("id", node_id.to_string())
                .param("node_type", node_type.clone())
                .param("title", title.clone().unwrap_or_default())
                .param("content", content.clone().unwrap_or_default())
                .param("position_x", *position_x)
                .param("position_y", *position_y);

                self.execute_query(query).await?;
            }

            NarrativeEvent::NodeUpdated {
                node_id,
                title,
                content,
                ..
            } => {
                // 动态构建 SET 子句（neo4rs 不支持链式动态参数，需预构建字符串）
                let mut set_parts = vec!["n.updated_at = timestamp()".to_string()];
                if title.is_some() { set_parts.push("n.title = $title".to_string()); }
                if content.is_some() { set_parts.push("n.content = $content".to_string()); }

                let query_str = format!(
                    "MATCH (n:StoryNode {{id: $id}}) SET {}",
                    set_parts.join(", ")
                );

                let mut q = neo4rs::query(&query_str).param("id", node_id.to_string());
                if let Some(t) = title {
                    q = q.param("title", t.clone());
                }
                if let Some(c) = content {
                    q = q.param("content", c.clone());
                }

                self.execute_query(q).await?;
            }

            NarrativeEvent::NodeDeleted { node_id, .. } => {
                let query = neo4rs::query(
                    "MATCH (n:StoryNode {id: $id}) DETACH DELETE n"
                )
                .param("id", node_id.to_string());

                self.execute_query(query).await?;
            }

            NarrativeEvent::EdgeLinked {
                edge_id,
                source_node_id,
                target_node_id,
                edge_type,
                probability,
                reward_signal,
                ..
            } => {
                let query = neo4rs::query(
                    "MATCH (a:StoryNode {id: $source}), (b:StoryNode {id: $target}) \
                     CREATE (a)-[r:BRANCH { \
                         id: $edge_id, \
                         edge_type: $edge_type, \
                         probability: $probability, \
                         reward_signal: $reward_signal \
                     }]->(b)"
                )
                .param("source", source_node_id.to_string())
                .param("target", target_node_id.to_string())
                .param("edge_id", edge_id.to_string())
                .param("edge_type", edge_type.clone())
                .param("probability", *probability)
                .param("reward_signal", *reward_signal);

                self.execute_query(query).await?;
            }

            NarrativeEvent::EdgeUpdated {
                edge_id,
                probability,
                observer_weight,
                ..
            } => {
                // 动态构建 SET 子句
                let mut set_parts = vec!["r.updated_at = timestamp()".to_string()];
                if probability.is_some() { set_parts.push("r.probability = $probability".to_string()); }
                if observer_weight.is_some() { set_parts.push("r.observer_weight = $ow".to_string()); }

                let query_str = format!(
                    "MATCH ()-[r:BRANCH {{id: $edge_id}}]->() SET {}",
                    set_parts.join(", ")
                );

                let mut q = neo4rs::query(&query_str).param("edge_id", edge_id.to_string());
                if let Some(p) = probability {
                    q = q.param("probability", *p);
                }
                if let Some(ow) = observer_weight {
                    q = q.param("ow", *ow);
                }

                self.execute_query(q).await?;
            }

            NarrativeEvent::EdgeDeleted { edge_id, .. } => {
                let query = neo4rs::query(
                    "MATCH ()-[r:BRANCH {id: $edge_id}]->() DELETE r"
                )
                .param("edge_id", edge_id.to_string());

                self.execute_query(query).await?;
            }

            NarrativeEvent::BranchCollapsed {
                node_id,
                collapsed_path_id,
                shifts,
                ..
            } => {
                // 标记坍缩选中的路径
                let query = neo4rs::query(
                    "MATCH (n:StoryNode {id: $node_id}) \
                     SET n.last_collapsed_path = $path_id, n.collapse_count = coalesce(n.collapse_count, 0) + 1"
                )
                .param("node_id", node_id.to_string())
                .param("path_id", collapsed_path_id.to_string());

                self.execute_query(query).await?;

                // 更新坍缩后的概率
                for shift in shifts {
                    let q = neo4rs::query(
                        "MATCH ()-[r:BRANCH {id: $edge_id}]->() \
                         SET r.probability = $new_prob, r.last_collapse_reason = $reason"
                    )
                    .param("edge_id", shift.edge_id.to_string())
                    .param("new_prob", shift.probability_after)
                    .param("reason", shift.reason.clone());

                    self.execute_query(q).await?;
                }
            }

            // Chronos 事件不投影到图数据库（使用 PostgreSQL world_checkpoints 表）
            NarrativeEvent::CheckpointCreated { .. }
            | NarrativeEvent::BranchRolledBack { .. } => {}
        }

        Ok(())
    }

    /// 全量重建投影
    ///
    /// 从 Event Store 加载全部事件并逐一投影。
    /// 适用于初始化或数据修复场景。
    pub async fn rebuild_from_events(
        &self,
        story_id: Uuid,
        events: &[NarrativeEvent],
    ) -> Result<(), ProjectionError> {
        // 先清除该故事的现有图数据
        let clear_query = neo4rs::query(
            "MATCH (n:StoryNode) WHERE n.story_id = $story_id DETACH DELETE n"
        )
        .param("story_id", story_id.to_string());

        self.execute_query(clear_query).await?;

        // 逐事件投影
        for event in events {
            self.project_event(event).await?;
        }

        tracing::info!(
            "Memgraph 投影已全量重建: story_id={}, events={}",
            story_id,
            events.len()
        );
        Ok(())
    }

    /// 内部执行 Cypher 查询
    async fn execute_query(&self, query: neo4rs::Query) -> Result<(), ProjectionError> {
        self.graph
            .run(query)
            .await
            .map_err(|e| ProjectionError::CypherExecution(format!("Cypher 执行失败: {}", e)))?;
        Ok(())
    }
}