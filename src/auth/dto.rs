use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Debug)]
pub struct AuthResponse {
    pub token: String,
}
