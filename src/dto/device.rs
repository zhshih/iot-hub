use crate::domain::device::Device;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, ToSchema)]
pub struct RegisterDeviceRequest {
    pub name: String,
    pub description: Option<String>,
}

// A missing field means "leave unchanged"; there's no way to clear
// `description` to NULL through this endpoint.
#[derive(Deserialize, ToSchema)]
pub struct UpdateDeviceRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Deserialize, ToSchema)]
pub struct TransferDeviceRequest {
    pub new_owner_id: Uuid,
}

#[derive(Serialize, ToSchema)]
pub struct GenericDeviceResponse<T> {
    pub device_id: T,
}

#[derive(Serialize, ToSchema)]
pub struct GetDevicesResponse {
    pub devices: Vec<Device>,
}

#[derive(Serialize, ToSchema)]
pub struct GetDeviceResponse {
    pub device: Device,
}

pub type RegisterDeviceResponse = GenericDeviceResponse<String>;
pub type DeleteDeviceResponse = GenericDeviceResponse<String>;
pub type TransferDeviceResponse = GenericDeviceResponse<String>;
