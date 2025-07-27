use crate::auth::middleware::AuthUser;
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

async fn post_reading(Path(id): Path<String>, AuthUser(user): AuthUser) -> String {
    format!("Posted post_reading {} for user {}", id, user.sub)
}

async fn get_reading(Path(id): Path<String>, AuthUser(user): AuthUser) -> String {
    format!("Got reading {} for user {}", id, user.sub)
}
