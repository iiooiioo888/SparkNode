//! DAG (有向无环图) 模块
//!
//! 星轨编织器的底层数据结构。
//! 支持节点增删改查、拓扑排序、BFS/DFS 遍历、
//! 环路检测、以及平行宇宙 Diff 算法。

pub mod graph;
pub mod operations;
pub mod traversal;
pub mod diff;

pub use graph::*;
pub use operations::*;
pub use traversal::*;
pub use diff::*;