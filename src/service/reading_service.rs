use crate::{
    domain::reading::Reading, error::AppError, repository::reading_repo::ReadingRepository,
    truncate_to_seconds,
};
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub struct ReadingService<R: ReadingRepository> {
    repo: R,
}

pub struct PostResult {
    pub inserted: u64,
    pub device_id: Option<Uuid>,
    pub created_at: Option<DateTime<Utc>>,
}

impl<R: ReadingRepository> ReadingService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn post_readings(
        &self,
        device_id: Uuid,
        readings: Vec<Reading>,
    ) -> Result<PostResult, AppError> {
        if device_id == Uuid::nil() {
            return Err(AppError::MissingArgument(
                "Device ID is required".to_string(),
            ));
        }

        let (count, created_at) = self.repo.insert_readings(device_id, &readings).await?;

        Ok(PostResult {
            inserted: count,
            device_id: Some(device_id),
            created_at: Some(created_at),
        })
    }

    pub async fn get_readings_filtered_paginated(
        &self,
        device_id: Uuid,
        from: Option<chrono::DateTime<chrono::Utc>>,
        to: Option<chrono::DateTime<chrono::Utc>>,
        cursor: Option<chrono::DateTime<chrono::Utc>>,
        limit: Option<usize>,
    ) -> Result<crate::repository::reading_repo::PaginatedResult<Reading>, AppError> {
        let from = from.map(truncate_to_seconds);
        let to = to.map(truncate_to_seconds);
        let cursor = cursor.map(truncate_to_seconds);
        self.repo
            .get_readings_filtered_paginated(device_id, from, to, cursor, limit)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::reading::{Reading, ReadingType},
        error::AppError,
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
    async fn test_post_readings_success() {
        let mut mock_repo = MockReadingRepository::new();
        let device_id = Uuid::new_v4();
        let readings = vec![make_test_reading(device_id), make_test_reading(device_id)];

        mock_repo
            .expect_insert_readings()
            .returning(|_device_id, _readings| Ok((1, Utc::now())));

        let service = ReadingService::new(mock_repo);
        let result = service.post_readings(device_id, readings).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_post_readings_bad_request() {
        let mock_repo = MockReadingRepository::new();
        let service = ReadingService::new(mock_repo);

        let device_id = Uuid::nil();
        let readings = vec![make_test_reading(device_id)];

        let result = service.post_readings(device_id, readings).await;
        assert!(matches!(result, Err(AppError::MissingArgument(_))));
    }

    #[tokio::test]
    async fn test_get_readings_filtered_paginated_success() {
        let mut mock_repo = MockReadingRepository::new();
        let device_id = Uuid::new_v4();

        let r1 = make_test_reading(device_id);
        let r2 = make_test_reading(device_id);

        let expected_result = PaginatedResult {
            data: vec![r1.clone(), r2.clone()],
            has_more: false,
            next_cursor: Some(r2.arrived_timestamp),
        };

        mock_repo
            .expect_get_readings_filtered_paginated()
            .return_once(move |_, _, _, _, _| Ok(expected_result.clone()));

        let service = ReadingService::new(mock_repo);

        let result = service
            .get_readings_filtered_paginated(device_id, None, None, None, Some(2))
            .await
            .unwrap();

        assert_eq!(result.data.len(), 2);
        assert!(!result.has_more);
        assert_eq!(result.next_cursor, Some(r2.arrived_timestamp));
    }

    #[tokio::test]
    async fn test_get_readings_filtered_paginated_returns_empty_result() {
        let mut mock_repo = MockReadingRepository::new();
        let device_id = Uuid::new_v4();

        let empty_result = PaginatedResult {
            data: vec![],
            has_more: false,
            next_cursor: None,
        };

        mock_repo
            .expect_get_readings_filtered_paginated()
            .return_once(move |_, _, _, _, _| Ok(empty_result));

        let service = ReadingService::new(mock_repo);

        let result = service
            .get_readings_filtered_paginated(device_id, None, None, None, Some(5))
            .await
            .unwrap();

        assert!(result.data.is_empty());
        assert!(!result.has_more);
    }
}
