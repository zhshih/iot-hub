use super::error::ApiError;
use crate::app_state::AppState;
use crate::auth::{
    jwt::{self, AuthRequest, AuthResponse, Claims},
    middleware::AuthUser,
};
use crate::domain::user::{SignupRequest, User, UserRole};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use chrono::Utc;
use password_hash::{SaltString, rand_core::OsRng};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

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
    if payload.username.is_empty() || payload.password.is_empty() || payload.email.is_empty() {
        return Err(ApiError::BadRequest(
            "Username, email, and password are required".to_string(),
        ));
    }

    let salt = SaltString::generate(&mut OsRng);
    let hashed_password = Argon2::default()
        .hash_password(payload.password.as_bytes(), &salt)
        .map_err(|_| ApiError::Internal("Password hashing failed".to_string()))?
        .to_string();

    let user_id = Uuid::new_v4();
    let created_at = Utc::now();

    let default_role = "Operator";
    let pool = state.db_pool.clone();

    sqlx::query!(
        r#"
        INSERT INTO users (id, username, email, hashed_password, role, created_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        user_id,
        payload.username,
        payload.email,
        hashed_password,
        default_role,
        created_at,
    )
    .execute(&*pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to create user: {}", e)))?;

    let token = jwt::encode_jwt(payload.username.clone())
        .map_err(|_| ApiError::Internal("Failed to generate token".to_string()))?;

    Ok(Json(json!({
        "message": "User created successfully",
        "token": token
    })))
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    if payload.username.is_empty() || payload.password.is_empty() {
        return Err(ApiError::BadRequest(
            "Username and password are required".to_string(),
        ));
    }

    let pool = state.db_pool.clone();
    let user = verify_user_credentials(&pool, &payload.username, &payload.password).await?;

    let token = jwt::encode_jwt(user.username.clone())
        .map_err(|_| ApiError::InternalServerError("Failed to create token".to_string()))?;
    Ok(Json(AuthResponse { token }))
}

async fn me(AuthUser(user): AuthUser, State(state): State<AppState>) -> Result<String, ApiError> {
    let pool = state.db_pool.clone();
    let user = validate_user_exists(&user, &pool).await?;
    Ok(format!(
        "current user: {}, email: {}, created at: {}",
        user.username, user.email, user.created_at
    ))
}

async fn health_check(State(state): State<AppState>) -> Json<String> {
    let pool = state.db_pool.clone();
    let row: (i32,) = sqlx::query_as("SELECT 1").fetch_one(&*pool).await.unwrap();

    Json(format!("DB says: {}", row.0))
}

async fn list_users(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<User>>, (StatusCode, String)> {
    let pool = state.db_pool.clone();
    validate_user_role(&user, &pool, &UserRole::Admin)
        .await
        .map_err(|e| {
            (
                StatusCode::FORBIDDEN,
                format!("Authorization failed: {:?}", e),
            )
        })?;

    let users = sqlx::query_as::<_, User>("SELECT * FROM users")
        .fetch_all(&*pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch users: {}", e),
            )
        })?;

    Ok(Json(users))
}

async fn validate_user_exists(claims: &Claims, pool: &PgPool) -> Result<User, ApiError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(&claims.sub)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::InternalServerError("DB error".to_string()))?;

    user.ok_or(ApiError::NotFound("User not found".to_string()))
}

async fn validate_user_role(
    claims: &Claims,
    pool: &PgPool,
    required_role: &UserRole,
) -> Result<User, ApiError> {
    let user = validate_user_exists(claims, pool).await?;

    if user.role != *required_role {
        return Err(ApiError::Forbidden("Insufficient permissions".to_string()));
    }

    Ok(user)
}

