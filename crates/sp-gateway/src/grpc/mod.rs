//! gRPC 客户端模块
//!
//! 连接 Python AI 服务层的 gRPC 客户端封装。
//! 支持 HTTP/2 连接池复用（GrpcPool）与向后兼容的 AiClient。

pub mod client;

pub use client::{AiClient, GrpcPool};
