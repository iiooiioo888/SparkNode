//! # 灵犀节点 (SparkNode) API 网关
//!
//! 基于 Axum 的高性能 API 网关，提供:
//! - RESTful API (故事、节点、边的 CRUD)
//! - WebSocket 实时协作 (PulseStream)
//! - gRPC 服务端 (连接 Python AI 层)
//! - JWT 认证中间件

mod config;
mod error;
mod routes;
mod middleware;
mod services;
mod ws;
mod grpc;

use axum::{Router, routing::get};
use tower_http::cors::{CorsLayer, Any};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use std::net::SocketAddr;

use config::GatewayConfig;
use grpc::GrpcPool;

/// 应用共享状态
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub redis: redis::aio::ConnectionManager,
    pub config: GatewayConfig,
    /// gRPC 连接池（AI 推理 + Memgraph 状态机）
    /// 为 Option：当底层服务未就绪时网关仍可正常启动
    pub grpc_pool: Option<GrpcPool>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── 初始化日志 ──
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info,sp_gateway=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // ── 加载配置 ──
    let config = GatewayConfig::from_env()?;
    tracing::info!("灵犀节点网关启动中...");
    tracing::info!("监听地址: {}:{}", config.host, config.port);

    // ── 连接数据库 ──
    let db = sqlx::PgPool::connect(&config.database_url)
        .await
        .map_err(|e| anyhow::anyhow!("PostgreSQL 连接失败: {}", e))?;
    tracing::info!("✓ PostgreSQL 已连接");

    // ── 连接 Redis ──
    let redis_client = redis::Client::open(config.redis_url.clone())?;
    let redis = redis::aio::ConnectionManager::new(redis_client).await?;
    tracing::info!("✓ Redis 已连接");

    // ── 初始化 gRPC 连接池 ──
    let grpc_pool = match GrpcPool::connect(
        &config.llm_router_url,
        "http://localhost:50052", // Memgraph 状态机（预留）
    ).await {
        Ok(pool) => {
            tracing::info!("✓ gRPC 连接池已就绪");
            Some(pool)
        }
        Err(e) => {
            tracing::warn!("gRPC 连接池初始化失败（降级运行）: {}", e);
            None
        }
    };

    // ── 构建共享状态 ──
    let state = AppState {
        db,
        redis,
        config: config.clone(),
        grpc_pool,
    };

    // ── 构建路由 ──
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api = Router::new()
        .nest("/stories", routes::stories::router())
        .merge(routes::collapse::router())
        .nest("/generate", routes::generate::router())
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::rate_limit_middleware,
        ))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::auth_middleware,
        ));

    let app = Router::new()
        .route("/api/v1/health", get(routes::health::health_check))
        .nest("/api/v1", api)
        .route("/ws/stories/:story_id", get(ws::handler::ws_handler))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state.clone());

    // ── 启动 gRPC 服务端 (NarrativeService) ──
    let grpc_addr = format!("{}:{}", config.host, config.grpc_port)
        .parse::<SocketAddr>()
        .expect("无效的 gRPC 地址");
    let grpc_state = state.clone();
    tokio::spawn(async move {
        use sp_common::narrative_proto::narrative_service_server::NarrativeServiceServer;
        use grpc::narrative_server::NarrativeServiceImpl;

        let service = NarrativeServiceImpl { state: grpc_state };

        match tonic::transport::Server::builder()
            .add_service(NarrativeServiceServer::new(service))
            .serve(grpc_addr)
            .await
        {
            Ok(_) => tracing::info!("gRPC 服务端已停止"),
            Err(e) => tracing::error!("gRPC 服务端错误: {}", e),
        }
    });
    tracing::info!("✓ gRPC 服务端已启动 @ {}", grpc_addr);

    tracing::info!("✓ 限流中间件已挂载 (IP + User + Role 三维滑动窗口)");

    // ── 启动 HTTP 服务 ──
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("🚀 灵犀节点网关已就绪 @ {}", addr);
    tracing::info!("WebSocket 协作 @ ws://{}/ws/stories/:story_id", addr);

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;

    Ok(())
}