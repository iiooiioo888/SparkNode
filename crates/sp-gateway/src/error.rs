//! Axum HTTP 錯誤適配

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use sp_common::error::SpError;

impl IntoResponse for SpError {
    fn into_response(self) -> Response {
        let status =
            StatusCode::from_u16(self.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = Json(json!({
            "error": self.to_string(),
            "code": self.status_code(),
        }));
        (status, body).into_response()
    }
}
