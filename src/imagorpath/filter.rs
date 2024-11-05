use crate::imagorpath::{color::Color, type_utils::F32};
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
    Hue(F32),
    Label(LabelParams),
    MaxBytes(usize),
    MaxFrames(usize),
    Modulate(F32, F32, F32),
    Orient(i32),
    Padding(Color, PaddingParams),
    Page(usize),
    Dpi(u32),
    Proportion(F32),
    Quality(u8),
    Rgb(F32, F32, F32),
    Rotate(i32),
    RoundCorner(RoundedCornerParams),
    Saturation(F32),
    Sharpen(F32),
    StripExif,
    StripIcc,
    StripMetadata,
    Upscale,
    Watermark(WatermarkParams),
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
            Filter::Padding(color, params) => write!(f, "padding({},{})", color, params),
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

impl Filter {
    pub fn name(&self) -> String {
        let name = match self {
            Filter::BackgroundColor(_) => "background_color",
            Filter::Blur(_) => "blur",
            Filter::Brightness(_) => "brightness",
            Filter::Contrast(_) => "contrast",
            Filter::Fill(_) => "fill",
            Filter::Focal(_) => "focal",
            Filter::Format(_) => "format",
            Filter::Grayscale => "grayscale",
            Filter::Hue(_) => "hue",
            Filter::Label(_) => "label",
            Filter::MaxBytes(_) => "max_bytes",
            Filter::MaxFrames(_) => "max_frames",
            Filter::Modulate(_, _, _) => "modulate",
            Filter::Orient(_) => "orient",
            Filter::Padding(_, _) => "padding",
            Filter::Page(_) => "page",
            Filter::Dpi(_) => "dpi",
            Filter::Proportion(_) => "proportion",
            Filter::Quality(_) => "quality",
            Filter::Rgb(_, _, _) => "rgb",
            Filter::Rotate(_) => "rotate",
            Filter::RoundCorner(_) => "round_corner",
            Filter::Saturation(_) => "saturation",
            Filter::Sharpen(_) => "sharpen",
            Filter::StripExif => "strip_exif",
            Filter::StripIcc => "strip_icc",
            Filter::StripMetadata => "strip_metadata",
            Filter::Upscale => "upscale",
            Filter::Watermark(_) => "watermark",
        };

        return name.to_string();
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
    pub fn to_content_type(&self) -> String {
        return format!("image/{}", self.to_string().to_lowercase());
    }

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
pub enum PaddingParams {
    All(i32),
    VerticalHorizontal(i32, i32), // first is vertical (top/bottom), second is horizontal (left/right)
    FourSides(i32, i32, i32, i32), // top, right, bottom, left
}

impl PaddingParams {
    pub fn get_values(&self) -> (i32, i32, i32, i32) {
        match self {
            PaddingParams::All(value) => (*value, *value, *value, *value),
            PaddingParams::VerticalHorizontal(vertical, horizontal) => {
                (*horizontal, *horizontal, *vertical, *vertical)
            }
            PaddingParams::FourSides(top, right, bottom, left) => (*left, *right, *top, *bottom),
        }
    }
}

impl std::fmt::Display for PaddingParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaddingParams::All(value) => write!(f, "{}", value),
            PaddingParams::VerticalHorizontal(vertical, horizontal) => {
                write!(f, "{},{}", vertical, horizontal)
            }
            PaddingParams::FourSides(top, right, bottom, left) => {
                write!(f, "{},{},{},{}", top, right, bottom, left)
            }
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
