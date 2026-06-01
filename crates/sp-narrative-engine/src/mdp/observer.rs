//! 观察者坍缩模块
//!
//! 接收前端情感计算的 ObserverSignal，
//! 将其转化为 MDP 概率矩阵的权重注入。

use std::collections::HashMap;
use uuid::Uuid;
use sp_common::types::ObserverSignal;

/// 观察者坍缩处理器
pub struct ObserverCollapse {
    pub alpha: f64, // 观察者效应强度系数
}

impl ObserverCollapse {
    pub fn new(alpha: f64) -> Self {
        Self { alpha }
    }

    /// 将观察者信号转换为目标节点的权重映射
    ///
    /// 当读者的视线聚焦在某个实体上超过阈值时，
    /// 系统判定该实体为「观察对象」，提升其相关叙事路径的概率。
    pub fn compute_weights(
        &self,
        signal: &ObserverSignal,
        entity_edge_map: &HashMap<Uuid, Vec<Uuid>>, // entity_id → [edge_ids]
    ) -> HashMap<Uuid, f64> {
        let mut weights = HashMap::new();
        let signal_strength = signal.to_weight_vector();

        if let Some(ref focused_id) = signal.focused_entity_id {
            if let Some(edge_ids) = entity_edge_map.get(focused_id) {
                for &edge_id in edge_ids {
                    weights.insert(edge_id, signal_strength * self.alpha);
                }
            }
        }

        weights
    }
}