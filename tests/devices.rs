mod common;

use axum::{Router, http::StatusCode};
use common::{TestApp, cleanup_test_state, send_json, setup_test_state};
use iot_hub::api::devices::routes;
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

const DEVICES_TABLE: &str = "devices";

impl TestApp {
    async fn new() -> Self {
        let app_state = setup_test_state(DEVICES_TABLE).await;
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
            cleanup_test_state(DEVICES_TABLE).await;
        };
        tokio::spawn(fut);
    }
}

#[tokio::test]
#[serial]
async fn test_register_device() {
    let test_app = TestApp::new().await;

    let device = json!({
        "name": "My Device",
        "owner_id": Uuid::new_v4(),
        "registered_at": chrono::Utc::now(),
        "description": "integration test"
    });

    let (status, json) = send_json(test_app.app(), "POST", "/", Some(device)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "success");

    let data = &json["data"];

    let device_id = data["device_id"].as_str().unwrap();
    assert!(!device_id.is_empty(), "Device ID should be returned");
}

#[tokio::test]
#[serial]
async fn test_get_devices() {
    let test_app = TestApp::new().await;

    for i in 1..=3 {
        let device = json!({
            "name": format!("Test Device {}", i),
            "owner_id": Uuid::new_v4(),
            "registered_at": chrono::Utc::now(),
            "description": format!("Device number {}", i),
        });

        let (status, json) = send_json(test_app.app(), "POST", "/", Some(device)).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["status"], "success");
    }

    let (status, json) = send_json::<()>(test_app.app(), "GET", "/", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "success");

    let data = &json["data"];

    assert_eq!(data["devices"].as_array().unwrap().len(), 3);
}

#[tokio::test]
#[serial]
async fn test_get_device() {
    let test_app = TestApp::new().await;

    let device = json!({
        "name": "Test Device 1",
        "owner_id": Uuid::new_v4(),
        "registered_at": chrono::Utc::now(),
        "description": null
    });

    let (_, created) = send_json(test_app.app(), "POST", "/", Some(device)).await;
    let device_id = created["data"]["device_id"].as_str().unwrap();
    let (status, json) =
        send_json::<()>(test_app.app(), "GET", &format!("/{}", device_id), None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "success");

    let data = &json["data"];

    assert_eq!(data["device"]["id"].as_str().unwrap(), device_id);
}

#[tokio::test]
#[serial]
async fn test_delete_device() {
    let test_app = TestApp::new().await;

    let device = json!({
        "name": "Temp Device",
        "owner_id": Uuid::new_v4(),
        "registered_at": chrono::Utc::now(),
        "description": null
    });

    let (_, created) = send_json(test_app.app(), "POST", "/", Some(device)).await;
    let device_id = created["data"]["device_id"].as_str().unwrap();
    let (status, json) =
        send_json::<()>(test_app.app(), "DELETE", &format!("/{}", device_id), None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "success");

    let data = &json["data"];

    assert_eq!(data["device_id"].as_str().unwrap(), device_id);
}
