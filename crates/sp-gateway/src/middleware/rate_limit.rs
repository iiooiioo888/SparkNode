//! 三维滑动窗口限流中间件
//!
//! 基于 "IP + User ID + 角色等级" 的多维限流策略，
//! 通过 Redis/Dragonfly INCR + EXPIRE 实现高併发滑动窗口计数。
//!
//! 限流阈值按角色分级:
//! - `free`:    30 req/min
//! - `premium`: 100 req/min
//! - `admin`:   无限制

use axum::{
    extract::ConnectInfo,
    http::Request,
    middleware::Next,
    response::Response,
};
use std::net::SocketAddr;

use super::auth_layer::AuthUser;

/// 限流阈值配置（每分钟请求数）
const RATE_LIMIT_FREE: u64 = 30;
const RATE_LIMIT_PREMIUM: u64 = 100;
// admin 不限流

/// 滑动窗口大小（秒）
const WINDOW_SECS: u64 = 60;

/// 三维限流中间件
///
/// 提取客户端 IP、用户 ID 和角色等级，组合为限流键，
/// 在 Redis/Dragonfly 中执行滑动窗口计数。
///
/// **前置条件**: 此中间件必须在 `auth_middleware` 之后挂载，
/// 以确保 `Request::extensions()` 中已注入 `AuthUser`。
pub async fn rate_limit_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let ip = addr.ip().to_string();

    // 从 auth_middleware 注入的扩展中提取用户 ID
    let user_id = req
        .extensions()
        .get::<AuthUser>()
        .map(|u| u.0.to_string())
        .unwrap_or_else(|| "anonymous".to_string());

    // 获取限流阈值（基于角色等级）
    // TODO: 当 auth_layer 注入 Claims 包含 role 字段后，从此处提取角色
    let role = extract_role(&req).unwrap_or_else(|| "free".to_string());
    let limit = role_limit(&role);

    // 构造三维限流键: rate_limit:{role}:{user_id}:{ip}
    let key = format!("rate_limit:{}:{}:{}", role, user_id, ip);

    // 执行滑动窗口限流检查
    match check_rate_limit(&key, limit).await {
        Ok(allowed) => {
            if !allowed {
                // 触发限流 — 返回 429
                let error_body = serde_json::json!({
                    "error": "请求过于频繁",
                    "code": 429u16,
                    "retry_after": WINDOW_SECS,
                });
                return Response::builder()
                    .status(axum::http::StatusCode::TOO_MANY_REQUESTS)
                    .header("Retry-After", WINDOW_SECS.to_string())
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::to_string(&error_body).unwrap_or_default(),
                    ))
                    .unwrap_or_else(|_| Response::default());
            }
        }
        Err(e) => {
            // Redis 故障时降级放行（宁可放过也不阻断服务）
            tracing::warn!("限流服务异常，降级放行: {}", e);
        }
    }

    next.run(req).await
}

/// 从请求扩展中提取角色等级
///
/// 当前实现: 若 Claims 包含 role 字段则提取，否则返回 "free"。
/// TODO: 待 auth_layer 注入完整 Claims 后，从此处读取真实角色。
fn extract_role(req: &Request<axum::body::Body>) -> Option<String> {
    // 预留: 从 Claims 扩展中读取 role
    // req.extensions().get::<Claims>().map(|c| c.role.clone())
    None
}

/// 根据角色等级返回限流阈值
fn role_limit(role: &str) -> u64 {
    match role {
        "admin" => u64::MAX,   // 管理员不限流
        "premium" => RATE_LIMIT_PREMIUM,
        _ => RATE_LIMIT_FREE,  // 默认 free 等级
    }
}

/// Redis/Dragonfly 滑动窗口限流检查
///
/// 使用 INCR + EXPIRE 实现固定窗口计数器（近似滑动窗口）。
///
/// **对接 Dragonfly**: Dragonfly 完全兼容 Redis 协议，
/// 可直接使用相同的 `redis::Cmd` 调用，无需额外适配。
///
/// # 伪代码（生产实现）
/// ```ignore
/// let count: u64 = redis::cmd("INCR")
///     .arg(&key)
///     .query_async(&mut conn)
///     .await?;
///
/// if count == 1 {
///     // 首次访问，设置窗口过期
///     redis::cmd("EXPIRE")
///         .arg(&key)
///         .arg(WINDOW_SECS)
///         .query_async(&mut conn)
///         .await?;
/// }
///
/// Ok(count <= limit)
/// ```
async fn check_rate_limit(key: &str, limit: u64) -> Result<bool, String> {
    // ── 骨架实现（Redis/Dragonfly 对接点）──
    // 当 AppState 中的 redis::ConnectionManager 传入后，
    // 替换为真实的 Redis INCR + EXPIRE 调用。
    //
    // 暂时返回 true（放行），避免在 Redis 未就绪时阻断开发。
    //
    // 完整实现示例:
    //
    // use redis::AsyncCommands;
    // let mut conn = state.redis.clone();
    // let count: u64 = conn.incr(&key, 1u64).await
    //     .map_err(|e| format!("Redis INCR 失败: {}", e))?;
    // if count == 1 {
    //     let _: () = conn.expire(&key, WINDOW_SECS as i64).await
    //         .map_err(|e| format!("Redis EXPIRE 失败: {}", e))?;
    // }
    // Ok(count <= limit)

    let _ = (key, limit); // 抑制未使用警告
    Ok(true)
}