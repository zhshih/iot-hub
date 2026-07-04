use crate::domain::ids::{DeviceId, UserId};
use crate::dto::device::RegisterDeviceRequest;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: DeviceId,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: UserId,
    pub registered_at: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct RegisteredDevice {
    pub name: String,
    pub description: Option<String>,
    pub owner_id: UserId,
    pub registered_at: DateTime<Utc>,
}

impl RegisteredDevice {
    pub fn from_request(req: RegisterDeviceRequest, owner_id: UserId) -> Self {
        Self {
            name: req.name,
            description: req.description,
            owner_id,
            registered_at: Utc::now(),
        }
    }
}
