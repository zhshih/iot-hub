use crate::{api::error::ApiError, domain::device::Device};
use sqlx::PgPool;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait DeviceRepository: Send + Sync {
    async fn insert_device(&self, device: &Device) -> Result<(), ApiError>;
    async fn find_device_by_id(&self, id: Uuid) -> Result<Option<Device>, ApiError>;
    async fn list_all_device(&self) -> Result<Vec<Device>, ApiError>;
    async fn delete_device_by_id(&self, id: Uuid) -> Result<u64, ApiError>;
}

#[async_trait::async_trait]
impl DeviceRepository for PgPool {
    async fn insert_device(&self, device: &Device) -> Result<(), ApiError> {
        sqlx::query!(
            r#"
            INSERT INTO devices (id, name, description, owner_id, registered_at, is_active)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            device.id,
            device.name,
            device.description,
            device.owner_id,
            device.registered_at,
            device.is_active,
        )
        .execute(self)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to insert device: {}", e)))?;

        Ok(())
    }

    async fn find_device_by_id(&self, id: Uuid) -> Result<Option<Device>, ApiError> {
        let device = sqlx::query_as!(
            Device,
            r#"
            SELECT id, name, description, owner_id, registered_at, is_active
            FROM devices
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(self)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to fetch device: {}", e)))?;

        Ok(device)
    }

    async fn list_all_device(&self) -> Result<Vec<Device>, ApiError> {
        let devices = sqlx::query_as!(
            Device,
            r#"
            SELECT id, name, description, owner_id, registered_at, is_active
            FROM devices
            ORDER BY registered_at DESC
            "#
        )
        .fetch_all(self)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to fetch devices: {}", e)))?;

        Ok(devices)
    }

    async fn delete_device_by_id(&self, id: Uuid) -> Result<u64, ApiError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM devices
            WHERE id = $1
            "#,
            id
        )
        .execute(self)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to delete device: {}", e)))?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use mockall::mock;
    use uuid::Uuid;

    mock! {
        pub DeviceRepository {}

        #[async_trait]
        impl DeviceRepository for DeviceRepository {
            async fn insert_device(&self, device: &Device) -> Result<(), ApiError>;
            async fn find_device_by_id(&self, id: Uuid) -> Result<Option<Device>, ApiError>;
            async fn list_all_device(&self) -> Result<Vec<Device>, ApiError>;
            async fn delete_device_by_id(&self, id: Uuid) -> Result<u64, ApiError>;
        }
    }

    #[tokio::test]
    async fn test_find_device_by_id_returns_device() {
        let mut mock_repo = MockDeviceRepository::new();

        let device = Device {
            id: Uuid::new_v4(),
            name: "Mocked Device".to_string(),
            description: Some("From mock".to_string()),
            owner_id: Uuid::new_v4(),
            registered_at: Utc::now(),
            is_active: true,
        };

        mock_repo
            .expect_find_device_by_id()
            .returning(move |_| Ok(Some(device.clone())));

        let found = mock_repo.find_device_by_id(Uuid::new_v4()).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Mocked Device");
    }

    #[tokio::test]
    async fn test_list_all_device_returns_multiple() {
        let mut mock_repo = MockDeviceRepository::new();

        let devices = vec![
            Device {
                id: Uuid::new_v4(),
                name: "Device 1".to_string(),
                description: None,
                owner_id: Uuid::new_v4(),
                registered_at: Utc::now(),
                is_active: true,
            },
            Device {
                id: Uuid::new_v4(),
                name: "Device 2".to_string(),
                description: None,
                owner_id: Uuid::new_v4(),
                registered_at: Utc::now(),
                is_active: false,
            },
        ];

        mock_repo
            .expect_list_all_device()
            .returning(move || Ok(devices.clone()));

        let all = mock_repo.list_all_device().await.unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].name, "Device 1");
    }

    #[tokio::test]
    async fn test_insert_device_success() {
        let mut mock_repo = MockDeviceRepository::new();
        mock_repo.expect_insert_device().returning(|_| Ok(()));

        let device = Device {
            id: Uuid::new_v4(),
            name: "Insert Test".to_string(),
            description: None,
            owner_id: Uuid::new_v4(),
            registered_at: Utc::now(),
            is_active: true,
        };

        let result = mock_repo.insert_device(&device).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_device_success() {
        let mut mock_repo = MockDeviceRepository::new();

        mock_repo.expect_delete_device_by_id().returning(|_| Ok(1));

        let result = mock_repo.delete_device_by_id(Uuid::new_v4()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }
}
