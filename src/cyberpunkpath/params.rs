use std::str::FromStr;

use serde::{Deserialize, Serialize};

pub const H_ALIGN_LEFT: &str = "left";
pub const H_ALIGN_RIGHT: &str = "right";
pub const V_ALIGN_TOP: &str = "top";
pub const V_ALIGN_BOTTOM: &str = "bottom";

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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
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
    pub h_align: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v_align: Option<String>,
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
