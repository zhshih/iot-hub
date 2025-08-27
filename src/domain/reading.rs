use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Type)]
#[sqlx(type_name = "TEXT")]
pub enum ReadingType {
    Temperature,
    Voltage,
    Humidity,
}

impl fmt::Display for ReadingType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ReadingType::Temperature => write!(f, "temperature"),
            ReadingType::Voltage => write!(f, "voltage"),
            ReadingType::Humidity => write!(f, "humidity"),
        }
    }
}
