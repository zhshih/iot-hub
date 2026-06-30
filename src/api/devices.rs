use crate::{
    api::response::{ApiResponse, HandlerResult},
    app_state::AppState,
    auth::extractor::AuthUser,
    domain::device::RegisteredDevice,
    dto::device::{
        DeleteDeviceResponse, GetDeviceResponse, GetDevicesResponse, RegisterDeviceRequest,
        RegisterDeviceResponse,
    },
    service::device_service::DeviceService,
};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get, post},
};
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
    AuthUser(claims): AuthUser,
    Json(payload): Json<RegisterDeviceRequest>,
) -> HandlerResult<RegisterDeviceResponse> {
    let owner_id = claims.user_id()?;
    let service = DeviceService::new(state.db_pool.clone());

    let device = RegisteredDevice::from_request(payload, owner_id);
    let id = service.register_device(device).await?;

    Ok(Json(ApiResponse::success(RegisterDeviceResponse {
        device_id: id,
    })))
}

async fn get_devices(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> HandlerResult<GetDevicesResponse> {
    let owner_id = claims.user_id()?;
    let service = DeviceService::new(state.db_pool.clone());
    let devices = service.get_devices(owner_id).await?;

    Ok(Json(ApiResponse::success(GetDevicesResponse { devices })))
}

async fn get_device(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> HandlerResult<GetDeviceResponse> {
    let requester_id = claims.user_id()?;
    let service = DeviceService::new(state.db_pool.clone());
    let device = service.get_device(id, requester_id).await?;

    Ok(Json(ApiResponse::success(GetDeviceResponse { device })))
}

async fn delete_device(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> HandlerResult<DeleteDeviceResponse> {
    let requester_id = claims.user_id()?;
    let service = DeviceService::new(state.db_pool.clone());
    service.delete_device(id, requester_id).await?;

    Ok(Json(ApiResponse::success(DeleteDeviceResponse {
        device_id: id.to_string(),
    })))
}
