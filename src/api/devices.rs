use crate::{
    api::response::{ApiResponse, HandlerResult},
    app_state::AppState,
    auth::extractor::AuthUser,
    domain::device::{Device, RegisteredDevice},
    service::device_service::DeviceService,
};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get, post},
};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct GenericDeviceResponse<T> {
    pub device_id: T,
}

#[derive(Serialize)]
struct GetDevicesResponse {
    pub devices: Vec<Device>,
}

#[derive(Serialize)]
pub struct GetDeviceResponse {
    pub device: Device,
}

type RegisterDeviceResponse = GenericDeviceResponse<String>;
type DeleteDeviceResponse = GenericDeviceResponse<String>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", post(register_device))
        .route("/", get(get_devices))
        .route("/{device_id}", get(get_device))
        .route("/{device_id}", delete(delete_device))
}

async fn register_device(
    State(state): State<AppState>,
    AuthUser(_user): AuthUser,
    Json(payload): Json<RegisteredDevice>,
) -> HandlerResult<RegisterDeviceResponse> {
    let service = DeviceService::new(state.db_pool.clone());
    let id = service.register_device(payload).await?;

    Ok(Json(ApiResponse::success(RegisterDeviceResponse {
        device_id: id,
    })))
}

async fn get_devices(
    AuthUser(_user): AuthUser,
    State(state): State<AppState>,
) -> HandlerResult<GetDevicesResponse> {
    let service = DeviceService::new(state.db_pool.clone());
    let devices = service.get_devices().await?;

    println!("Devices fetched: {:?}", devices);
    Ok(Json(ApiResponse::success(GetDevicesResponse { devices })))
}

async fn get_device(
    AuthUser(_user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> HandlerResult<GetDeviceResponse> {
    let service = DeviceService::new(state.db_pool.clone());
    let device = service.get_device(id).await?;

    Ok(Json(ApiResponse::success(GetDeviceResponse { device })))
}

async fn delete_device(
    AuthUser(_user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> HandlerResult<DeleteDeviceResponse> {
    let service = DeviceService::new(state.db_pool.clone());
    service.delete_device(id).await?;

    Ok(Json(ApiResponse::success(DeleteDeviceResponse {
        device_id: id.to_string(),
    })))
}
