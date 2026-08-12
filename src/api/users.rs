use crate::{
    api::response::{ApiResponse, HandlerResult},
    app_state::AppState,
    auth::extractor::AuthUser,
    domain::user::SignupUser,
    dto::{
        auth::AuthRequest,
        user::{
            HealthCheckResponse, ListUsersResponse, LoginResponse, MeResponse, SignupRequest,
            SignupResponse, UpdateUserRequest,
        },
    },
    service::user_service::UserService,
};
use axum::{Json, extract::State};
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_users))
        .routes(routes!(signup))
        .routes(routes!(login))
        .routes(routes!(me, update_me))
        .routes(routes!(health_check))
}

#[utoipa::path(
    post,
    path = "/signup",
    request_body = SignupRequest,
    responses(
        (status = 200, description = "User registered; response includes a token", body = ApiResponse<SignupResponse>),
        (status = 400, description = "Missing/invalid fields"),
    ),
    description = "No auth required.",
    tag = "users",
)]
pub(crate) async fn signup(
    State(state): State<AppState>,
    Json(payload): Json<SignupRequest>,
) -> HandlerResult<SignupResponse> {
    let service = UserService::new(state.db_pool.clone());

    let user = SignupUser::from_request(payload);
    let (token, user_id) = service.signup(user).await?;

    Ok(Json(ApiResponse::success(SignupResponse {
        token,
        user_id,
    })))
}

#[utoipa::path(
    post,
    path = "/login",
    request_body = AuthRequest,
    responses(
        (status = 200, description = "Authenticated; response includes a token", body = ApiResponse<LoginResponse>),
        (status = 401, description = "Invalid username or password"),
    ),
    description = "No auth required.",
    tag = "users",
)]
pub(crate) async fn login(
    State(state): State<AppState>,
    Json(payload): Json<AuthRequest>,
) -> HandlerResult<LoginResponse> {
    let service = UserService::new(state.db_pool.clone());
    let token = service.login(payload).await?;

    Ok(Json(ApiResponse::success(LoginResponse { token })))
}

#[utoipa::path(
    get,
    path = "/me",
    responses(
        (status = 200, description = "The caller's own user info", body = ApiResponse<MeResponse>),
    ),
    description = "Requires a Bearer JWT.",
    tag = "users",
)]
pub(crate) async fn me(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> HandlerResult<MeResponse> {
    let service = UserService::new(state.db_pool.clone());
    let user = service.get_current_user_info(&claims).await?;

    Ok(Json(ApiResponse::success(MeResponse { user })))
}

#[utoipa::path(
    patch,
    path = "/me",
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "Updated user (partial update; omitted fields unchanged)", body = ApiResponse<MeResponse>),
        (status = 400, description = "Empty username/email"),
    ),
    description = "Requires a Bearer JWT.",
    tag = "users",
)]
pub(crate) async fn update_me(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<UpdateUserRequest>,
) -> HandlerResult<MeResponse> {
    let service = UserService::new(state.db_pool.clone());
    let user = service
        .update_current_user(&claims, payload.username, payload.email)
        .await?;

    Ok(Json(ApiResponse::success(MeResponse { user })))
}

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy", body = ApiResponse<HealthCheckResponse>),
    ),
    description = "No auth required.",
    tag = "users",
)]
pub(crate) async fn health_check(
    State(state): State<AppState>,
) -> HandlerResult<HealthCheckResponse> {
    let service = UserService::new(state.db_pool.clone());
    service.health_check().await?;

    Ok(Json(ApiResponse::success(HealthCheckResponse {})))
}

#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "All users", body = ApiResponse<ListUsersResponse>),
        (status = 403, description = "Caller is not an Admin"),
    ),
    description = "Requires a Bearer JWT for an Admin user.",
    tag = "users",
)]
pub(crate) async fn list_users(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> HandlerResult<ListUsersResponse> {
    let service = UserService::new(state.db_pool.clone());
    let users = service.list_users(&claims).await?;

    Ok(Json(ApiResponse::success(ListUsersResponse { users })))
}
