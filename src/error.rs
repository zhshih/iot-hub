use crate::api::error::ApiError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TokenError {
    #[error("Token parsing failed: {0}")]
    InvalidKey(String),

    #[error("Token error - Token expired")]
    Expired,

    #[error("Token error - Generation failed: {0}")]
    GenerationFailed(String),
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Validation error - Invalid input: {0}")]
    InvalidInput(String),

    #[error("Validation error - Missing field: {0}")]
    MissingField(String),

    #[error("Validation error - Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Validation error - Parsed error: {0}")]
    ParsedError(String),
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Environment variable error: {0}")]
    EnvVarError(String),

    #[error(transparent)]
    ValidationError(ValidationError),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Missing argument: {0}")]
    MissingArgument(String),

    #[error(transparent)]
    TokenError(TokenError),

    #[error("Health check failed")]
    HealthCheckFailed,
}

impl From<AppError> for ApiError {
    fn from(err: AppError) -> Self {
        match err {
            AppError::ValidationError(ve) => match ve {
                ValidationError::InvalidInput(msg)
                | ValidationError::MissingField(msg)
                | ValidationError::PermissionDenied(msg) => ApiError::BadRequest(msg),
                ValidationError::ParsedError(msg) => ApiError::InternalServerError(msg),
            },
            AppError::DatabaseError(msg) | AppError::EnvVarError(msg) => {
                ApiError::InternalServerError(msg)
            }
            AppError::NotFound(msg) => ApiError::NotFound(msg),
            AppError::MissingArgument(msg) => ApiError::BadRequest(msg),
            AppError::TokenError(e) => ApiError::Unauthorized(e.to_string()),
            AppError::HealthCheckFailed => {
                ApiError::InternalServerError("Health check failed".into())
            }
        }
    }
}
