use std::time::Duration;

use crate::imagorpath::normalize::{normalize, SafeCharsType};
use crate::storage::storage::{Blob, ImageStorage};
use aws_sdk_s3::config::{Credentials, Region};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::{Client, Config};
use axum::async_trait;
use color_eyre::Result;
use tracing::info;

#[derive(Clone)]
pub struct S3Storage {
    pub base_dir: String,
    pub path_prefix: String,
    pub safe_chars: SafeCharsType,
    pub client: Client,
    pub bucket: String,
    // pub expiration: time::Duration,
    // pub acl: String,
}

#[async_trait]
impl ImageStorage for S3Storage {
    #[tracing::instrument(skip(self))]
    async fn get(&self, key: &str) -> Result<Blob> {
        let full_path = self.get_full_path(key);

        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(full_path)
            .send()
            .await?;

        let data = output.body.collect().await?.into_bytes();
        Ok(Blob::new(data.to_vec()))
    }

    #[tracing::instrument(skip(self, blob))]
    async fn put(&self, key: &str, blob: Blob) -> Result<()> {
        let full_path = self.get_full_path(key);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(full_path)
            .body(ByteStream::from(blob.data))
            .send()
            .await?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn delete(&self, key: &str) -> Result<()> {
        let full_path = self.get_full_path(key);

        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(full_path)
            .send()
            .await?;

        Ok(())
    }
}

impl S3Storage {
    pub async fn new(
        base_dir: String,
        path_prefix: String,
        safe_chars: SafeCharsType,
        bucket: String,
        config: Config,
    ) -> Self {
        let client = Client::from_conf(config);
        S3Storage {
            base_dir,
            path_prefix,
            safe_chars,
            client,
            bucket,
        }
    }

    #[tracing::instrument]
    pub async fn new_with_minio(
        base_dir: String,
        path_prefix: String,
        safe_chars: SafeCharsType,
        bucket: String,
        endpoint: String,
        access_key: String,
        secret_key: String,
    ) -> Result<Self> {
        // Create custom credentials
        let credentials = Credentials::new(
            access_key, secret_key, None, // session token
            None, // expiry
            "minio",
        );

        // Create the config
        let config = aws_sdk_s3::Config::builder()
            .behavior_version_latest()
            .region(Region::new("us-east-1")) // MinIO defaults to us-east-1
            .endpoint_url(&endpoint)
            .credentials_provider(credentials)
            .force_path_style(true) // This is important for MinIO
            .build();

        let client = Client::from_conf(config);

        // Wait for MinIO to be ready
        wait_for_minio(&client, 5, Duration::from_secs(2)).await?;

        Ok(S3Storage {
            base_dir,
            path_prefix,
            safe_chars,
            client,
            bucket,
        })
    }

    pub async fn ensure_bucket_exists(&self) -> Result<()> {
        let exists = self
            .client
            .head_bucket()
            .bucket(&self.bucket)
            .send()
            .await
            .is_ok();

        if !exists {
            self.client
                .create_bucket()
                .bucket(&self.bucket)
                .send()
                .await?;
        }

        Ok(())
    }

    pub fn get_full_path(&self, key: &str) -> String {
        let safe_key = normalize(key, &self.safe_chars);
        format!("{}/{}", self.path_prefix, safe_key)
    }
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
