use crate::imagorpath::{normalize::SafeCharsType, params::Params};
use crate::metrics::{setup_metrics_recorder, track_metrics};
use crate::state::AppStateDyn;
use crate::storage::file::FileStorage;
use axum::extract::{MatchedPath, Request, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{middleware, Json};
use axum::{serve::Serve, Router};
use color_eyre::eyre::WrapErr;
use color_eyre::Result;
use std::future::ready;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{info, info_span};

pub struct Application {
    port: u16,
    server: Serve<Router, Router>,
}

impl Application {
    pub async fn build(port: u16) -> Result<Self> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await.wrap_err(
            "Failed to bind to the port. Make sure you have the correct permissions to bind to the port",
        )?;

        let server = run(listener).await?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

async fn run(listener: TcpListener) -> Result<Serve<Router, Router>> {
    let recorder_handle = setup_metrics_recorder();

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
        .route("/metrics", get(move || ready(recorder_handle.render())))
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
        .route_layer(middleware::from_fn(track_metrics))
        .with_state(state);

    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    let server = axum::serve(listener, app);

    Ok(server)
}

#[tracing::instrument(skip(state))]
async fn handler(
    State(state): State<AppStateDyn>,
    params: Params,
) -> Result<Json<Params>, (StatusCode, String)> {
    info!("params: {:?}", params);

    // TODO: check cache for image and serve if found

    // if image is not in cache, fetch image
    let img = params.image.as_ref().ok_or((
        StatusCode::BAD_REQUEST,
        "Image parameter is missing".to_string(),
    ))?;
    let _img_data = state.storage.get(img).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            format!("Failed to fetch image: {}", e),
        )
    })?;

    // apply transforms

    // TODO: save image to cache

    // return image

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
