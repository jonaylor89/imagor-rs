use core::fmt;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

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
    pub fit_in: bool,
    pub stretch: bool,
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

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct Filter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
}

impl Filter {
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.args.is_none()
    }
}
