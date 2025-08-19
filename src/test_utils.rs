use crate::app_state::AppState;
use crate::auth::{jwt::Claims, middleware::AuthUser};
use sqlx::PgPool;

#[cfg(test)]
pub fn setup_env() {
    unsafe {
        std::env::set_var("JWT_SECRET", "test_secret");
    }
}

#[cfg(test)]
pub async fn setup_test_state(truncate_table: &str) -> AppState {
    setup_env();
    let database_url = "postgres://test_user:test_password@localhost/iot_monitoring_test";
    let pool = PgPool::connect(database_url).await.unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    sqlx::query(&format!("TRUNCATE TABLE {} CASCADE", truncate_table))
        .execute(&pool)
        .await
        .unwrap();

    AppState { db_pool: pool }
}

#[cfg(test)]
pub fn mock_auth_user(sub: &str) -> AuthUser {
    AuthUser(Claims {
        sub: sub.to_string(),
        exp: 0,
        iat: 0,
    })
}
