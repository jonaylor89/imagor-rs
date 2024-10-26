use crate::imagorpath::{color::Color, type_utils::F32};
use color_eyre::{eyre, Result};
use libvips::{
    ops::{self, Composite2Options, FlattenOptions, TextOptions},
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
    Modulate(u32, u32, u32),
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
            Filter::RoundCorner(params) => {
                let width = img.get_width();
                let height = img.get_height();

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
                let img = if !img.image_hasalpha() {
                    &ops::bandjoin_const(&img, &mut [255.0])?
                } else {
                    img
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
                ops::multiply(img, &mask)
                    .map_err(|e| eyre::eyre!("Failed to apply rounded corners: {}", e))
            }
            Filter::Rotate(angle) => {
                let angle = *angle as f64;
                ops::rotate(img, angle)
                    .map_err(|e| eyre::eyre!("Failed to apply rotate filter: {}", e))
            }
            Filter::Label(params) => {
                // Ensure image is in RGB/RGBA color space
                let img = match img.get_interpretation()? as i32 {
                    // Compare raw discriminant values instead of enum variants
                    x if x == ops::Interpretation::BW as i32
                        || x == ops::Interpretation::Cmyk as i32
                        || x == ops::Interpretation::Lab as i32 =>
                    {
                        &ops::colourspace(img, ops::Interpretation::Srgb)?
                    }
                    _ => img,
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
                ops::composite_2_with_opts(
                    img,
                    &text,
                    ops::BlendMode::Over,
                    &Composite2Options {
                        x,
                        y,
                        ..Default::default()
                    },
                )
                .map_err(|e| eyre::eyre!("Failed to apply label: {}", e))
            }
            Filter::Grayscale => ops::colourspace(img, ops::Interpretation::BW)
                .map_err(|e| eyre::eyre!("Failed to apply grayscale filter: {}", e)),
            Filter::Brightness(brightness) => {
                let size = if img.image_hasalpha() { 4 } else { 3 };
                let adjusted_brightness = *brightness as f64 / 255.0;

                let mut alpha = vec![1.0; size];
                let mut beta = vec![adjusted_brightness; size];

                ops::linear(img, alpha.as_mut_slice(), beta.as_mut_slice())
                    .map_err(|e| eyre::eyre!("Failed to apply brightness filter: {}", e))
            }
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
            Filter::Contrast(contrast) => {
                let adjusted_contrast = *contrast as f64 / 255.0;

                let a = adjusted_contrast.clamp(-255.0, 255.0);
                let a = (259.0 * (a + 255.0)) / (255.0 * (259.0 - a));
                let b = 128.0 - a * 128.0;

                let size = if img.image_hasalpha() { 4 } else { 3 };
                let mut alpha = vec![a; size];
                let mut beta = vec![b; size];

                ops::linear(img, alpha.as_mut_slice(), beta.as_mut_slice())
                    .map_err(|e| eyre::eyre!("Failed to apply contrast filter: {}", e))
            }
            Filter::Modulate(brightness, saturation, hue) => {
                let b = 1.0 + (*brightness as f64) / 100.0;
                let s = 1.0 + (*saturation as f64) / 100.0;
                let h = *hue as f64;

                let colorspace = match img.get_interpretation()? {
                    ops::Interpretation::Rgb => ops::Interpretation::Srgb,
                    cs => cs,
                };

                let mut multiplications: Vec<f64> = if img.image_hasalpha() {
                    vec![b, s, 1.0, 1.0]
                } else {
                    vec![b, s, 1.0]
                };
                let mut additions: Vec<f64> = if img.image_hasalpha() {
                    vec![0.0, 0.0, h, 0.0]
                } else {
                    vec![0.0, 0.0, h]
                };

                let colorspace_img = ops::colourspace(img, ops::Interpretation::Lch)?;
                let linear_img = ops::linear(
                    &colorspace_img,
                    multiplications.as_mut_slice(),
                    additions.as_mut_slice(),
                )?;
                let final_img = ops::colourspace(&linear_img, colorspace)?;

                Ok(final_img)
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
            Filter::Modulate(b, s, h) => write!(f, "modulate({}, {}, {})", b, s, h),
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
