use crate::imagorpath::normalize::{normalize, SafeCharsType};
use crate::storage::storage::{Blob, ImageStorage};
use axum::async_trait;
use color_eyre::Result;
use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::http::objects::delete::DeleteObjectRequest;
use google_cloud_storage::http::objects::download::Range;
use google_cloud_storage::http::objects::get::GetObjectRequest;
use google_cloud_storage::http::objects::upload::{Media, UploadObjectRequest, UploadType};
use std::time;

#[derive(Clone)]
pub struct GCloudStorage {
    pub base_dir: String,
    pub path_prefix: String,
    pub acl: String,
    pub safe_chars: SafeCharsType,
    pub expiration: time::Duration,
    pub client: Client,
    pub bucket: String,
}

#[async_trait]
impl ImageStorage for GCloudStorage {
    #[tracing::instrument(skip(self))]
    async fn get(&self, key: &str) -> Result<Blob> {
        let full_path = self.get_full_path(key);
        let buffer = self
            .client
            .download_object(
                &GetObjectRequest {
                    bucket: self.bucket.clone(),
                    object: full_path,
                    ..Default::default()
                },
                &Range::default(),
            )
            .await?;

        Ok(Blob::new(buffer))
    }

    #[tracing::instrument(skip(self, blob))]
    async fn put(&self, key: &str, blob: Blob) -> Result<()> {
        let full_path = self.get_full_path(key);
        let upload_type = UploadType::Simple(Media::new(full_path));
        let blob_data = blob.data;
        self.client
            .upload_object(
                &UploadObjectRequest {
                    bucket: self.bucket.clone(),
                    ..Default::default()
                },
                blob_data,
                &upload_type,
            )
            .await?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn delete(&self, key: &str) -> Result<()> {
        let full_path = self.get_full_path(key);
        self.client
            .delete_object(&DeleteObjectRequest {
                bucket: self.bucket.clone(),
                object: full_path,
                ..Default::default()
            })
            .await?;
        Ok(())
    }
}

impl GCloudStorage {
    pub fn new(
        base_dir: String,
        path_prefix: String,
        acl: String,
        safe_chars: SafeCharsType,
        expiration: time::Duration,
        bucket: String,
        config: ClientConfig,
    ) -> Self {
        let client = Client::new(config);
        GCloudStorage {
            base_dir,
            path_prefix,
            acl,
            safe_chars,
            expiration,
            client,
            bucket,
        }
    }

    pub fn get_full_path(&self, key: &str) -> String {
        let safe_key = normalize(key, &self.safe_chars);
        format!("{}/{}", self.path_prefix, safe_key)
    }
}
