use crate::auth::middleware::AuthUser;
use axum::{
    Router,
    extract::Path,
    routing::{delete, get, post},
};

pub fn routes() -> Router {
    Router::new()
        .route("/", post(register_device))
        .route("/", get(get_devices))
        .route("/{device_id}", get(get_device))
        .route("/{device_id}", delete(delete_device))
}

async fn register_device(AuthUser(user): AuthUser) -> String {
    format!("Registered device for user: {}", user.sub)
}

async fn get_devices(AuthUser(user): AuthUser) -> String {
    format!("Got devices for user: {}", user.sub)
}

async fn get_device(Path(device_id): Path<String>, AuthUser(user): AuthUser) -> String {
    format!("Got device {} for {}", device_id, user.sub)
}

async fn delete_device(Path(device_id): Path<String>, AuthUser(user): AuthUser) -> String {
    format!("Deleted device {} for user {}", device_id, user.sub)
}
