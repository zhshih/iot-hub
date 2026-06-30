use crate::domain::device::Device;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct RegisterDeviceRequest {
    pub name: String,
    pub description: Option<String>,
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
