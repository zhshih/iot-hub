use super::error::ApiError;
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum ApiResponse<T> {
    Success { data: T },
    Error { message: String },
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        ApiResponse::Success { data }
    }

    pub fn error(message: impl Into<String>) -> Self {
        ApiResponse::Error {
            message: message.into(),
        }
    }
}

pub type HandlerResult<T> = Result<Json<ApiResponse<T>>, ApiError>;
