pub mod api;
pub mod app_state;
pub mod auth;
pub mod domain;
pub mod error;
pub mod repository;
pub mod service;

use crate::{app_state::AppState, error::AppError};
use axum::Router;
use chrono::{DateTime, Timelike, Utc};
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env::VarError;
use std::{env, net::SocketAddr};

pub async fn build_app() -> Result<(Router, SocketAddr), AppError> {
    tracing_subscriber::fmt::init();
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").map_err(|e| match e {
        VarError::NotPresent => AppError::EnvVarError("DATABASE_URL is not set".into()),
        VarError::NotUnicode(_) => {
            AppError::EnvVarError("DATABASE_URL is not valid unicode".into())
        }
    })?;
    let _ = env::var("JWT_SECRET").map_err(|e| match e {
        VarError::NotPresent => AppError::EnvVarError("JWT_SECRET is not set".into()),
        VarError::NotUnicode(_) => AppError::EnvVarError("JWT_SECRET is not valid unicode".into()),
    })?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let app_state = AppState { db_pool: pool };
    let app: Router = create_app(app_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    Ok((app, addr))
}

pub fn truncate_to_seconds(dt: DateTime<Utc>) -> DateTime<Utc> {
    dt.with_nanosecond(0).unwrap()
}

fn create_app(state: AppState) -> Router {
    Router::new()
        .nest("/devices", api::devices::routes())
        .nest("/readings", api::readings::routes())
        .nest("/users", api::users::routes())
        .with_state(state)
}
