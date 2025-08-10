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
    use argon2::{Argon2, PasswordHasher};
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
        response::Response,
    };
    use password_hash::{SaltString, rand_core::OsRng};
    use serde_json::{Value, json};
    use sqlx::{Executor, PgPool};
    use tower::util::ServiceExt;
    use uuid::Uuid;

    async fn setup_app() -> (Router, PgPool) {
        let state = setup_test_state().await;
        let app = routes().with_state(state.clone());
        (app, state.db_pool)
    }

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

        pool.execute("TRUNCATE TABLE users CASCADE")
            .await
            .expect("Failed to truncate");

        AppState { db_pool: pool }
    }

    async fn do_login(pool: &PgPool, username: &str, password: &str) -> String {
        let app = routes().with_state(AppState {
            db_pool: pool.clone(),
        });
        let payload = json!({ "username": username, "password": password });
        let resp = post_json(app, "/login", payload).await;
        let json: Value = resp_json(resp).await;
        json["token"].as_str().unwrap().to_string()
    }

    async fn post_json(app: Router, uri: &str, payload: Value) -> Response {
        let req = Request::builder()
            .uri(uri)
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap();
        app.oneshot(req).await.unwrap()
    }

    async fn get_with_auth(app: Router, uri: &str, token: &str) -> Response {
        let req = Request::builder()
            .uri(uri)
            .method("GET")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        app.oneshot(req).await.unwrap()
    }

    async fn resp_json<T: serde::de::DeserializeOwned>(resp: Response) -> T {
        let body = to_bytes(resp.into_body(), 1024 * 16).await.unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    async fn resp_text(resp: Response) -> String {
        let body = to_bytes(resp.into_body(), 1024 * 16).await.unwrap();
        String::from_utf8(body.to_vec()).unwrap()
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
    async fn test_signup() {
        let (app, _) = setup_app().await;
        let payload = json!({
            "username": "newuser",
            "email": "newuser@example.com",
            "password": "password123"
        });

        let resp = post_json(app, "/signup", payload).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let json: Value = resp_json(resp).await;
        assert_eq!(json["message"], "User created successfully");
        assert!(json.get("token").is_some());
    }

    #[tokio::test]
    async fn test_login() {
        let (app, pool) = setup_app().await;
        insert_test_user(
            &pool,
            "loginuser",
            "login@example.com",
            "password123",
            "Operator",
        )
        .await;

        let payload = json!({
            "username": "loginuser",
            "password": "password123"
        });
        let resp = post_json(app, "/login", payload).await;

        assert_eq!(resp.status(), StatusCode::OK);
        let json: Value = resp_json(resp).await;
        assert!(json.get("token").is_some());
    }

    #[tokio::test]
    async fn test_me() {
        let (app, pool) = setup_app().await;
        insert_test_user(
            &pool,
            "meuser",
            "meuser@example.com",
            "password123",
            "Operator",
        )
        .await;
        let token = do_login(&pool, "meuser", "password123").await;

        let resp = get_with_auth(app, "/me", &token).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body = resp_text(resp).await;
        assert!(body.contains("meuser@example.com"));
    }

    #[tokio::test]
    async fn test_health_check() {
        let (app, _) = setup_app().await;

        let req = Request::builder()
            .uri("/health")
            .method("GET")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp_text(resp).await;
        assert!(body.contains("DB says"));
    }

    #[tokio::test]
    async fn test_list_users() {
        let (app, pool) = setup_app().await;
        insert_test_user(
            &pool,
            "adminuser",
            "admin@example.com",
            "adminpass",
            "Admin",
        )
        .await;
        insert_test_user(
            &pool,
            "regularuser",
            "user@example.com",
            "userpass",
            "Operator",
        )
        .await;

        let token = do_login(&pool, "adminuser", "adminpass").await;
        let resp = get_with_auth(app, "/", &token).await;

        assert_eq!(resp.status(), StatusCode::OK);
        let users: Vec<User> = resp_json(resp).await;
        assert!(users.iter().any(|u| u.username == "adminuser"));
        assert!(users.iter().any(|u| u.username == "regularuser"));
    }
}
