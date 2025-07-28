use crate::auth::{
    jwt::{self, AuthRequest, AuthResponse},
    middleware::AuthUser,
};
use axum::{
    Json, Router,
    http::StatusCode,
    routing::{get, post},
};

pub fn routes() -> Router {
    Router::new()
        .route("/signup", post(signup))
        .route("/login", post(login))
        .route("/me", get(me))
}

async fn signup() -> &'static str {
    "User signed up"
}

async fn login(
    Json(payload): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    if payload.username.is_empty() || payload.password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Username and password are required".to_string(),
        ));
    }

    // FIXME
    if payload.username != "admin" || payload.password != "password" {
        return Err((StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()));
    }

    let token = jwt::encode_jwt(payload.username).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create token".to_string(),
        )
    })?;
    Ok(Json(AuthResponse { token }))
}

async fn me(AuthUser(user): AuthUser) -> String {
    format!(
        "current user: {}, issue at {}, expired at {}",
        user.sub, user.iat, user.exp
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{jwt::Claims, middleware::AuthUser};
    use axum::Json;
    use axum::http::StatusCode;

    fn setup_env() {
        unsafe {
            std::env::set_var("JWT_SECRET", "test_secret");
        }
    }

    #[tokio::test]
    async fn test_signup() {
        setup_env();
        let result = signup().await;
        assert_eq!(result, "User signed up");
    }

    #[tokio::test]
    async fn test_login_success() {
        setup_env();
        let payload = AuthRequest {
            username: "admin".to_string(),
            password: "password".to_string(),
        };
        let result = login(Json(payload)).await;

        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(!json.token.is_empty());
    }

    #[tokio::test]
    async fn test_login_fail_empty() {
        setup_env();
        let payload = AuthRequest {
            username: "".to_string(),
            password: "".to_string(),
        };
        let result = login(Json(payload)).await;

        assert!(result.is_err());
        let (status, msg) = result.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(msg, "Username and password are required");
    }

    #[tokio::test]
    async fn test_login_fail_invalid() {
        setup_env();
        let payload = AuthRequest {
            username: "user".to_string(),
            password: "wrong".to_string(),
        };
        let result = login(Json(payload)).await;

        assert!(result.is_err());
        let (status, msg) = result.unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(msg, "Invalid credentials");
    }

    #[tokio::test]
    async fn test_me() {
        setup_env();
        let claims = Claims {
            sub: "admin".to_string(),
            iat: 123,
            exp: 456,
        };
        let user = AuthUser(claims);
        let result = me(user).await;
        assert_eq!(result, "current user: admin, issue at 123, expired at 456");
    }
}
