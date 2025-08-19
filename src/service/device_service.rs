use crate::{
    api::error::ApiError,
    domain::device::{Device, RegisteredDevice},
    repository::device_repo::DeviceRepository,
};
use uuid::Uuid;

pub struct DeviceService<R: DeviceRepository> {
    repo: R,
}

impl<R: DeviceRepository> DeviceService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn register_device(&self, payload: RegisteredDevice) -> Result<String, ApiError> {
        if payload.name.is_empty() || payload.owner_id == Uuid::nil() {
            return Err(ApiError::BadRequest(
                "Device name and owner ID are required".to_string(),
            ));
        }

        let id = Uuid::new_v4();
        let device = Device {
            id,
            name: payload.name.clone(),
            description: payload.description.clone(),
            owner_id: payload.owner_id,
            registered_at: payload.registered_at,
            is_active: true,
        };

        self.repo.insert_device(&device).await?;

        Ok(id.to_string())
    }

    pub async fn get_device(&self, id: Uuid) -> Result<Device, ApiError> {
        let device = self
            .repo
            .find_device_by_id(id)
            .await?
            .ok_or(ApiError::NotFound("Device not found".to_string()))?;

        Ok(device)
    }

    pub async fn get_devices(&self) -> Result<Vec<Device>, ApiError> {
        let devices = self.repo.list_all_device().await?;
        if devices.is_empty() {
            return Err(ApiError::NotFound("Devices not found".to_string()));
        }

        Ok(devices)
    }

    pub async fn delete_device(&self, id: Uuid) -> Result<(), ApiError> {
        let affected = self.repo.delete_device_by_id(id).await?;
        if affected == 0 {
            return Err(ApiError::NotFound("Device not found".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::error::ApiError,
        domain::device::{Device, RegisteredDevice},
    };
    use async_trait::async_trait;
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

    fn make_test_device() -> Device {
        Device {
            id: Uuid::new_v4(),
            name: "Test Device".into(),
            description: Some("Description".into()),
            owner_id: Uuid::new_v4(),
            registered_at: chrono::Utc::now(),
            is_active: true,
        }
    }

    fn make_registered_device() -> RegisteredDevice {
        RegisteredDevice {
            name: "New Device".into(),
            description: Some("Description".into()),
            owner_id: Uuid::new_v4(),
            registered_at: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_register_device_success() {
        let mut mock_repo = MockDeviceRepository::new();
        mock_repo.expect_insert_device().returning(|_| Ok(()));

        let service = DeviceService::new(mock_repo);

        let payload = make_registered_device();
        let result = service.register_device(payload).await;

        assert!(result.is_ok());
        let id = result.unwrap();
        assert!(!id.is_empty());
    }

    #[tokio::test]
    async fn test_register_device_bad_request() {
        let mock_repo = MockDeviceRepository::new();
        let service = DeviceService::new(mock_repo);

        let bad_payload = RegisteredDevice {
            name: "".into(),
            description: None,
            owner_id: Uuid::nil(),
            registered_at: chrono::Utc::now(),
        };

        let result = service.register_device(bad_payload).await;
        assert!(matches!(result, Err(ApiError::BadRequest(_))));
    }

    #[tokio::test]
    async fn test_get_device_success() {
        let mut mock_repo = MockDeviceRepository::new();
        let device = make_test_device();
        let expected_id = device.id;
        let device_for_closure = device.clone();

        mock_repo.expect_find_device_by_id().returning(move |id| {
            assert_eq!(id, expected_id);
            Ok(Some(device_for_closure.clone()))
        });

        let service = DeviceService::new(mock_repo);
        let result = service.get_device(expected_id).await;

        assert!(result.is_ok());

        let returned_device = result.unwrap();
        assert_eq!(returned_device.id, expected_id);
        assert_eq!(returned_device.name, device.name);
        assert_eq!(returned_device.description, device.description);
        assert_eq!(returned_device.owner_id, device.owner_id);
        assert_eq!(returned_device.is_active, device.is_active);
    }

    #[tokio::test]
    async fn test_get_device_not_found() {
        let mut mock_repo = MockDeviceRepository::new();
        mock_repo.expect_find_device_by_id().returning(|_| Ok(None));

        let service = DeviceService::new(mock_repo);
        let result = service.get_device(Uuid::new_v4()).await;

        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_devices_success() {
        let mut mock_repo = MockDeviceRepository::new();
        let devices = vec![make_test_device()];

        mock_repo
            .expect_list_all_device()
            .returning(move || Ok(devices.clone()));

        let service = DeviceService::new(mock_repo);
        let result = service.get_devices().await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_get_devices_not_found() {
        let mut mock_repo = MockDeviceRepository::new();
        mock_repo.expect_list_all_device().returning(|| Ok(vec![]));

        let service = DeviceService::new(mock_repo);
        let result = service.get_devices().await;

        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_device_success() {
        let mut mock_repo = MockDeviceRepository::new();
        mock_repo.expect_delete_device_by_id().returning(|_| Ok(1));

        let service = DeviceService::new(mock_repo);
        let result = service.delete_device(Uuid::new_v4()).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_device_not_found() {
        let mut mock_repo = MockDeviceRepository::new();
        mock_repo.expect_delete_device_by_id().returning(|_| Ok(0));

        let service = DeviceService::new(mock_repo);
        let result = service.delete_device(Uuid::new_v4()).await;

        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }
}
