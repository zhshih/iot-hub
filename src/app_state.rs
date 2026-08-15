use crate::domain::reading::Reading;
use sqlx::PgPool;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub readings_tx: broadcast::Sender<Reading>,
}
