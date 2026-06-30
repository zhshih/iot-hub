use crate::error::{AppError, ValidationError};
use axum::http::StatusCode;
use chrono::{Duration, Utc};
#[cfg(not(feature = "mock-auth"))]
use jsonwebtoken::{DecodingKey, TokenData, Validation, decode};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use std::env;
use std::env::VarError;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Claims {
    pub exp: usize,
    pub iat: usize,
    /// The authenticated user's id (as a UUID string), not their username.
    pub sub: String,
}

impl Claims {
    pub fn user_id(&self) -> Result<Uuid, AppError> {
        Uuid::parse_str(&self.sub).map_err(|_| {
            AppError::ValidationError(ValidationError::InvalidInput(
                "Invalid user id in token".to_string(),
            ))
        })
    }
}

pub fn encode_jwt(id: String) -> Result<String, StatusCode> {
    let now = Utc::now();
    let expiration = now
        .checked_add_signed(Duration::hours(24))
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .timestamp() as usize;
    let iat: usize = now.timestamp() as usize;

    let claims = Claims {
        exp: expiration,
        iat,
        sub: id,
    };

    let secret = load_jwt_secret()?;

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(token)
}

#[cfg(not(feature = "mock-auth"))]
pub fn decode_jwt(jwt_token: &str) -> Result<TokenData<Claims>, StatusCode> {
    let secret = load_jwt_secret()?;

    let result = decode::<Claims>(
        jwt_token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(result)
}

fn load_jwt_secret() -> Result<String, StatusCode> {
    env::var("JWT_SECRET").map_err(|err| match err {
        VarError::NotPresent => StatusCode::SERVICE_UNAVAILABLE,
        VarError::NotUnicode(_) => StatusCode::INTERNAL_SERVER_ERROR,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    #[test]
    #[serial]
    fn test_load_jwt_secret_missing() {
        unsafe { env::remove_var("JWT_SECRET") };
        let result = load_jwt_secret();
        assert_eq!(result.unwrap_err(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    #[serial]
    fn test_load_jwt_secret_present() {
        unsafe {
            env::set_var("JWT_SECRET", "mysecret");
        }
        let result = load_jwt_secret();
        assert_eq!(result.unwrap().as_str(), "mysecret");
    }

    #[test]
    fn test_encode_and_decode_jwt() {
        unsafe {
            env::set_var("JWT_SECRET", "supersecret");
        }
        #[cfg(not(feature = "mock-auth"))]
        {
            let user_id = "user123".to_string();
            let token = encode_jwt(user_id.clone()).expect("Failed to encode JWT");
            let decoded = decode_jwt(&token).expect("Failed to decode JWT");
            assert_eq!(decoded.claims.sub, user_id);
        }
    }
}
