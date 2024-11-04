use std::{thread::available_parallelism, time::Instant};

use super::image::{Image, ProcessError};
use crate::{
    imagorpath::{
        color::Color,
        filter::{Filter, ImageType},
        params::{Fit, HAlign, Params, VAlign},
        type_utils::F32,
    },
    storage::storage::Blob,
};
use color_eyre::Result;
use libvips::{
    ops::{
        self, ForeignHeifCompression, ForeignPngFilter, HeifsaveBufferOptions, Interesting,
        JpegsaveBufferOptions, PngsaveBufferOptions, Size, ThumbnailBufferOptions,
        TiffsaveBufferOptions, WebpsaveBufferOptions,
    },
    VipsImage,
};
use tracing::{debug, error};

pub trait ImageProcessor: Send + Sync {
    fn startup(&self) -> Result<()>;
    fn process(&self, blob: &Blob, params: &Params) -> Result<Blob>;
    fn shutdown(&self) -> Result<()>;
}

#[derive(Debug, Default)]
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

#[derive(Debug)]
struct ExportOptions {
    quality: Option<i32>,
    compression: Option<i32>,
    palette: bool,
    bitdepth: Option<i32>,
    strip_metadata: bool,
    max_bytes: usize,
}

impl ImageProcessor for Processor {
    #[tracing::instrument(skip(self))]
    fn startup(&self) -> Result<()> {
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    #[tracing::instrument(skip(self, blob))]
    fn process(&self, blob: &Blob, params: &Params) -> Result<Blob> {
        let processing_params = self.preprocess(blob, params);
        let img = self.load_image(blob, params, &processing_params)?;
        let img = img.apply_orientation(processing_params.orient)?;
        let (width, height) = img.calculate_dimensions(params, processing_params.upscale);
        let img = img.resize_image(width, height, params.fit, processing_params.upscale, params)?;
        let img = img.apply_flip(params.h_flip, params.v_flip)?;

        let img = self.apply_filters(img, params, &processing_params)?;

        // if p.meta {
        //     // metadata without export
        //     return imagor.NewBlobFromJsonMarshal(metadata(img, format, stripExif)), nil
        // }

        let inferred_format: Option<ImageType> =
            infer::get(&blob.data).map(|t| match t.mime_type() {
                "image/png" => ImageType::PNG,
                "image/jpeg" => ImageType::JPEG,
                "image/jpg" => ImageType::JPEG,
                "image/webp" => ImageType::WEBP,
                "image/gif" => ImageType::GIF,
                "image/tiff" => ImageType::TIFF,
                "image/heic" => ImageType::HEIF,
                "image/avif" => ImageType::AVIF,
                "image/bmp" => ImageType::BMP,
                "image/jp2" => ImageType::JP2K,
                "image/svg+xml" => ImageType::SVG,
                "image/magick" => ImageType::MAGICK,
                "application/pdf" => ImageType::PDF,
                _ => ImageType::JPEG,
            });
        let exportable_bytes = self.export(&img, &processing_params, inferred_format)?;

        Ok(exportable_bytes)
    }
}

impl Processor {
    pub fn new(p_options: ProcessorOptions) -> Self {
        let mut disabled_filters = p_options.disabled_filters;
        if p_options.disable_blur {
            disabled_filters.push(Filter::Blur(F32(0.0)));
        }

        let concurrency = p_options.concurrency.unwrap_or_else(|| {
            let default_parallelism_approx = available_parallelism().unwrap().get();
            if default_parallelism_approx > 1 {
                default_parallelism_approx as i32
            } else {
                1
            }
        });

        Processor {
            disable_blur: p_options.disable_blur,
            disable_filters: disabled_filters,
            max_width: 100_000,
            max_height: 100_000,
            concurrency,
            ..Default::default()
        }
    }

    #[tracing::instrument(skip(self, blob))]
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
        // Check if blob is valid
        if blob.as_ref().is_empty() {
            return Err(ProcessError::ImageLoadError);
        }

