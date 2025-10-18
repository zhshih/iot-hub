use crate::{
    api::response::{ApiResponse, HandlerResult},
    app_state::AppState,
    auth::extractor::AuthUser,
    domain::user::SignupUser,
    dto::{
        auth::AuthRequest,
        user::{
            HealthCheckResponse, ListUsersResponse, LoginResponse, MeResponse, SignupRequest,
            SignupResponse,
        },
    },
    service::user_service::UserService,
};
use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_users))
        .route("/signup", post(signup))
        .route("/login", post(login))
        .route("/me", get(me))
        .route("/health", get(health_check))
}

async fn signup(
    State(state): State<AppState>,
    Json(payload): Json<SignupRequest>,
) -> HandlerResult<SignupResponse> {
    let service = UserService::new(state.db_pool.clone());

    let user = SignupUser::from_request(payload);
    let token = service.signup(user).await?;

    Ok(Json(ApiResponse::success(SignupResponse { token })))
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<AuthRequest>,
) -> HandlerResult<LoginResponse> {
    let service = UserService::new(state.db_pool.clone());
    let token = service.login(payload).await?;

    Ok(Json(ApiResponse::success(LoginResponse { token })))
}

async fn me(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> HandlerResult<MeResponse> {
    let service = UserService::new(state.db_pool.clone());
    let user = service.get_current_user_info(&claims).await?;

    Ok(Json(ApiResponse::success(MeResponse { user })))
}

async fn health_check(State(state): State<AppState>) -> HandlerResult<HealthCheckResponse> {
    let service = UserService::new(state.db_pool.clone());
    service.health_check().await?;

    Ok(Json(ApiResponse::success(HealthCheckResponse {})))
}

async fn list_users(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> HandlerResult<ListUsersResponse> {
    let service = UserService::new(state.db_pool.clone());
    let users = service.list_users(&claims).await?;

    Ok(Json(ApiResponse::success(ListUsersResponse { users })))
}
