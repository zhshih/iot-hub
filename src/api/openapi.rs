use crate::api;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        api::devices::register_device,
        api::devices::get_devices,
        api::devices::get_device,
        api::devices::update_device,
        api::devices::delete_device,
        api::readings::post_readings,
        api::readings::get_readings,
        api::readings::get_latest_readings,
        api::users::signup,
        api::users::login,
        api::users::me,
        api::users::update_me,
        api::users::health_check,
        api::users::list_users,
    ),
    components(schemas(
        crate::dto::device::RegisterDeviceRequest,
        crate::dto::device::UpdateDeviceRequest,
        crate::dto::device::GetDevicesResponse,
        crate::dto::device::GetDeviceResponse,
        crate::dto::user::SignupRequest,
        crate::dto::user::UpdateUserRequest,
        crate::dto::user::SignupResponse,
        crate::dto::user::MeResponse,
        crate::dto::user::HealthCheckResponse,
        crate::dto::user::ListUsersResponse,
        crate::dto::reading::ReadingRequest,
        crate::dto::reading::PostReadingResponse,
        crate::dto::reading::GetReadingResponse,
        crate::dto::reading::GetPaginatedReadingResponse,
        crate::dto::auth::AuthRequest,
        crate::domain::device::Device,
        crate::domain::user::PublicUser,
        crate::domain::user::UserRole,
        crate::domain::reading::Reading,
        crate::domain::reading::ReadingType,
        crate::domain::ids::UserId,
        crate::domain::ids::DeviceId,
    )),
    tags(
        (name = "devices", description = "Device management"),
        (name = "readings", description = "Device telemetry readings"),
        (name = "users", description = "User accounts and auth"),
    )
)]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_spec_builds() {
        let _ = ApiDoc::openapi();
    }
}
