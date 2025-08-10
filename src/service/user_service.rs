use crate::{
    api::error::ApiError,
    auth::jwt::{self, AuthRequest, Claims},
    domain::user::{SignupRequest, User, UserRole},
};

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::Utc;
use password_hash::{SaltString, rand_core::OsRng};
use sqlx::PgPool;
use uuid::Uuid;

pub struct UserService {
    pool: PgPool,
}

impl UserService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn signup(&self, payload: SignupRequest) -> Result<String, ApiError> {
        if payload.username.is_empty() || payload.password.is_empty() || payload.email.is_empty() {
            return Err(ApiError::BadRequest(
                "Username, email, and password are required".to_string(),
            ));
        }

        let salt = SaltString::generate(&mut OsRng);
        let hashed_password = Argon2::default()
            .hash_password(payload.password.as_bytes(), &salt)
            .map_err(|_| ApiError::InternalServerError("Password hashing failed".to_string()))?
            .to_string();

        let user_id = Uuid::new_v4();
        let created_at = Utc::now();

        let default_role = "Operator";

        sqlx::query!(
            r#"
            INSERT INTO users (id, username, email, hashed_password, role, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            user_id,
            payload.username,
            payload.email,
            hashed_password,
            default_role,
            created_at,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to create user: {}", e)))?;

        let token = jwt::encode_jwt(payload.username.clone())
            .map_err(|_| ApiError::InternalServerError("Failed to generate token".to_string()))?;

        Ok(token)
    }

    pub async fn login(&self, payload: AuthRequest) -> Result<String, ApiError> {
        if payload.username.is_empty() || payload.password.is_empty() {
            return Err(ApiError::BadRequest(
                "Username and password are required".to_string(),
            ));
        }

        let user =
            verify_user_credentials(&self.pool, &payload.username, &payload.password).await?;
        jwt::encode_jwt(user.username.clone())
            .map_err(|_| ApiError::InternalServerError("Failed to create token".to_string()))
    }

    pub async fn get_current_user_info(&self, claims: &Claims) -> Result<String, ApiError> {
        let user = validate_user_exists(claims, &self.pool).await?;
        Ok(format!(
            "current user: {}, email: {}, created at: {}",
            user.username, user.email, user.created_at
        ))
    }

    pub async fn list_users(&self, claims: &Claims) -> Result<Vec<User>, ApiError> {
        validate_user_role(claims, &self.pool, &UserRole::Admin).await?;
        let users = sqlx::query_as::<_, User>("SELECT * FROM users")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to fetch users: {}", e)))?;
        Ok(users)
    }

    pub async fn health_check(&self) -> Result<String, ApiError> {
        let row: (i32,) = sqlx::query_as("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ApiError::InternalServerError(format!("DB error: {}", e)))?;

        Ok(format!("DB says: {}", row.0))
    }
}

async fn validate_user_exists(claims: &Claims, pool: &PgPool) -> Result<User, ApiError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(&claims.sub)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::InternalServerError("DB error".to_string()))?;

    user.ok_or(ApiError::NotFound("User not found".to_string()))
}

async fn validate_user_role(
    claims: &Claims,
    pool: &PgPool,
    required_role: &UserRole,
) -> Result<User, ApiError> {
    let user = validate_user_exists(claims, pool).await?;

    if user.role != *required_role {
        return Err(ApiError::Forbidden("Insufficient permissions".to_string()));
    }

    Ok(user)
}

async fn verify_user_credentials(
    pool: &PgPool,
    username: &str,
    password: &str,
) -> Result<User, ApiError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::InternalServerError("DB error".into()))?
        .ok_or(ApiError::Unauthorized(
            "Invalid username or password".into(),
        ))?;

    let parsed_hash = PasswordHash::new(&user.hashed_password)
        .map_err(|_| ApiError::InternalServerError("Invalid hash format".into()))?;

    let is_valid = Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok();

    if !is_valid {
        return Err(ApiError::Unauthorized(
            "Invalid username or password".into(),
        ));
    }

    Ok(user)
}
