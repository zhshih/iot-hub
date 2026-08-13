use crate::domain::ids::UserId;
use crate::domain::user::PublicUser;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct SignupRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

// A missing field means "leave unchanged".
#[derive(Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub username: Option<String>,
    pub email: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct TokenResponse<T> {
    pub token: T,
}

#[derive(Serialize, ToSchema)]
pub struct SignupResponse {
    pub token: String,
    pub user_id: UserId,
}

#[derive(Serialize, ToSchema)]
pub struct MeResponse {
    pub user: PublicUser,
}

#[derive(Serialize, ToSchema)]
pub struct HealthCheckResponse;

#[derive(Serialize, ToSchema)]
pub struct ListUsersResponse {
    pub users: Vec<PublicUser>,
}

pub type LoginResponse = TokenResponse<String>;
