use super::error::ApiError;
use crate::{
    app_state::AppState, auth::middleware::AuthUser, domain::device::RegisteredDevice,
    service::device_service::DeviceService,
};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get, post},
};
use serde_json::json;
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", post(register_device))
        .route("/", get(get_devices))
        .route("/{device_id}", get(get_device))
        .route("/{device_id}", delete(delete_device))
}

async fn register_device(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(payload): Json<RegisteredDevice>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let service = DeviceService::new(state.db_pool.clone());
    let id = service.register_device(payload).await?;

    Ok(Json(json!({
        "message": format!("Registered device for user: {}", user.sub),
        "device_id": id,
        "status": "success"
    })))
}

async fn get_devices(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let service = DeviceService::new(state.db_pool.clone());
    let devices = service.get_devices().await?.into_iter().collect::<Vec<_>>();

    Ok(Json(json!({
        "message": format!("Got devices for user: {}", user.sub),
        "status": "success",
        "devices": devices,
    })))
}

async fn get_device(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let service = DeviceService::new(state.db_pool.clone());
    let device = service.get_device(id).await?;

    Ok(Json(json!({
        "message": format!("Got device {} for {}", id, user.sub),
        "status": "success",
        "device": device,
    })))
}

async fn delete_device(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let service = DeviceService::new(state.db_pool.clone());
    service.delete_device(id).await?;

    Ok(Json(json!({
        "message": format!("Deleted device {} for user {}", id, user.sub),
        "status": "success",
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{jwt::Claims, middleware::AuthUser};
    use crate::test_utils::{mock_auth_user, setup_test_state};
    use axum::{
        Json,
        extract::{Path, State},
    };
    use serde_json::Value;
    use serial_test::serial;
    use uuid::Uuid;

    const DEVICES_TABLE: &str = "devices";
    const DUMMY_DEVICE: &str = "Test Device";
    const DUMMY_USER: &str = "test_user";

    fn extract_message(json: Json<Value>) -> String {
        json.0.get("message").unwrap().as_str().unwrap().to_string()
    }

    async fn call_register_device(
        app_state: AppState,
        user: AuthUser,
        device: RegisteredDevice,
    ) -> serde_json::Value {
        let resp = register_device(State(app_state), user, Json(device))
            .await
            .unwrap();
        resp.0
    }

    #[tokio::test]
    #[serial]
    async fn test_register_device() {
        let app_state = setup_test_state(DEVICES_TABLE).await;
        let user = mock_auth_user(DUMMY_USER);

        let device = RegisteredDevice {
            name: DUMMY_DEVICE.into(),
            owner_id: Uuid::new_v4(),
            registered_at: chrono::Utc::now(),
            description: Some("integration test device".into()),
        };

        let json = call_register_device(app_state, user, device).await;
        assert_eq!(json["status"], "success");
        assert!(json["message"].as_str().unwrap().contains(DUMMY_USER));
    }

    #[tokio::test]
    #[serial]
    async fn test_get_devices() {
        let user = mock_auth_user(DUMMY_USER);
        let app_state = setup_test_state(DEVICES_TABLE).await;

        let claims = user.0;

        for i in 1..=3 {
            let device = RegisteredDevice {
                name: format!("Test Device {}", i),
                owner_id: Uuid::new_v4(),
                registered_at: chrono::Utc::now(),
                description: Some(format!("Device number {}", i)),
            };

            let _ = register_device(
                State(app_state.clone()),
                AuthUser(Claims {
                    sub: claims.sub.clone(),
                    exp: claims.exp,
                    iat: claims.iat,
                }),
                Json(device),
            )
            .await
            .unwrap();
        }

        let result = get_devices(
            AuthUser(Claims {
                sub: claims.sub.clone(),
                exp: claims.exp,
                iat: claims.iat,
            }),
            State(app_state),
        )
        .await
        .unwrap();

        let json = result.0;
        assert_eq!(json["status"], "success");
        assert_eq!(json["devices"].as_array().unwrap().len(), 3);
        assert_eq!(
            extract_message(Json(json.clone())),
            "Got devices for user: test_user"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_get_device() {
        let user = mock_auth_user(DUMMY_USER);
        let app_state = setup_test_state(DEVICES_TABLE).await;

        let device = RegisteredDevice {
            name: DUMMY_DEVICE.into(),
            owner_id: Uuid::new_v4(),
            registered_at: chrono::Utc::now(),
            description: None,
        };

        let claims = user.0;

        let register_res = register_device(
            State(app_state.clone()),
            AuthUser(Claims {
                sub: claims.sub.clone(),
                exp: claims.exp,
                iat: claims.iat,
            }),
            Json(device),
        )
        .await
        .unwrap();

        let id = register_res.0.get("device_id").unwrap().as_str().unwrap();
        let uuid = Uuid::parse_str(id).unwrap();

        let result = get_device(
            AuthUser(Claims {
                sub: claims.sub.clone(),
                exp: claims.exp,
                iat: claims.iat,
            }),
            State(app_state),
            Path(uuid),
        )
        .await
        .unwrap();

        assert!(extract_message(result).contains("Got device"));
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_device() {
        let user = mock_auth_user(DUMMY_USER);
        let app_state = setup_test_state(DEVICES_TABLE).await;

        let device = RegisteredDevice {
            name: "Temp Device".into(),
            owner_id: Uuid::new_v4(),
            registered_at: chrono::Utc::now(),
            description: None,
        };

        let claims = user.0;

        let register_res = register_device(
            State(app_state.clone()),
            AuthUser(Claims {
                sub: claims.sub.clone(),
                exp: claims.exp,
                iat: claims.iat,
            }),
            Json(device),
        )
        .await
        .unwrap();

        let id = register_res.0.get("device_id").unwrap().as_str().unwrap();
        let uuid = Uuid::parse_str(id).unwrap();

        let result = delete_device(
            AuthUser(Claims {
                sub: claims.sub.clone(),
                exp: claims.exp,
                iat: claims.iat,
            }),
            State(app_state),
            Path(uuid),
        )
        .await
        .unwrap();

        assert!(extract_message(result).contains("Deleted device"));
    }
}
