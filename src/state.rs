use crate::{
    cache::cache::ImageCache, processor::processor::ImageProcessor, storage::storage::ImageStorage,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppStateDyn {
    pub storage: Arc<dyn ImageStorage>,
    pub processor: Arc<dyn ImageProcessor>,
    pub cache: Arc<dyn ImageCache>,
}
