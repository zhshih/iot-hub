use super::error::ApiError;
use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum ApiResponse<T> {
    Success { data: T },
    Error { code: String, message: String },
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        ApiResponse::Success { data }
    }

    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        ApiResponse::Error {
            code: code.into(),
            message: message.into(),
        }
    }
}

pub type HandlerResult<T> = Result<Json<ApiResponse<T>>, ApiError>;
