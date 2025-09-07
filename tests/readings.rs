mod common;

use axum::{Router, http::StatusCode};
use common::{TestApp, cleanup_test_state, send_json, setup_test_state};
use iot_hub::api::readings::routes;
use serde_json::json;
use serial_test::serial;
use urlencoding::encode;
use uuid::Uuid;

const READINGS_TABLE: &str = "readings";

impl TestApp {
    async fn new() -> Self {
        let app_state = setup_test_state(READINGS_TABLE).await;
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
            cleanup_test_state(READINGS_TABLE).await;
        };
        tokio::spawn(fut);
    }
}

#[tokio::test]
#[serial]
async fn test_post_reading_single() {
    let test_app = TestApp::new().await;
    let device_id = Uuid::new_v4();
    let reading = json!({
        "device_id": device_id,
        "arrived_timestamp": chrono::Utc::now().to_rfc3339(),
        "processed_timestamp": chrono::Utc::now().to_rfc3339(),
        "reading_type": "Voltage",
        "value": 51.5,
    });

    let (status, json) = send_json(
        test_app.app(),
        "POST",
        &format!("/{}/readings", device_id),
        Some(reading),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "success");
}

#[tokio::test]
#[serial]
async fn test_post_readings_bulk() {
    let test_app = TestApp::new().await;
    let device_id = Uuid::new_v4();

    let readings = json!([
        {
            "device_id": device_id,
            "arrived_timestamp": chrono::Utc::now(),
            "processed_timestamp": chrono::Utc::now(),
            "reading_type": "Temperature",
            "value": 23.5,
        },
        {
            "device_id": device_id,
            "arrived_timestamp": chrono::Utc::now() + chrono::Duration::seconds(10),
            "processed_timestamp": chrono::Utc::now() + chrono::Duration::seconds(10),
            "reading_type": "Temperature",
            "value": 29.0,
        }
    ]);

    let (status, json) = send_json(
        test_app.app(),
        "POST",
        &format!("/{}/readings", device_id),
        Some(readings),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "success");
    assert_eq!(json["data"]["inserted"].as_u64().unwrap(), 2);
    assert_eq!(json["data"]["inserted"].as_u64().unwrap(), 2);
    assert_eq!(
        json["data"]["device_id"].as_str().unwrap(),
        device_id.to_string()
    );
    assert!(json["data"]["created_at"].is_string())
}

#[tokio::test]
#[serial]
async fn test_get_readings_with_query() {
    let test_app = TestApp::new().await;
    let device_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    for i in 0..3 {
        let reading = json!({
            "device_id": device_id,
            "arrived_timestamp": now + chrono::Duration::seconds(i),
            "processed_timestamp": now + chrono::Duration::seconds(i),
            "reading_type": "Temperature",
            "value": 20.0 + i as f64,
        });

        send_json(
            test_app.app(),
            "POST",
            &format!("/{}/readings", device_id),
            Some(reading),
        )
        .await;
    }

    let from_ts = (now - chrono::Duration::minutes(1)).timestamp();
    let to_ts = (now + chrono::Duration::minutes(1)).timestamp();

    let (status, json) = send_json::<()>(
        test_app.app(),
        "GET",
        &format!(
            "/{}/readings?from={}&to={}&limit=2",
            device_id, from_ts, to_ts
        ),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "success");

    let readings = json["data"]["readings"].as_array().unwrap();
    assert_eq!(readings.len(), 2);
    assert!(json["data"]["has_more"].as_bool().unwrap());
    assert!(json["data"]["next_cursor"].is_i64());
}

#[tokio::test]
#[serial]
async fn test_get_latest_reading() {
    let test_app = TestApp::new().await;
    let device_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let reading = json!({
        "device_id": device_id,
        "arrived_timestamp": now,
        "processed_timestamp": now,
        "reading_type": "Voltage",
        "value": 51.5,
    });

    send_json(
        test_app.app(),
        "POST",
        &format!("/{}/readings", device_id),
        Some(reading),
    )
    .await;

    let (status, json) = send_json::<()>(
        test_app.app(),
        "GET",
        &format!("/{}/readings/latest", device_id),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "success");

    let readings = json["data"]["readings"].as_array().unwrap();
    assert_eq!(readings.len(), 1);
    assert_eq!(
        json["data"]["device_id"].as_str().unwrap(),
        device_id.to_string()
    );
}

#[tokio::test]
#[serial]
async fn test_get_readings_in_range() {
    let test_app = TestApp::new().await;
    let device_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let reading = json!({
        "device_id": device_id,
        "arrived_timestamp": now,
        "processed_timestamp": now,
        "reading_type": "Voltage",
        "value": 51.5,
    });

    send_json(
        test_app.app(),
        "POST",
        &format!("/{}/readings", device_id),
        Some(reading),
    )
    .await;

    let from_ts = (now - chrono::Duration::minutes(1)).timestamp();
    let to_ts = (now + chrono::Duration::minutes(1)).timestamp();

    let uri = format!("/{}/readings?from={}&to={}", device_id, from_ts, to_ts);
    println!("URI: {}", uri);

    let (status, json) = send_json::<()>(
        test_app.app(),
        "GET",
        &format!("/{}/readings?from={}&to={}", device_id, from_ts, to_ts),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "success");

    let readings = json["data"]["readings"].as_array().unwrap();
    assert_eq!(readings.len(), 1);

    assert_eq!(
        json["data"]["device_id"].as_str().unwrap(),
        device_id.to_string()
    );
}

#[tokio::test]
#[serial]
async fn test_get_readings_pagination_multiple_pages() {
    let test_app = TestApp::new().await;
    let device_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    for i in 0..5 {
        let reading = json!({
            "device_id": device_id,
            "arrived_timestamp": now + chrono::Duration::seconds(i),
            "processed_timestamp": now + chrono::Duration::seconds(i),
            "reading_type": "Temperature",
            "value": 20.0 + i as f64,
        });

        send_json(
            test_app.app(),
            "POST",
            &format!("/{}/readings", device_id),
            Some(reading),
        )
        .await;
    }

    let (status, json) = send_json::<()>(
        test_app.app(),
        "GET",
        &format!("/{}/readings?limit=2", device_id),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let readings = json["data"]["readings"].as_array().unwrap();
    assert_eq!(readings.len(), 2);
    assert!(json["data"]["has_more"].as_bool().unwrap());
    let next_cursor = json["data"]["next_cursor"].as_i64().unwrap().to_string();
    let cursor_encoded = encode(&next_cursor);

    let (status2, json2) = send_json::<()>(
        test_app.app(),
        "GET",
        &format!("/{}/readings?limit=2&cursor={}", device_id, cursor_encoded),
        None,
    )
    .await;

    assert_eq!(status2, StatusCode::OK);
    let readings2 = json2["data"]["readings"].as_array().unwrap();
    assert_eq!(readings2.len(), 2);
    assert!(json2["data"]["has_more"].as_bool().unwrap());

    let last_cursor = json2["data"]["next_cursor"].as_i64().unwrap().to_string();
    let cursor_encoded = encode(&last_cursor);
    let (_, json3) = send_json::<()>(
        test_app.app(),
        "GET",
        &format!("/{}/readings?limit=2&cursor={}", device_id, cursor_encoded),
        None,
    )
    .await;

    let readings3 = json3["data"]["readings"].as_array().unwrap();
    assert_eq!(readings3.len(), 1);
    assert!(!json3["data"]["has_more"].as_bool().unwrap());
}
