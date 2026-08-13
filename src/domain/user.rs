use crate::domain::ids::UserId;
use crate::dto::user::SignupRequest;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use std::fmt;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub email: String,
    pub hashed_password: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
}

/// `User` is reserved for a future, more restrictive permission tier (e.g. read-only
/// access) and is not yet produced or checked anywhere in the codebase — `signup()`
/// only ever assigns `Admin` (via `ADMIN_BOOTSTRAP_EMAIL`) or `Operator`. Only Admin
/// vs. non-Admin is currently enforced (see `UserService::list_users`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Type, ToSchema)]
#[sqlx(type_name = "TEXT")]
pub enum UserRole {
    Admin,
    Operator,
    User,
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UserRole::Admin => write!(f, "Admin"),
            UserRole::Operator => write!(f, "Operator"),
            UserRole::User => write!(f, "User"),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SignupUser {
    pub username: String,
    pub email: String,
    pub password: String,
}

impl SignupUser {
    pub fn from_request(req: SignupRequest) -> Self {
        Self {
            username: req.username,
            email: req.email,
            password: req.password,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PublicUser {
    pub id: UserId,
    pub username: String,
    pub email: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
}

impl From<User> for PublicUser {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            role: user.role,
            created_at: user.created_at,
        }
    }
}
