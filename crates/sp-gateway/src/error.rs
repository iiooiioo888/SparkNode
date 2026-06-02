//! Axum HTTP 錯誤適配

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use sp_common::error::SpError;

/// 包裝 SpError 以實現外部 trait
pub struct SpErrorWrapper(pub SpError);

impl IntoResponse for SpErrorWrapper {
    fn into_response(self) -> Response {
        let status =
            StatusCode::from_u16(self.0.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = Json(json!({
            "error": self.0.to_string(),
            "code": self.0.status_code(),
        }));
        (status, body).into_response()
    }
}

impl From<SpError> for SpErrorWrapper {
    fn from(err: SpError) -> Self {
        SpErrorWrapper(err)
    }
}
