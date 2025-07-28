use crate::auth::middleware::AuthUser;
use axum::{
    Router,
    extract::Path,
    routing::{delete, get, post},
};

pub fn routes() -> Router {
    Router::new()
        .route("/", post(register_device))
        .route("/", get(get_devices))
        .route("/{device_id}", get(get_device))
        .route("/{device_id}", delete(delete_device))
}

async fn register_device(AuthUser(user): AuthUser) -> String {
    format!("Registered device for user: {}", user.sub)
}

async fn get_devices(AuthUser(user): AuthUser) -> String {
    format!("Got devices for user: {}", user.sub)
}

async fn get_device(Path(device_id): Path<String>, AuthUser(user): AuthUser) -> String {
    format!("Got device {} for {}", device_id, user.sub)
}

async fn delete_device(Path(device_id): Path<String>, AuthUser(user): AuthUser) -> String {
    format!("Deleted device {} for user {}", device_id, user.sub)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{jwt::Claims, middleware::AuthUser};

    fn mock_auth_user(sub: &str) -> AuthUser {
        AuthUser(Claims {
            sub: sub.to_string(),
            exp: 0,
            iat: 0,
        })
    }

    #[tokio::test]
    async fn test_register_device() {
        let user = mock_auth_user("test_user");
        let result = register_device(user).await;
        assert_eq!(result, "Registered device for user: test_user");
    }

    #[tokio::test]
    async fn test_get_devices() {
        let user = mock_auth_user("test_user");
        let result = get_devices(user).await;
        assert_eq!(result, "Got devices for user: test_user");
    }

    #[tokio::test]
    async fn test_get_device() {
        let user = mock_auth_user("test_user");
        let path = Path("dev123".to_string());
        let result = get_device(path, user).await;
        assert_eq!(result, "Got device dev123 for test_user");
    }

    #[tokio::test]
    async fn test_delete_device() {
        let user = mock_auth_user("test_user");
        let path = Path("dev123".to_string());
        let result = delete_device(path, user).await;
        assert_eq!(result, "Deleted device dev123 for user test_user");
    }
}
