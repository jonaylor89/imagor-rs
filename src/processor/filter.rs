use color_eyre::Result;
use libvips::{
    ops::{self, FlattenOptions},
    VipsImage,
};

use crate::imagorpath::color::Color;

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

                let (r, g, b) = color.to_rgb()?;

                ops::flatten_with_opts(
                    img,
                    &FlattenOptions {
                        background: vec![r, g, b],
                        ..Default::default()
                    },
                )
            }
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
