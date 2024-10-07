use crate::{imagorpath::params::Params, storage::storage::Blob};
use color_eyre::Result;

pub struct Processor {
    disable_blur: bool,
    disable_filters: Vec<String>,
    max_filter_ops: i32,
    concurrency: i32,
    max_cache_files: i32,
    max_cache_mem: i32,
    max_cache_size: i32,
    max_width: i32,
    max_height: i32,
    max_resolution: i32,
    max_animation_frames: i32,
    mozjpeg: bool,
    strip_metadata: bool,
    avif_speed: i32,
}

impl Processor {
    #[tracing::instrument(skip(self))]
    pub fn startup(&self) -> Result<()> {
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn process(&self, blob: &Blob, params: &Params) -> Result<()> {
        let _processed_image = self.preprocess(blob, params)?;
        // .and_then(|img| self.core_process(img, params))?
        // .and_then(|img| self.apply_filters(img, &params.filters))?;

        // let export_ready = self.export(&processed_image, _params)?;

        // Ok(export_ready)
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    fn preprocess(&self, blob: &Blob, params: &Params) -> Result<Blob> {
        Ok(blob)
    }
}
