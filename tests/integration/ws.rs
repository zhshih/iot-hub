use crate::common::{seed_device, setup_test_state, spawn_test_server};
use axum::{
    Router,
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
};
use futures_util::StreamExt;
use iot_hub::auth::extractor::DEFAULT_MOCK_USER_ID;
use iot_hub::domain::ids::{DeviceId, UserId};
use serde_json::{Value, json};
use serial_test::serial;
use std::net::SocketAddr;
use std::str::FromStr;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tower::ServiceExt;

const READINGS_TABLE: &str = "readings";

/// Like `common::send_json`, but inserts a `ConnectInfo` extension so
/// create_app's GovernorLayer can key rate limiting by peer IP -- `oneshot`'s
/// in-memory request/response has none by default.
async fn post_reading(app: &Router, uri: &str, payload: Value) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))));

    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json = serde_json::from_slice(&body_bytes).unwrap();
    (status, json)
}

// A single test, not two: axum_prometheus::PrometheusMetricLayer::pair()
// (invoked inside create_app) installs a process-global metrics recorder,
// so a second create_app call in the same test binary panics.
#[tokio::test]
#[serial]
async fn test_ws_reading_stream() {
    let app_state = setup_test_state(READINGS_TABLE).await;
    let device_id = DeviceId::new();
    let owner_id = UserId::from_str(DEFAULT_MOCK_USER_ID).unwrap();
    seed_device(&app_state.db_pool, device_id, owner_id).await;

    let app = iot_hub::create_app(app_state);
    let addr = spawn_test_server(app.clone()).await;

    let (mut ws_stream, _) = connect_async(format!(
        "ws://{addr}/api/v1/devices/{device_id}/readings/stream"
    ))
    .await
    .expect("failed to connect to WS stream");

    let reading = json!({
        "arrived_timestamp": chrono::Utc::now().to_rfc3339(),
        "reading_type": "Voltage",
        "value": 51.5,
    });

    let (status, _) = post_reading(
        &app,
        &format!("/api/v1/devices/{device_id}/readings"),
        reading,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let msg = tokio::time::timeout(std::time::Duration::from_secs(5), ws_stream.next())
        .await
        .expect("timed out waiting for WS message")
        .expect("WS stream closed unexpectedly")
        .expect("WS stream returned an error");

    let Message::Text(text) = msg else {
        panic!("expected a text message, got {msg:?}");
    };
    let received: Value = serde_json::from_str(&text).unwrap();

    assert_eq!(received["device_id"], device_id.to_string());
    assert_eq!(received["reading_type"], "Voltage");
    assert_eq!(received["value"], 51.5);

    ws_stream.close(None).await.ok();

    // Same server/app instance, a device that was never seeded: same 404
    // ownership convention as every other readings/devices endpoint.
    let unowned_device_id = DeviceId::new();
    let result = connect_async(format!(
        "ws://{addr}/api/v1/devices/{unowned_device_id}/readings/stream"
    ))
    .await;

    match result {
        Err(tokio_tungstenite::tungstenite::Error::Http(response)) => {
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        }
        other => panic!("expected an HTTP 404 upgrade rejection, got {other:?}"),
    }
}
