use super::error::ApiError;
use crate::{
    app_state::AppState,
    auth::{
        jwt::{AuthRequest, AuthResponse},
        middleware::AuthUser,
    },
    domain::user::{SignupRequest, User},
    service::user_service::UserService,
};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use serde_json::json;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_users))
        .route("/signup", post(signup))
        .route("/login", post(login))
        .route("/me", get(me))
        .route("/health", get(health_check))
}

async fn signup(
    State(state): State<AppState>,
    Json(payload): Json<SignupRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let service = UserService::new(state.db_pool.clone());
    let token = service.signup(payload).await?;
    Ok(Json(json!({
        "message": "User created successfully",
        "token": token
    })))
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let service = UserService::new(state.db_pool.clone());
    let token = service.login(payload).await?;
    Ok(Json(AuthResponse { token }))
}

async fn me(AuthUser(claims): AuthUser, State(state): State<AppState>) -> Result<String, ApiError> {
    let service = UserService::new(state.db_pool.clone());
    service.get_current_user_info(&claims).await
}

async fn health_check(State(state): State<AppState>) -> Result<Json<String>, ApiError> {
    let service = UserService::new(state.db_pool.clone());
    let msg = service.health_check().await?;
    Ok(Json(msg))
}

async fn list_users(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<User>>, (StatusCode, String)> {
    let service = UserService::new(state.db_pool.clone());
    service
        .list_users(&claims)
        .await
        .map(Json)
        .map_err(|e| match e {
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            ApiError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            other => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Unexpected error: {:?}", other),
            ),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{jwt::Claims, middleware::AuthUser};
    use argon2::{Argon2, PasswordHasher};
    use password_hash::{SaltString, rand_core::OsRng};
    use serial_test::serial;
    use sqlx::PgPool;
    use uuid::Uuid;

    fn setup_env() {
        unsafe {
            std::env::set_var("JWT_SECRET", "test_secret");
        }
    }

    async fn setup_test_state() -> AppState {
        setup_env();
        let database_url = "postgres://test_user:test_password@localhost/iot_monitoring_test";
        let pool = PgPool::connect(database_url)
            .await
            .expect("DB connect failed");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        sqlx::query("TRUNCATE TABLE users CASCADE")
            .execute(&pool)
            .await
            .expect("Failed to truncate");

        AppState { db_pool: pool }
    }

    fn mock_auth_user(sub: &str) -> AuthUser {
        AuthUser(Claims {
            sub: sub.to_string(),
            exp: 0,
            iat: 0,
        })
    }

    async fn insert_test_user(
        pool: &PgPool,
        username: &str,
        email: &str,
        password: &str,
        role: &str,
    ) {
        let salt = SaltString::generate(&mut OsRng);
        let hashed_password = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        let user_id = Uuid::new_v4();
        let created_at = chrono::Utc::now();

        sqlx::query!(
            "INSERT INTO users (id, username, email, hashed_password, role, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
            user_id,
            username,
            email,
            hashed_password,
            role,
            created_at,
        )
        .execute(pool)
        .await
        .expect("Insert test user failed");
    }

    #[tokio::test]
    #[serial]
    async fn test_signup() {
        let state = setup_test_state().await;

        let payload = SignupRequest {
            username: "newuser".into(),
            email: "newuser@example.com".into(),
            password: "password123".into(),
        };

        let resp = signup(State(state), Json(payload)).await.unwrap();

        assert_eq!(resp.0["message"], "User created successfully");
        assert!(resp.0.get("token").is_some());
    }

    #[tokio::test]
    #[serial]
    async fn test_login() {
        let state = setup_test_state().await;

        insert_test_user(
            &state.db_pool,
            "loginuser",
            "login@example.com",
            "password123",
            "Operator",
        )
        .await;

        let payload = AuthRequest {
            username: "loginuser".into(),
            password: "password123".into(),
        };

        let resp = login(State(state), Json(payload)).await.unwrap();

        assert!(!resp.token.is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn test_me() {
        let state = setup_test_state().await;

        insert_test_user(
            &state.db_pool,
            "meuser",
            "meuser@example.com",
            "password123",
            "Operator",
        )
        .await;

        let user = mock_auth_user("meuser");

        let resp = me(AuthUser(user.0), State(state)).await.unwrap();

        assert!(resp.contains("meuser@example.com"));
    }

    #[tokio::test]
    async fn test_health_check() {
        let state = setup_test_state().await;

        let resp = health_check(State(state)).await.unwrap();

        assert!(resp.contains("DB says"));
    }

    #[tokio::test]
    #[serial]
    async fn test_list_users() {
        let state = setup_test_state().await;

        insert_test_user(
            &state.db_pool,
            "adminuser",
            "admin@example.com",
            "adminpass",
            "Admin",
        )
        .await;
        insert_test_user(
            &state.db_pool,
            "regularuser",
            "user@example.com",
            "userpass",
            "Operator",
        )
        .await;

        let admin_user = mock_auth_user("adminuser");
        let users = list_users(AuthUser(admin_user.0), State(state))
            .await
            .unwrap();

        assert!(users.iter().any(|u| u.username == "adminuser"));
        assert!(users.iter().any(|u| u.username == "regularuser"));
    }
}
