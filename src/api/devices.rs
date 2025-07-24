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

async fn register_device() -> &'static str {
    "Registered device"
}

async fn get_devices() -> &'static str {
    "Got devices"
}

async fn get_device(Path(device_id): Path<String>) -> String {
    format!("Got device {}", device_id)
}

async fn delete_device(Path(device_id): Path<String>) -> String {
    format!("Deleted device {}", device_id)
}
