//! # 灵犀节点 叙事引擎 (Narrative Engine)
//!
//! 从「DAG 分支」到「量子叙事叠加态」的核心引擎。
//!
//! ## 架构分层
//! - `domain`: CQRS 领域模型 (Command → Event → Aggregate Root)
//! - `persistence`: 持久化层 (PostgreSQL Event Store + Memgraph Graph Projection)
//! - `dag`: 有向无环图结构 (邻接表 + 拓扑排序 + 环路检测)
//! - `mdp`: 马尔可夫决策过程矩阵 (概率转移 + 观察者坍缩)
//! - `chronos`: 时间轴引擎 (快照 + 回溯 + 蝴蝶效应 Diff)
//! - `merge`: 多分支合并冲突解决
//!
//! ## CQRS 流程
//! ```text
//! Command → handle_command() → [Event] → apply_event() → State
//!                               ↓
//!                    append_events() (PostgreSQL)
//!                               ↓
//!                    project_event() (Memgraph)
//! ```

pub mod domain;
pub mod persistence;
pub mod dag;
pub mod mdp;
pub mod chronos;
pub mod merge;
