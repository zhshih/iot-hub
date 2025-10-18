use crate::domain::user::{PublicUser, User};
use serde::{Deserialize, Serialize};

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
pub struct MeResponse {
    pub user: PublicUser,
}

#[derive(Serialize)]
pub struct HealthCheckResponse;

#[derive(Serialize)]
pub struct ListUsersResponse {
    pub users: Vec<User>,
}

pub type SignupResponse = TokenResponse<String>;
pub type LoginResponse = TokenResponse<String>;
