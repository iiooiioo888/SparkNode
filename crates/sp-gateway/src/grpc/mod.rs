//! gRPC 模块
//!
//! - `client`: 连接 Python AI 服务层的 gRPC 客户端封装（HTTP/2 连接池复用）
//! - `narrative_server`: NarrativeService gRPC 服务端实现（CQRS 写入端入口）

pub mod client;
pub mod narrative_server;

pub use client::{AiClient, GrpcPool};
