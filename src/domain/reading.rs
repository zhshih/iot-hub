use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reading {
    pub id: Uuid,
    pub device_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub reading_type: ReadingType,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReadingType {
    Temperature,
    Voltage,
    Humidity,
    Custom(String),
}
