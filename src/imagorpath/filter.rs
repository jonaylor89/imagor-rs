use crate::imagorpath::{color::Color, type_utils::F32};
use color_eyre::{eyre, Result};
use libvips::{
    ops::{self, DrawRectOptions, FlattenOptions},
    VipsImage,
};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Filter {
    BackgroundColor(Color),
    Blur(F32),
    Brightness(i32),
    Contrast(i32),
    Fill(Color),
    Focal(FocalParams),
    Format(ImageType),
    Grayscale,
    Hue(i32),
    Label(LabelParams),
    MaxBytes(usize),
    MaxFrames(usize),
    Orient(i32),
    Page(usize),
    Dpi(u32),
    Proportion(F32),
    Quality(u8),
    Rgb(i32, i32, i32),
    Rotate(i32),
    RoundCorner(RoundedCornerParams),
    Saturation(i32),
    Sharpen(F32),
    StripExif,
    StripIcc,
    StripMetadata,
    Upscale,
    Watermark(WatermarkParams),
}

impl Filter {
    #[tracing::instrument(skip(img))]
    pub fn apply(&self, img: &VipsImage) -> Result<VipsImage> {
        // Apply the filter to the imag
        match self {
            Filter::BackgroundColor(color) => {
                if !img.image_hasalpha() {
                    return Ok(img.clone());
                }

                let (r, g, b) = color.to_rgb(img).ok_or(eyre::eyre!("Invalid color"))?;

                ops::flatten_with_opts(
                    img,
                    &FlattenOptions {
                        background: vec![r.into(), g.into(), b.into()],
                        ..Default::default()
                    },
                )
                .map_err(|e| {
                    color_eyre::Report::msg(format!("Failed to apply background color: {}", e))
                })
            }
            Filter::Grayscale => img
                .grayscale()
                .map_err(|e| eyre!("Failed to apply grayscale filter: {}", e)),
            Filter::RoundCorner(rcp) => {
                let width = img.get_width();
                let height = img.get_height();

                // Create a black rectangle with alpha channel
                let mask = ops::black(width, height)?;

                // Create white rounded rectangle
                let radius_x = params.rx as f64;
                let radius_y = params.ry.unwrap_or(params.rx) as f64;

                // Draw rounded rectangle on the mask
                let mask = ops::draw_rect_with_opts(
                    &mask,
                    255.0,  // white
                    0,      // x
                    0,      // y
                    width,  // w
                    height, // h
                    &DrawOptions {
                        radius_x,
                        radius_y,
                        fill: true,
                        ..Default::default()
                    },
                )?;

                // If image doesn't have alpha channel, add one
                let img = if !img.image_hasalpha() {
                    ops::bandjoin_const(&img, &[255.0])?
                } else {
                    img.clone()
                };

                // Multiply the image's alpha channel with our mask
                ops::multiply(&img, &mask).map_err(|e| {
                    color_eyre::Report::msg(format!("Failed to apply rounded corners: {}", e))
                })
            }
            Filter::Rotate(angle) => {
                ops::rotate(img, angle).map_err(|e| eyre!("Failed to apply rotate filter: {}", e))
            }
            _ => Ok(img.clone()),
        }
    }
}

impl std::fmt::Display for Filter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Filter::BackgroundColor(color) => write!(f, "background_color({})", color),
            Filter::Blur(amount) => write!(f, "blur({})", amount.0),
            Filter::Brightness(value) => write!(f, "brightness({})", value),
            Filter::Contrast(value) => write!(f, "contrast({})", value),
            Filter::Fill(color) => write!(f, "fill({})", color),
            Filter::Focal(value) => write!(f, "focal({})", value),
            Filter::Format(format) => write!(f, "format({:?})", format),
            Filter::Grayscale => write!(f, "grayscale()"),
            Filter::Hue(value) => write!(f, "hue({})", value),
            Filter::Label(params) => write!(f, "label({:?})", params),
            Filter::MaxBytes(value) => write!(f, "max_bytes({})", value),
            Filter::MaxFrames(value) => write!(f, "max_frames({})", value),
            Filter::Orient(value) => write!(f, "orient({})", value),
            Filter::Page(value) => write!(f, "page({})", value),
            Filter::Dpi(value) => write!(f, "dpi({})", value),
            Filter::Proportion(value) => write!(f, "proportion({})", value.0),
            Filter::Quality(value) => write!(f, "quality({})", value),
            Filter::Rgb(r, g, b) => write!(f, "rgb({},{},{})", r, g, b),
            Filter::Rotate(value) => write!(f, "rotate({})", value),
            Filter::RoundCorner(params) => write!(f, "round_corner({:?})", params),
            Filter::Saturation(value) => write!(f, "saturation({})", value),
            Filter::Sharpen(value) => write!(f, "sharpen({})", value.0),
            Filter::StripExif => write!(f, "strip_exif()"),
            Filter::StripIcc => write!(f, "strip_icc()"),
            Filter::StripMetadata => write!(f, "strip_metadata()"),
            Filter::Upscale => write!(f, "upscale()"),
            Filter::Watermark(params) => write!(f, "watermark({:?})", params),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImageType {
    GIF,
    JPEG,
    PNG,
    MAGICK,
    PDF,
    SVG,
    TIFF,
    WEBP,
    HEIF,
    BMP,
    AVIF,
    JP2K,
}

impl ImageType {
    pub fn is_animation_supported(&self) -> bool {
        matches!(self, ImageType::GIF | ImageType::WEBP)
    }
}

impl std::fmt::Display for ImageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageType::GIF => write!(f, "gif"),
            ImageType::JPEG => write!(f, "jpeg"),
            ImageType::PNG => write!(f, "png"),
            ImageType::MAGICK => write!(f, "magick"),
            ImageType::PDF => write!(f, "pdf"),
            ImageType::SVG => write!(f, "svg"),
            ImageType::TIFF => write!(f, "tiff"),
            ImageType::WEBP => write!(f, "webp"),
            ImageType::HEIF => write!(f, "heif"),
            ImageType::BMP => write!(f, "bmp"),
            ImageType::AVIF => write!(f, "avif"),
            ImageType::JP2K => write!(f, "jp2k"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct WatermarkParams {
    pub image: String,
    pub x: WatermarkPosition,
    pub y: WatermarkPosition,
    pub alpha: u8,
    pub w_ratio: Option<F32>,
    pub h_ratio: Option<F32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum WatermarkPosition {
    Pixels(i32),
    Percentage(F32),
    Left,
    Right,
    Center,
    Top,
    Bottom,
    Repeat,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RoundedCornerParams {
    pub rx: u32,
    pub ry: Option<u32>,
    pub color: Option<Color>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct LabelParams {
    pub text: String,
    pub x: LabelPosition,
    pub y: LabelPosition,
    pub size: u32,
    pub color: Color,
    pub alpha: Option<u8>,
    pub font: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum LabelPosition {
    Pixels(i32),
    Percentage(F32),
    Left,
    Right,
    Center,
    Top,
    Bottom,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum FocalParams {
    Region {
        top_left: (F32, F32),
        bottom_right: (F32, F32),
    },
    Point(F32, F32),
}

impl fmt::Display for FocalParams {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FocalParams::Region {
                top_left,
                bottom_right,
            } => {
                write!(
                    f,
                    "{}x{}:{}x{}",
                    top_left.0, top_left.1, bottom_right.0, bottom_right.1
                )
            }
            FocalParams::Point(x, y) => write!(f, "{}x{}", x, y),
        }
    }
}
