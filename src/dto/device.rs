use crate::domain::device::Device;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct RegisterDeviceRequest {
    pub name: String,
    pub description: Option<String>,
}

// A missing field means "leave unchanged"; there's no way to clear
// `description` to NULL through this endpoint.
#[derive(Deserialize)]
pub struct UpdateDeviceRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Serialize)]
pub struct GenericDeviceResponse<T> {
    pub device_id: T,
}

#[derive(Serialize)]
pub struct GetDevicesResponse {
    pub devices: Vec<Device>,
}

#[derive(Serialize)]
pub struct GetDeviceResponse {
    pub device: Device,
}

pub type RegisterDeviceResponse = GenericDeviceResponse<String>;
pub type DeleteDeviceResponse = GenericDeviceResponse<String>;
