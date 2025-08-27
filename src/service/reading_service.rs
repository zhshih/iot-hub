use crate::{
    api::error::ApiError, domain::reading::Reading, repository::reading_repo::ReadingRepository,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

const DEFAULT_LIMIT: i64 = 20;

#[derive(Deserialize)]
pub struct PaginationParams {
    pub cursor: Option<DateTime<Utc>>,
    pub limit: i64,
}

pub struct ReadingService<R: ReadingRepository> {
    repo: R,
}

impl<R: ReadingRepository> ReadingService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn post_reading(&self, device_id: Uuid, reading: &Reading) -> Result<bool, ApiError> {
        if device_id == Uuid::nil() {
            return Err(ApiError::BadRequest("Device ID is required".to_string()));
        }

        self.repo.insert_reading(device_id, reading).await?;

        Ok(true)
    }

    pub async fn post_readings(
        &self,
        device_id: Uuid,
        readings: Vec<Reading>,
    ) -> Result<bool, ApiError> {
        if device_id == Uuid::nil() {
            return Err(ApiError::BadRequest("Device ID is required".to_string()));
        }

        self.repo.insert_readings(device_id, &readings).await?;

        Ok(true)
    }

    pub async fn get_readings(&self, device_id: Uuid) -> Result<Vec<Reading>, ApiError> {
        let readings = self
            .repo
            .get_readings_by_device(device_id, DEFAULT_LIMIT)
            .await?;

        if readings.is_empty() {
            return Err(ApiError::NotFound(
                "No readings found for device".to_string(),
            ));
        }

        Ok(readings)
    }

    pub async fn get_latest_reading(&self, device_id: Uuid) -> Result<Reading, ApiError> {
        let reading = self
            .repo
            .get_latest_reading(device_id)
            .await?
            .ok_or(ApiError::NotFound(
                "No readings found for device".to_string(),
            ))?;

        Ok(reading)
    }

    pub async fn get_readings_in_range(
        &self,
        device_id: Uuid,
        from: chrono::DateTime<chrono::Utc>,
        to: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Reading>, ApiError> {
        let readings = self.repo.get_readings_in_range(device_id, from, to).await?;

        if readings.is_empty() {
            return Err(ApiError::NotFound(
                "No readings found for device in the specified range".to_string(),
            ));
        }

        Ok(readings)
    }

    pub async fn get_readings_paginated(
        &self,
        device_id: Uuid,
        cursor: Option<chrono::DateTime<chrono::Utc>>,
        limit: i64,
    ) -> Result<crate::repository::reading_repo::PaginatedResult<Reading>, ApiError> {
        let paginated_result = self
            .repo
            .get_readings_by_device_paginated(device_id, cursor, limit)
            .await?;

        if paginated_result.data.is_empty() {
            return Err(ApiError::NotFound(
                "No readings found for device".to_string(),
            ));
        }

        Ok(paginated_result)
    }

    pub async fn get_readings_in_range_paginated(
        &self,
        device_id: Uuid,
        from: chrono::DateTime<chrono::Utc>,
        to: chrono::DateTime<chrono::Utc>,
        cursor: Option<chrono::DateTime<chrono::Utc>>,
        limit: i64,
    ) -> Result<crate::repository::reading_repo::PaginatedResult<Reading>, ApiError> {
        let paginated_result = self
            .repo
            .get_readings_in_range_paginated(device_id, from, to, cursor, limit)
            .await?;

        if paginated_result.data.is_empty() {
            return Err(ApiError::NotFound(
                "No readings found for device in the specified range".to_string(),
            ));
        }

        Ok(paginated_result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::error::ApiError,
        domain::reading::{Reading, ReadingType},
        repository::reading_repo::PaginatedResult,
    };
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
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

    fn make_test_reading(device_id: Uuid) -> Reading {
        Reading {
            device_id,
            arrived_timestamp: Utc::now(),
            processed_timestamp: Utc::now(),
            reading_type: ReadingType::Temperature,
            value: 42.0,
        }
    }

    #[tokio::test]
    async fn test_post_reading_success() {
        let mut mock_repo = MockReadingRepository::new();
        let device_id = Uuid::new_v4();
        let reading = make_test_reading(device_id);

        mock_repo
            .expect_insert_reading()
            .returning(|_device_id, _reading| Ok(()));

        let service = ReadingService::new(mock_repo);
        let result = service.post_reading(reading.device_id, &reading).await;

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_post_reading_bad_request() {
        let mock_repo = MockReadingRepository::new();
        let service = ReadingService::new(mock_repo);

        let bad_reading = Reading {
            device_id: Uuid::nil(),
            arrived_timestamp: Utc::now(),
            processed_timestamp: Utc::now(),
            reading_type: ReadingType::Temperature,
            value: 10.0,
        };

        let result = service
            .post_reading(bad_reading.device_id, &bad_reading)
            .await;
        assert!(matches!(result, Err(ApiError::BadRequest(_))));
    }

    #[tokio::test]
    async fn test_post_readings_success() {
        let mut mock_repo = MockReadingRepository::new();
        let device_id = Uuid::new_v4();
        let readings = vec![make_test_reading(device_id), make_test_reading(device_id)];

        mock_repo
            .expect_insert_readings()
            .returning(|_device_id, _readings| Ok(()));

        let service = ReadingService::new(mock_repo);
        let result = service.post_readings(device_id, readings).await;

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_post_readings_bad_request() {
        let mock_repo = MockReadingRepository::new();
        let service = ReadingService::new(mock_repo);

        let device_id = Uuid::nil();
        let readings = vec![make_test_reading(device_id)];

        let result = service.post_readings(device_id, readings).await;
        assert!(matches!(result, Err(ApiError::BadRequest(_))));
    }

    #[tokio::test]
    async fn test_get_readings_success() {
        let mut mock_repo = MockReadingRepository::new();

        let device_id = Uuid::new_v4();
        let reading = make_test_reading(device_id);

        mock_repo
            .expect_get_readings_by_device()
            .returning(move |_, _| Ok(vec![reading.clone(), reading.clone()]));

        let service = ReadingService::new(mock_repo);
        let result = service.get_readings(device_id).await;

        assert!(result.is_ok());
        let readings = result.unwrap();
        assert_eq!(readings.len(), 2);
        assert!(readings.iter().all(|r| r.device_id == device_id));
    }

    #[tokio::test]
    async fn test_get_readings_not_found() {
        let mut mock_repo = MockReadingRepository::new();
        let device_id = Uuid::new_v4();

        mock_repo
            .expect_get_readings_by_device()
            .returning(|_, _| Ok(vec![]));

        let service = ReadingService::new(mock_repo);
        let result = service.get_readings(device_id).await;

        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_latest_reading_success() {
        let mut mock_repo = MockReadingRepository::new();
        let device_id = Uuid::new_v4();
        let reading = make_test_reading(device_id);

        mock_repo
            .expect_get_latest_reading()
            .returning(move |_| Ok(Some(reading.clone())));

        let service = ReadingService::new(mock_repo);
        let result = service.get_latest_reading(device_id).await;

        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.device_id, device_id);
    }

    #[tokio::test]
    async fn test_get_latest_reading_not_found() {
        let mut mock_repo = MockReadingRepository::new();
        let device_id = Uuid::new_v4();

        mock_repo
            .expect_get_latest_reading()
            .returning(|_| Ok(None));

        let service = ReadingService::new(mock_repo);
        let result = service.get_latest_reading(device_id).await;

        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_readings_in_range_success() {
        let mut mock_repo = MockReadingRepository::new();
        let device_id = Uuid::new_v4();
        let from = Utc::now();
        let to = from + chrono::Duration::hours(1);
        let reading = make_test_reading(device_id);

        mock_repo
            .expect_get_readings_in_range()
            .returning(move |_, _, _| Ok(vec![reading.clone()]));

        let service = ReadingService::new(mock_repo);
        let result = service.get_readings_in_range(device_id, from, to).await;

        assert!(result.is_ok());
        let readings = result.unwrap();
        assert_eq!(readings.len(), 1);
        assert_eq!(readings[0].device_id, device_id);
    }

    #[tokio::test]
    async fn test_get_readings_in_range_not_found() {
        let mut mock_repo = MockReadingRepository::new();
        let device_id = Uuid::new_v4();
        let from = Utc::now();
        let to = from + chrono::Duration::hours(1);

        mock_repo
            .expect_get_readings_in_range()
            .returning(|_, _, _| Ok(vec![]));

        let service = ReadingService::new(mock_repo);
        let result = service.get_readings_in_range(device_id, from, to).await;

        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_readings_paginated_success() {
        let mut mock_repo = MockReadingRepository::new();
        let device_id = Uuid::new_v4();
        let reading = make_test_reading(device_id);

        mock_repo
            .expect_get_readings_by_device_paginated()
            .returning(move |_, _, _| {
                Ok(PaginatedResult {
                    data: vec![reading.clone()],
                    next_cursor: Some(reading.arrived_timestamp),
                    has_more: false,
                })
            });

        let service = ReadingService::new(mock_repo);
        let result = service.get_readings_paginated(device_id, None, 10).await;

        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.data.len(), 1);
        assert_eq!(page.data[0].device_id, device_id);
        assert!(!page.has_more);
    }

    #[tokio::test]
    async fn test_get_readings_paginated_not_found() {
        let mut mock_repo = MockReadingRepository::new();
        let device_id = Uuid::new_v4();

        mock_repo
            .expect_get_readings_by_device_paginated()
            .returning(|_, _, _| {
                Ok(PaginatedResult {
                    data: vec![],
                    next_cursor: None,
                    has_more: false,
                })
            });

        let service = ReadingService::new(mock_repo);
        let result = service.get_readings_paginated(device_id, None, 10).await;

        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_readings_paginated_repo_error() {
        let mut mock_repo = MockReadingRepository::new();
        let device_id = Uuid::new_v4();

        mock_repo
            .expect_get_readings_by_device_paginated()
            .returning(|_, _, _| Err(ApiError::InternalServerError("db error".to_string())));

        let service = ReadingService::new(mock_repo);
        let result = service.get_readings_paginated(device_id, None, 10).await;

        assert!(matches!(result, Err(ApiError::InternalServerError(_))));
    }

    #[tokio::test]
    async fn test_get_readings_in_range_paginated_success() {
        let mut mock_repo = MockReadingRepository::new();
        let device_id = Uuid::new_v4();
        let from = Utc::now();
        let to = from + chrono::Duration::hours(1);
        let reading = make_test_reading(device_id);

        mock_repo
            .expect_get_readings_in_range_paginated()
            .returning(move |_, _, _, _, _| {
                Ok(PaginatedResult {
                    data: vec![reading.clone()],
                    next_cursor: Some(reading.arrived_timestamp),
                    has_more: true,
                })
            });

        let service = ReadingService::new(mock_repo);
        let result = service
            .get_readings_in_range_paginated(device_id, from, to, None, 10)
            .await;

        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.data.len(), 1);
        assert_eq!(page.data[0].device_id, device_id);
        assert!(page.has_more);
    }

    #[tokio::test]
    async fn test_get_readings_in_range_paginated_not_found() {
        let mut mock_repo = MockReadingRepository::new();
        let device_id = Uuid::new_v4();
        let from = Utc::now();
        let to = from + chrono::Duration::hours(1);

        mock_repo
            .expect_get_readings_in_range_paginated()
            .returning(|_, _, _, _, _| {
                Ok(PaginatedResult {
                    data: vec![],
                    next_cursor: None,
                    has_more: false,
                })
            });

        let service = ReadingService::new(mock_repo);
        let result = service
            .get_readings_in_range_paginated(device_id, from, to, None, 10)
            .await;

        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_readings_in_range_paginated_repo_error() {
        let mut mock_repo = MockReadingRepository::new();
        let device_id = Uuid::new_v4();
        let from = Utc::now();
        let to = from + chrono::Duration::hours(1);

        mock_repo
            .expect_get_readings_in_range_paginated()
            .returning(|_, _, _, _, _| {
                Err(ApiError::InternalServerError("query failed".to_string()))
            });

        let service = ReadingService::new(mock_repo);
        let result = service
            .get_readings_in_range_paginated(device_id, from, to, None, 10)
            .await;

        assert!(matches!(result, Err(ApiError::InternalServerError(_))));
    }
}
