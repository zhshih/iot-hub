use crate::{
    domain::device::{Device, RegisteredDevice},
    error::AppError,
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

    pub async fn register_device(&self, payload: RegisteredDevice) -> Result<String, AppError> {
        if payload.name.is_empty() || payload.owner_id == Uuid::nil() {
            return Err(AppError::MissingArgument(
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

    pub async fn get_device(&self, id: Uuid, requester_id: Uuid) -> Result<Device, AppError> {
        let device = self
            .repo
            .find_device_by_id(id)
            .await?
            .ok_or(AppError::NotFound("Device not found".to_string()))?;

        if device.owner_id != requester_id {
            return Err(AppError::NotFound("Device not found".to_string()));
        }

        Ok(device)
    }

    pub async fn get_devices(&self, owner_id: Uuid) -> Result<Vec<Device>, AppError> {
        self.repo.list_devices_by_owner(owner_id).await
    }

    pub async fn delete_device(&self, id: Uuid, requester_id: Uuid) -> Result<(), AppError> {
        let affected = self
            .repo
            .delete_device_by_id_and_owner(id, requester_id)
            .await?;
        if affected == 0 {
            return Err(AppError::NotFound(format!(
                "Device with id {} not found",
                id
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::device::{Device, RegisteredDevice};
    use async_trait::async_trait;
    use mockall::mock;
    use uuid::Uuid;

    mock! {
        pub DeviceRepository {}

        #[async_trait]
        impl DeviceRepository for DeviceRepository {
            async fn insert_device(&self, device: &Device) -> Result<(), AppError>;
            async fn find_device_by_id(&self, id: Uuid) -> Result<Option<Device>, AppError>;
            async fn list_all_device(&self) -> Result<Vec<Device>, AppError>;
            async fn list_devices_by_owner(&self, owner_id: Uuid) -> Result<Vec<Device>, AppError>;
            async fn delete_device_by_id_and_owner(&self, id: Uuid, owner_id: Uuid) -> Result<u64, AppError>;
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
    async fn test_register_device_missing_argument_error() {
        let mock_repo = MockDeviceRepository::new();
        let service = DeviceService::new(mock_repo);

        let bad_payload = RegisteredDevice {
            name: "".into(),
            description: None,
            owner_id: Uuid::nil(),
            registered_at: chrono::Utc::now(),
        };

        let result = service.register_device(bad_payload).await;
        assert!(matches!(result, Err(AppError::MissingArgument(_))));
    }

    #[tokio::test]
    async fn test_get_device_success() {
        let mut mock_repo = MockDeviceRepository::new();
        let device = make_test_device();
        let expected_id = device.id;
        let owner_id = device.owner_id;
        let device_for_closure = device.clone();

        mock_repo.expect_find_device_by_id().returning(move |id| {
            assert_eq!(id, expected_id);
            Ok(Some(device_for_closure.clone()))
        });

        let service = DeviceService::new(mock_repo);
        let result = service.get_device(expected_id, owner_id).await;

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
        let result = service.get_device(Uuid::new_v4(), Uuid::new_v4()).await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_device_forbidden_when_not_owner() {
        let mut mock_repo = MockDeviceRepository::new();
        let device = make_test_device();
        let expected_id = device.id;
        let device_for_closure = device.clone();

        mock_repo
            .expect_find_device_by_id()
            .returning(move |_| Ok(Some(device_for_closure.clone())));

        let service = DeviceService::new(mock_repo);
        let other_user_id = Uuid::new_v4();
        let result = service.get_device(expected_id, other_user_id).await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_devices_success() {
        let mut mock_repo = MockDeviceRepository::new();
        let devices = vec![make_test_device()];
        let owner_id = devices[0].owner_id;

        mock_repo
            .expect_list_devices_by_owner()
            .returning(move |_| Ok(devices.clone()));

        let service = DeviceService::new(mock_repo);
        let result = service.get_devices(owner_id).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_get_devices_returns_empty_list() {
        let mut mock_repo = MockDeviceRepository::new();
        mock_repo
            .expect_list_devices_by_owner()
            .returning(|_| Ok(vec![]));

        let service = DeviceService::new(mock_repo);
        let result = service.get_devices(Uuid::new_v4()).await;

        assert!(matches!(result, Ok(devices) if devices.is_empty()));
    }

    #[tokio::test]
    async fn test_delete_device_success() {
        let mut mock_repo = MockDeviceRepository::new();
        mock_repo
            .expect_delete_device_by_id_and_owner()
            .returning(|_, _| Ok(1));

        let service = DeviceService::new(mock_repo);
        let result = service.delete_device(Uuid::new_v4(), Uuid::new_v4()).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_device_not_found() {
        let mut mock_repo = MockDeviceRepository::new();
        mock_repo
            .expect_delete_device_by_id_and_owner()
            .returning(|_, _| Ok(0));

        let service = DeviceService::new(mock_repo);
        let result = service.delete_device(Uuid::new_v4(), Uuid::new_v4()).await;

        match result {
            Err(AppError::NotFound(msg)) => {
                assert!(msg.contains("Device with id"));
            }
            _ => panic!("Expected AppError::NotFound, got {:?}", result),
        }
    }
}
