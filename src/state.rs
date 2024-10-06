use crate::storage::storage::ImageStorage;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppStateDyn {
    pub storage: Arc<dyn ImageStorage>,
}
