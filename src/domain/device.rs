use crate::dto::device::RegisterDeviceRequest;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: Uuid,
    pub registered_at: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct RegisteredDevice {
    pub name: String,
    pub description: Option<String>,
    pub owner_id: Uuid,
    pub registered_at: DateTime<Utc>,
}

impl RegisteredDevice {
    pub fn from_request(req: RegisterDeviceRequest) -> Self {
        Self {
            name: req.name,
            description: req.description,
            owner_id: req.owner_id,
            registered_at: Utc::now(),
        }
    }
}
