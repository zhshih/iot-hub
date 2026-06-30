use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use iot_hub::app_state::AppState;
use serde::Serialize;
use serde_json::Value;
use sqlx::{self, Executor, PgPool};
use tower::ServiceExt;
use uuid::Uuid;

pub const TEST_DATABASE_URL: &str =
    "postgres://test_user:test_password@localhost/iot_monitoring_test";
pub struct TestApp {
    pub app: Router,
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

    AppState { db_pool: pool }
}

pub async fn cleanup_test_state(table: &str) {
    setup_env();

    let database_url = TEST_DATABASE_URL;

    let pool = sqlx::PgPool::connect(&std::env::var(database_url).unwrap())
        .await
        .expect("failed to connect for cleanup");

    let query = format!("TRUNCATE TABLE {} CASCADE", table);
    pool.execute(query.as_str())
        .await
        .expect("failed to cleanup test state");
}

/// Inserts a device row directly, bypassing the API. Needed by tests that
/// exercise readings endpoints, which now require the device to exist and
/// be owned by the caller before accepting/returning any readings for it.
pub async fn seed_device(pool: &PgPool, device_id: Uuid, owner_id: Uuid) {
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
