//! JWT 認證層（可選：無 token 時使用開發預設用戶）

use axum::{
    body::Body,
    extract::State,
    http::{header::AUTHORIZATION, Request},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::AppState;
use super::auth::verify_token;

/// 請求上下文中的當前用戶 ID
#[derive(Clone, Copy, Debug)]
pub struct AuthUser(pub Uuid);

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let user_id = extract_user_id(&req, &state.config.jwt_secret)
        .unwrap_or(state.config.dev_user_id);
    req.extensions_mut().insert(AuthUser(user_id));
    next.run(req).await
}

fn extract_user_id(req: &Request<Body>, secret: &str) -> Option<Uuid> {
    let header = req.headers().get(AUTHORIZATION)?;
    let value = header.to_str().ok()?;
    let token = value.strip_prefix("Bearer ")?;
    verify_token(token, secret).ok().map(|c| c.sub)
}
