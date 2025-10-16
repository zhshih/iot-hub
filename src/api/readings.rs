use super::error::ApiError;
use crate::{
    api::response::{ApiResponse, HandlerResult},
    auth::extractor::AuthUser,
    service::reading_service::ReadingService,
    {app_state::AppState, domain::reading::Reading},
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ReadingQuery {
    pub from: Option<i64>,
    pub to: Option<i64>,
    pub cursor: Option<i64>,
    pub limit: Option<usize>,
}

#[derive(Serialize)]
pub struct PostReadingResponse {
    pub inserted: u64,
    pub device_id: Option<Uuid>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct GetReadingResponse {
    pub device_id: Uuid,
    pub readings: Vec<Reading>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}

impl<T> OneOrMany<T> {
    pub fn into_vec(self) -> Vec<T> {
        match self {
            OneOrMany::One(t) => vec![t],
            OneOrMany::Many(v) => v,
        }
    }
}

#[derive(Serialize)]
pub struct GetPaginatedReadingResponse {
    pub device_id: Uuid,
    pub readings: Vec<Reading>,
    pub next_cursor: Option<i64>,
    pub has_more: bool,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/{device_id}/readings", post(post_readings))
        .route("/{device_id}/readings", get(get_readings))
        .route("/{device_id}/readings/latest", get(get_latest_readings))
}

async fn post_readings(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    AuthUser(_user): AuthUser,
    Json(payload): Json<OneOrMany<Reading>>,
) -> HandlerResult<PostReadingResponse> {
    let service = ReadingService::new(state.db_pool.clone());
    let readings = payload.into_vec();

    let readings: Vec<Reading> = readings
        .into_iter()
        .map(|mut r| {
            r.device_id = id;
            r
        })
        .collect();

    let result = service.post_readings(id, readings).await?;

    Ok(Json(ApiResponse::success(PostReadingResponse {
        inserted: result.inserted,
        device_id: result.device_id,
        created_at: result.created_at,
    })))
}

async fn get_readings(
    State(state): State<AppState>,
    Path(device_id): Path<Uuid>,
    Query(params): Query<ReadingQuery>,
    AuthUser(_user): AuthUser,
) -> HandlerResult<GetPaginatedReadingResponse> {
    let service = ReadingService::new(state.db_pool.clone());

    let from = params.from.and_then(|ts| Utc.timestamp_opt(ts, 0).single());
    let to = params.to.and_then(|ts| Utc.timestamp_opt(ts, 0).single());
    let cursor = params
        .cursor
        .and_then(|ts| Utc.timestamp_opt(ts, 0).single());

    let result = service
        .get_readings_filtered_paginated(device_id, from, to, cursor, params.limit)
        .await?;

    Ok(Json(ApiResponse::success(GetPaginatedReadingResponse {
        device_id,
        readings: result.data,
        next_cursor: result.next_cursor.map(|dt| dt.timestamp()),
        has_more: result.has_more,
    })))
}

async fn get_latest_readings(
    State(state): State<AppState>,
    Path(device_id): Path<Uuid>,
    AuthUser(_user): AuthUser,
) -> HandlerResult<GetReadingResponse> {
    let service = ReadingService::new(state.db_pool.clone());
    let result = service
        .get_readings_filtered_paginated(device_id, None, None, None, Some(1))
        .await?;

    let reading =
        result.data.into_iter().next().ok_or_else(|| {
            ApiError::NotFound(format!("No readings found for device {}", device_id))
        })?;

    Ok(Json(ApiResponse::success(GetReadingResponse {
        device_id,
        readings: vec![reading],
    })))
}
