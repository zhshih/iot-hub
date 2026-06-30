use crate::domain::user::PublicUser;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct SignupRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct TokenResponse<T> {
    pub token: T,
}

#[derive(Serialize)]
pub struct SignupResponse {
    pub token: String,
    pub user_id: Uuid,
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
