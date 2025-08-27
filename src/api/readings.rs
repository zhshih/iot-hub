use super::error::ApiError;
use crate::auth::middleware::AuthUser;
use crate::service::reading_service::{PaginationParams, ReadingService};
use crate::{app_state::AppState, domain::reading::Reading};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use serde_json::json;
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/{device_id}", post(post_reading))
        .route("/{device_id}/readings", post(post_readings))
        .route("/{device_id}", get(get_readings))
        .route("/{device_id}/latest", get(get_latest_readings))
        .route("/{device_id}/{from}/{to}", get(get_readings_in_range))
        .route("/{device_id}/paginated", get(get_readings_paginated))
        .route(
            "/{device_id}/{from}/{to}/paginated",
            get(get_readings_in_range_paginated),
        )
}

async fn post_reading(
    State(state): State<AppState>,
    Path(device_id): Path<Uuid>,
    AuthUser(user): AuthUser,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let reading: Reading = serde_json::from_value(payload)
        .map_err(|e| ApiError::BadRequest(format!("Invalid JSON: {}", e)))?;
    let service = ReadingService::new(state.db_pool.clone());
    service.post_reading(device_id, &reading).await?;

    Ok(Json(json!({
        "message": format!("Posted reading {} for user {}", device_id, user.sub),
        "status": "success"
    })))
}

async fn post_readings(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    AuthUser(user): AuthUser,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let readings: Vec<Reading> = serde_json::from_value(payload)
        .map_err(|e| ApiError::BadRequest(format!("Invalid JSON: {}", e)))?;
    let service = ReadingService::new(state.db_pool.clone());
    service.post_readings(id, readings).await?;

    Ok(Json(json!({
        "message": format!("Posted reading {} for user {}", id, user.sub),
        "status": "success"
    })))
}

