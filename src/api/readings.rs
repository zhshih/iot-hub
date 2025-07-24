use axum::{
    Router,
    extract::Path,
    routing::{get, post},
};

pub fn routes() -> Router {
    Router::new()
        .route("/{id}", post(post_reading))
        .route("/{id}", get(get_reading))
}

async fn post_reading(Path(id): Path<String>) -> String {
    format!("Posted post_reading {}", id)
}

async fn get_reading(Path(id): Path<String>) -> String {
    format!("Got reading {}", id)
}