async fn verify_user_credentials(
    pool: &PgPool,
    username: &str,
    password: &str,
) -> Result<User, ApiError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::InternalServerError("DB error".into()))?
        .ok_or(ApiError::Unauthorized(
            "Invalid username or password".into(),
        ))?;

    let parsed_hash = PasswordHash::new(&user.hashed_password)
        .map_err(|_| ApiError::InternalServerError("Invalid hash format".into()))?;

    let is_valid = Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok();

    if !is_valid {
        return Err(ApiError::Unauthorized(
            "Invalid username or password".into(),
        ));
    }

    Ok(user)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{jwt::Claims, middleware::AuthUser};
    use axum::{Json, extract::State};
    use sqlx::{PgPool, postgres::PgPoolOptions};
    use std::sync::Arc;

    fn setup_env() {
        unsafe {
            std::env::set_var("JWT_SECRET", "test_secret");
            std::env::set_var(
                "DATABASE_URL",
                "postgres://test_user:test_password@localhost/iot_monitoring_test",
            )
        }
    }

    async fn setup_db_pool() -> Arc<PgPool> {
        let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for test");
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&db_url)
            .await
            .expect("Failed to connect to DB");

        sqlx::query("DELETE FROM users WHERE username IN ('testuser', 'admin', 'noroleuser')")
            .execute(&pool)
            .await
            .unwrap();

        let salt = SaltString::generate(&mut rand::thread_rng());
        let hashed_password = Argon2::default()
            .hash_password("password".as_bytes(), &salt)
            .unwrap()
            .to_string();

        sqlx::query!(
            r#"
            INSERT INTO users (id, username, email, hashed_password, role, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            Uuid::new_v4(),
            "admin",
            "admin@example.com",
            hashed_password,
            "Admin",
            Utc::now()
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query!(
            r#"
            INSERT INTO users (id, username, email, hashed_password, role, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            Uuid::new_v4(),
            "noroleuser",
            "norole@example.com",
            hashed_password,
            "Operator",
            Utc::now()
        )
        .execute(&pool)
        .await
        .unwrap();

        Arc::new(pool)
    }

    async fn clear_database() -> Result<(), sqlx::Error> {
        let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for test");
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&db_url)
            .await
            .expect("Failed to connect to DB");

        sqlx::query("TRUNCATE TABLE users CASCADE")
            .execute(&pool)
            .await?;
        println!("Database cleared");

        Ok(())
    }

    async fn init_test_app_state() -> AppState {
        setup_env();
        let pool = setup_db_pool().await;
        AppState {
            db_pool: pool.clone(),
        }
    }

    async fn with_test_app_state<F, Fut>(test: F)
    where
        F: FnOnce(AppState) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let app_state = init_test_app_state().await;
        test(app_state).await;
        clear_database().await.unwrap();
    }

    async fn with_test_db_pool<F, Fut>(test: F)
    where
        F: FnOnce(Arc<PgPool>) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        setup_env();
        let pool = setup_db_pool().await;
        test(pool.clone()).await;
        clear_database().await.unwrap();
    }

    #[tokio::test]
    async fn test_signup() {
        with_test_app_state(|app_state| async move {
            let payload = SignupRequest {
                username: "testuser".into(),
                email: "test@example.com".into(),
                password: "securepass".into(),
            };

            let result = signup(State(app_state), Json(payload)).await;
            assert!(result.is_ok());

            let json = result.unwrap().0;
            assert_eq!(json["message"], "User created successfully");
            assert!(json["token"].as_str().is_some());
        })
        .await;
    }

    #[tokio::test]
    async fn test_login_success() {
        with_test_app_state(|app_state| async move {
            let payload = AuthRequest {
                username: "admin".into(),
                password: "password".into(),
            };

            let result = login(State(app_state), Json(payload)).await;
            assert!(result.is_ok());

            let json = result.unwrap();
            assert!(!json.token.is_empty());
        })
        .await;
    }

    #[tokio::test]
    async fn test_login_fail_empty() {
        with_test_app_state(|app_state| async move {
            let payload = AuthRequest {
                username: "".into(),
                password: "".into(),
            };

            let result = login(State(app_state), Json(payload)).await;
            assert!(result.is_err());

            match result.unwrap_err() {
                ApiError::BadRequest(msg) => assert_eq!(msg, "Username and password are required"),
                err => panic!("expected BadRequest, got {:?}", err),
            }
        })
        .await;
    }

    #[tokio::test]
    async fn test_login_fail_invalid() {
        with_test_app_state(|app_state| async move {
            let payload = AuthRequest {
                username: "user".to_string(),
                password: "wrong".to_string(),
            };
            let result = login(State(app_state), Json(payload)).await;

            assert!(result.is_err());
            match result.unwrap_err() {
                ApiError::Unauthorized(msg) => {
                    assert_eq!(msg, "Invalid username or password");
                }
                err => panic!("expected Unauthorized, got {:?}", err),
            }
        })
        .await;
    }

    #[tokio::test]
    async fn test_me() {
        with_test_app_state(|app_state| async move {
            let claims = Claims {
                sub: "admin".to_string(),
                iat: 123,
                exp: 456,
            };
            let user = AuthUser(claims);

            let result = me(user, State(app_state)).await;

            assert!(result.is_ok());
            let text = result.unwrap();
            assert!(text.contains("current user: admin"));
        })
        .await;
    }

    #[tokio::test]
    async fn test_health_check() {
        setup_env();
        let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for test");
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&db_url)
            .await
            .unwrap();

        let app_state = AppState {
            db_pool: Arc::new(pool),
        };

        let result = health_check(State(app_state)).await;
        assert_eq!(result.0, "DB says: 1");
    }

    #[tokio::test]
    async fn test_list_users() {
        with_test_app_state(|app_state| async move {
            let claims = Claims {
                sub: "admin".to_string(),
                iat: 0,
                exp: 9999999999,
            };
            let auth_user = AuthUser(claims);

            let result = list_users(auth_user, State(app_state)).await;

            assert!(result.is_ok());
            let users = result.unwrap().0;
            assert!(!users.is_empty());
            assert!(users.iter().any(|u| u.username == "admin"));
        })
        .await;
    }

    #[tokio::test]
    async fn test_validate_user_exists_success() {
        with_test_db_pool(|pool| async move {
            let claims = Claims {
                sub: "admin".to_string(),
                iat: 0,
                exp: 9999999999,
            };

            let user = validate_user_exists(&claims, &pool).await.unwrap();
            assert_eq!(user.username, "admin");
        })
        .await;
    }

    #[tokio::test]
    async fn test_validate_user_exists_fail() {
        with_test_db_pool(|pool| async move {
            let claims = Claims {
                sub: "unknownuser".to_string(),
                iat: 0,
                exp: 9999999999,
            };

            let result = validate_user_exists(&claims, &pool).await;
            assert!(result.is_err());
            match result.unwrap_err() {
                ApiError::NotFound(msg) => {
                    assert_eq!(msg, "User not found");
                }
                err => panic!("expected NotFound, got {:?}", err),
            }
        })
        .await;
    }

    #[tokio::test]
    async fn test_validate_user_role_success() {
        with_test_db_pool(|pool| async move {
            let claims = Claims {
                sub: "admin".to_string(),
                iat: 0,
                exp: 9999999999,
            };

            let user = validate_user_role(&claims, &pool, &UserRole::Admin)
                .await
                .unwrap();
            assert_eq!(user.username, "admin");
            assert_eq!(user.role, UserRole::Admin);
        })
        .await;
    }

    #[tokio::test]
    async fn test_validate_user_role_fail_insufficient_permissions() {
        with_test_db_pool(|pool| async move {
            let claims = Claims {
                sub: "noroleuser".to_string(),
                iat: 0,
                exp: 9999999999,
            };

            let result = validate_user_role(&claims, &pool, &UserRole::Admin).await;
            assert!(result.is_err());
            match result.unwrap_err() {
                ApiError::Forbidden(msg) => {
                    assert_eq!(msg, "Insufficient permissions");
                }
                err => panic!("expected Forbidden, got {:?}", err),
            }
        })
        .await;
    }

    #[tokio::test]
    async fn test_verify_user_credentials_success() {
        with_test_db_pool(|pool| async move {
            let user = verify_user_credentials(&pool, "admin", "password")
                .await
                .unwrap();
            assert_eq!(user.username, "admin");
        })
        .await;
    }

    #[tokio::test]
    async fn test_verify_user_credentials_fail_wrong_password() {
        with_test_db_pool(|pool| async move {
            let result = verify_user_credentials(&pool, "admin", "wrongpassword").await;
            assert!(result.is_err());
            match result.unwrap_err() {
                ApiError::Unauthorized(msg) => {
                    assert_eq!(msg, "Invalid username or password");
                }
                err => panic!("expected Unauthorized, got {:?}", err),
            }
        })
        .await;
    }

    #[tokio::test]
    async fn test_verify_user_credentials_fail_unknown_user() {
        with_test_db_pool(|pool| async move {
            let result = verify_user_credentials(&pool, "unknownuser", "password").await;
            assert!(result.is_err());
            match result.unwrap_err() {
                ApiError::Unauthorized(msg) => {
                    assert_eq!(msg, "Invalid username or password");
                }
                err => panic!("expected Unauthorized, got {:?}", err),
            }
        })
        .await;
    }
}
