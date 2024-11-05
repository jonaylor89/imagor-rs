use secrecy::SecretString;
use serde::Deserialize;
use serde_aux::prelude::deserialize_number_from_string;

use crate::imagorpath::normalize::SafeCharsType;

#[derive(serde::Deserialize, Clone)]
pub struct Settings {
    pub application: ApplicationSettings,
    pub processor: ProcessorSettings,
    pub storage: StorageSettings,
    pub cache: CacheSettings,
}

#[derive(serde::Deserialize, Clone)]
pub struct ApplicationSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub base_url: String,
    pub hmac_secret: SecretString,
}

#[derive(serde::Deserialize, Clone)]
pub struct ProcessorSettings {
    pub disable_blur: bool,
    pub disabled_filters: Vec<String>,
    pub max_filter_ops: usize,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<i32>,
    pub max_cache_files: i32,
    pub max_cache_mem: i32,
    pub max_cache_size: i32,
    pub max_width: i32,
    pub max_height: i32,
    pub max_resolution: i32,
    pub max_animation_frames: usize,
    pub strip_metadata: bool,
    pub avif_speed: i32,
}

#[derive(Deserialize, Clone)]
pub struct StorageSettings {
    pub base_dir: String,
    pub path_prefix: String,
    pub safe_chars: SafeCharsType,
    pub client: StorageClient,
}

#[derive(Deserialize, Clone)]
pub enum StorageClient {
    S3(S3Settings),
    GCS(GCSSettings),
    Filesystem(FilesystemSettings),
}

#[derive(Deserialize, Clone)]
pub struct S3Settings {
    pub region: String,
    pub bucket: String,
    pub access_key: SecretString,
    pub secret_key: SecretString,
}

#[derive(Deserialize, Clone)]
pub struct GCSSettings {
    pub bucket: String,
    pub credentials: SecretString,
}

#[derive(Deserialize, Clone)]
pub struct FilesystemSettings {
    pub base_dir: String,
}

#[derive(Deserialize, Clone)]
pub enum CacheSettings {
    Redis(RedisSettings),
    Filesystem(String),
}

#[derive(Deserialize, Clone)]
pub struct RedisSettings {
    pub uri: String,
}

#[derive(Deserialize, Clone)]
pub struct FilesystemCacheSettings {
    pub base_dir: String,
}

pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. Use either `local` or `production`",
                other
            )),
        }
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine the current directory");
    let configuration_directory = base_path.join("config");

    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT");

    let builder = config::Config::builder()
        .add_source(config::File::from(configuration_directory.join("base")).required(true))
        .add_source(
            config::File::from(configuration_directory.join(environment.as_str())).required(true),
        )
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        );

    builder.build()?.try_deserialize::<Settings>()
}
