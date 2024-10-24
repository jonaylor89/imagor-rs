use super::color::Color;
use core::fmt;
use libvips::VipsImage;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
pub enum HAlign {
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
    #[serde(rename = "center")]
    Center,
}

impl fmt::Display for HAlign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HAlign::Left => write!(f, "left"),
            HAlign::Right => write!(f, "right"),
            HAlign::Center => write!(f, "center"),
        }
    }
}

impl FromStr for HAlign {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "left" => Ok(HAlign::Left),
            "right" => Ok(HAlign::Right),
            _ => Err(format!("Invalid HAlign value: {}", s)),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
pub enum VAlign {
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "bottom")]
    Bottom,
    #[serde(rename = "middle")]
    Middle,
}

impl fmt::Display for VAlign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VAlign::Top => write!(f, "top"),
            VAlign::Bottom => write!(f, "bottom"),
            VAlign::Middle => write!(f, "middle"),
        }
    }
}

impl FromStr for VAlign {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "top" => Ok(VAlign::Top),
            "bottom" => Ok(VAlign::Bottom),
            _ => Err(format!("Invalid VAlign value: {}", s)),
        }
    }
}

// Newtype wrapper around f64
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct F32(pub f32);

impl fmt::Display for F32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Implement PartialEq to override NaN behavior
impl PartialEq for F32 {
    fn eq(&self, other: &Self) -> bool {
        if self.0.is_nan() && other.0.is_nan() {
            true // Treat NaN as equal to NaN
        } else {
            self.0 == other.0
        }
    }
}

// Now implement Eq, since reflexivity is guaranteed
impl Eq for F32 {}

impl FromStr for F32 {
    type Err = std::num::ParseFloatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let f = s.parse::<f32>()?;
        Ok(F32(f))
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum TrimBy {
    #[serde(rename = "top-left")]
    #[default]
    TopLeft,
    #[serde(rename = "bottom-right")]
    BottomRight,
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq)]
pub struct Params {
    #[serde(skip)]
    pub params: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    pub unsafe_: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    pub meta: bool,
    pub trim: bool,
    pub trim_by: TrimBy,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trim_tolerance: Option<F32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crop_left: Option<F32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crop_top: Option<F32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crop_right: Option<F32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crop_bottom: Option<F32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fit: Option<Fit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_left: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_top: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_right: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_bottom: Option<i32>,
    pub h_flip: bool,
    pub v_flip: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h_align: Option<HAlign>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v_align: Option<VAlign>,
    pub smart: bool,
    pub filters: Vec<Filter>,
}

#[derive(Error, Debug, Clone)]
pub enum FilterParseError {
    #[error("Unknown filter: {0}")]
    UnknownFilter(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Missing required argument")]
    MissingArgument,

    #[error("Parse error: {0}")]
    ParseError(String),
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

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub enum Fit {
    FitIn,
    Stretch,
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
pub struct WatermarkParams {
    pub image: String,
    pub x: WatermarkPosition,
    pub y: WatermarkPosition,
    pub alpha: u8,
    pub w_ratio: Option<F32>,
    pub h_ratio: Option<F32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RoundedCornerParams {
    pub rx: u32,
    pub ry: Option<u32>,
    pub color: Option<Color>,
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Angle {
    Deg0,
    Deg90,
    Deg180,
    Deg270,
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
pub struct RgbAdjustment {
    pub r: i8,
    pub g: i8,
    pub b: i8,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum UtilityFilter {
    Attachment(Option<String>),
    Expire(u64),
    Preview,
    Raw,
}
