use std::ops::Deref;

use crate::imagorpath::{
    color::Color,
    filter::{Filter, LabelPosition},
    params::{Fit, Params},
};
use color_eyre::{
    eyre::{self, Context},
    Result,
};
use libvips::{
    ops::{
        self, Composite2Options, Direction, EmbedOptions, FlattenOptions, Interesting,
        SharpenOptions, Size, TextOptions, ThumbnailImageOptions,
    },
    VipsImage,
};
use metrics::IntoF64;
use thiserror::Error;
use tracing::instrument;

#[derive(Error, Debug)]
pub enum ProcessError {
    #[error("Image processing failed: {0}")]
    ImageProcessingError(String),
    #[error("Failed to load image")]
    ImageLoadError,
}

#[derive(Debug, Clone)]
pub struct Image(VipsImage);

impl Image {
    pub fn new(image: VipsImage) -> Self {
        Image(image)
    }

    pub fn from_file(path: &str) -> Result<Self> {
        let image = VipsImage::new_from_file(path)?;
        Ok(Image(image))
    }

    // Method to get the inner VipsImage if needed
    pub fn into_inner(self) -> VipsImage {
        self.0
    }

    // Method to get a reference to the inner VipsImage
    pub fn as_inner(&self) -> &VipsImage {
        &self.0
    }

    pub fn is_animated(&self) -> bool {
        self.0.get_height() > self.0.get_page_height()
    }

    #[instrument(skip(self))]
    pub fn apply_orientation(&self, orient: i32) -> Result<Self, ProcessError> {
        if orient > 0 {
            let rotated = ops::rotate(&self.0, orient.into_f64()).map_err(|_| {
                ProcessError::ImageProcessingError("Failed to apply orientation".into())
            })?;

            Ok(Image::new(rotated))
        } else {
            Ok(self.clone())
        }
    }

    pub fn calculate_dimensions(&self, params: &Params, upscale: bool) -> (i32, i32) {
        match (params.width, params.height) {
            (None, None) => (self.0.get_width(), self.0.get_page_height()),
            (None, Some(h)) => {
                let w = self.0.get_width() * h / self.0.get_page_height();
                (
                    if !upscale {
                        w.min(self.0.get_width())
                    } else {
                        w
                    },
                    h,
                )
            }
            (Some(w), None) => {
                let h = self.0.get_page_height() * w / self.0.get_width();
                (
                    w,
                    if !upscale {
                        h.min(self.0.get_page_height())
                    } else {
                        h
                    },
                )
            }
            (Some(w), Some(h)) => (w, h),
        }
    }

    pub fn resize_image(
        &self,
        width: i32,
        height: i32,
        fit: Option<Fit>,
        upscale: bool,
        _params: &Params,
    ) -> Result<Image, ProcessError> {
        let should_resize =
            upscale || width < self.0.get_width() || height < self.0.get_page_height();
        let size = match fit {
            Some(Fit::FitIn) => Size::Both,
            Some(Fit::Stretch) => Size::Force,
            _ => return Ok(self.to_owned()),
        };

        if should_resize {
            let thumbnail = ops::thumbnail_image_with_opts(
                &self.0,
                width,
                &ThumbnailImageOptions {
                    height,
                    crop: Interesting::None,
                    size,
                    ..Default::default()
                },
            )
            .map_err(|_| ProcessError::ImageProcessingError("Failed to resize image".into()))?;

            Ok(Image::new(thumbnail))
        } else {
            Ok(self.to_owned())
        }
    }

