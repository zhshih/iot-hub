use crate::{
    api::response::{ApiResponse, HandlerResult},
    app_state::AppState,
    auth::extractor::AuthUser,
    domain::device::RegisteredDevice,
    domain::ids::DeviceId,
    dto::device::{
        DeleteDeviceResponse, GetDeviceResponse, GetDevicesResponse, RegisterDeviceRequest,
        RegisterDeviceResponse, UpdateDeviceRequest,
    },
    service::device_service::DeviceService,
};
use axum::{
    Json,
    extract::{Path, State},
};
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(register_device, get_devices))
        .routes(routes!(get_device, delete_device, update_device))
}

#[utoipa::path(
    post,
    path = "/",
    request_body = RegisterDeviceRequest,
    responses(
        (status = 200, description = "Device registered", body = ApiResponse<RegisterDeviceResponse>),
        (status = 400, description = "Missing/invalid fields"),
    ),
    description = "Requires a Bearer JWT.",
    tag = "devices",
)]
pub(crate) async fn register_device(
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

#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "The caller's own devices", body = ApiResponse<GetDevicesResponse>),
    ),
    description = "Requires a Bearer JWT.",
    tag = "devices",
)]
pub(crate) async fn get_devices(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> HandlerResult<GetDevicesResponse> {
    let owner_id = claims.user_id()?;
    let service = DeviceService::new(state.db_pool.clone());
    let devices = service.get_devices(owner_id).await?;

    Ok(Json(ApiResponse::success(GetDevicesResponse { devices })))
}

#[utoipa::path(
    get,
    path = "/{device_id}",
    params(("device_id" = DeviceId, Path, description = "Device id")),
    responses(
        (status = 200, description = "Device details", body = ApiResponse<GetDeviceResponse>),
        (status = 404, description = "Not found, or not owned by the caller"),
    ),
    description = "Requires a Bearer JWT. A device you don't own returns 404, not 403.",
    tag = "devices",
)]
pub(crate) async fn get_device(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<DeviceId>,
) -> HandlerResult<GetDeviceResponse> {
    let requester_id = claims.user_id()?;
    let service = DeviceService::new(state.db_pool.clone());
    let device = service.get_device(id, requester_id).await?;

    Ok(Json(ApiResponse::success(GetDeviceResponse { device })))
}

#[utoipa::path(
    delete,
    path = "/{device_id}",
    params(("device_id" = DeviceId, Path, description = "Device id")),
    responses(
        (status = 200, description = "Device deleted", body = ApiResponse<DeleteDeviceResponse>),
        (status = 404, description = "Not found, or not owned by the caller"),
    ),
    description = "Requires a Bearer JWT. A device you don't own returns 404, not 403.",
    tag = "devices",
)]
pub(crate) async fn delete_device(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<DeviceId>,
) -> HandlerResult<DeleteDeviceResponse> {
    let requester_id = claims.user_id()?;
    let service = DeviceService::new(state.db_pool.clone());
    service.delete_device(id, requester_id).await?;

    Ok(Json(ApiResponse::success(DeleteDeviceResponse {
        device_id: id.to_string(),
    })))
}

#[utoipa::path(
    patch,
    path = "/{device_id}",
    params(("device_id" = DeviceId, Path, description = "Device id")),
    request_body = UpdateDeviceRequest,
    responses(
        (status = 200, description = "Updated device (partial update; omitted fields unchanged)", body = ApiResponse<GetDeviceResponse>),
        (status = 404, description = "Not found, or not owned by the caller"),
    ),
    description = "Requires a Bearer JWT. A device you don't own returns 404, not 403.",
    tag = "devices",
)]
pub(crate) async fn update_device(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<DeviceId>,
    Json(payload): Json<UpdateDeviceRequest>,
) -> HandlerResult<GetDeviceResponse> {
    let requester_id = claims.user_id()?;
    let service = DeviceService::new(state.db_pool.clone());
    let device = service
        .update_device(
            id,
            requester_id,
            payload.name,
            payload.description,
            payload.is_active,
        )
        .await?;

    Ok(Json(ApiResponse::success(GetDeviceResponse { device })))
}
