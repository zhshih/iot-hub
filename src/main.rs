mod api;
mod auth;

use axum::Router;
use dotenvy::dotenv;
use std::env;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    dotenv().ok();
    let _ = env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    let app = Router::new()
        .nest("/devices", api::devices::routes())
        .nest("/readings", api::readings::routes())
        .nest("/users", api::users::routes());

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::info!("Listening on {}", addr);
    axum::serve(listener, app).await.unwrap();
}
