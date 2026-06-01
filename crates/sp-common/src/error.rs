//! SparkNode 全局错误类型

use thiserror::Error;

/// SparkNode 统一错误枚举
#[derive(Error, Debug)]
pub enum SpError {
    // ── 数据库错误 ──
    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis 错误: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("向量数据库错误: {message}")]
    VectorDb { message: String },

    #[error("图数据库错误: {message}")]
    GraphDb { message: String },

    // ── 叙事引擎错误 ──
    #[error("DAG 循环依赖检测: 节点 {source_id} → {target_id} 将形成环路")]
    DagCycleDetected {
        source_id: uuid::Uuid,
        target_id: uuid::Uuid,
    },

    #[error("节点 {0} 不存在")]
    NodeNotFound(uuid::Uuid),

    #[error("边 {0} 不存在")]
    EdgeNotFound(uuid::Uuid),

    #[error("故事 {0} 不存在")]
    StoryNotFound(uuid::Uuid),

    #[error("MDP 矩阵状态异常: {reason}")]
    MdpMatrixError { reason: String },

    #[error("概率分布未归一化: 总和 = {sum}")]
    ProbabilityNotNormalized { sum: f64 },

    // ── 灵魂引擎错误 ──
    #[error("NPC {0} 不存在")]
    NpcNotFound(uuid::Uuid),

    #[error("记忆编码失败: {reason}")]
    MemoryEncodingFailed { reason: String },

    #[error("记忆检索失败: {reason}")]
    MemoryRecallFailed { reason: String },

    // ── LLM 错误 ──
    #[error("LLM 提供商 {provider} 调用失败: {reason}")]
    LlmProviderError { provider: String, reason: String },

    #[error("所有 LLM 提供商均不可用")]
    AllProvidersUnavailable,

    // ── gRPC 错误 ──
    #[error("gRPC 调用失败: {0}")]
    Grpc(#[from] tonic::Status),

    // ── 通用错误 ──
    #[error("序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP 请求错误: {0}")]
    Http(#[from] reqwest::Error),

    #[error("配置错误: {0}")]
    Config(String),

    #[error("认证失败: {0}")]
    Auth(String),

    #[error("权限不足: {0}")]
    Forbidden(String),

    #[error("内部错误: {0}")]
    Internal(String),
}

/// 将 SpError 转换为 HTTP 状态码
impl SpError {
    pub fn status_code(&self) -> u16 {
        match self {
            SpError::StoryNotFound(_)
            | SpError::NodeNotFound(_)
            | SpError::EdgeNotFound(_)
            | SpError::NpcNotFound(_) => 404,

            SpError::Auth(_) => 401,
            SpError::Forbidden(_) => 403,

            SpError::DagCycleDetected { .. }
            | SpError::ProbabilityNotNormalized { .. }
            | SpError::MdpMatrixError { .. } => 422,

            _ => 500,
        }
    }
}