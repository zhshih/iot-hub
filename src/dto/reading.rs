use crate::domain::reading::Reading;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ReadingRequest {
    pub arrived_timestamp: DateTime<Utc>,
    pub reading_type: String,
    pub value: f64,
}

#[derive(Serialize)]
pub struct PostReadingResponse {
    pub inserted: u64,
    pub device_id: Option<Uuid>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct GetReadingResponse {
    pub device_id: Uuid,
    pub readings: Vec<Reading>,
}

#[derive(Serialize)]
pub struct GetPaginatedReadingResponse {
    pub device_id: Uuid,
    pub readings: Vec<Reading>,
    pub next_cursor: Option<i64>,
    pub has_more: bool,
}
