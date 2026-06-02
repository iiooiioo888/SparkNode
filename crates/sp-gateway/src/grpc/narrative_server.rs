//! NarrativeService gRPC Server 实现
//!
//! 将 gRPC 外部请求转换为 CQRS Command，
//! 调用 `sp-narrative-engine` 的聚合根处理，
//! 并通过 Event Store 和 Graph Projection 持久化。
//!
//! 架构：gRPC Request → Command → Aggregate.handle_command
//!       → [Event] → EventStore.append → GraphProjection.project

use tonic::{Request, Response, Status};
use uuid::Uuid;

use sp_common::narrative_proto::narrative_service_server::NarrativeService;
use sp_common::narrative_proto::*;

use sp_narrative_engine::domain::{
    aggregate::{NarrativeGraph, AggregateError},
    commands::*,
    events::NarrativeEvent,
};
use sp_narrative_engine::persistence::event_store;

use crate::AppState;

/// NarrativeService gRPC 服务端实现
pub struct NarrativeServiceImpl {
    pub state: AppState,
}

#[tonic::async_trait]
impl NarrativeService for NarrativeServiceImpl {
    /// 流式生成（保留原有接口）
    type GenerateStreamStream =
        tokio_stream::wrappers::ReceiverStream<Result<GenerateChunk, Status>>;

    async fn generate_stream(
        &self,
        _request: Request<GenerateRequest>,
    ) -> Result<Response<Self::GenerateStreamStream>, Status> {
        // 委托给 Python LLM Router（保留原有逻辑）
        Err(Status::unimplemented("GenerateStream 待对接 Python AI 层"))
    }

    /// 观察者坍缩（保留原有接口）
    async fn collapse(
        &self,
        request: Request<CollapseRequest>,
    ) -> Result<Response<CollapseResponse>, Status> {
        let req = request.into_inner();
        let story_id = Uuid::parse_str(&req.story_id)
            .map_err(|e| Status::invalid_argument(format!("无效的 story_id: {}", e)))?;
        let reader_id = Uuid::parse_str(&req.reader_id)
            .map_err(|e| Status::invalid_argument(format!("无效的 reader_id: {}", e)))?;

        // 从 Event Store 加载聚合根
        let pool = &self.state.db;
        let mut graph = event_store::load_aggregate(pool, story_id)
            .await
            .map_err(|e| Status::internal(format!("加载聚合根失败: {}", e)))?;

        let signal = req.signal.ok_or_else(|| Status::invalid_argument("缺少 ObserverSignal"))?;

        // 构建 CollapseBranch Command
        let cmd = NarrativeCommand::CollapseBranch(CollapseBranchCommand {
            story_id,
            node_id: Uuid::parse_str(&req.story_id).unwrap_or_default(), // TODO: 从请求中获取正确的 node_id
            reader_id,
            signal: ObserverSignalPayload {
                valence: signal.valence as f64,
                arousal: signal.arousal as f64,
                dominance: signal.dominance as f64,
                gaze_duration_ms: Some(signal.gaze_duration_ms as f64),
                focused_entity_id: Some(signal.focused_entity_id),
            },
        });

        let actor_id = reader_id;
        let events = graph.handle_command(cmd, actor_id)
            .map_err(|e| Status::failed_precondition(format!("命令处理失败: {}", e)))?;

        // 持久化到 Event Store
        event_store::append_events(pool, story_id, &events)
            .await
            .map_err(|e| Status::internal(format!("事件持久化失败: {}", e)))?;

        // 投影到 Memgraph（如果可用）
        // TODO: 从 AppState 获取 GraphProjection 实例

        let shifts: Vec<ProbabilityShift> = if let Some(NarrativeEvent::BranchCollapsed { shifts, .. }) = events.first() {
            shifts.iter().map(|s| ProbabilityShift {
                edge_id: s.edge_id.to_string(),
                probability_before: s.probability_before as f32,
                probability_after: s.probability_after as f32,
                reason: s.reason.clone(),
            }).collect()
        } else {
            vec![]
        };

        let collapsed_path_id = if let Some(NarrativeEvent::BranchCollapsed { collapsed_path_id, .. }) = events.first() {
            collapsed_path_id.to_string()
        } else {
            String::new()
        };

        Ok(Response::new(CollapseResponse {
            success: true,
            collapsed_path_id,
            shifts,
            generated_content: String::new(),
        }))
    }

    /// 获取 MDP 转移矩阵（保留原有接口）
    async fn get_transition_matrix(
        &self,
        _request: Request<MatrixRequest>,
    ) -> Result<Response<MatrixResponse>, Status> {
        Err(Status::unimplemented("GetTransitionMatrix 待实现"))
    }

    // ═══════════════════════════════════════════════
    //  CQRS 新增 RPC
    // ═══════════════════════════════════════════════