    pub fn apply_flip(&self, h_flip: bool, v_flip: bool) -> Result<Self, ProcessError> {
        let flipped = if h_flip {
            &ops::flip(&self.0, Direction::Horizontal).map_err(|_| {
                ProcessError::ImageProcessingError("Failed to apply horizontal flip".into())
            })?
        } else {
            &self.0
        };

        if v_flip {
            let v_flipped = ops::flip(flipped, Direction::Vertical).map_err(|_| {
                ProcessError::ImageProcessingError("Failed to apply vertical flip".into())
            })?;

            Ok(Image::new(v_flipped))
        } else {
            Ok(Image::new(flipped.clone()))
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn apply(&self, filter: &Filter) -> Result<Self> {
        // Apply the filter to the imag
        match filter {
            Filter::RoundCorner(params) => {
                let width = self.get_width();
                let height = self.get_height();

                // Create a mask image
                let mask = ops::black(width, height)?;

                // Draw filled rectangle without corners
                ops::draw_rect(
                    &mask,
                    &mut [255.0], // white
                    0,            // x
                    0,            // y
                    width,        // w
                    height,       // h
                )?;

                // Ensure image has alpha channel
                let img = if !self.0.image_hasalpha() {
                    &ops::bandjoin_const(self.as_inner(), &mut [255.0])?
                } else {
                    &self.0
                };

                // Calculate corner radius
                let rx = params.rx as f64;
                let ry = params.ry.unwrap_or(params.rx) as f64;

                // Create a corner mask
                let corner = ops::black(rx as i32, ry as i32)?;
                ops::draw_circle(
                    &corner,
                    &mut [255.0],
                    rx as i32 / 2,
                    ry as i32 / 2,
                    rx as i32 / 2,
                )?;

                // Copy corner to all 4 corners of the mask (rotated appropriately)
                let corners = [
                    (0, 0),                                  // Top-left
                    (width - rx as i32, 0),                  // Top-right
                    (0, height - ry as i32),                 // Bottom-left
                    (width - rx as i32, height - ry as i32), // Bottom-right
                ];

                for (x, y) in corners.iter() {
                    ops::composite_2_with_opts(
                        &mask,
                        &corner,
                        ops::BlendMode::Over,
                        &Composite2Options {
                            x: *x,
                            y: *y,
                            ..Default::default()
                        },
                    )?;
                }

                // Multiply the image's alpha channel with our mask
                let img = ops::multiply(img, &mask)
                    .map_err(|e| eyre::eyre!("Failed to apply rounded corners: {}", e))?;

                Ok(Image::new(img))
            }
            Filter::Rotate(angle) => {
                let angle = *angle as f64;
                let img = ops::rotate(&self.0, angle)
                    .map_err(|e| eyre::eyre!("Failed to apply rotate filter: {}", e))?;

                Ok(Image::new(img))
            }
            Filter::Label(params) => {
                // Ensure image is in RGB/RGBA color space
                let img = match self.0.get_interpretation()? as i32 {
                    // Compare raw discriminant values instead of enum variants
                    x if x == ops::Interpretation::BW as i32
                        || x == ops::Interpretation::Cmyk as i32
                        || x == ops::Interpretation::Lab as i32 =>
                    {
                        &ops::colourspace(&self.0, ops::Interpretation::Srgb)?
                    }
                    _ => &self.0,
                };

                // Add alpha channel if not present
                let img = if !img.image_hasalpha() {
                    &ops::bandjoin_const(&img, &mut [255.0])?
                } else {
                    img
                };

                // Calculate x position
                let width = img.get_width();
                let x = match params.x {
                    LabelPosition::Center => width / 2,
                    LabelPosition::Right => width,
                    LabelPosition::Left => 0,
                    LabelPosition::Pixels(px) => {
                        if px < 0 {
                            width + px
                        } else {
                            px
                        }
                    }
                    LabelPosition::Percentage(pct) => (pct.0 * width as f32) as i32,
                    _ => 0,
                };

                // Calculate y position
                let height = img.get_height();
                let y = match params.y {
                    LabelPosition::Center => (height - params.size as i32) / 2,
                    LabelPosition::Top => 0,
                    LabelPosition::Bottom => height - params.size as i32,
                    LabelPosition::Pixels(px) => {
                        if px < 0 {
                            height + px - params.size as i32
                        } else {
                            px
                        }
                    }
                    LabelPosition::Percentage(pct) => (pct.0 * height as f32) as i32,
                    _ => 0,
                };

                // Get text color
                let (r, g, b) = params
                    .color
                    .to_rgb(&img)
                    .ok_or(eyre::eyre!("Invalid color"))?;

                // Calculate alpha value (default to fully opaque if not specified)
                let alpha = params.alpha.unwrap_or(255);

                // Use default font if none specified
                let font = params.font.as_deref().unwrap_or("sans");

                // Create text overlay
                let text = ops::text_with_opts(
                    &params.text,
                    &TextOptions {
                        font: font.to_string(),
                        width,
                        height: params.size as i32,
                        align: match params.x {
                            LabelPosition::Center => ops::Align::Centre,
                            LabelPosition::Right => ops::Align::High,
                            _ => ops::Align::Low,
                        },
                        dpi: 72,
                        justify: true,
                        rgba: true,
                        spacing: 0,
                        ..Default::default()
                    },
                )?;

                // Colorize the text
                let text = ops::linear(
                    &text,
                    &mut [
                        r as f64 / 255.0,
                        g as f64 / 255.0,
                        b as f64 / 255.0,
                        alpha as f64 / 255.0,
                    ],
                    &mut [0.0, 0.0, 0.0, 0.0],
                )?;

                // Composite text onto image
                let img = ops::composite_2_with_opts(
                    img,
                    &text,
                    ops::BlendMode::Over,
                    &Composite2Options {
                        x,
                        y,
                        ..Default::default()
                    },
                )
                .map_err(|e| eyre::eyre!("Failed to apply label: {}", e))?;

                Ok(Self(img))
            }
            Filter::Grayscale => ops::colourspace(&self.0, ops::Interpretation::BW)
                .map_err(|e| eyre::eyre!("Failed to apply grayscale filter: {}", e))
                .map(Self),
            Filter::Brightness(brightness) => {
                let size = if self.0.image_hasalpha() { 4 } else { 3 };
                let adjusted_brightness = *brightness as f64 / 255.0;

                let mut alpha = vec![1.0; size];
                let mut beta = vec![adjusted_brightness; size];

                let img = ops::linear(&self.0, alpha.as_mut_slice(), beta.as_mut_slice())
                    .map_err(|e| eyre::eyre!("Failed to apply brightness filter: {}", e))?;

                Ok(Self(img))
            }
            Filter::BackgroundColor(color) => {
                if !self.0.image_hasalpha() {
                    return Ok(self.to_owned());
                }

                let (r, g, b) = color
                    .to_rgb(self.as_inner())
                    .ok_or(eyre::eyre!("Invalid color"))?;

                let flattened = ops::flatten_with_opts(
                    &self.0,
                    &FlattenOptions {
                        background: vec![r.into(), g.into(), b.into()],
                        ..Default::default()
                    },
                )
                .map_err(|e| {
                    color_eyre::Report::msg(format!("Failed to apply background color: {}", e))
                })?;

                Ok(Self(flattened))
            }
            Filter::Contrast(contrast) => {
                let adjusted_contrast = *contrast as f64 / 255.0;

                let a = adjusted_contrast.clamp(-255.0, 255.0);
                let a = (259.0 * (a + 255.0)) / (255.0 * (259.0 - a));
                let b = 128.0 - a * 128.0;

                let size = if self.0.image_hasalpha() { 4 } else { 3 };
                let mut alpha = vec![a; size];
                let mut beta = vec![b; size];

                let img = ops::linear(&self.0, alpha.as_mut_slice(), beta.as_mut_slice())
                    .map_err(|e| eyre::eyre!("Failed to apply contrast filter: {}", e))?;

                Ok(Self(img))
            }
            Filter::Modulate(brightness, saturation, hue) => {
                let b = 1.0 + (*brightness as f64) / 100.0;
                let s = 1.0 + (*saturation as f64) / 100.0;
                let h = *hue as f64;

                let colorspace = match self.0.get_interpretation()? {
                    ops::Interpretation::Rgb => ops::Interpretation::Srgb,
                    cs => cs,
                };

                let mut multiplications: Vec<f64> = if self.0.image_hasalpha() {
                    vec![b, s, 1.0, 1.0]
                } else {
                    vec![b, s, 1.0]
                };
                let mut additions: Vec<f64> = if self.0.image_hasalpha() {
                    vec![0.0, 0.0, h, 0.0]
                } else {
                    vec![0.0, 0.0, h]
                };

                let colorspace_img = ops::colourspace(&self.0, ops::Interpretation::Lch)?;
                let linear_img = ops::linear(
                    &colorspace_img,
                    multiplications.as_mut_slice(),
                    additions.as_mut_slice(),
                )?;
                let final_img = ops::colourspace(&linear_img, colorspace)?;

                Ok(Image::new(final_img))
            }
            Filter::Hue(hue) => {
                todo!()
            }
            Filter::Saturation(s) => {
                todo!()
            }
            Filter::Rgb(r, g, b) => {
                todo!()
            }
            Filter::Blur(blur) => {
                if self.is_animated() {
                    return Ok(self.to_owned());
                }

                let sigma = blur.0 as f64;

                if sigma > 0.0 {
                    return ops::gaussblur(&self.0, sigma)
                        .map_err(|e| eyre::eyre!("Failed to apply blur filter: {}", e))
                        .map(Self);
                }

                Ok(self.to_owned())
            }
            Filter::Sharpen(sharpen) => {
                if self.is_animated() {
                    return Ok(self.to_owned());
                }

                let sigma = (1.0 + sharpen.0 * 2.0) as f64;

                if sigma <= 0.0 {
                    return Ok(self.to_owned());
                }

                ops::sharpen_with_opts(
                    &self.0,
                    &SharpenOptions {
                        sigma,
                        x_1: 1.0,
                        m_1: 2.0,
                        ..Default::default()
                    },
                )
                .map_err(|e| eyre::eyre!("Failed to apply sharpen filter: {}", e))
                .map(Self)
            }
            Filter::StripIcc => {
                todo!()
            }
            Filter::StripExif => {
                todo!()
            }
            // Filter::Trim => {
            //     todo!()
            // }
            // Filter::SetFrames(frames) => {
            //     todo!()
            //
            Filter::Padding(color, padding) => {
                let (left, top, right, bottom) = padding.get_values();

                self.fill(
                    self.0.get_width(),
                    self.0.get_height(),
                    left,
                    top,
                    right,
                    bottom,
                    color,
                )
            }
            Filter::Proportion(proporation) => {
                let mut scale = proporation.0.clamp(0.0, 100.0);
                if scale > 1.0 {
                    scale /= 100.0
                }

                let width = (self.0.get_width() as f32 * scale).round() as i32;
                let height = (self.0.get_height() as f32 * scale).round() as i32;

                let thumbnail = ops::thumbnail_image_with_opts(
                    &self.0,
                    width,
                    &ThumbnailImageOptions {
                        height,
                        crop: Interesting::None,
                        ..Default::default()
                    },
                )
                .wrap_err("Failed to apply proportion filter")?;

                Ok(Self(thumbnail))
            }
            _ => Ok(self.to_owned()),
        }
    }

    #[tracing::instrument(skip(self))]
    fn fill(
        &self,
        width: i32,
        height: i32,
        p_left: i32,
        p_top: i32,
        p_right: i32,
        p_bottom: i32,
        color: &Color,
    ) -> Result<Self> {
        let left = (width - self.0.get_width()) / 2 + p_left;
        let top = (height - self.0.get_page_height()) / 2 + p_top;
        let total_width = width + p_left + p_right;
        let total_height = height + p_top + p_bottom;

        match color {
            Color::None => {
                // Handle transparent padding
                let img = if self.0.get_bands() < 3 {
                    // Convert to sRGB if needed
                    ops::colourspace(&self.0, ops::Interpretation::Srgb)?
                } else {
                    self.0.clone()
                };

                // Add alpha channel if needed
                let img = if !img.image_hasalpha() {
                    ops::bandjoin_const(&img, &mut [255.0])?
                } else {
                    img
                };

                // Embed with transparent background
                let embedded = ops::embed_with_opts(
                    &img,
                    left,
                    top,
                    total_width,
                    total_height,
                    &ops::EmbedOptions {
                        extend: ops::Extend::Background,
                        background: vec![0.0, 0.0, 0.0, 0.0],
                        ..Default::default()
                    },
                )?;

                Ok(Self(embedded))
            }
            Color::Blur if !self.is_animated() => {
                // Handle blur padding (if image is not animated)
                let copy = self.0.clone();

                // Create blurred background
                let blurred = ops::thumbnail_image_with_opts(
                    &self.0,
                    total_width,
                    &ThumbnailImageOptions {
                        height: total_height,
                        size: Size::Force,
                        ..Default::default()
                    },
                )?;
                let blurred = ops::gaussblur(&blurred, 50.0)?;

                // Composite original image over blurred background
                let result = ops::composite_2_with_opts(
                    &blurred,
                    &copy,
                    ops::BlendMode::Over,
                    &Composite2Options {
                        x: left,
                        y: top,
                        ..Default::default()
                    },
                )?;

                Ok(Self(result))
            }
            _ => {
                // Handle solid color padding
                let (r, g, b) = color
                    .to_rgb(self.as_inner())
                    .ok_or_else(|| eyre::eyre!("Invalid color"))?;

                // Flatten image if it has alpha channel
                let img = if self.0.image_hasalpha() {
                    ops::flatten_with_opts(
                        &self.0,
                        &FlattenOptions {
                            background: vec![r.into(), g.into(), b.into()],
                            ..Default::default()
                        },
                    )?
                } else {
                    self.0.clone()
                };

                // Embed with colored background
                let embedded = ops::embed_with_opts(
                    &img,
                    left,
                    top,
                    total_width,
                    total_height,
                    &EmbedOptions {
                        extend: ops::Extend::Background,
                        background: vec![r.into(), g.into(), b.into()],
                        ..Default::default()
                    },
                )?;

                Ok(Self(embedded))
            }
        }
    }
}

impl Deref for Image {
    type Target = VipsImage;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
