use std::time::Instant;

use crate::{
    imagorpath::{
        color::Color,
        filter::{Filter, ImageType},
        params::{Fit, HAlign, Params, VAlign},
    },
    storage::storage::Blob,
};
use color_eyre::Result;
use libvips::{
    ops::{self, Direction, Interesting, Size, ThumbnailBufferOptions, ThumbnailImageOptions},
    VipsApp, VipsImage,
};
use metrics::IntoF64;
use tracing::{debug, error};

use super::image::{Image, ProcessError};

pub struct Processor {
    disable_blur: bool,
    disable_filters: Vec<Filter>,
    max_filter_ops: usize,
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
    vips_app: VipsApp,
}

#[derive(Clone, Debug)]
pub struct ProcessingParams {
    thumbnail_not_supported: bool,
    upscale: bool,
    thumbnail: bool,
    strip_exif: bool,
    strip_metadata: bool,
    orient: i32,
    format: Option<ImageType>,
    max_n: usize,
    max_bytes: usize,
    page: usize,
    dpi: u32,
    focal_rects: Vec<FocalPoint>,
}

#[derive(Debug, Clone)]
pub struct FocalPoint {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
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
        let processing_params = self.preprocess(blob, params);
        let img = self.load_image(blob, params, &processing_params)?;
        let img = img.apply_orientation(processing_params.orient)?;
        let (width, height) = img.calculate_dimensions(params, processing_params.upscale);
        let img = img.resize_image(width, height, params.fit, processing_params.upscale, params)?;
        let img = img.apply_flip(params.h_flip, params.v_flip)?;

        // TODO: Apply filters
        let _filted_img = self.apply_filters(img, params, &processing_params);

        // let export_ready = self.export(&processed_image, _params)?;

        // Ok(export_ready)
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    fn preprocess(&self, blob: &Blob, params: &Params) -> ProcessingParams {
        let initial_params = ProcessingParams {
            thumbnail_not_supported: params.trim,
            upscale: params.fit != Some(Fit::FitIn),
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
                if self.disable_filters.contains(filter) {
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

    #[tracing::instrument(skip(self, blob))]
    fn load_image(
        &self,
        blob: &Blob,
        params: &Params,
        processing_params: &ProcessingParams,
    ) -> Result<Image, ProcessError> {
        if !processing_params.thumbnail_not_supported
            && params.crop_bottom.is_none()
            && params.crop_top.is_none()
            && params.crop_left.is_none()
            && params.crop_right.is_none()
        {
            let img = match (params.fit, params.width, params.height) {
                (Some(Fit::FitIn), Some(width), Some(height)) => {
                    let w = width.max(1);
                    let h = height.max(1);
                    let size = if processing_params.upscale {
                        Size::Both
                    } else {
                        Size::Down
                    };
                    ops::thumbnail_buffer_with_opts(
                        blob.as_ref(),
                        w,
                        &ThumbnailBufferOptions {
                            height: h,
                            size,
                            ..Default::default()
                        },
                    )
                    .map_err(|_| {
                        ProcessError::ImageProcessingError(
                            "Failed to create thumbnail for fit_in".into(),
                        )
                    })
                }
                (Some(Fit::Stretch), Some(width), Some(height)) => ops::thumbnail_buffer_with_opts(
                    blob.as_ref(),
                    width,
                    &ThumbnailBufferOptions {
                        height,
                        crop: Interesting::None,
                        size: Size::Force,
                        ..Default::default()
                    },
                )
                .map_err(|_| {
                    ProcessError::ImageProcessingError(
                        "Failed to create thumbnail for stretch".into(),
                    )
                }),

                (None, Some(width), Some(height)) => {
                    let interest = match (params.v_align, params.h_align) {
                        _ if params.smart => Interesting::Attention,
                        (Some(VAlign::Top), None) | (None, Some(HAlign::Left)) => Interesting::Low,
                        (Some(VAlign::Bottom), None) | (None, Some(HAlign::Right)) => {
                            Interesting::High
                        }
                        (None | Some(VAlign::Middle), None | Some(HAlign::Center)) => {
                            Interesting::Centre
                        }
                        _ => Interesting::None,
                    };

                    ops::thumbnail_buffer_with_opts(
                        blob.as_ref(),
                        width,
                        &ThumbnailBufferOptions {
                            height,
                            crop: interest,
                            size: Size::Both,
                            ..Default::default()
                        },
                    )
                    .map_err(|_| {
                        ProcessError::ImageProcessingError(
                            "Failed to create smart/aligned thumbnail".into(),
                        )
                    })
                }
                (None, Some(width), None) => ops::thumbnail_buffer_with_opts(
                    blob.as_ref(),
                    width,
                    &ThumbnailBufferOptions {
                        height: self.max_height,
                        crop: Interesting::None,
                        size: Size::Both,
                        ..Default::default()
                    },
                )
                .map_err(|_| {
                    ProcessError::ImageProcessingError(
                        "Failed to create width-only thumbnail".into(),
                    )
                }),

                (None, None, Some(height)) => ops::thumbnail_buffer_with_opts(
                    blob.as_ref(),
                    self.max_width,
                    &ThumbnailBufferOptions {
                        height,
                        crop: Interesting::None,
                        size: Size::Both,
                        ..Default::default()
                    },
                )
                .map_err(|_| {
                    ProcessError::ImageProcessingError(
                        "Failed to create height-only thumbnail".into(),
                    )
                }),

                _ => VipsImage::new_from_buffer(blob.as_ref(), "")
                    .map_err(|_| ProcessError::ImageLoadError),
            };

            return img.map(Image::new);
        };

        // If we couldn't create a thumbnail, load the full image
        let img = if processing_params.thumbnail_not_supported {
            VipsImage::new_from_buffer(blob.as_ref(), "").map_err(|_| ProcessError::ImageLoadError)
        } else {
            ops::thumbnail_buffer_with_opts(
                blob.as_ref(),
                self.max_width,
                &ThumbnailBufferOptions {
                    height: self.max_height,
                    crop: Interesting::None,
                    size: Size::Down,
                    ..Default::default()
                },
            )
            .map_err(|_| {
                ProcessError::ImageProcessingError("Failed to create default thumbnail".into())
            })
        };

        return img.map(Image::new);
    }

    #[tracing::instrument(skip(self, img))]
    fn apply_filters(
        &self,
        img: Image,
        params: &Params,
        processing_params: &ProcessingParams,
    ) -> Result<Image, ProcessError> {
        let truncate_length = if self.max_filter_ops > 0 {
            self.max_filter_ops.min(params.filters.len())
        } else {
            params.filters.len()
        };

        if truncate_length < params.filters.len() {
            debug!("max-filter-ops-exceeded |{}|", params.filters.len());
        }
        let filters_slice: &[Filter] = &params.filters[..truncate_length];

        let filtered = filters_slice.iter().fold(img, |img, filter| {
            if self.disable_filters.contains(filter) {
                return img;
            }

            let start = Instant::now();
            let new_image = img.apply(filter);
            let elapsed = start.elapsed().as_millis();

            debug!("filter |{}| took {}", filter, elapsed);

            match new_image {
                Ok(new_image) => new_image,
                Err(err) => {
                    error!("filter |{}| failed: {:?}", filter, err);
                    img
                }
            }
        });

        Ok(filtered)
    }
}
