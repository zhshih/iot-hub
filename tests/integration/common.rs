use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use iot_hub::app_state::AppState;
use iot_hub::auth::extractor::DEFAULT_MOCK_USER_ID;
use iot_hub::domain::ids::{DeviceId, UserId};
use serde::Serialize;
use serde_json::Value;
use sqlx::{self, Executor, PgPool};
use std::str::FromStr;
use tower::ServiceExt;
use utoipa_axum::router::OpenApiRouter;

pub const TEST_DATABASE_URL: &str =
    "postgres://test_user:test_password@localhost/iot_monitoring_test";

pub struct TestApp {
    pub app: Router,
}

impl TestApp {
    pub async fn new(table: &'static str, routes: OpenApiRouter<AppState>) -> Self {
        let app_state = setup_test_state(table).await;
        let router: Router<AppState> = routes.into();
        let app = router.with_state(app_state);
        Self { app }
    }

    pub fn app(&self) -> &Router {
        &self.app
    }
}

pub fn setup_env() {
    unsafe {
        std::env::set_var("JWT_SECRET", "test_secret");
    }
}

pub async fn setup_test_state(table: &str) -> AppState {
    setup_env();

    let database_url = TEST_DATABASE_URL;

    let pool = PgPool::connect(database_url)
        .await
        .expect("failed to connect to test database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");

    let query = format!("TRUNCATE TABLE {} CASCADE", table);
    pool.execute(query.as_str())
        .await
        .expect("failed to truncate table in setup");

    seed_default_mock_user(&pool).await;

    AppState { db_pool: pool }
}

/// The mock-auth extractor's fallback identity (used whenever a test doesn't
/// send an `x-mock-user` header) needs a real row in `users` now that
/// `devices.owner_id` has a foreign key constraint. Idempotent so it's safe
/// to call after every table truncation, including of `users` itself.
async fn seed_default_mock_user(pool: &PgPool) {
    let user_id = UserId::from_str(DEFAULT_MOCK_USER_ID).unwrap();
    sqlx::query(
        "INSERT INTO users (id, username, email, hashed_password, role, created_at)
         VALUES ($1, 'mock-default-user', 'mock-default-user@example.com', 'not-a-real-hash', 'Operator', NOW())
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(user_id)
    .execute(pool)
    .await
    .expect("failed to seed default mock user fixture");
}

/// Inserts a device row directly, bypassing the API. Needed by tests that
/// exercise readings endpoints, which now require the device to exist and
/// be owned by the caller before accepting/returning any readings for it.
pub async fn seed_device(pool: &PgPool, device_id: DeviceId, owner_id: UserId) {
    sqlx::query(
        "INSERT INTO devices (id, name, description, owner_id, registered_at, is_active)
         VALUES ($1, 'seeded-device', NULL, $2, NOW(), TRUE)",
    )
    .bind(device_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("failed to seed device fixture");
}

pub async fn send_request<T: Serialize>(
    app: &Router,
    method: &str,
    uri: &str,
    payload: Option<T>,
) -> (StatusCode, String) {
    let mut req = Request::builder().method(method).uri(uri);

    let body = if let Some(data) = payload {
        req = req.header("content-type", "application/json");
        Body::from(serde_json::to_string(&data).unwrap())
    } else {
        Body::empty()
    };

    let req = req.body(body).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();

    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8(body_bytes.to_vec()).unwrap();

    (status, body)
}

pub async fn send_json<T: Serialize>(
    app: &Router,
    method: &str,
    uri: &str,
    payload: Option<T>,
) -> (StatusCode, Value) {
    let (status, body) = send_request(app, method, uri, payload).await;
    match serde_json::from_str(&body) {
        Ok(json) => (status, json),
        Err(_) => {
            panic!(
                "Response was not valid JSON. Status: {:?}, Body: {}",
                status, body
            );
        }
    }
}

pub async fn send_json_with_header<T: Serialize>(
    app: &Router,
    method: &str,
    uri: &str,
    payload: Option<T>,
    header_name: &str,
    header_value: &str,
) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(uri);

    let body = if let Some(data) = payload {
        req = req.header("content-type", "application/json");
        Body::from(serde_json::to_string(&data).unwrap())
    } else {
        Body::empty()
    };

    let req = req.header(header_name, header_value).body(body).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();

    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8(body_bytes.to_vec()).unwrap();

    match serde_json::from_str(&body) {
        Ok(json) => (status, json),
        Err(_) => {
            panic!(
                "Response was not valid JSON. Status: {:?}, Body: {}",
                status, body
            );
        }
    }
}
