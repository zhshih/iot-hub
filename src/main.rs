use axum::serve;
use iot_hub::{build_app, init_tracing};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let (app, addr) = build_app().await?;
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::info!("Listening on {}", addr);
    serve(listener, app).await.unwrap();

    Ok(())
}
