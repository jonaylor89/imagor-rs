use crate::cache::redis::RedisCache;
use crate::imagorpath::hasher::verify_hash;
use crate::imagorpath::{normalize::SafeCharsType, params::Params};
use crate::metrics::{setup_metrics_recorder, track_metrics};
use crate::middleware::cache_middleware;
use crate::processor::processor::{Processor, ProcessorOptions};
use crate::state::AppStateDyn;
use crate::storage::s3::S3Storage;
use crate::storage::storage::{Blob, ImageStorage};
use aws_sdk_s3::Client;
use axum::body::Body;
use axum::extract::{MatchedPath, Request, State};
use axum::http::{header, Response, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{middleware, Json};
use axum::{serve::Serve, Router};
use color_eyre::eyre::WrapErr;
use color_eyre::Result;
use libvips::VipsApp;
use reqwest;
use std::future::ready;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{info, info_span};

pub struct Application {
    port: u16,
    server: Serve<Router, Router>,

    // This is a hack to keep the VipsApp alive for the lifetime of the application
    _vips_app: VipsApp,
}

impl Application {
    pub async fn build(port: u16) -> Result<Self> {
        let _vips_app = VipsApp::new("imagor_rs", true).wrap_err("Failed to initialize VipsApp")?;
        _vips_app.concurrency_set(4);

        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await.wrap_err(
            "Failed to bind to the port. Make sure you have the correct permissions to bind to the port",
        )?;

        let server = run(listener).await?;

        Ok(Self {
            port,
            server,
            _vips_app,
        })
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

    let storage = S3Storage::new_with_minio(
        "base_dir".into(),
        "images_dir".into(),
        SafeCharsType::Default,
        "imagor-rs".into(),
        "http://minio:9000".into(),
        "minioadmin".into(),
        "minioadmin".into(),
    )
    .await?;

    // Wait for MinIO to be ready
    wait_for_minio(&storage.client, 5, Duration::from_secs(2)).await?;

    // Ensure bucket exists
    storage.ensure_bucket_exists().await?;

    // Now try to upload the test image
    if let Ok(test_image) = std::fs::read("samples/test2.png") {
        let blob = Blob {
            content_type: "image/png".into(),
            data: test_image,
        };

        storage.put("test2.png", blob).await.inspect_err(|e| {
            tracing::error!("Failed to put test2.png: {:?}", e);
        })?;
    } else {
        tracing::warn!("Test image not found at samples/test2.png");
    }

    let processor = Processor::new(ProcessorOptions {
        disable_blur: false,
        disabled_filters: vec![],
        concurrency: None,
    });
    let cache = RedisCache::new("redis://redis:6379")?;
    let state = AppStateDyn {
        storage: Arc::new(storage.clone()),
        processor: Arc::new(processor),
        cache: Arc::new(cache.clone()),
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(move || ready(recorder_handle.render())))
        .route("/", get(root))
        .route("/params/*imagorpath", get(params))
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
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            cache_middleware,
        ))
        .with_state(state);

    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    let server = axum::serve(listener, app);

    Ok(server)
}

#[tracing::instrument(skip(state))]
async fn handler(
    State(state): State<AppStateDyn>,
    params: Params,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    info!("params: {:?}", params);

    if let (Some(hash), Some(path)) = (&params.hash, &params.path) {
        verify_hash(hash.to_owned().into(), path.to_owned().into()).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Failed to verify hash: {}", e),
            )
        })?;
    }

    // TODO: check result bucket for image and serve if found

    // if image is not in cache, fetch image
    let img = params.image.as_ref().ok_or((
        StatusCode::BAD_REQUEST,
        "Image parameter is missing".to_string(),
    ))?;

    // TODO: add config in the config to allow/disallow fetching images from the internet
    let blob = if img.starts_with("https://") || img.starts_with("http://") {
        let raw_bytes = reqwest::get(img)
            .await
            .map_err(|e| {
                (
                    StatusCode::NOT_FOUND,
                    format!("Failed to fetch image: {}", e),
                )
            })?
            .bytes()
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to fetch image: {}", e),
                )
            })?
            .to_vec();

        let content_type = infer::get(&raw_bytes)
            .map(|mime| mime.to_string())
            .unwrap_or("image/jpeg".to_string());

        Blob {
            data: raw_bytes,
            content_type,
        }
    } else {
        state.storage.get(img).await.map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                format!("Failed to fetch image: {}", e),
            )
        })?
    };

    let blob = state.processor.process(&blob, &params).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to process image: {}", e),
        )
    })?;

    // TODO: save image to result bucket

    Response::builder()
        .header(header::CONTENT_TYPE, blob.content_type)
        .body(Body::from(blob.data))
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to build response: {}", e),
            )
        })
}

#[tracing::instrument]
async fn params(params: Params) -> Result<Json<Params>, (StatusCode, String)> {
    info!("params: {:?}", params);

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

async fn wait_for_minio(client: &Client, max_retries: u32, delay: Duration) -> Result<()> {
    for i in 0..max_retries {
        match client.list_buckets().send().await {
            Ok(_) => {
                info!("Successfully connected to MinIO");
                return Ok(());
            }
            Err(e) => {
                if i == max_retries - 1 {
                    return Err(e.into());
                }
                info!(
                    "Waiting for MinIO to be ready... (attempt {}/{})",
                    i + 1,
                    max_retries
                );
                tokio::time::sleep(delay).await;
            }
        }
    }
    Err(color_eyre::eyre::eyre!(
        "Failed to connect to MinIO after {} retries",
        max_retries
    ))
}
