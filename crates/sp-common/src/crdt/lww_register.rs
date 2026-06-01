//! Last-Write-Wins Register (LWW Register)
//!
//! 基于逻辑时钟的最后写入胜出寄存器。
//! 当多个作者同时修改同一字段时，时间戳最新的写入自动胜出，
//! 保证最终一致性而无需中心化冲突解决。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// LWW 寄存器
///
/// 每个值附带写入者 ID 与逻辑时间戳。
/// 合并时，时间戳较大者胜出；时间戳相同时，UUID 较大者胜出 (确定性决胜)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LwwRegister<T: Clone + Serialize> {
    pub value: T,
    pub writer_id: Uuid,
    pub timestamp: DateTime<Utc>,
}

impl<T: Clone + Serialize + PartialEq> LwwRegister<T> {
    /// 创建新的 LWW 寄存器
    pub fn new(value: T, writer_id: Uuid) -> Self {
        Self {
            value,
            writer_id,
            timestamp: Utc::now(),
        }
    }

    /// 写入新值 (自动更新时间戳)
    pub fn write(&mut self, value: T, writer_id: Uuid) {
        self.value = value;
        self.writer_id = writer_id;
        self.timestamp = Utc::now();
    }

    /// 合并另一个寄存器 (最后写入胜出)
    /// 返回 true 表示发生了合并 (值被更新)
    pub fn merge(&mut self, other: &Self) -> bool {
        if other.timestamp > self.timestamp
            || (other.timestamp == self.timestamp && other.writer_id > self.writer_id)
        {
            self.value = other.value.clone();
            self.writer_id = other.writer_id;
            self.timestamp = other.timestamp;
            true
        } else {
            false
        }
    }
}