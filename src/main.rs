use std::sync::Arc;

use axum::extract::{MatchedPath, Request};
use axum::http::StatusCode;
use axum::Json;
use axum::{routing::get, Router};
use color_eyre::Result;
use imagor_rs::imagorpath::normalize::SafeCharsType;
use imagor_rs::imagorpath::params::Params;
use imagor_rs::state::AppStateDyn;
use imagor_rs::storage::file::FileStorage;
use imagor_rs::telemetry::{get_subscriber, init_subscriber};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{info, info_span};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let subscriber = get_subscriber("imagor_rs".into(), "debug".into(), std::io::stdout);
    init_subscriber(subscriber);

    // let configuration = configuration::read().expect("Failed to read configuration");

    let storage = FileStorage::new(
        "base_dir".into(),
        "images_dir".into(),
        SafeCharsType::Default,
    );
    let state = AppStateDyn {
        storage: Arc::new(storage.clone()),
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/", get(root))
        .route("/*imagorpath", get(handler))
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
                // Log the matched route's path (with placeholders not filled in).
                // Use request.uri() or OriginalUri if you want the real path.
                let matched_path = request
                    .extensions()
                    .get::<MatchedPath>()
                    .map(MatchedPath::as_str);

                info_span!(
                    "http_request",
                    method = ?request.method(),
                    matched_path,
                    some_other_field = tracing::field::Empty,
                )
            }),
        )
        .with_state(state);

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

    // 2. check cache for image and serve if found

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
