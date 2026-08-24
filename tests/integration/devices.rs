use crate::common::{TEST_DATABASE_URL, TestApp, seed_user, send_json, send_json_with_header};
use axum::http::StatusCode;
use iot_hub::api::devices::routes;
use iot_hub::auth::extractor::DEFAULT_MOCK_USER_ID;
use iot_hub::domain::ids::UserId;
use serde_json::json;
use serial_test::serial;
use sqlx::PgPool;
use std::str::FromStr;
use uuid::Uuid;

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

#[tokio::test]
#[serial]
async fn test_update_device() {
    let test_app = TestApp::new(DEVICES_TABLE, routes()).await;

    let device = json!({
        "name": "Original Name",
        "description": "original description"
    });
    let (_, created) = send_json(test_app.app(), "POST", "/", Some(device)).await;
    let device_id = created["data"]["device_id"].as_str().unwrap();

    let patch = json!({ "is_active": false });
    let (status, json) = send_json(
        test_app.app(),
        "PATCH",
        &format!("/{}", device_id),
        Some(patch),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let data = &json["data"]["device"];
    assert_eq!(data["is_active"], false);
    assert_eq!(data["name"], "Original Name");
    assert_eq!(data["description"], "original description");
}

#[tokio::test]
#[serial]
async fn test_update_device_not_owner_returns_404() {
    let test_app = TestApp::new(DEVICES_TABLE, routes()).await;

    let device = json!({ "name": "Someone Else's Device", "description": null });
    let (_, created) = send_json(test_app.app(), "POST", "/", Some(device)).await;
    let device_id = created["data"]["device_id"].as_str().unwrap();

    let other_user = Uuid::new_v4().to_string();
    let patch = json!({ "name": "Hijacked" });
    let (status, _) = send_json_with_header(
        test_app.app(),
        "PATCH",
        &format!("/{}", device_id),
        Some(patch),
        "x-mock-user",
        &other_user,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
#[serial]
async fn test_transfer_device_ownership_commits_both_writes() {
    let test_app = TestApp::new(DEVICES_TABLE, routes()).await;
    let pool = PgPool::connect(TEST_DATABASE_URL)
        .await
        .expect("failed to connect to test database");

    let previous_owner_id = UserId::from_str(DEFAULT_MOCK_USER_ID).unwrap();
    let new_owner_id = UserId::new();
    seed_user(&pool, new_owner_id).await;

    let device = json!({ "name": "Transferable Device", "description": null });
    let (_, created) = send_json(test_app.app(), "POST", "/", Some(device)).await;
    let device_id = created["data"]["device_id"].as_str().unwrap();

    let payload = json!({ "new_owner_id": new_owner_id.to_string() });
    let (status, json) = send_json(
        test_app.app(),
        "PATCH",
        &format!("/{}/transfer", device_id),
        Some(payload),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "success");

    let owner_id: Uuid = sqlx::query_scalar("SELECT owner_id FROM devices WHERE id = $1")
        .bind(Uuid::parse_str(device_id).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(owner_id, new_owner_id.into_inner());

    let history_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ownership_history
         WHERE device_id = $1 AND previous_owner_id = $2 AND new_owner_id = $3",
    )
    .bind(Uuid::parse_str(device_id).unwrap())
    .bind(previous_owner_id.into_inner())
    .bind(new_owner_id.into_inner())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(history_count, 1);
}

#[tokio::test]
#[serial]
async fn test_transfer_device_ownership_rolls_back_on_invalid_target() {
    let test_app = TestApp::new(DEVICES_TABLE, routes()).await;
    let pool = PgPool::connect(TEST_DATABASE_URL)
        .await
        .expect("failed to connect to test database");

    let device = json!({ "name": "Non-transferable Device", "description": null });
    let (_, created) = send_json(test_app.app(), "POST", "/", Some(device)).await;
    let device_id = created["data"]["device_id"].as_str().unwrap();

    // A random uuid with no corresponding users row trips the
    // ownership_history.new_owner_id foreign key mid-transaction.
    let nonexistent_owner_id = Uuid::new_v4();
    let payload = json!({ "new_owner_id": nonexistent_owner_id.to_string() });
    let (status, _) = send_json(
        test_app.app(),
        "PATCH",
        &format!("/{}/transfer", device_id),
        Some(payload),
    )
    .await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);

    let owner_id: Uuid = sqlx::query_scalar("SELECT owner_id FROM devices WHERE id = $1")
        .bind(Uuid::parse_str(device_id).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        owner_id,
        Uuid::from_str(DEFAULT_MOCK_USER_ID).unwrap(),
        "owner_id must be unchanged after the transaction rolled back"
    );

    let history_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM ownership_history WHERE device_id = $1")
            .bind(Uuid::parse_str(device_id).unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        history_count, 0,
        "0 rows in ownership_history after forced FK failure"
    );
}
