use crate::common::{TestApp, send_json};
use axum::http::StatusCode;
use iot_hub::api::devices::routes;
use serde_json::json;
use serial_test::serial;

const DEVICES_TABLE: &str = "devices";

#[tokio::test]
#[serial]
async fn test_register_device() {
    let test_app = TestApp::new(DEVICES_TABLE, routes()).await;

    let device = json!({
        "name": "My Device",
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
    let test_app = TestApp::new(DEVICES_TABLE, routes()).await;

    for i in 1..=3 {
        let device = json!({
            "name": format!("Test Device {}", i),
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
    let test_app = TestApp::new(DEVICES_TABLE, routes()).await;

    let device = json!({
        "name": "Test Device 1",
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
    let test_app = TestApp::new(DEVICES_TABLE, routes()).await;

    let device = json!({
        "name": "Temp Device",
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
