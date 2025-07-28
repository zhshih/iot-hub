use axum::http::StatusCode;
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Claims {
    pub exp: usize,
    pub iat: usize,
    pub sub: String,
}

#[derive(Deserialize, Debug)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Debug)]
pub struct AuthResponse {
    pub token: String,
}

pub fn encode_jwt(id: String) -> Result<String, StatusCode> {
    let now = Utc::now();
    let expiration = now
        .checked_add_signed(Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;
    let iat: usize = now.timestamp() as usize;

    let claims = Claims {
        exp: expiration,
        iat,
        sub: id,
    };

    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub fn decode_jwt(jwt_token: &str) -> Result<TokenData<Claims>, StatusCode> {
    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let result = decode::<Claims>(
        jwt_token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(result)
}
