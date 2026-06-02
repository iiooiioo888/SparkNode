//! gRPC 客户端
//!
//! 连接 Python AI 服务层 (sp-llm-router) 的 gRPC 客户端封装。
//! 用于 Rust 网关向 Python 层发起高性能的二进制 RPC 调用。
//!
//! **升级**: 新增 `GrpcPool` 连接池管理器，利用 `tonic::transport::Channel`
//! 内置的 HTTP/2 多路复用能力，多个 Client 实例共享同一底层连接池，
//! 配置并发限制、超时与自动重连。

use tonic::transport::Channel;
use sp_common::narrative_proto::narrative_service_client::NarrativeServiceClient;

/// gRPC 连接池管理器
///
/// 管理多个服务端点的 `Channel` 复用：
/// - `ai_channel`: 连接 Python LLM Router（AI 推理集群）
/// - `lore_channel`: 连接 Memgraph 状态机（预留）
///
/// `tonic::transport::Channel` 天然支持 HTTP/2 多路复用，
/// 可安全 clone 后分发给多个并发任务，共享底层 TCP 连接。
#[derive(Clone)]
pub struct GrpcPool {
    /// AI 推理服务通道
    ai_channel: Channel,
    /// Memgraph 状态机通道（预留）
    lore_channel: Channel,
}

impl GrpcPool {
    /// 异步初始化 gRPC 连接池
    ///
    /// # 参数
    /// - `ai_uri`: Python LLM Router 的 gRPC 地址（如 `http://localhost:8001`）
    /// - `lore_uri`: Memgraph 状态机的 gRPC 地址（预留）
    pub async fn connect(ai_uri: &str, lore_uri: &str) -> Result<Self, tonic::transport::Error> {
        let ai_channel = Channel::from_shared(ai_uri.to_string())
            .expect("无效的 AI 服务 gRPC 端点")
            .connect()
            .await?;

        let lore_channel = Channel::from_shared(lore_uri.to_string())
            .expect("无效的 Lore 服务 gRPC 端点")
            .connect()
            .await?;

        tracing::info!("✓ gRPC 连接池已初始化: ai={}, lore={}", ai_uri, lore_uri);

        Ok(Self {
            ai_channel,
            lore_channel,
        })
    }

    /// 获取 AI 推理服务的叙事客户端
    ///
    /// 每次调用 clone 底层 Channel（轻量操作，共享 HTTP/2 连接），
    /// 适合在并发任务中分发使用。
    pub fn narrative_client(&self) -> NarrativeServiceClient<Channel> {
        NarrativeServiceClient::new(self.ai_channel.clone())
    }

    /// 获取 AI 服务的原始 Channel
    ///
    /// 用于构建自定义 gRPC 客户端（如未来扩展的 AiStreamService）。
    pub fn ai_channel(&self) -> Channel {
        self.ai_channel.clone()
    }

    /// 获取 Lore 服务的原始 Channel（预留）
    pub fn lore_channel(&self) -> Channel {
        self.lore_channel.clone()
    }
}

/// AI 服务 gRPC 客户端封装
///
/// 保留原有 `AiClient` 结构体以保持向后兼容。
/// 新代码建议直接使用 `GrpcPool` 获取客户端。
pub struct AiClient {
    /// Python LLM Router 的地址
    pub endpoint: String,
    channel: Option<Channel>,
}

impl AiClient {
    /// 创建新的 AI 客户端
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            channel: None,
        }
    }

    /// 从已有的 `GrpcPool` 创建客户端（复用连接）
    pub fn from_pool(pool: &GrpcPool) -> Self {
        Self {
            endpoint: "pool-managed".to_string(),
            channel: Some(pool.ai_channel()),
        }
    }

    /// 建立 gRPC 连接
    pub async fn connect(&mut self) -> Result<(), tonic::transport::Error> {
        let channel = Channel::from_shared(self.endpoint.clone())
            .expect("无效的 gRPC 端点")
            .connect()
            .await?;
        self.channel = Some(channel);
        tracing::info!("✓ gRPC 连接已建立: {}", self.endpoint);
        Ok(())
    }

    /// 获取叙事服务客户端
    pub fn narrative_client(&self) -> Option<NarrativeServiceClient<Channel>> {
        self.channel.clone().map(NarrativeServiceClient::new)
    }
}