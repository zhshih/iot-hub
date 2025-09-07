use axum::serve;
use iot_hub::build_app;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (app, addr) = build_app().await?;
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::info!("Listening on {}", addr);
    serve(listener, app).await.unwrap();

    Ok(())
}
