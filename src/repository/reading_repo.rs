use crate::{
    api::error::ApiError,
    domain::reading::{Reading, ReadingType},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct PaginatedResult<T> {
    pub data: Vec<T>,
    pub next_cursor: Option<DateTime<Utc>>,
    pub has_more: bool,
}

#[async_trait::async_trait]
pub trait ReadingRepository: Send + Sync {
    async fn insert_reading(&self, device_id: Uuid, reading: &Reading) -> Result<(), ApiError>;
    async fn insert_readings(&self, device_id: Uuid, readings: &[Reading]) -> Result<(), ApiError>;
    async fn get_readings_by_device(
        &self,
        device_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Reading>, ApiError>;
    async fn get_latest_reading(&self, device_id: Uuid) -> Result<Option<Reading>, ApiError>;
    async fn get_readings_in_range(
        &self,
        device_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Reading>, ApiError>;
    async fn get_readings_by_device_paginated(
        &self,
        device_id: Uuid,
        cursor: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<PaginatedResult<Reading>, ApiError>;

    async fn get_readings_in_range_paginated(
        &self,
        device_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        cursor: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<PaginatedResult<Reading>, ApiError>;
}

#[async_trait::async_trait]
impl ReadingRepository for PgPool {
    async fn insert_reading(&self, device_id: Uuid, reading: &Reading) -> Result<(), ApiError> {
        sqlx::query!(
            r#"
            INSERT INTO readings (device_id, arrived_timestamp, processed_timestamp, reading_type, value)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            device_id,
            reading.arrived_timestamp,
            reading.processed_timestamp,
            reading.reading_type as ReadingType,
            reading.value,
        )
        .execute(self)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to insert readings: {}", e)))?;

        Ok(())
    }

    async fn insert_readings(&self, device_id: Uuid, readings: &[Reading]) -> Result<(), ApiError> {
        for reading in readings {
            self.insert_reading(device_id, reading).await?;
        }

        Ok(())
    }

    async fn get_readings_by_device(
        &self,
        device_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Reading>, ApiError> {
        let readings = if limit > 0 {
            sqlx::query_as!(
                Reading,
                r#"
                SELECT device_id, arrived_timestamp, processed_timestamp, reading_type as "reading_type: _", value
                FROM readings
                WHERE device_id = $1
                ORDER BY arrived_timestamp ASC
                LIMIT $2
                "#,
                device_id,
                limit
            )
            .fetch_all(self)
            .await
        } else {
            sqlx::query_as!(
                Reading,
                r#"
                SELECT device_id, arrived_timestamp, processed_timestamp, reading_type as "reading_type: _", value
                FROM readings
                WHERE device_id = $1
                ORDER BY arrived_timestamp ASC
                "#,
                device_id
            )
            .fetch_all(self)
            .await
        }
        .map_err(|e| {
            ApiError::InternalServerError(format!("Failed to fetch readings by device: {}", e))
        })?;

        Ok(readings)
    }

    async fn get_latest_reading(&self, device_id: Uuid) -> Result<Option<Reading>, ApiError> {
        let reading = sqlx::query_as!(
            Reading,
            r#"
            SELECT device_id, arrived_timestamp, processed_timestamp, reading_type as "reading_type: _", value
            FROM readings
            WHERE device_id = $1
            ORDER BY arrived_timestamp DESC
            LIMIT 1
            "#,
            device_id
        )
        .fetch_optional(self)
        .await
        .map_err(|e| {
            ApiError::InternalServerError(format!(
                "Failed to fetch latest reading by device: {}",
                e
            ))
        })?;

        Ok(reading)
    }

    async fn get_readings_in_range(
        &self,
        device_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Reading>, ApiError> {
        let readings = sqlx::query_as!(
            Reading,
            r#"
            SELECT device_id, arrived_timestamp, processed_timestamp, reading_type as "reading_type: _", value
            FROM readings
            WHERE device_id = $1
              AND arrived_timestamp >= $2
              AND arrived_timestamp <= $3
            ORDER BY arrived_timestamp ASC
            "#,
            device_id,
            from,
            to
        )
        .fetch_all(self)
        .await
        .map_err(|e| {
            ApiError::InternalServerError(format!("Failed to fetch readings by device: {}", e))
        })?;

        Ok(readings)
    }

    async fn get_readings_by_device_paginated(
        &self,
        device_id: Uuid,
        cursor: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<PaginatedResult<Reading>, ApiError> {
        let readings = if let Some(cursor) = cursor {
            sqlx::query_as!(
                Reading,
                r#"
                SELECT device_id, arrived_timestamp, processed_timestamp, reading_type as "reading_type: _", value
                FROM readings
                WHERE device_id = $1
                AND arrived_timestamp > $2
                ORDER BY arrived_timestamp ASC
                LIMIT $3
                "#,
                device_id,
                cursor,
                limit,
            )
            .fetch_all(self)
            .await
        } else {
            sqlx::query_as!(
                Reading,
                r#"
                SELECT device_id, arrived_timestamp, processed_timestamp, reading_type as "reading_type: _", value
                FROM readings
                WHERE device_id = $1
                ORDER BY arrived_timestamp ASC
                LIMIT $2
                "#,
                device_id,
                limit,
            )
            .fetch_all(self)
            .await
        }
        .map_err(|e| {
            ApiError::InternalServerError(format!("Failed to fetch paginated readings: {}", e))
        })?;

        let next_cursor = readings.last().map(|r| r.arrived_timestamp);

        let has_more = readings.len() as i64 == limit;

        Ok(PaginatedResult {
            data: readings,
            next_cursor,
            has_more,
        })
    }

    async fn get_readings_in_range_paginated(
        &self,
        device_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        cursor: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<PaginatedResult<Reading>, ApiError> {
        let readings = if let Some(cursor) = cursor {
            sqlx::query_as!(
                Reading,
                r#"
                SELECT device_id, arrived_timestamp, processed_timestamp, reading_type as "reading_type: _", value
                FROM readings
                WHERE device_id = $1
                AND arrived_timestamp > $2
                AND arrived_timestamp <= $3
                ORDER BY arrived_timestamp ASC
                LIMIT $4
                "#,
                device_id,
                cursor,
                to,
                limit,
            )
            .fetch_all(self)
            .await
        } else {
            sqlx::query_as!(
                Reading,
                r#"
                SELECT device_id, arrived_timestamp, processed_timestamp, reading_type as "reading_type: _", value
                FROM readings
                WHERE device_id = $1
                AND arrived_timestamp >= $2
                AND arrived_timestamp <= $3
                ORDER BY arrived_timestamp ASC
                LIMIT $4
                "#,
                device_id,
                from,
                to,
                limit,
            )
            .fetch_all(self)
            .await
        }
        .map_err(|e| {
            ApiError::InternalServerError(format!("Failed to fetch paginated readings: {}", e))
        })?;

        let next_cursor = readings.last().map(|r| r.arrived_timestamp);

        let has_more = readings.len() as i64 == limit;

        Ok(PaginatedResult {
            data: readings,
            next_cursor,
            has_more,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::reading::ReadingType;
    use async_trait::async_trait;
    use chrono::{Duration, Utc};
    use mockall::mock;
    use uuid::Uuid;

    mock! {
        pub ReadingRepository {}

        #[async_trait]
        impl ReadingRepository for ReadingRepository {
            async fn insert_reading(&self, device_id: Uuid,reading: &Reading) -> Result<(), ApiError>;
            async fn insert_readings(&self, device_id: Uuid,readings: &[Reading]) -> Result<(), ApiError>;
            async fn get_readings_by_device(
                &self,
                device_id: Uuid,
                limit: i64,
            ) -> Result<Vec<Reading>, ApiError>;
            async fn get_latest_reading(&self, device_id: Uuid) -> Result<Option<Reading>, ApiError>;
            async fn get_readings_in_range(
                &self,
                device_id: Uuid,
                from: DateTime<Utc>,
                to: DateTime<Utc>,
            ) -> Result<Vec<Reading>, ApiError>;

            async fn get_readings_by_device_paginated(
                &self,
                device_id: Uuid,
                cursor: Option<DateTime<Utc>>,
                limit: i64,
            ) -> Result<PaginatedResult<Reading>, ApiError>;

            async fn get_readings_in_range_paginated(
                &self,
                device_id: Uuid,
                from: DateTime<Utc>,
                to: DateTime<Utc>,
                cursor: Option<DateTime<Utc>>,
                limit: i64,
            ) -> Result<PaginatedResult<Reading>, ApiError>;
        }
    }

    fn sample_reading(device_id: Uuid, offset_secs: i64) -> Reading {
        Reading {
            device_id,
            arrived_timestamp: Utc::now() + Duration::seconds(offset_secs),
            processed_timestamp: Utc::now() + Duration::seconds(offset_secs + 1),
            reading_type: ReadingType::Temperature,
            value: 42.0,
        }
    }

    #[tokio::test]
    async fn test_insert_reading_success() {
        let mut mock_repo = MockReadingRepository::new();
        mock_repo
            .expect_insert_reading()
            .returning(|_device_id, _reading| Ok(()));

        let device_id = Uuid::new_v4();
        let reading = sample_reading(device_id, 0);

        let result = mock_repo.insert_reading(device_id, &reading).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_insert_readings_success() {
        let mut mock_repo = MockReadingRepository::new();
        mock_repo
            .expect_insert_readings()
            .returning(|_device_id, _reading| Ok(()));

        let device_id = Uuid::new_v4();
        let readings = vec![sample_reading(device_id, 0), sample_reading(device_id, 10)];

        let result = mock_repo.insert_readings(device_id, &readings).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_readings_by_device_returns_multiple() {
        let mut mock_repo = MockReadingRepository::new();

        let device_id = Uuid::new_v4();
        let readings = vec![sample_reading(device_id, 0), sample_reading(device_id, 60)];

        mock_repo
            .expect_get_readings_by_device()
            .returning(move |_, _| Ok(readings.clone()));

        let found = mock_repo
            .get_readings_by_device(device_id, 0)
            .await
            .unwrap();
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].device_id, device_id);
    }

    #[tokio::test]
    async fn test_get_readings_by_device_with_limit() {
        let mut mock_repo = MockReadingRepository::new();

        let device_id = Uuid::new_v4();
        let readings = vec![sample_reading(device_id, 0), sample_reading(device_id, 60)];

        mock_repo
            .expect_get_readings_by_device()
            .returning(move |_, limit| {
                if limit > 0 {
                    Ok(readings.clone().into_iter().take(limit as usize).collect())
                } else {
                    Ok(readings.clone())
                }
            });

        let found = mock_repo
            .get_readings_by_device(device_id, 1)
            .await
            .unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].device_id, device_id);
    }

    #[tokio::test]
    async fn test_get_latest_reading_returns_last() {
        let mut mock_repo = MockReadingRepository::new();

        let device_id = Uuid::new_v4();
        let latest = sample_reading(device_id, 100);

        let latest_cloned = latest.clone();
        mock_repo
            .expect_get_latest_reading()
            .returning(move |_| Ok(Some(latest_cloned.clone())));

        let found = mock_repo.get_latest_reading(device_id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap(), latest);
    }

    #[tokio::test]
    async fn test_get_readings_in_range_returns_subset() {
        let mut mock_repo = MockReadingRepository::new();

        let device_id = Uuid::new_v4();
        let from = Utc::now();
        let to = from + Duration::minutes(5);

        let readings = vec![sample_reading(device_id, 10), sample_reading(device_id, 20)];

        mock_repo
            .expect_get_readings_in_range()
            .returning(move |_, _, _| Ok(readings.clone()));

        let found = mock_repo
            .get_readings_in_range(device_id, from, to)
            .await
            .unwrap();
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].device_id, device_id);
    }

    #[tokio::test]
    async fn test_get_readings_by_device_paginated_no_cursor() {
        let mut mock_repo = MockReadingRepository::new();

        let device_id = Uuid::new_v4();
        let readings = vec![sample_reading(device_id, 10), sample_reading(device_id, 20)];

        mock_repo
            .expect_get_readings_by_device_paginated()
            .returning(move |_, cursor, limit| {
                assert!(cursor.is_none());
                let mut result = readings.clone();
                result.truncate(limit as usize);
                Ok(PaginatedResult {
                    data: result.clone(),
                    next_cursor: result.last().map(|r| r.arrived_timestamp),
                    has_more: result.len() as i64 == limit,
                })
            });

        let result = mock_repo
            .get_readings_by_device_paginated(device_id, None, 2)
            .await
            .unwrap();

        assert_eq!(result.data.len(), 2);
        assert!(result.next_cursor.is_some());
        assert!(result.has_more);
    }

    #[tokio::test]
    async fn test_get_readings_by_device_paginated_with_cursor() {
        let mut mock_repo = MockReadingRepository::new();

        let device_id = Uuid::new_v4();
        let cursor = Utc::now();
        let readings = vec![sample_reading(device_id, 30)];
        mock_repo
            .expect_get_readings_by_device_paginated()
            .returning(move |_, cur, limit| {
                assert!(cur.is_some());
                assert_eq!(limit, 1);
                Ok(PaginatedResult {
                    data: readings.clone(),
                    next_cursor: Some(readings[0].arrived_timestamp),
                    has_more: readings.len() as i64 == limit,
                })
            });

        let result = mock_repo
            .get_readings_by_device_paginated(device_id, Some(cursor), 1)
            .await
            .unwrap();

        assert_eq!(result.data.len(), 1);
        assert!(result.next_cursor.is_some());
        assert!(result.has_more);
    }

    #[tokio::test]
    async fn test_get_readings_in_range_paginated_no_cursor() {
        let mut mock_repo = MockReadingRepository::new();

        let device_id = Uuid::new_v4();
        let from = Utc::now();
        let to = from + Duration::minutes(5);
        let readings = vec![sample_reading(device_id, 10), sample_reading(device_id, 20)];

        mock_repo
            .expect_get_readings_in_range_paginated()
            .returning(move |_, f, t, cursor, limit| {
                assert!(cursor.is_none());
                assert!(f <= t);
                let mut result = readings.clone();
                result.truncate(limit as usize);
                Ok(PaginatedResult {
                    data: result.clone(),
                    next_cursor: result.last().map(|r| r.arrived_timestamp),
                    has_more: result.len() as i64 == limit,
                })
            });

        let result = mock_repo
            .get_readings_in_range_paginated(device_id, from, to, None, 2)
            .await
            .unwrap();

        assert_eq!(result.data.len(), 2);
        assert!(result.next_cursor.is_some());
        assert!(result.has_more);
    }

    #[tokio::test]
    async fn test_get_readings_in_range_paginated_with_cursor() {
        let mut mock_repo = MockReadingRepository::new();

        let device_id = Uuid::new_v4();
        let from = Utc::now();
        let to = from + Duration::minutes(5);
        let cursor = from + Duration::seconds(30);
        let readings = vec![sample_reading(device_id, 40)];

        mock_repo
            .expect_get_readings_in_range_paginated()
            .returning(move |_, f, t, cur, limit| {
                assert!(cur.is_some());
                assert!(f <= t);
                assert_eq!(limit, 1);
                Ok(PaginatedResult {
                    data: readings.clone(),
                    next_cursor: Some(readings[0].arrived_timestamp),
                    has_more: false,
                })
            });

        let result = mock_repo
            .get_readings_in_range_paginated(device_id, from, to, Some(cursor), 1)
            .await
            .unwrap();

        assert_eq!(result.data.len(), 1);
        assert!(result.next_cursor.is_some());
        assert!(!result.has_more);
    }

    #[tokio::test]
    async fn test_get_readings_by_device_paginated_error() {
        let mut mock_repo = MockReadingRepository::new();

        let device_id = Uuid::new_v4();

        mock_repo
            .expect_get_readings_by_device_paginated()
            .returning(move |_, _, _| {
                Err(ApiError::InternalServerError(
                    "DB connection failed".to_string(),
                ))
            });

        let result = mock_repo
            .get_readings_by_device_paginated(device_id, None, 10)
            .await;

        assert!(result.is_err());
        match result {
            Err(ApiError::InternalServerError(msg)) => {
                assert!(msg.contains("DB connection failed"));
            }
            _ => panic!("Expected InternalServerError"),
        }
    }

    #[tokio::test]
    async fn test_get_readings_in_range_paginated_error() {
        let mut mock_repo = MockReadingRepository::new();

        let device_id = Uuid::new_v4();
        let from = Utc::now();
        let to = from + Duration::minutes(5);

        mock_repo
            .expect_get_readings_in_range_paginated()
            .returning(move |_, _, _, _, _| {
                Err(ApiError::InternalServerError("Query timeout".to_string()))
            });

        let result = mock_repo
            .get_readings_in_range_paginated(device_id, from, to, None, 5)
            .await;

        assert!(result.is_err());
        match result {
            Err(ApiError::InternalServerError(msg)) => {
                assert!(msg.contains("Query timeout"));
            }
            _ => panic!("Expected InternalServerError"),
        }
    }
}
