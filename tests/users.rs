mod common;

use axum::{Router, http::StatusCode};
use common::{TestApp, cleanup_test_state, send_json, send_request, setup_test_state};
use iot_hub::api::users::routes;
use serde_json::Value;
use serde_json::json;
use serial_test::serial;

const USERS_TABLE: &str = "users";

impl TestApp {
    async fn new() -> Self {
        let app_state = setup_test_state(USERS_TABLE).await;
        let app = routes().with_state(app_state);
        Self { app }
    }

    fn app(&self) -> &Router {
        &self.app
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        let fut = async {
            cleanup_test_state(USERS_TABLE).await;
        };
        tokio::spawn(fut);
    }
}

#[tokio::test]
#[serial]
async fn test_signup() {
    let test_app = TestApp::new().await;

    let payload = json!({
        "username": "alice",
        "email": "alice@example.com",
        "password": "password123"
    });

    let (status, json) = send_json(test_app.app(), "POST", "/signup", Some(payload)).await;

    assert_eq!(status, StatusCode::OK);

    let token = json["data"]["token"].as_str();
    assert!(token.is_some(), "Token should be present in the response");
}

#[tokio::test]
#[serial]
async fn test_login() {
    let test_app = TestApp::new().await;

    let _ = send_json(
        test_app.app(),
        "POST",
        "/signup",
        Some(json!({
            "username": "test_user",
            "email": "bob@example.com",
            "password": "secret"
        })),
    )
    .await;

    let (status, json) = send_json(
        test_app.app(),
        "POST",
        "/login",
        Some(json!({
            "username": "test_user",
            "email": "bob@example.com",
            "password": "secret"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);

    let token = json["data"]["token"].as_str();
    assert!(token.is_some(), "Token should be present in the response");
}

#[tokio::test]
#[serial]
async fn test_me() {
    let test_app = TestApp::new().await;

    let _ = send_json(
        test_app.app(),
        "POST",
        "/signup",
        Some(json!({
            "username": "test_user",
            "email": "test_user@example.com",
            "password": "password123"
        })),
    )
    .await;

    let (status, body_str) = send_request::<()>(test_app.app(), "GET", "/me", None).await;

    assert_eq!(status, StatusCode::OK);

    let body: Value = serde_json::from_str(&body_str).expect("Response should be valid JSON");

    let username = body["data"]["user"]["username"]
        .as_str()
        .expect("username should exist");
    assert_eq!(username, "test_user");
}

#[tokio::test]
#[serial]
async fn test_list_users() {
    let test_app = TestApp::new().await;

    let _ = send_json(
        test_app.app(),
        "POST",
        "/signup",
        Some(json!({
            "username": "test_user",
            "email": "test_user@example.com",
            "password": "password123"
        })),
    )
    .await;

    let (status, body_str) = send_request::<()>(test_app.app(), "GET", "/", None).await;

    assert_eq!(status, StatusCode::OK);

    let body: serde_json::Value =
        serde_json::from_str(&body_str).expect("Response should be valid JSON");

    let users = &body["data"]["users"];
    assert!(users.is_array(), "users should be an array");
    let users_array = users.as_array().unwrap();
    assert!(!users_array.is_empty(), "users array should not be empty");
}

#[tokio::test]
#[serial]
async fn test_health_check() {
    let test_app = TestApp::new().await;

    let (status, body_str) = send_request::<()>(test_app.app(), "GET", "/health", None).await;
    assert_eq!(status, StatusCode::OK);

    let body: serde_json::Value =
        serde_json::from_str(&body_str).expect("Response should be valid JSON");

    assert_eq!(
        body["status"].as_str().expect("status should exist"),
        "success"
    );
}
