use crate::{
    domain::user::{User, UserRole},
    error::AppError,
};
use sqlx::PgPool;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait UserRepository: Send + Sync {
    async fn insert_user(&self, user: &User) -> Result<(), AppError>;
    async fn find_user_by_username(&self, username: &str) -> Result<Option<User>, AppError>;
    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<User>, AppError>;
    async fn list_all_users(&self) -> Result<Vec<User>, AppError>;
    async fn health_check(&self) -> Result<bool, AppError>;
}

#[async_trait::async_trait]
impl UserRepository for PgPool {
    async fn insert_user(&self, user: &User) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO users (id, username, email, hashed_password, role, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            user.id,
            user.username,
            user.email,
            user.hashed_password,
            user.role as UserRole,
            user.created_at,
        )
        .execute(self)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to create user: {}", e)))?;

        Ok(())
    }

    async fn find_user_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
            .bind(username)
            .fetch_optional(self)
            .await
            .map_err(|_| AppError::DatabaseError(format!("Failed to fetch user: {}", username)))
    }

    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<User>, AppError> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(self)
            .await
            .map_err(|_| AppError::DatabaseError(format!("Failed to fetch user: {}", id)))
    }

    async fn list_all_users(&self) -> Result<Vec<User>, AppError> {
        sqlx::query_as::<_, User>("SELECT * FROM users")
            .fetch_all(self)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to fetch users: {}", e)))
    }

    async fn health_check(&self) -> Result<bool, AppError> {
        let row: (i32,) = sqlx::query_as("SELECT 1")
            .fetch_one(self)
            .await
            .map_err(|e| AppError::DatabaseError(format!("DB error: {}", e)))?;

        Ok(row.0 == 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::user::User;
    use crate::domain::user::UserRole;
    use async_trait::async_trait;
    use chrono::Utc;
    use mockall::mock;
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

    fn make_test_user(username: &str) -> User {
        User {
            id: Uuid::new_v4(),
            username: username.into(),
            email: format!("{}@example.com", username),
            hashed_password: "hashed".into(),
            role: UserRole::Operator,
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_find_user_by_username_found() {
        let mut mock_repo = MockUserRepository::new();
        let user = make_test_user("alice");

        mock_repo
            .expect_find_user_by_username()
            .withf(|uname| uname == "alice")
            .returning(move |_| Ok(Some(user.clone())));

        let result = mock_repo.find_user_by_username("alice").await.unwrap();
        assert_eq!(result.unwrap().username, "alice");
    }

    #[tokio::test]
    async fn test_find_user_by_username_not_found() {
        let mut mock_repo = MockUserRepository::new();
        mock_repo
            .expect_find_user_by_username()
            .returning(|_| Ok(None));

        let result = mock_repo.find_user_by_username("ghost").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_user_by_id_found() {
        let mut mock_repo = MockUserRepository::new();
        let user = make_test_user("alice");
        let user_id = user.id;

        mock_repo
            .expect_find_user_by_id()
            .withf(move |id| *id == user_id)
            .returning(move |_| Ok(Some(user.clone())));

        let result = mock_repo.find_user_by_id(user_id).await.unwrap();
        assert_eq!(result.unwrap().id, user_id);
    }

    #[tokio::test]
    async fn test_find_user_by_id_not_found() {
        let mut mock_repo = MockUserRepository::new();
        mock_repo.expect_find_user_by_id().returning(|_| Ok(None));

        let result = mock_repo.find_user_by_id(Uuid::new_v4()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_list_all_users_returns_two() {
        let mut mock_repo = MockUserRepository::new();
        let users = vec![make_test_user("bob"), make_test_user("carol")];

        mock_repo
            .expect_list_all_users()
            .returning(move || Ok(users.clone()));

        let result = mock_repo.list_all_users().await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_health_check_returns_one() {
        let mut mock_repo = MockUserRepository::new();
        mock_repo.expect_health_check().returning(|| Ok(true));

        let result = mock_repo.health_check().await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_insert_user_success() {
        let mut mock_repo = MockUserRepository::new();
        mock_repo.expect_insert_user().returning(|_| Ok(()));

        let user = make_test_user("eve");
        let result = mock_repo.insert_user(&user).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_insert_user_failure() {
        let mut mock_repo = MockUserRepository::new();
        mock_repo
            .expect_insert_user()
            .returning(|_| Err(AppError::DatabaseError("duplicate".into())));

        let user = make_test_user("duplicate");
        let result = mock_repo.insert_user(&user).await;
        assert!(matches!(result, Err(AppError::DatabaseError(_))));
    }
}
