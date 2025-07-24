use axum::{
    Router,
    routing::{get, post},
};

pub fn routes() -> Router {
    Router::new()
        .route("/signup", post(signup))
        .route("/login", post(login))
        .route("/me", get(me))
}

async fn signup() -> &'static str {
    "User signed up"
}

async fn login() -> &'static str {
    "User logged in"
}

async fn me() -> &'static str {
    "Current user"
}
