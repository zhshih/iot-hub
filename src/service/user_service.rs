use crate::{
    api::error::ApiError,
    auth::jwt::{self, AuthRequest, Claims},
    domain::user::{SignupRequest, User, UserRole},
    repository::user_repo::UserRepository,
};

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::Utc;
use password_hash::{SaltString, rand_core::OsRng};
use uuid::Uuid;

pub struct UserService<R: UserRepository> {
    repo: R,
}

impl<R: UserRepository> UserService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
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

        let user = User {
            id: Uuid::new_v4(),
            username: payload.username.clone(),
            email: payload.email.clone(),
            hashed_password,
            role: UserRole::Operator,
            created_at: Utc::now(),
        };

        self.repo.insert_user(&user).await?;

        let token = jwt::encode_jwt(payload.username)
            .map_err(|_| ApiError::InternalServerError("Failed to generate token".to_string()))?;

        Ok(token)
    }

    pub async fn login(&self, payload: AuthRequest) -> Result<String, ApiError> {
        if payload.username.is_empty() || payload.password.is_empty() {
            return Err(ApiError::BadRequest(
                "Username and password are required".to_string(),
            ));
        }

        let user = self
            .repo
            .find_user_by_username(&payload.username)
            .await?
            .ok_or(ApiError::Unauthorized(
                "Invalid username or password".into(),
            ))?;

        let parsed_hash = PasswordHash::new(&user.hashed_password)
            .map_err(|_| ApiError::InternalServerError("Invalid hash format".into()))?;

        let is_valid = Argon2::default()
            .verify_password(payload.password.as_bytes(), &parsed_hash)
            .is_ok();

        if !is_valid {
            return Err(ApiError::Unauthorized(
                "Invalid username or password".into(),
            ));
        }

        jwt::encode_jwt(user.username.clone())
            .map_err(|_| ApiError::InternalServerError("Failed to create token".to_string()))
    }

    pub async fn get_current_user_info(&self, claims: &Claims) -> Result<String, ApiError> {
        let user = self
            .repo
            .find_user_by_username(&claims.sub)
            .await?
            .ok_or(ApiError::NotFound("User not found".to_string()))?;

        Ok(format!(
            "current user: {}, email: {}, created at: {}",
            user.username, user.email, user.created_at
        ))
    }

    pub async fn list_users(&self, claims: &Claims) -> Result<Vec<User>, ApiError> {
        let user = self
            .repo
            .find_user_by_username(&claims.sub)
            .await?
            .ok_or(ApiError::NotFound("User not found".to_string()))?;

        if user.role != UserRole::Admin {
            return Err(ApiError::Forbidden("Insufficient permissions".to_string()));
        }

        self.repo.list_all_users().await
    }

    pub async fn health_check(&self) -> Result<String, ApiError> {
        let val = self.repo.health_check().await?;
        Ok(format!("DB says: {}", val))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::error::ApiError,
        domain::user::{User, UserRole},
    };
    use async_trait::async_trait;
    use chrono::Utc;
    use mockall::mock;
    use uuid::Uuid;

    mock! {
        pub UserRepository {}

        #[async_trait]
        impl UserRepository for UserRepository {
            async fn insert_user(&self, user: &User) -> Result<(), ApiError>;
            async fn find_user_by_username(&self, username: &str) -> Result<Option<User>, ApiError>;
            async fn list_all_users(&self) -> Result<Vec<User>, ApiError>;
            async fn health_check(&self) -> Result<i32, ApiError>;
        }
    }

    fn setup_env() {
        unsafe {
            std::env::set_var("JWT_SECRET", "test_secret");
        }
    }

    fn make_test_user(username: &str, password: &str, role: UserRole) -> User {
        let salt = SaltString::generate(&mut OsRng);
        let hashed_password = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string();
        User {
            id: Uuid::new_v4(),
            username: username.into(),
            email: format!("{}@example.com", username),
            hashed_password,
            role,
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_signup_should_create_user() {
        setup_env();
        let mut mock_repo = MockUserRepository::new();
        mock_repo.expect_insert_user().returning(|_| Ok(()));
        let service = UserService::new(mock_repo);
        let token = service
            .signup(SignupRequest {
                username: "test".into(),
                email: "test@example.com".into(),
                password: "pass123".into(),
            })
            .await
            .unwrap();

        assert!(!token.is_empty());
    }

    #[tokio::test]
    async fn test_signup_should_fail_with_empty_fields() {
        let mut mock_repo = MockUserRepository::new();
        mock_repo.expect_insert_user().returning(|_| Ok(()));
        let service = UserService::new(mock_repo);
        let res = service
            .signup(SignupRequest {
                username: "".into(),
                email: "".into(),
                password: "".into(),
            })
            .await;
        assert!(matches!(res, Err(ApiError::BadRequest(_))));
    }

    #[tokio::test]
    async fn test_login_should_succeed_with_correct_credentials() {
        let user = make_test_user("john", "secret", UserRole::Operator);
        let mut mock_repo = MockUserRepository::new();
        mock_repo
            .expect_find_user_by_username()
            .returning(move |username| {
                if username == "john" {
                    Ok(Some(user.clone()))
                } else {
                    Ok(None)
                }
            });
        let service = UserService::new(mock_repo);

        let token = service
            .login(AuthRequest {
                username: "john".into(),
                password: "secret".into(),
            })
            .await
            .unwrap();

        assert!(!token.is_empty());
    }

    #[tokio::test]
    async fn test_login_should_fail_with_wrong_password() {
        let user = make_test_user("john", "secret", UserRole::Operator);
        let mut mock_repo = MockUserRepository::new();
        mock_repo
            .expect_find_user_by_username()
            .returning(move |_| Ok(Some(user.clone())));
        let service = UserService::new(mock_repo);

        let res = service
            .login(AuthRequest {
                username: "john".into(),
                password: "wrong".into(),
            })
            .await;

        assert!(matches!(res, Err(ApiError::Unauthorized(_))));
    }

    #[tokio::test]
    async fn test_get_current_user_info_should_return_info() {
        let user = make_test_user("jane", "pass", UserRole::Operator);
        let user_clone = user.clone();
        let mut mock_repo = MockUserRepository::new();
        mock_repo
            .expect_find_user_by_username()
            .returning(move |_| Ok(Some(user_clone.clone())));
        let service = UserService::new(mock_repo);

        let claims = Claims {
            sub: user.username.clone(),
            iat: 0,
            exp: 0,
        };
        let info = service.get_current_user_info(&claims).await.unwrap();
        assert!(info.contains(&user.username));
        assert!(info.contains(&user.email));
    }

    #[tokio::test]
    async fn test_get_current_user_info_should_fail_if_not_found() {
        let mut mock_repo = MockUserRepository::new();
        mock_repo
            .expect_find_user_by_username()
            .returning(|_| Ok(None));
        let service = UserService::new(mock_repo);
        let claims = Claims {
            sub: "ghost".into(),
            iat: 0,
            exp: 0,
        };
        let res = service.get_current_user_info(&claims).await;
        assert!(matches!(res, Err(ApiError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_list_users_should_return_for_admin() {
        let admin = make_test_user("admin", "pass", UserRole::Admin);
        let other = make_test_user("bob", "pass", UserRole::Operator);
        let mut mock_repo = MockUserRepository::new();
        let admin_clone = admin.clone();
        mock_repo
            .expect_find_user_by_username()
            .returning(move |username| {
                if username == "admin" {
                    Ok(Some(admin_clone.clone()))
                } else {
                    Ok(None)
                }
            });
        let admin_clone = admin.clone();
        mock_repo
            .expect_list_all_users()
            .returning(move || Ok(vec![admin_clone.clone(), other.clone()]));

        let service = UserService::new(mock_repo);

        let claims = Claims {
            sub: "admin".into(),
            iat: 0,
            exp: 0,
        };
        let users = service.list_users(&claims).await.unwrap();
        assert_eq!(users.len(), 2);
    }

    #[tokio::test]
    async fn test_list_users_should_forbid_non_admin() {
        let user = make_test_user("bob", "pass", UserRole::Operator);
        let mut mock_repo = MockUserRepository::new();
        mock_repo
            .expect_find_user_by_username()
            .returning(move |username| {
                if username == "bob" {
                    Ok(Some(user.clone()))
                } else {
                    Ok(None)
                }
            });
        mock_repo.expect_list_all_users().returning(|| Ok(vec![]));

        let service = UserService::new(mock_repo);

        let claims = Claims {
            sub: "bob".into(),
            iat: 0,
            exp: 0,
        };
        let res = service.list_users(&claims).await;
        assert!(matches!(res, Err(ApiError::Forbidden(_))));
    }

    #[tokio::test]
    async fn test_health_check_should_return_db_value() {
        let mut mock_repo = MockUserRepository::new();
        mock_repo.expect_health_check().returning(|| Ok(1));
        let service = UserService::new(mock_repo);

        let result = service.health_check().await.unwrap();
        assert_eq!(result, "DB says: 1");
    }
}
