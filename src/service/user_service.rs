use crate::{
    auth::jwt::{self, Claims},
    domain::user::{PublicUser, SignupUser, User, UserRole},
    dto::auth::AuthRequest,
    error::{AppError, TokenError, ValidationError},
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

    pub async fn signup(&self, payload: SignupUser) -> Result<(String, Uuid), AppError> {
        if payload.username.is_empty() || payload.password.is_empty() || payload.email.is_empty() {
            return Err(AppError::MissingArgument(
                "Username, email, and password are required".to_string(),
            ));
        }

        let salt = SaltString::generate(&mut OsRng);
        let hashed_password = Argon2::default()
            .hash_password(payload.password.as_bytes(), &salt)
            .map_err(|_| {
                AppError::ValidationError(ValidationError::ParsedError(
                    "Password hashing failed".to_string(),
                ))
            })?
            .to_string();

        let role = match std::env::var("ADMIN_BOOTSTRAP_EMAIL") {
            Ok(bootstrap_email) if bootstrap_email == payload.email => UserRole::Admin,
            _ => UserRole::Operator,
        };

        let user = User {
            id: Uuid::new_v4(),
            username: payload.username.clone(),
            email: payload.email.clone(),
            hashed_password,
            role,
            created_at: Utc::now(),
        };

        self.repo.insert_user(&user).await?;

        let token = jwt::encode_jwt(user.id.to_string()).map_err(|_| {
            AppError::TokenError(TokenError::GenerationFailed(
                "Failed to generate token".to_string(),
            ))
        })?;

        Ok((token, user.id))
    }

    pub async fn login(&self, payload: AuthRequest) -> Result<String, AppError> {
        if payload.username.is_empty() || payload.password.is_empty() {
            return Err(AppError::MissingArgument(
                "Username and password are required".to_string(),
            ));
        }

        let user = self
            .repo
            .find_user_by_username(&payload.username)
            .await?
            .ok_or(AppError::ValidationError(ValidationError::InvalidInput(
                "Invalid username or password".into(),
            )))?;

        let parsed_hash = PasswordHash::new(&user.hashed_password).map_err(|_| {
            AppError::ValidationError(ValidationError::ParsedError("Invalid hash format".into()))
        })?;

        let is_valid = Argon2::default()
            .verify_password(payload.password.as_bytes(), &parsed_hash)
            .is_ok();

        if !is_valid {
            return Err(AppError::ValidationError(ValidationError::InvalidInput(
                "Invalid username or password".into(),
            )));
        }

        jwt::encode_jwt(user.id.to_string()).map_err(|_| {
            AppError::TokenError(TokenError::GenerationFailed(
                "Failed to generate token".to_string(),
            ))
        })
    }

    pub async fn get_current_user_info(&self, claims: &Claims) -> Result<PublicUser, AppError> {
        let user_id = claims.user_id()?;
        let user = self
            .repo
            .find_user_by_id(user_id)
            .await?
            .ok_or(AppError::NotFound("User not found".to_string()))?;

        let user = PublicUser::from(user);

        Ok(user)
    }

    pub async fn list_users(&self, claims: &Claims) -> Result<Vec<PublicUser>, AppError> {
        let user_id = claims.user_id()?;
        let user = self
            .repo
            .find_user_by_id(user_id)
            .await?
            .ok_or(AppError::NotFound("User not found".to_string()))?;

        if user.role != UserRole::Admin {
            return Err(AppError::ValidationError(
                ValidationError::PermissionDenied("Insufficient permissions".to_string()),
            ));
        }

        let users = self.repo.list_all_users().await?;
        Ok(users.into_iter().map(PublicUser::from).collect())
    }

    pub async fn health_check(&self) -> Result<(), AppError> {
        let is_healthy = self.repo.health_check().await?;
        if is_healthy {
            Ok(())
        } else {
            Err(AppError::HealthCheckFailed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::user::{User, UserRole};
    use async_trait::async_trait;
    use chrono::Utc;
    use mockall::mock;
    use serial_test::serial;
    use uuid::Uuid;

    mock! {
        pub UserRepository {}

        #[async_trait]
        impl UserRepository for UserRepository {
            async fn insert_user(&self, user: &User) -> Result<(), AppError>;
            async fn find_user_by_username(&self, username: &str) -> Result<Option<User>, AppError>;
            async fn find_user_by_id(&self, id: Uuid) -> Result<Option<User>, AppError>;
            async fn list_all_users(&self) -> Result<Vec<User>, AppError>;
            async fn health_check(&self) -> Result<bool, AppError>;
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
    #[serial(env_vars)]
    async fn test_signup_should_create_user() {
        setup_env();
        unsafe { std::env::remove_var("ADMIN_BOOTSTRAP_EMAIL") };
        let mut mock_repo = MockUserRepository::new();
        mock_repo
            .expect_insert_user()
            .withf(|user| user.role == UserRole::Operator)
            .returning(|_| Ok(()));
        let service = UserService::new(mock_repo);
        let (token, _user_id) = service
            .signup(SignupUser {
                username: "test".into(),
                email: "test@example.com".into(),
                password: "pass123".into(),
            })
            .await
            .unwrap();

        assert!(!token.is_empty());
    }

    #[tokio::test]
    #[serial(env_vars)]
    async fn test_signup_should_grant_admin_for_bootstrap_email() {
        setup_env();
        unsafe { std::env::set_var("ADMIN_BOOTSTRAP_EMAIL", "admin@example.com") };

        let mut mock_repo = MockUserRepository::new();
        mock_repo
            .expect_insert_user()
            .withf(|user| user.role == UserRole::Admin)
            .returning(|_| Ok(()));
        let service = UserService::new(mock_repo);

        let result = service
            .signup(SignupUser {
                username: "bootstrap_admin".into(),
                email: "admin@example.com".into(),
                password: "pass123".into(),
            })
            .await;

        unsafe { std::env::remove_var("ADMIN_BOOTSTRAP_EMAIL") };

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_signup_should_fail_with_empty_fields() {
        let mut mock_repo = MockUserRepository::new();
        mock_repo.expect_insert_user().returning(|_| Ok(()));
        let service = UserService::new(mock_repo);
        let res = service
            .signup(SignupUser {
                username: "".into(),
                email: "".into(),
                password: "".into(),
            })
            .await;
        assert!(matches!(res, Err(AppError::MissingArgument(_))));
    }

    #[tokio::test]
    #[serial(env_vars)]
    async fn test_login_should_succeed_with_correct_credentials() {
        setup_env();
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

        assert!(matches!(
            res,
            Err(AppError::ValidationError(ValidationError::InvalidInput(_)))
        ));
    }

    #[tokio::test]
    async fn test_get_current_user_info_should_return_info() {
        let user = make_test_user("jane", "pass", UserRole::Operator);
        let user_clone = user.clone();
        let user_id = user.id;

        let mut mock_repo = MockUserRepository::new();
        mock_repo
            .expect_find_user_by_id()
            .returning(move |_| Ok(Some(user_clone.clone())));

        let service = UserService::new(mock_repo);

        let claims = Claims {
            sub: user_id.to_string(),
            iat: 0,
            exp: 0,
        };

        let info = service.get_current_user_info(&claims).await.unwrap();

        assert_eq!(info.username, user.username);
        assert_eq!(info.email, user.email);
        assert_eq!(info.role, user.role);
        assert_eq!(info.id, user.id);
    }

    #[tokio::test]
    async fn test_get_current_user_info_should_fail_if_not_found() {
        let mut mock_repo = MockUserRepository::new();
        mock_repo.expect_find_user_by_id().returning(|_| Ok(None));
        let service = UserService::new(mock_repo);
        let claims = Claims {
            sub: Uuid::new_v4().to_string(),
            iat: 0,
            exp: 0,
        };
        let res = service.get_current_user_info(&claims).await;
        assert!(matches!(res, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_current_user_info_should_fail_on_invalid_claims_sub() {
        let mock_repo = MockUserRepository::new();
        let service = UserService::new(mock_repo);
        let claims = Claims {
            sub: "not-a-uuid".into(),
            iat: 0,
            exp: 0,
        };
        let res = service.get_current_user_info(&claims).await;
        assert!(matches!(
            res,
            Err(AppError::ValidationError(ValidationError::InvalidInput(_)))
        ));
    }

    #[tokio::test]
    async fn test_list_users_should_return_for_admin() {
        let admin = make_test_user("admin", "pass", UserRole::Admin);
        let other = make_test_user("bob", "pass", UserRole::Operator);
        let admin_id = admin.id;
        let mut mock_repo = MockUserRepository::new();
        let admin_clone = admin.clone();
        mock_repo
            .expect_find_user_by_id()
            .returning(move |id| {
                if id == admin_id {
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
            sub: admin_id.to_string(),
            iat: 0,
            exp: 0,
        };
        let users = service.list_users(&claims).await.unwrap();
        assert_eq!(users.len(), 2);
    }

    #[tokio::test]
    async fn test_list_users_should_forbid() {
        let user = make_test_user("bob", "pass", UserRole::User);
        let user_id = user.id;
        let mut mock_repo = MockUserRepository::new();
        mock_repo
            .expect_find_user_by_id()
            .returning(move |id| {
                if id == user_id {
                    Ok(Some(user.clone()))
                } else {
                    Ok(None)
                }
            });
        mock_repo.expect_list_all_users().returning(|| Ok(vec![]));

        let service = UserService::new(mock_repo);

        let claims = Claims {
            sub: user_id.to_string(),
            iat: 0,
            exp: 0,
        };
        let res = service.list_users(&claims).await;
        assert!(matches!(
            res,
            Err(AppError::ValidationError(
                ValidationError::PermissionDenied(_)
            ))
        ));
    }

    #[tokio::test]
    async fn test_health_check_should_return_db_value() {
        let mut mock_repo = MockUserRepository::new();
        mock_repo.expect_health_check().returning(|| Ok(true));
        let service = UserService::new(mock_repo);

        let result = service.health_check().await;
        assert!(result.is_ok());
    }
}
