use crate::dto::reading::ReadingRequest;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, FromRow)]
pub struct Reading {
    pub device_id: Uuid,
    pub arrived_timestamp: DateTime<Utc>,
    pub processed_timestamp: DateTime<Utc>,
    pub reading_type: ReadingType,
    pub value: f64,
}

impl fmt::Display for Reading {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Reading(device_id={}, arrived={}, processed={}, type={:?}, value={})",
            self.device_id,
            self.arrived_timestamp,
            self.processed_timestamp,
            self.reading_type,
            self.value
        )
    }
}

impl Reading {
    pub fn from_request(req: ReadingRequest, device_id: Uuid) -> Self {
        Self {
            device_id,
            arrived_timestamp: req.arrived_timestamp,
            processed_timestamp: Utc::now(),
            reading_type: ReadingType::from(req.reading_type),
            value: req.value,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Type)]
#[sqlx(type_name = "TEXT")]
pub enum ReadingType {
    Temperature,
    Voltage,
    Humidity,
    Unknown,
}

impl fmt::Display for ReadingType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ReadingType::Temperature => write!(f, "temperature"),
            ReadingType::Voltage => write!(f, "voltage"),
            ReadingType::Humidity => write!(f, "humidity"),
            ReadingType::Unknown => write!(f, "unknown"),
        }
    }
}

impl From<String> for ReadingType {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "temperature" => Self::Temperature,
            "humidity" => Self::Humidity,
            "voltage" => Self::Voltage,
            _ => Self::Unknown,
        }
    }
}