async fn get_readings(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
    AuthUser(_user): AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let device_id = Uuid::parse_str(&device_id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid UUID: {}", e)))?;

    let service = ReadingService::new(state.db_pool.clone());

    let readings = service.get_readings(device_id).await?;

    if readings.is_empty() {
        return Err(ApiError::NotFound(format!(
            "No readings found for device {}",
            device_id
        )));
    }

    Ok(Json(json!({
        "message": format!("Got {} readings for device {}", readings.len(), device_id),
        "status": "success",
        "device_id": device_id,
        "readings": readings,
    })))
}

async fn get_latest_readings(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
    AuthUser(_user): AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let device_id = Uuid::parse_str(&device_id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid UUID: {}", e)))?;

    let service = ReadingService::new(state.db_pool.clone());

    let reading = service.get_latest_reading(device_id).await?;

    Ok(Json(json!({
        "message": format!("Got {} reading for device {}", reading, device_id),
        "status": "success",
        "device_id": device_id,
        "readings": reading,
    })))
}

async fn get_readings_in_range(
    State(state): State<AppState>,
    Path((device_id, from, to)): Path<(
        String,
        chrono::DateTime<chrono::Utc>,
        chrono::DateTime<chrono::Utc>,
    )>,
    AuthUser(_user): AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let device_id = Uuid::parse_str(&device_id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid UUID: {}", e)))?;

    let service = ReadingService::new(state.db_pool.clone());

    let readings = service.get_readings_in_range(device_id, from, to).await?;

    Ok(Json(json!({
        "message": format!("Got {} readings for device {}", readings.len(), device_id),
        "status": "success",
        "device_id": device_id,
        "readings": readings,
    })))
}

async fn get_readings_paginated(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
    Query(params): Query<PaginationParams>,
    AuthUser(_user): AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let device_id = Uuid::parse_str(&device_id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid UUID: {}", e)))?;

    let service = ReadingService::new(state.db_pool.clone());

    let result = service
        .get_readings_paginated(device_id, params.cursor, params.limit)
        .await?;

    Ok(Json(json!({
        "message": format!("Got {} readings for device {}", result.data.len(), device_id),
        "status": "success",
        "device_id": device_id,
        "next_cursor": result.next_cursor,
        "has_more": result.has_more,
        "readings": result.data,
    })))
}

async fn get_readings_in_range_paginated(
    State(state): State<AppState>,
    Path((device_id, from, to)): Path<(
        String,
        chrono::DateTime<chrono::Utc>,
        chrono::DateTime<chrono::Utc>,
    )>,
    Query(params): Query<PaginationParams>,
    AuthUser(_user): AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let device_id = Uuid::parse_str(&device_id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid UUID: {}", e)))?;

    let service = ReadingService::new(state.db_pool.clone());

    let result = service
        .get_readings_in_range_paginated(device_id, from, to, params.cursor, params.limit)
        .await?;

    Ok(Json(json!({
        "message": format!("Got {} readings for device {}", result.data.len(), device_id),
        "status": "success",
        "device_id": device_id,
        "next_cursor": result.next_cursor,
        "has_more": result.has_more,
        "readings": result.data,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::middleware::AuthUser;
    use crate::domain::reading::ReadingType;
    use crate::test_utils::{mock_auth_user, setup_test_state};
    use axum::{
        Json,
        extract::{Path, State},
    };
    use chrono::Utc;
    use serde_json::Value;
    use serial_test::serial;
    use uuid::Uuid;

    const READINGS_TABLE: &str = "readings";
    const DUMMY_USER: &str = "test_user";

    fn extract_message(json: Json<Value>) -> String {
        json.0.get("message").unwrap().as_str().unwrap().to_string()
    }

    async fn call_post_reading(
        app_state: AppState,
        user: AuthUser,
        device_id: Uuid,
        reading: Reading,
    ) -> serde_json::Value {
        let resp = post_reading(
            State(app_state),
            Path(device_id),
            user,
            Json(serde_json::to_value(reading).unwrap()),
        )
        .await
        .unwrap();

        resp.0
    }

    #[tokio::test]
    #[serial]
    async fn test_post_reading() {
        let app_state = setup_test_state(READINGS_TABLE).await;
        let user = mock_auth_user(DUMMY_USER);

        let device_id = Uuid::new_v4();
        let reading = Reading {
            device_id,
            arrived_timestamp: Utc::now(),
            processed_timestamp: Utc::now(),
            reading_type: ReadingType::Temperature,
            value: 42.0,
        };

        let json = call_post_reading(app_state, user, device_id, reading).await;
        assert_eq!(json["status"], "success");
        assert!(json["message"].as_str().unwrap().contains(DUMMY_USER));
    }

    #[tokio::test]
    #[serial]
    async fn test_post_readings() {
        let app_state = setup_test_state(READINGS_TABLE).await;
        let user = mock_auth_user(DUMMY_USER);

        let device_id = Uuid::new_v4();
        let readings = vec![
            Reading {
                device_id,
                arrived_timestamp: Utc::now(),
                processed_timestamp: Utc::now(),
                reading_type: ReadingType::Temperature,
                value: 10.0,
            },
            Reading {
                device_id,
                arrived_timestamp: Utc::now(),
                processed_timestamp: Utc::now(),
                reading_type: ReadingType::Temperature,
                value: 20.0,
            },
        ];

        let resp = post_readings(
            State(app_state),
            Path(device_id),
            user,
            Json(serde_json::to_value(readings).unwrap()),
        )
        .await
        .unwrap();

        let json = resp.0;
        assert_eq!(json["status"], "success");
        assert!(json["message"].as_str().unwrap().contains(DUMMY_USER));
    }

    #[tokio::test]
    #[serial]
    async fn test_get_readings() {
        let app_state = setup_test_state(READINGS_TABLE).await;
        let user = mock_auth_user(DUMMY_USER);

        let device_id = Uuid::new_v4();

        for i in 1..=3 {
            let reading = Reading {
                device_id,
                arrived_timestamp: Utc::now(),
                processed_timestamp: Utc::now(),
                reading_type: ReadingType::Temperature,
                value: i as f64 * 10.0,
            };

            let _ = post_reading(
                State(app_state.clone()),
                Path(device_id),
                user.clone(),
                Json(serde_json::to_value(reading).unwrap()),
            )
            .await
            .unwrap();
        }

        let result = get_readings(State(app_state), Path(device_id.to_string()), user.clone())
            .await
            .unwrap();

        let json = result.0;
        println!("{}", json);
        assert_eq!(json["status"], "success");
        assert_eq!(json["readings"].as_array().unwrap().len(), 3);
        assert!(extract_message(Json(json.clone())).contains("Got 3 readings"));
    }

    #[tokio::test]
    #[serial]
    async fn test_get_readings_not_found() {
        let app_state = setup_test_state(READINGS_TABLE).await;
        let user = mock_auth_user(DUMMY_USER);
        let device_id = Uuid::new_v4();

        let result = get_readings(State(app_state), Path(device_id.to_string()), user).await;

        match result {
            Err(ApiError::NotFound(msg)) => {
                assert!(msg.contains("No readings found"));
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_get_latest_readings() {
        let app_state = setup_test_state(READINGS_TABLE).await;
        let user = mock_auth_user(DUMMY_USER);

        let device_id = Uuid::new_v4();
        let reading = Reading {
            device_id,
            arrived_timestamp: Utc::now(),
            processed_timestamp: Utc::now(),
            reading_type: ReadingType::Temperature,
            value: 99.9,
        };

        let _ = post_reading(
            State(app_state.clone()),
            Path(device_id),
            user.clone(),
            Json(serde_json::to_value(reading).unwrap()),
        )
        .await
        .unwrap();

        let resp = get_latest_readings(State(app_state), Path(device_id.to_string()), user)
            .await
            .unwrap();

        let json = resp.0;
        assert_eq!(json["status"], "success");
        assert_eq!(json["device_id"], device_id.to_string());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_readings_in_range() {
        let app_state = setup_test_state(READINGS_TABLE).await;
        let user = mock_auth_user(DUMMY_USER);
        let device_id = Uuid::new_v4();

        let now = Utc::now();

        for i in 0..3 {
            let reading = Reading {
                device_id,
                arrived_timestamp: now + chrono::Duration::minutes(i),
                processed_timestamp: now + chrono::Duration::minutes(i),
                reading_type: ReadingType::Temperature,
                value: (i * 5) as f64,
            };

            let _ = post_reading(
                State(app_state.clone()),
                Path(device_id),
                user.clone(),
                Json(serde_json::to_value(reading).unwrap()),
            )
            .await
            .unwrap();
        }

        let from = now;
        let to = now + chrono::Duration::minutes(5);

        let resp = get_readings_in_range(
            State(app_state),
            Path((device_id.to_string(), from, to)),
            user,
        )
        .await
        .unwrap();

        let json = resp.0;
        assert_eq!(json["status"], "success");
        assert!(!json["readings"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_readings_paginated() {
        let app_state = setup_test_state(READINGS_TABLE).await;
        let user = mock_auth_user(DUMMY_USER);
        let device_id = Uuid::new_v4();

        for _ in 0..3 {
            let reading = Reading {
                device_id,
                arrived_timestamp: Utc::now(),
                processed_timestamp: Utc::now(),
                reading_type: ReadingType::Temperature,
                value: 55.0,
            };

            let _ = post_reading(
                State(app_state.clone()),
                Path(device_id),
                user.clone(),
                Json(serde_json::to_value(reading).unwrap()),
            )
            .await
            .unwrap();
        }

        let params = PaginationParams {
            cursor: None,
            limit: 2,
        };

        let resp = get_readings_paginated(
            State(app_state),
            Path(device_id.to_string()),
            axum::extract::Query(params),
            user,
        )
        .await
        .unwrap();

        let json = resp.0;
        assert_eq!(json["status"], "success");
        assert_eq!(json["readings"].as_array().unwrap().len(), 2);
        assert!(json["has_more"].as_bool().unwrap());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_readings_in_range_paginated() {
        let app_state = setup_test_state(READINGS_TABLE).await;
        let user = mock_auth_user(DUMMY_USER);
        let device_id = Uuid::new_v4();

        let now = Utc::now();

        for i in 0..3 {
            let reading = Reading {
                device_id,
                arrived_timestamp: now + chrono::Duration::seconds(i),
                processed_timestamp: now + chrono::Duration::seconds(i),
                reading_type: ReadingType::Temperature,
                value: (i * 10) as f64,
            };

            let _ = post_reading(
                State(app_state.clone()),
                Path(device_id),
                user.clone(),
                Json(serde_json::to_value(reading).unwrap()),
            )
            .await
            .unwrap();
        }

        let from = now;
        let to = now + chrono::Duration::seconds(10);
        let params = PaginationParams {
            cursor: None,
            limit: 2,
        };

        let resp = get_readings_in_range_paginated(
            State(app_state),
            Path((device_id.to_string(), from, to)),
            axum::extract::Query(params),
            user,
        )
        .await
        .unwrap();

        let json = resp.0;
        assert_eq!(json["status"], "success");
        assert!(json["readings"].as_array().unwrap().len() <= 2);
    }

    #[tokio::test]
    async fn test_get_readings_invalid_uuid() {
        let app_state = setup_test_state(READINGS_TABLE).await;
        let user = mock_auth_user(DUMMY_USER);

        let bad_uuid = "not-a-uuid".to_string();

        let result = get_readings(State(app_state), Path(bad_uuid), user).await;

        match result {
            Err(ApiError::BadRequest(msg)) => assert!(msg.contains("Invalid UUID")),
            _ => panic!("Expected BadRequest error"),
        }
    }
}
