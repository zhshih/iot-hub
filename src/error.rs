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
                ValidationError::InvalidInput(msg) | ValidationError::MissingField(msg) => {
                    ApiError::BadRequest(msg)
                }
                ValidationError::PermissionDenied(msg) => ApiError::Forbidden(msg),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_invalid_input_maps_to_bad_request() {
        let err = AppError::ValidationError(ValidationError::InvalidInput("bad input".into()));
        match ApiError::from(err) {
            ApiError::BadRequest(msg) => assert_eq!(msg, "bad input"),
            other => panic!("expected BadRequest, got {other:?}"),
        }
    }

    #[test]
    fn validation_missing_field_maps_to_bad_request() {
        let err = AppError::ValidationError(ValidationError::MissingField("name".into()));
        match ApiError::from(err) {
            ApiError::BadRequest(msg) => assert_eq!(msg, "name"),
            other => panic!("expected BadRequest, got {other:?}"),
        }
    }

    #[test]
    fn validation_permission_denied_maps_to_forbidden() {
        let err =
            AppError::ValidationError(ValidationError::PermissionDenied("not allowed".into()));
        match ApiError::from(err) {
            ApiError::Forbidden(msg) => assert_eq!(msg, "not allowed"),
            other => panic!("expected Forbidden, got {other:?}"),
        }
    }

    #[test]
    fn validation_parsed_error_maps_to_internal_server_error() {
        let err = AppError::ValidationError(ValidationError::ParsedError("bad hash".into()));
        match ApiError::from(err) {
            ApiError::InternalServerError(msg) => assert_eq!(msg, "bad hash"),
            other => panic!("expected InternalServerError, got {other:?}"),
        }
    }

    #[test]
    fn database_error_maps_to_internal_server_error() {
        let err = AppError::DatabaseError("connection lost".into());
        match ApiError::from(err) {
            ApiError::InternalServerError(msg) => assert_eq!(msg, "connection lost"),
            other => panic!("expected InternalServerError, got {other:?}"),
        }
    }

    #[test]
    fn env_var_error_maps_to_internal_server_error() {
        let err = AppError::EnvVarError("JWT_SECRET missing".into());
        match ApiError::from(err) {
            ApiError::InternalServerError(msg) => assert_eq!(msg, "JWT_SECRET missing"),
            other => panic!("expected InternalServerError, got {other:?}"),
        }
    }

    #[test]
    fn not_found_maps_to_not_found() {
        let err = AppError::NotFound("device".into());
        match ApiError::from(err) {
            ApiError::NotFound(msg) => assert_eq!(msg, "device"),
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn missing_argument_maps_to_bad_request() {
        let err = AppError::MissingArgument("username".into());
        match ApiError::from(err) {
            ApiError::BadRequest(msg) => assert_eq!(msg, "username"),
            other => panic!("expected BadRequest, got {other:?}"),
        }
    }

    #[test]
    fn token_error_maps_to_unauthorized() {
        let err = AppError::TokenError(TokenError::Expired);
        match ApiError::from(err) {
            ApiError::Unauthorized(msg) => assert_eq!(msg, TokenError::Expired.to_string()),
            other => panic!("expected Unauthorized, got {other:?}"),
        }
    }

    #[test]
    fn health_check_failed_maps_to_internal_server_error() {
        let err = AppError::HealthCheckFailed;
        match ApiError::from(err) {
            ApiError::InternalServerError(msg) => assert_eq!(msg, "Health check failed"),
            other => panic!("expected InternalServerError, got {other:?}"),
        }
    }
}
