use crate::domain::ids::DeviceId;
use crate::domain::reading::Reading;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct ReadingRequest {
    pub arrived_timestamp: DateTime<Utc>,
    pub reading_type: String,
    pub value: f64,
}

#[derive(Serialize, ToSchema)]
pub struct PostReadingResponse {
    pub inserted: u64,
    pub device_id: Option<DeviceId>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, ToSchema)]
pub struct GetReadingResponse {
    pub device_id: DeviceId,
    pub readings: Vec<Reading>,
}

#[derive(Serialize, ToSchema)]
pub struct GetPaginatedReadingResponse {
    pub device_id: DeviceId,
    pub readings: Vec<Reading>,
    pub next_cursor: Option<i64>,
    pub has_more: bool,
}
