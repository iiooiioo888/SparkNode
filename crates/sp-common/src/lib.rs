//! # 灵犀节点 (SparkNode) 共享类型库
//!
//! 提供跨所有引擎共享的核心数据结构、协议定义与工具函数。
//! 包含叙事节点、MDP 概率分布、NPC 灵魂架构、CRDT 协作协议等。

pub mod types;
pub mod crdt;
pub mod error;

// gRPC 生成的代码 (由 build.rs 编译 proto 文件)
pub mod narrative_proto {
    tonic::include_proto!("sp.narrative");
}

pub mod soul_proto {
    tonic::include_proto!("sp.soul");
}