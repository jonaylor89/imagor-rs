use crate::{
    imagorpath::params::{Color, Filter, ImageType, Params},
    storage::storage::Blob,
};
use color_eyre::Result;
use rand::Fill;

pub struct Processor {
    disable_blur: bool,
    disable_filters: Vec<Filter>,
    max_filter_ops: i32,
    concurrency: i32,
    max_cache_files: i32,
    max_cache_mem: i32,
    max_cache_size: i32,
    max_width: i32,
    max_height: i32,
    max_resolution: i32,
    max_animation_frames: usize,
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
        let _processing_params = self.preprocess(blob, params);
        // .and_then(|img| self.core_process(img, params))?
        // .and_then(|img| self.apply_filters(img, &params.filters))?;

        // let export_ready = self.export(&processed_image, _params)?;

        // Ok(export_ready)
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    fn preprocess(&self, blob: &Blob, params: &Params) -> ProcessingParams {
        let initial_params = ProcessingParams {
            thumbnail_not_supported: params.trim,
            upscale: !params.fit_in,
            stretch: params.stretch,
            thumbnail: false,
            strip_exif: false,
            strip_metadata: self.strip_metadata,
            orient: 0,
            format: None,
            max_n: self.max_animation_frames.max(1),
            max_bytes: 0,
            page: 1,
            dpi: 0,
            focal_rects: Vec::new(),
        };

        let params_after_blob = if blob.supports_animation() {
            initial_params
        } else {
            ProcessingParams {
                max_n: 1,
                ..initial_params
            }
        };

        params
            .filters
            .iter()
            .fold(params_after_blob, |acc, filter| {
                if self.disable_filters.contains(&filter) {
                    return acc;
                }

                match filter {
                    Filter::Format(format) => {
                        let new_max_n = if !format.is_animation_supported() {
                            1
                        } else {
                            acc.max_n
                        };
                        ProcessingParams {
                            format: Some(format.to_owned()),
                            max_n: new_max_n,
                            ..acc
                        }
                    }
                    Filter::MaxFrames(frames) => {
                        let new_max_n = if *frames > 0 && *frames < acc.max_n {
                            *frames
                        } else {
                            acc.max_n
                        };
                        ProcessingParams {
                            max_n: new_max_n,
                            ..acc
                        }
                    }
                    Filter::Upscale => ProcessingParams {
                        upscale: true,
                        ..acc
                    },
                    Filter::Fill(color) | Filter::BackgroundColor(color) => match color {
                        Color::Auto => ProcessingParams {
                            thumbnail_not_supported: true,
                            ..acc
                        },
                        _ => acc,
                    },
                    Filter::Page(page) => {
                        let new_page = *page.max(&1);
                        ProcessingParams {
                            page: new_page,
                            ..acc
                        }
                    }
                    Filter::Dpi(dpi) => {
                        let new_dpi = *dpi.max(&0);
                        ProcessingParams {
                            dpi: new_dpi,
                            ..acc
                        }
                    }
                    Filter::Orient(orient) => {
                        if *orient > 0 {
                            ProcessingParams {
                                orient: *orient,
                                thumbnail_not_supported: true,
                                ..acc
                            }
                        } else {
                            acc
                        }
                    }
                    Filter::MaxBytes(max_bytes) => ProcessingParams {
                        max_bytes: *max_bytes,
                        thumbnail_not_supported: true,
                        ..acc
                    },
                    Filter::Focal(_) | Filter::Rotate(_) => ProcessingParams {
                        thumbnail_not_supported: true,
                        ..acc
                    },
                    Filter::StripExif => ProcessingParams {
                        strip_exif: true,
                        ..acc
                    },
                    Filter::StripMetadata => ProcessingParams {
                        strip_metadata: true,
                        ..acc
                    },
                    _ => acc,
                }
            })
    }
}

#[derive(Clone, Debug)]
pub struct ProcessingParams {
    thumbnail_not_supported: bool,
    upscale: bool,
    stretch: bool,
    thumbnail: bool,
    strip_exif: bool,
    strip_metadata: bool,
    orient: i32,
    format: Option<ImageType>,
    max_n: usize,
    max_bytes: usize,
    page: usize,
    dpi: u32,
    focal_rects: Vec<Focal>,
}

#[derive(Clone, Debug)]
pub struct Focal {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}
