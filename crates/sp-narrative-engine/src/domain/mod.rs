//! 领域模型层 (Domain Layer)
//!
//! CQRS 架构的核心，定义：
//! - `commands`: 写入端命令（Command）
//! - `events`: 领域事件（Event）
//! - `aggregate`: 聚合根（Aggregate Root）
//!
//! 所有状态变更遵循 `Command → Event → State Mutation` 流程。

pub mod aggregate;
pub mod commands;
pub mod events;