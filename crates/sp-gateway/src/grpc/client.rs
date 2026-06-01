//! gRPC 客户端
//!
//! 连接 Python AI 服务层 (sp-llm-router) 的 gRPC 客户端封装。
//! 用于 Rust 网关向 Python 层发起高性能的二进制 RPC 调用。

use tonic::transport::Channel;
use sp_common::narrative_proto::narrative_service_client::NarrativeServiceClient;

/// AI 服务 gRPC 客户端封装
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