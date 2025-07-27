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
