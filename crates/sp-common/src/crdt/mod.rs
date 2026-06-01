//! CRDT (Conflict-free Replicated Data Type) 协作协议
//!
//! 为「星轨编织器」的多人实时协作提供无冲突复制数据类型。
//! 支持多作者同时编辑 DAG 图谱，保证最终一致性。

pub mod lww_register;
pub mod graph_crdt;

pub use lww_register::*;
pub use graph_crdt::*;