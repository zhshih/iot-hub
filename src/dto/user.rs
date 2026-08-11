use crate::domain::ids::UserId;
use crate::domain::user::PublicUser;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct SignupRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

// A missing field means "leave unchanged".
#[derive(Deserialize)]
pub struct UpdateUserRequest {
    pub username: Option<String>,
    pub email: Option<String>,
}

#[derive(Serialize)]
pub struct TokenResponse<T> {
    pub token: T,
}

#[derive(Serialize)]
pub struct SignupResponse {
    pub token: String,
    pub user_id: UserId,
}

#[derive(Serialize)]
pub struct MeResponse {
    pub user: PublicUser,
}

#[derive(Serialize)]
pub struct HealthCheckResponse;

#[derive(Serialize)]
pub struct ListUsersResponse {
    pub users: Vec<PublicUser>,
}

pub type LoginResponse = TokenResponse<String>;