    /// 执行领域命令（CQRS 写入端入口）
    ///
    /// 流程：
    /// 1. 反序列化 CommandRequest.command_json → NarrativeCommand
    /// 2. 从 Event Store 加载聚合根（或使用缓存）
    /// 3. 调用 aggregate.handle_command
    /// 4. 将产出的 Event 追加到 Event Store
    /// 5. 异步投影到 Memgraph
    async fn execute_command(
        &self,
        request: Request<CommandRequest>,
    ) -> Result<Response<CommandResponse>, Status> {
        let req = request.into_inner();
        let story_id = Uuid::parse_str(&req.story_id)
            .map_err(|e| Status::invalid_argument(format!("无效的 story_id: {}", e)))?;
        let actor_id = Uuid::parse_str(&req.actor_id)
            .map_err(|e| Status::invalid_argument(format!("无效的 actor_id: {}", e)))?;

        // 反序列化 Command
        let command: NarrativeCommand = serde_json::from_str(&req.command_json)
            .map_err(|e| Status::invalid_argument(format!("Command 反序列化失败: {}", e)))?;

        let pool = &self.state.db;

        // 加载聚合根
        let graph = event_store::load_aggregate(pool, story_id)
            .await
            .map_err(|e| Status::internal(format!("加载聚合根失败: {}", e)))?;

        // 执行命令
        let events = graph.handle_command(command, actor_id)
            .map_err(|e| match e {
                AggregateError::CycleDetected(source, target) => {
                    Status::failed_precondition(format!("环路检测: {} → {}", source, target))
                }
                AggregateError::NodeNotFound(id) => {
                    Status::not_found(format!("节点不存在: {}", id))
                }
                AggregateError::EdgeNotFound(id) => {
                    Status::not_found(format!("边不存在: {}", id))
                }
                AggregateError::InvalidProbability(p) => {
                    Status::out_of_range(format!("概率值超出范围: {}", p))
                }
                other => Status::failed_precondition(other.to_string()),
            })?;

        // 持久化到 Event Store
        event_store::append_events(pool, story_id, &events)
            .await
            .map_err(|e| Status::internal(format!("事件持久化失败: {}", e)))?;

        // 构造响应
        let event_ids: Vec<String> = events.iter().map(|e| e.meta().event_id.to_string()).collect();
        let events_json = serde_json::to_string(&events)
            .map_err(|e| Status::internal(format!("事件序列化失败: {}", e)))?;

        tracing::info!(
            "CQRS 命令已执行: story_id={}, actor_id={}, events={}",
            story_id, actor_id, event_ids.len()
        );

        Ok(Response::new(CommandResponse {
            success: true,
            error_message: String::new(),
            event_ids,
            events_json,
        }))
    }

    /// 获取时间轴快照列表（Chronos 读取端）
    async fn get_timeline(
        &self,
        request: Request<TimelineRequest>,
    ) -> Result<Response<TimelineResponse>, Status> {
        let req = request.into_inner();
        let story_id = Uuid::parse_str(&req.story_id)
            .map_err(|e| Status::invalid_argument(format!("无效的 story_id: {}", e)))?;

        let pool = &self.state.db;

        // 查询 world_checkpoints 表
        let rows = sqlx::query_as::<_, (String, String, String, String)>(
            r#"
            SELECT id::text, node_id::text, checkpoint_type, created_at::text
            FROM world_checkpoints
            WHERE story_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(story_id)
        .fetch_all(pool)
        .await
        .map_err(|e| Status::internal(format!("查询时间轴失败: {}", e)))?;

        let checkpoints: Vec<Checkpoint> = rows
            .into_iter()
            .map(|(id, node_id, checkpoint_type, created_at)| Checkpoint {
                id,
                node_id,
                checkpoint_type,
                created_at,
            })
            .collect();

        Ok(Response::new(TimelineResponse { checkpoints }))
    }

    /// 回溯到指定快照（Chronos 写入端）
    async fn rollback(
        &self,
        request: Request<RollbackRequest>,
    ) -> Result<Response<RollbackResponse>, Status> {
        let req = request.into_inner();
        let story_id = Uuid::parse_str(&req.story_id)
            .map_err(|e| Status::invalid_argument(format!("无效的 story_id: {}", e)))?;
        let checkpoint_id = Uuid::parse_str(&req.checkpoint_id)
            .map_err(|e| Status::invalid_argument(format!("无效的 checkpoint_id: {}", e)))?;

        let pool = &self.state.db;
        let mut graph = event_store::load_aggregate(pool, story_id)
            .await
            .map_err(|e| Status::internal(format!("加载聚合根失败: {}", e)))?;

        let cmd = NarrativeCommand::Rollback(RollbackCommand {
            story_id,
            checkpoint_id,
        });

        // 使用系统 Actor ID
        let actor_id = Uuid::nil();
        let events = graph.handle_command(cmd, actor_id)
            .map_err(|e| Status::failed_precondition(format!("回溯命令失败: {}", e)))?;

        event_store::append_events(pool, story_id, &events)
            .await
            .map_err(|e| Status::internal(format!("事件持久化失败: {}", e)))?;

        Ok(Response::new(RollbackResponse {
            success: true,
            message: format!("已回溯到快照 {}", checkpoint_id),
        }))
    }
}