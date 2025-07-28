use crate::auth::middleware::AuthUser;
use axum::{
    Router,
    extract::Path,
    routing::{get, post},
};

pub fn routes() -> Router {
    Router::new()
        .route("/{id}", post(post_reading))
        .route("/{id}", get(get_reading))
}

async fn post_reading(Path(id): Path<String>, AuthUser(user): AuthUser) -> String {
    format!("Posted post_reading {} for user {}", id, user.sub)
}

async fn get_reading(Path(id): Path<String>, AuthUser(user): AuthUser) -> String {
    format!("Got reading {} for user {}", id, user.sub)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{jwt::Claims, middleware::AuthUser};

    #[tokio::test]
    async fn test_post_reading() {
        let id = "abc123".to_string();
        let user = AuthUser(Claims {
            sub: "user42".into(),
            ..Default::default()
        });

        let result = post_reading(Path(id.clone()), user).await;
        assert_eq!(
            result,
            format!("Posted post_reading {} for user user42", id)
        );
    }

    #[tokio::test]
    async fn test_get_reading() {
        let id = "xyz789".to_string();
        let user = AuthUser(Claims {
            sub: "user99".into(),
            ..Default::default()
        });

        let result = get_reading(Path(id.clone()), user).await;
        assert_eq!(result, format!("Got reading {} for user user99", id));
    }
}
