//! 网关配置管理

use serde::Deserialize;

/// 网关配置
#[derive(Debug, Clone, Deserialize)]
pub struct GatewayConfig {
    pub host: String,
    pub port: u16,
    pub ws_port: u16,
    pub grpc_port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_expiration_hours: i64,
    pub llm_router_url: String,
    /// 開發環境預設作者（對應 migrations/003_seed_dev_user.sql）
    pub dev_user_id: uuid::Uuid,
}

impl GatewayConfig {
    /// 从环境变量加载配置
    pub fn from_env() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        Ok(Self {
            host: std::env::var("GATEWAY_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("GATEWAY_PORT")
                .unwrap_or_else(|_| "3001".to_string())
                .parse()?,
            ws_port: std::env::var("GATEWAY_WS_PORT")
                .unwrap_or_else(|_| "3002".to_string())
                .parse()?,
            grpc_port: std::env::var("GATEWAY_GRPC_PORT")
                .unwrap_or_else(|_| "3003".to_string())
                .parse()?,
            database_url: format!(
                "postgres://{}:{}@{}:{}/{}",
                std::env::var("POSTGRES_USER").unwrap_or_else(|_| "spark".to_string()),
                std::env::var("POSTGRES_PASSWORD").unwrap_or_else(|_| "spark_dev_password".to_string()),
                std::env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string()),
                std::env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string()),
                std::env::var("POSTGRES_DB").unwrap_or_else(|_| "sparknode".to_string()),
            ),
            redis_url: format!(
                "redis://{}:{}",
                std::env::var("REDIS_HOST").unwrap_or_else(|_| "localhost".to_string()),
                std::env::var("REDIS_PORT").unwrap_or_else(|_| "6379".to_string()),
            ),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "dev_secret".to_string()),
            jwt_expiration_hours: std::env::var("JWT_EXPIRATION_HOURS")
                .unwrap_or_else(|_| "72".to_string())
                .parse()?,
            llm_router_url: format!(
                "http://{}:{}",
                std::env::var("LLM_ROUTER_HOST").unwrap_or_else(|_| "localhost".to_string()),
                std::env::var("LLM_ROUTER_PORT").unwrap_or_else(|_| "8001".to_string()),
            ),
            dev_user_id: std::env::var("DEV_USER_ID")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| {
                    uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001")
                        .expect("valid dev user uuid")
                }),
        })
    }
}