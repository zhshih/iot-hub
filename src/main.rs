mod api;
mod auth;

use axum::{Extension, Router};
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::{env, net::SocketAddr};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let _ = env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to the database");

    let app = Router::new()
        .nest("/devices", api::devices::routes())
        .nest("/readings", api::readings::routes())
        .nest("/users", api::users::routes())
        .layer(Extension(pool.clone()));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::info!("Listening on {}", addr);
    axum::serve(listener, app).await.unwrap();
}
