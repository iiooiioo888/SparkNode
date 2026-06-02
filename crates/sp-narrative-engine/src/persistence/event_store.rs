//! PostgreSQL Event Store（Write Model）
//!
//! 对接 `generation_events` 表，实现不可变事件日志的追加与查询。
//!
//! 表 Schema（来自 migrations/002_create_tables.sql）：
//! ```sql
//! CREATE TABLE generation_events (
//!     id           UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
//!     story_id     UUID NOT NULL REFERENCES stories(id),
//!     event_type   VARCHAR(50) NOT NULL,
//!     actor_id     UUID,
//!     payload      JSONB NOT NULL,
//!     vector_clock JSONB DEFAULT '{}',
//!     created_at   TIMESTAMPTZ DEFAULT NOW()
//! );
//! ```

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::events::NarrativeEvent;

/// Event Store 错误类型
#[derive(Debug, thiserror::Error)]
pub enum EventStoreError {
    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),

    #[error("事件序列化失败: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// 批量追加事件到 Event Store
///
/// 使用事务确保原子性：所有事件要么全部写入，要么全部回滚。
/// 写入 `generation_events` 表，保留完整的不可变审计日志。
pub async fn append_events(
    pool: &PgPool,
    story_id: Uuid,
    events: &[NarrativeEvent],
) -> Result<(), EventStoreError> {
    if events.is_empty() {
        return Ok(());
    }

    let mut tx = pool.begin().await?;

    for event in events {
        let meta = event.meta();
        let event_type = event.event_type_label();
        let payload = serde_json::to_value(event)?;

        sqlx::query(
            r#"
            INSERT INTO generation_events (id, story_id, event_type, actor_id, payload, vector_clock, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(meta.event_id)
        .bind(story_id)
        .bind(event_type)
        .bind(meta.actor_id)
        .bind(payload)
        .bind(&meta.vector_clock)
        .bind(meta.timestamp)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    tracing::debug!("已追加 {} 个事件到 Event Store (story_id={})", events.len(), story_id);
    Ok(())
}

/// 加载指定故事的全部事件（按时间顺序）
///
/// 从 `generation_events` 表中按 `created_at` 排序读取全部事件，
/// 用于从零重建聚合根（Event Sourcing 回放）。
pub async fn load_events(
    pool: &PgPool,
    story_id: Uuid,
) -> Result<Vec<NarrativeEvent>, EventStoreError> {
    let rows = sqlx::query_as::<_, EventRow>(
        r#"
        SELECT id, story_id, event_type, actor_id, payload, vector_clock, created_at
        FROM generation_events
        WHERE story_id = $1
        ORDER BY created_at ASC
        "#,
    )
    .bind(story_id)
    .fetch_all(pool)
    .await?;

    let events = rows
        .into_iter()
        .filter_map(|row| match serde_json::from_value::<NarrativeEvent>(row.payload) {
            Ok(event) => Some(event),
            Err(e) => {
                tracing::warn!("事件反序列化失败 (id={}): {}", row.id, e);
                None
            }
        })
        .collect();

    Ok(events)
}

/// 增量加载指定时间点之后的事件
///
/// 用于 Graph Projection Worker 的增量同步：
/// 只加载自上次同步以来的新事件，避免全量重建。
pub async fn load_events_since(
    pool: &PgPool,
    story_id: Uuid,
    since: DateTime<Utc>,
) -> Result<Vec<NarrativeEvent>, EventStoreError> {
    let rows = sqlx::query_as::<_, EventRow>(
        r#"
        SELECT id, story_id, event_type, actor_id, payload, vector_clock, created_at
        FROM generation_events
        WHERE story_id = $1 AND created_at > $2
        ORDER BY created_at ASC
        "#,
    )
    .bind(story_id)
    .bind(since)
    .fetch_all(pool)
    .await?;

    let events = rows
        .into_iter()
        .filter_map(|row| match serde_json::from_value::<NarrativeEvent>(row.payload) {
            Ok(event) => Some(event),
            Err(e) => {
                tracing::warn!("事件反序列化失败 (id={}): {}", row.id, e);
                None
            }
        })
        .collect();

    Ok(events)
}

/// 从 Event Store 加载聚合根
///
/// 等价于 `load_events` + `NarrativeGraph::replay`，
/// 是 CQRS 读取端的标准入口。
pub async fn load_aggregate(
    pool: &PgPool,
    story_id: Uuid,
) -> Result<crate::domain::aggregate::NarrativeGraph, EventStoreError> {
    let events = load_events(pool, story_id).await?;
    Ok(crate::domain::aggregate::NarrativeGraph::replay(story_id, &events))
}

/// 获取故事的最新事件时间戳
///
/// 用于 Projection Worker 记录同步进度。
pub async fn latest_event_timestamp(
    pool: &PgPool,
    story_id: Uuid,
) -> Result<Option<DateTime<Utc>>, EventStoreError> {
    let row: Option<(DateTime<Utc>,)> = sqlx::query_as(
        "SELECT MAX(created_at) FROM generation_events WHERE story_id = $1"
    )
    .bind(story_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(ts,)| ts))
}

/// 数据库行映射结构
#[derive(sqlx::FromRow)]
struct EventRow {
    id: Uuid,
    story_id: Uuid,
    event_type: String,
    actor_id: Option<Uuid>,
    payload: serde_json::Value,
    vector_clock: serde_json::Value,
    created_at: DateTime<Utc>,
}