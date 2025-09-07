use crate::{
    domain::reading::{Reading, ReadingType},
    error::AppError,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, query_builder::QueryBuilder};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct PaginatedResult<T> {
    pub data: Vec<T>,
    pub next_cursor: Option<DateTime<Utc>>,
    pub has_more: bool,
}

#[async_trait::async_trait]
pub trait ReadingRepository: Send + Sync {
    async fn insert_reading(&self, device_id: Uuid, reading: &Reading) -> Result<(), AppError>;
    async fn insert_readings(
        &self,
        device_id: Uuid,
        readings: &[Reading],
    ) -> Result<(u64, DateTime<Utc>), AppError>;
    async fn get_readings_filtered_paginated(
        &self,
        device_id: Uuid,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        cursor: Option<DateTime<Utc>>,
        limit: Option<usize>,
    ) -> Result<PaginatedResult<Reading>, AppError>;
}

#[async_trait::async_trait]
impl ReadingRepository for PgPool {
    async fn insert_reading(&self, device_id: Uuid, reading: &Reading) -> Result<(), AppError> {
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
        .map_err(|e| AppError::DatabaseError(format!("Failed to insert readings: {}", e)))?;

        Ok(())
    }

    async fn insert_readings(
        &self,
        device_id: Uuid,
        readings: &[Reading],
    ) -> Result<(u64, DateTime<Utc>), AppError> {
        if readings.is_empty() {
            return Ok((0, Utc::now()));
        }

        let mut query_builder: sqlx::QueryBuilder<Postgres> = sqlx::QueryBuilder::new(
            "INSERT INTO readings (device_id, arrived_timestamp, processed_timestamp, reading_type, value) ",
        );
        query_builder.push_values(readings, |mut b, reading| {
            b.push_bind(device_id)
                .push_bind(reading.arrived_timestamp)
                .push_bind(reading.processed_timestamp)
                .push_bind(reading.reading_type)
                .push_bind(reading.value);
        });

        let result =
            query_builder.build().execute(self).await.map_err(|e| {
                AppError::DatabaseError(format!("Failed to insert readings: {}", e))
            })?;

        Ok((result.rows_affected(), Utc::now()))
    }

    async fn get_readings_filtered_paginated(
        &self,
        device_id: Uuid,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        cursor: Option<DateTime<Utc>>,
        limit: Option<usize>,
    ) -> Result<PaginatedResult<Reading>, AppError> {
        let limit = limit.unwrap_or(100);

        let mut qb = QueryBuilder::new("SELECT * FROM readings WHERE device_id = ");
        qb.push_bind(device_id);

        if let Some(from) = from {
            qb.push(" AND arrived_timestamp >= ").push_bind(from);
        }

        if let Some(to) = to {
            qb.push(" AND arrived_timestamp <= ").push_bind(to);
        }

        if let Some(cursor) = cursor {
            qb.push(" AND arrived_timestamp < ").push_bind(cursor);
        }

        qb.push(" ORDER BY arrived_timestamp DESC ");
        qb.push(" LIMIT ").push_bind(limit as i64 + 1);
        let query = qb.build_query_as::<Reading>();

        let rows: Vec<Reading> = query.fetch_all(&*self).await.map_err(|e| {
            AppError::DatabaseError(format!("Failed to fetch paginated readings: {}", e))
        })?;

        let has_more = rows.len() > limit;
        let data = rows.into_iter().take(limit).collect::<Vec<_>>();
        let next_cursor = data.last().map(|r| r.arrived_timestamp);

        Ok(PaginatedResult {
            data,
            has_more,
            next_cursor,
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
            async fn insert_reading(&self, device_id: Uuid,reading: &Reading) -> Result<(), AppError>;
            async fn insert_readings(&self, device_id: Uuid,readings: &[Reading]) -> Result<(u64, DateTime<Utc>), AppError>;
            async fn get_readings_filtered_paginated(
                &self,
                device_id: Uuid,
                from: Option<DateTime<Utc>>,
                to: Option<DateTime<Utc>>,
                cursor: Option<DateTime<Utc>>,
                limit: Option<usize>,
            ) -> Result<PaginatedResult<Reading>, AppError>;
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
            .returning(|_device_id, _reading| Ok((1, Utc::now())));

        let device_id = Uuid::new_v4();
        let readings = vec![sample_reading(device_id, 0), sample_reading(device_id, 10)];

        let result = mock_repo.insert_readings(device_id, &readings).await;

        assert!(result.is_ok());
        let (count, timestamp) = result.unwrap();
        assert_eq!(count, 1);
        assert_ne!(timestamp.to_string().len(), 0);
    }

    #[tokio::test]
    async fn test_get_readings_filtered_paginated_success() {
        let mut mock_repo = MockReadingRepository::new();

        let device_id = Uuid::new_v4();

        let r1 = sample_reading(device_id, -30);
        let r2 = sample_reading(device_id, -20);
        let r3 = sample_reading(device_id, -10);

        let expected_result = PaginatedResult {
            data: vec![r1.clone(), r2.clone(), r3.clone()],
            has_more: false,
            next_cursor: Some(r3.arrived_timestamp),
        };

        mock_repo
            .expect_get_readings_filtered_paginated()
            .withf(move |d, from, to, cursor, limit| {
                *d == device_id
                    && from.is_none()
                    && to.is_none()
                    && cursor.is_none()
                    && limit == &Some(3)
            })
            .return_once(move |_, _, _, _, _| Ok(expected_result.clone()));

        let result = mock_repo
            .get_readings_filtered_paginated(device_id, None, None, None, Some(3))
            .await
            .unwrap();

        assert_eq!(result.data.len(), 3);
        assert!(!result.has_more);
        assert_eq!(result.next_cursor, Some(r3.arrived_timestamp));
    }

    #[tokio::test]
    async fn test_get_readings_filtered_paginated_empty() {
        let mut mock_repo = MockReadingRepository::new();

        let device_id = Uuid::new_v4();

        mock_repo
            .expect_get_readings_filtered_paginated()
            .return_once(|_, _, _, _, _| {
                Ok(PaginatedResult {
                    data: vec![],
                    has_more: false,
                    next_cursor: None,
                })
            });

        let result = mock_repo
            .get_readings_filtered_paginated(device_id, None, None, None, Some(5))
            .await
            .unwrap();

        assert!(result.data.is_empty());
        assert!(!result.has_more);
        assert!(result.next_cursor.is_none());
    }
}