        // Try to get image format
        if let Some(format) = infer::get(&blob.data) {
            debug!("Detected image format: {}", format.mime_type());
        }

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
                    .map_err(|e| {
                        ProcessError::ImageProcessingError(
                            format!("Failed to create thumbnail for fit_in {:?}", e).into(),
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
                .map_err(|e| {
                    ProcessError::ImageProcessingError(
                        format!("Failed to create thumbnail for stretch {:?}", e).into(),
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
                .map_err(|e| {
                    ProcessError::ImageProcessingError(
                        format!("Failed to create width-only thumbnail {:?}", e).into(),
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
                .map_err(|e| {
                    ProcessError::ImageProcessingError(
                        format!("Failed to create height-only thumbnail {:?}", e).into(),
                    )
                }),

                _ => VipsImage::new_from_buffer(blob.as_ref(), "")
                    .map_err(|_| ProcessError::ImageLoadError),
            };

            return img.map(Image::new);
        };

        // If we couldn't create a thumbnail, load the full image
        let img = if processing_params.thumbnail_not_supported {
            VipsImage::new_from_buffer(blob.as_ref(), "").map_err(|e| {
                debug!(
                    "failed to create image from buffer of size {} - {}",
                    blob.as_ref().len(),
                    e
                );
                ProcessError::ImageLoadError
            })
        } else {
            // ops::thumbnail_buffer_with_opts(
            //     blob.as_ref(),
            //     self.max_width,
            //     &ThumbnailBufferOptions {
            //         height: self.max_height,
            //         crop: Interesting::None,
            //         size: Size::Down,
            //         no_rotate: true, // Add this to prevent rotation issues
            //         ..Default::default()
            //     },
            // )
            ops::thumbnail_buffer(blob.as_ref(), 100).map_err(|e| {
                ProcessError::ImageProcessingError(
                    format!(
                        "Failed to create default thumbnail of buffer size {} - {}",
                        blob.as_ref().len(),
                        e
                    )
                    .into(),
                )
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
            let new_image = img.apply(filter, params);
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

    #[tracing::instrument(skip(self, img, params))]
    fn export(
        &self,
        img: &Image,
        params: &ProcessingParams,
        inferred: Option<ImageType>,
    ) -> Result<Blob> {
        let format = params.format.unwrap_or(inferred.unwrap_or(ImageType::JPEG));

        let mut options = ExportOptions {
            quality: None, // Set from params if needed
            compression: None,
            palette: false,
            bitdepth: None,
            strip_metadata: params.strip_metadata,
            max_bytes: params.max_bytes,
        };

        loop {
            let buf: Blob = match format {
                ImageType::PNG => ops::pngsave_buffer_with_opts(
                    img.as_inner(),
                    &PngsaveBufferOptions {
                        compression: options.compression.unwrap_or(6),
                        filter: ForeignPngFilter::None,
                        palette: options.palette,
                        q: options.quality.unwrap_or(75),
                        ..Default::default()
                    },
                )
                .map(|b| Blob {
                    data: b,
                    content_type: format.to_content_type(),
                })?,
                ImageType::WEBP => ops::webpsave_buffer_with_opts(
                    img.as_inner(),
                    &WebpsaveBufferOptions {
                        q: options.quality.unwrap_or(75),
                        ..Default::default()
                    },
                )
                .map(|b| Blob {
                    data: b,
                    content_type: format.to_content_type(),
                })?,
                ImageType::TIFF => ops::tiffsave_buffer_with_opts(
                    img.as_inner(),
                    &TiffsaveBufferOptions {
                        q: options.quality.unwrap_or(75),
                        ..Default::default()
                    },
                )
                .map(|b| Blob {
                    data: b,
                    content_type: format.to_content_type(),
                })?,
                ImageType::GIF => ops::gifsave_buffer(img.as_inner()).map(|b| Blob {
                    data: b,
                    content_type: format.to_content_type(),
                })?,
                ImageType::AVIF => ops::heifsave_buffer_with_opts(
                    img.as_inner(),
                    &HeifsaveBufferOptions {
                        q: options.quality.unwrap_or(75),
                        compression: ForeignHeifCompression::Av1,
                        ..Default::default()
                    },
                )
                .map(|b| Blob {
                    data: b,
                    content_type: format.to_content_type(),
                })?,
                ImageType::HEIF => ops::heifsave_buffer_with_opts(
                    img.as_inner(),
                    &HeifsaveBufferOptions {
                        q: options.quality.unwrap_or(75),
                        compression: ForeignHeifCompression::Hevc,
                        ..Default::default()
                    },
                )
                .map(|b| Blob {
                    data: b,
                    content_type: format.to_content_type(),
                })?,
                _ => {
                    // Default to JPEG
                    ops::jpegsave_buffer_with_opts(
                        img.as_inner(),
                        &JpegsaveBufferOptions {
                            q: options.quality.unwrap_or(75),
                            optimize_coding: true,
                            interlace: true,
                            trellis_quant: true,
                            quant_table: 3,
                            ..Default::default()
                        },
                    )
                    .map(|b| Blob {
                        data: b,
                        content_type: ImageType::JPEG.to_content_type(),
                    })?
                }
            };

            // Handle max bytes logic
            if options.max_bytes > 0
                && (options.quality.unwrap_or(0) > 10 || options.quality.is_none())
                && format != ImageType::PNG
            {
                let len = buf.data.len();
                debug!(
                    "max_bytes check: bytes={}, quality={:?}",
                    len, options.quality
                );

                if len > options.max_bytes {
                    let current_quality = options.quality.unwrap_or(80);
                    let delta = len as f64 / options.max_bytes as f64;

                    options.quality = Some(match delta {
                        d if d > 3.0 => current_quality * 25 / 100,
                        d if d > 1.5 => current_quality * 50 / 100,
                        _ => current_quality * 75 / 100,
                    });

                    continue;
                }
            }

            return Ok(buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};
    use libvips::VipsApp;
    use rand::Rng;

    #[test]
    fn test_basic_image_load() {
        let _vips_app = VipsApp::new("imagor_rs test", true).expect("Failed to initialize VipsApp");
        _vips_app.concurrency_set(4);

        // Create a 100x100 random RGB image
        let width = 100u32;
        let height = 100u32;
        let mut rng = rand::thread_rng();

        let img_buf: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(width, height, |_x, _y| {
                Rgb([
                    rng.gen_range(0..255),
                    rng.gen_range(0..255),
                    rng.gen_range(0..255),
                ])
            });

        // Convert to JPEG
        let mut jpeg_data = Vec::new();
        img_buf
            .write_to(
                &mut std::io::Cursor::new(&mut jpeg_data),
                image::ImageFormat::Jpeg,
            )
            .expect("Failed to create JPEG");

        // Create blob
        let blob = Blob {
            data: jpeg_data,
            content_type: "image/jpeg".to_string(),
        };

        let processor = Processor::new(ProcessorOptions {
            disable_blur: false,
            disabled_filters: vec![],
            concurrency: Some(1),
        });

        let params = Params::default();
        let result = processor.process(&blob, &params);

        assert!(result.is_ok());
    }
}
