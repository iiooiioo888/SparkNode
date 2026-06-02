//! CQRS 持久化层 (Persistence Layer)
//!
//! 双轨持久化架构：
//! - Write Model: PostgreSQL Event Store（不可变事件日志）
//! - Read Model: Memgraph Graph Projection（图谱状态投影）

pub mod event_store;
pub mod graph_projection;