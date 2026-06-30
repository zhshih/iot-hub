pub mod api;
pub mod app_state;
pub mod auth;
pub mod domain;
pub mod dto;
pub mod error;
pub mod repository;
pub mod service;

use crate::{app_state::AppState, error::AppError};
use axum::{Router, http, routing::get};
use axum_prometheus::PrometheusMetricLayer;
use chrono::{DateTime, Timelike, Utc};
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env::VarError;
use std::{env, net::SocketAddr};
use tower::limit::ConcurrencyLimitLayer;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

pub async fn build_app() -> Result<(Router, SocketAddr), AppError> {
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

pub fn init_tracing() {
    let fmt_layer = fmt::layer().pretty().with_target(false);

    let json_layer = fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(true);

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(fmt_layer)
        .with(json_layer)
        .init();
}

pub fn truncate_to_seconds(dt: DateTime<Utc>) -> DateTime<Utc> {
    dt.with_nanosecond(0).unwrap()
}

fn create_app(state: AppState) -> Router {
    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();

    let governor_conf = Box::new(
        GovernorConfigBuilder::default()
            .per_second(10)
            .burst_size(30)
            .finish()
            .unwrap(),
    );

    Router::new()
        .nest("/devices", api::devices::routes())
        .nest("/readings", api::readings::routes())
        .nest("/users", api::users::routes())
        .route(
            "/metrics",
            get({
                let handle = metric_handle.clone();
                move || async move { handle.render() }
            }),
        )
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &http::Request<_>| {
                    tracing::info_span!(
                        "request",
                        method = %request.method(),
                        uri = %request.uri(),
                        version = ?request.version(),
                    )
                })
                .on_response(
                    |response: &http::Response<_>,
                     latency: std::time::Duration,
                     span: &tracing::Span| {
                        span.record("status", tracing::field::display(response.status()));
                        tracing::info!(
                            parent: span,
                            status = ?response.status(),
                            latency = ?latency,
                            "response generated"
                        );
                    },
                ),
        )
        .layer(GovernorLayer::new(governor_conf))
        .layer(ConcurrencyLimitLayer::new(100))
        .layer(prometheus_layer)
}
