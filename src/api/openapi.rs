use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(tags(
    (name = "devices", description = "Device management"),
    (name = "readings", description = "Device telemetry readings"),
    (name = "users", description = "User accounts and auth"),
))]
pub struct ApiDoc;
