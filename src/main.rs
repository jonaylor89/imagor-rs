use axum::http::StatusCode;
use axum::Json;
use axum::{routing::get, Router};
use color_eyre::Result;
use imagor_rs::cyberpunkpath::params::Params;
use imagor_rs::telemetry::{get_subscriber, init_subscriber};
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let subscriber = get_subscriber("imagor_rs".into(), "debug".into(), std::io::stdout);
    init_subscriber(subscriber);

    // let configuration = configuration::read().expect("Failed to read configuration");

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/", get(root))
        .route("/*cyperpunkpath", get(handler));

    let listener = TcpListener::bind("127.0.0.1:8080")
        .await
        .expect("Failed to bind port");
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.expect("server failed");

    Ok(())
}

#[tracing::instrument]
async fn handler(params: Params) -> Result<Json<Params>, (StatusCode, String)> {
    info!("params: {:?}", params);

    // 2. check cache for image
    // 3. if image is not in cache, fetch image

    // 4. apply transforms

    // 5. save image to cache

    // 6. return image

    Ok(Json(params))
}

#[tracing::instrument]
async fn root() -> &'static str {
    "Hello, World"
}

#[tracing::instrument]
async fn health_check() -> &'static str {
    tracing::info!("Health check called");
    "OK"
}
