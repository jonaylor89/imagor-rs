use std::fmt;

use libvips::{ops, VipsImage};
use serde::{Deserialize, Serialize};

macro_rules! define_colors {
    ($($name:ident => ($r:expr, $g:expr, $b:expr)),* $(,)?) => {
        #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
        pub enum NamedColor {
            $($name),*
        }

        impl NamedColor {
            pub fn to_rgb(&self) -> Color {
                match self {
                    $(Self::$name => Color::Rgb($r, $g, $b)),*
                }
            }

            pub fn from_str(s: &str) -> Option<Self> {
                match s {
                    $(
                        s if s.eq_ignore_ascii_case(stringify!($name)) => Some(Self::$name),
                    )*
                    _ => None,
                }
            }
        }

        impl std::fmt::Display for NamedColor {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(Self::$name => write!(f, "{}", stringify!($name))),*
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Color {
    Named(NamedColor),
    Hex(String),
    Rgb(u8, u8, u8),
    Auto,
    Blur,
    None,
}

impl Color {
    pub fn to_rgb(&self, img: &VipsImage) -> Option<(u8, u8, u8)> {
        match self {
            Color::Named(named) => {
                let Color::Rgb(r, g, b) = named.to_rgb() else {
                    unreachable!()
                };
                Some((r, g, b))
            }
            Color::Rgb(r, g, b) => Some((*r, *g, *b)),
            Color::Hex(hex) => {
                let hex = hex.trim_start_matches('#');
                if hex.len() == 6 {
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        u8::from_str_radix(&hex[0..2], 16),
                        u8::from_str_radix(&hex[2..4], 16),
                        u8::from_str_radix(&hex[4..6], 16),
                    ) {
                        return Some((r, g, b));
                    }
                }
                None
            }
            Color::Auto => {
                let point = ops::getpoint(img, 0, 0).ok().map(|p| {
                    if p.len() >= 3 {
                        (p[0] as u8, p[1] as u8, p[2] as u8)
                    } else {
                        (0, 0, 0)
                    }
                });

                point
            }
            _ => None,
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Color::Named(name) => write!(f, "{}", name),
            Color::Hex(hex) => write!(f, "{}", hex),
            Color::Rgb(r, g, b) => write!(f, "{},{},{}", r, g, b),
            Color::Auto => write!(f, "auto"),
            Color::Blur => write!(f, "blur"),
            Color::None => write!(f, "none"),
        }
    }
}

define_colors! {
    AliceBlue => (0xf0, 0xf8, 0xff),
    AntiqueWhite => (0xfa, 0xeb, 0xd7),
    Aqua => (0x00, 0xff, 0xff),
    Aquamarine => (0x7f, 0xff, 0xd4),
    Azure => (0xf0, 0xff, 0xff),
    Beige => (0xf5, 0xf5, 0xdc),
    Bisque => (0xff, 0xe4, 0xc4),
    Black => (0x00, 0x00, 0x00),
    BlanchedAlmond => (0xff, 0xeb, 0xcd),
    Blue => (0x00, 0x00, 0xff),
    BlueViolet => (0x8a, 0x2b, 0xe2),
    Brown => (0xa5, 0x2a, 0x2a),
    BurlyWood => (0xde, 0xb8, 0x87),
    CadetBlue => (0x5f, 0x9e, 0xa0),
    Chartreuse => (0x7f, 0xff, 0x00),
    Chocolate => (0xd2, 0x69, 0x1e),
    Coral => (0xff, 0x7f, 0x50),
    CornflowerBlue => (0x64, 0x95, 0xed),
    Cornsilk => (0xff, 0xf8, 0xdc),
    Crimson => (0xdc, 0x14, 0x3c),
    Cyan => (0x00, 0xff, 0xff),
    DarkBlue => (0x00, 0x00, 0x8b),
    DarkCyan => (0x00, 0x8b, 0x8b),
    DarkGoldenRod => (0xb8, 0x86, 0x0b),
    DarkGray => (0xa9, 0xa9, 0xa9),
    DarkGreen => (0x00, 0x64, 0x00),
    DarkKhaki => (0xbd, 0xb7, 0x6b),
    DarkMagenta => (0x8b, 0x00, 0x8b),
    DarkOliveGreen => (0x55, 0x6b, 0x2f),
    DarkOrange => (0xff, 0x8c, 0x00),
    DarkOrchid => (0x99, 0x32, 0xcc),
    DarkRed => (0x8b, 0x00, 0x00),
    DarkSalmon => (0xe9, 0x96, 0x7a),
    DarkSeaGreen => (0x8f, 0xbc, 0x8f),
    DarkSlateBlue => (0x48, 0x3d, 0x8b),
    DarkSlateGray => (0x2f, 0x4f, 0x4f),
    DarkTurquoise => (0x00, 0xce, 0xd1),
    DarkViolet => (0x94, 0x00, 0xd3),
    DeepPink => (0xff, 0x14, 0x93),
    DeepSkyBlue => (0x00, 0xbf, 0xff),
    DimGray => (0x69, 0x69, 0x69),
    DodgerBlue => (0x1e, 0x90, 0xff),
    FireBrick => (0xb2, 0x22, 0x22),
    FloralWhite => (0xff, 0xfa, 0xf0),
    ForestGreen => (0x22, 0x8b, 0x22),
    Fuchsia => (0xff, 0x00, 0xff),
    Gainsboro => (0xdc, 0xdc, 0xdc),
    GhostWhite => (0xf8, 0xf8, 0xff),
    Gold => (0xff, 0xd7, 0x00),
    GoldenRod => (0xda, 0xa5, 0x20),
    Gray => (0x80, 0x80, 0x80),
    Green => (0x00, 0x80, 0x00),
    GreenYellow => (0xad, 0xff, 0x2f),
    HoneyDew => (0xf0, 0xff, 0xf0),
    HotPink => (0xff, 0x69, 0xb4),
    IndianRed => (0xcd, 0x5c, 0x5c),
    Indigo => (0x4b, 0x00, 0x82),
    Ivory => (0xff, 0xff, 0xf0),
    Khaki => (0xf0, 0xe6, 0x8c),
    Lavender => (0xe6, 0xe6, 0xfa),
    LavenderBlush => (0xff, 0xf0, 0xf5),
    LawnGreen => (0x7c, 0xfc, 0x00),
    LemonChiffon => (0xff, 0xfa, 0xcd),
    LightBlue => (0xad, 0xd8, 0xe6),
    LightCoral => (0xf0, 0x80, 0x80),
    LightCyan => (0xe0, 0xff, 0xff),
    LightGoldenRodYellow => (0xfa, 0xfa, 0xd2),
    LightGray => (0xd3, 0xd3, 0xd3),
    LightGreen => (0x90, 0xee, 0x90),
    LightPink => (0xff, 0xb6, 0xc1),
    LightSalmon => (0xff, 0xa0, 0x7a),
    LightSeaGreen => (0x20, 0xb2, 0xaa),
    LightSkyBlue => (0x87, 0xce, 0xfa),
    LightSlateGray => (0x77, 0x88, 0x99),
    LightSteelBlue => (0xb0, 0xc4, 0xde),
    LightYellow => (0xff, 0xff, 0xe0),
    Lime => (0x00, 0xff, 0x00),
    LimeGreen => (0x32, 0xcd, 0x32),
    Linen => (0xfa, 0xf0, 0xe6),
    Magenta => (0xff, 0x00, 0xff),
    Maroon => (0x80, 0x00, 0x00),
    MediumAquaMarine => (0x66, 0xcd, 0xaa),
    MediumBlue => (0x00, 0x00, 0xcd),
    MediumOrchid => (0xba, 0x55, 0xd3),
    MediumPurple => (0x93, 0x70, 0xdb),
    MediumSeaGreen => (0x3c, 0xb3, 0x71),
    MediumSlateBlue => (0x7b, 0x68, 0xee),
    MediumSpringGreen => (0x00, 0xfa, 0x9a),
    MediumTurquoise => (0x48, 0xd1, 0xcc),
    MediumVioletRed => (0xc7, 0x15, 0x85),
    MidnightBlue => (0x19, 0x19, 0x70),
    MintCream => (0xf5, 0xff, 0xfa),
    MistyRose => (0xff, 0xe4, 0xe1),
    Moccasin => (0xff, 0xe4, 0xb5),
    NavajoWhite => (0xff, 0xde, 0xad),
    Navy => (0x00, 0x00, 0x80),
    OldLace => (0xfd, 0xf5, 0xe6),
    Olive => (0x80, 0x80, 0x00),
    OliveDrab => (0x6b, 0x8e, 0x23),
    Orange => (0xff, 0xa5, 0x00),
    OrangeRed => (0xff, 0x45, 0x00),
    Orchid => (0xda, 0x70, 0xd6),
    PaleGoldenRod => (0xee, 0xe8, 0xaa),
    PaleGreen => (0x98, 0xfb, 0x98),
    PaleTurquoise => (0xaf, 0xee, 0xee),
    PaleVioletRed => (0xdb, 0x70, 0x93),
    PapayaWhip => (0xff, 0xef, 0xd5),
    PeachPuff => (0xff, 0xda, 0xb9),
    Peru => (0xcd, 0x85, 0x3f),
    Pink => (0xff, 0xc0, 0xcb),
    Plum => (0xdd, 0xa0, 0xdd),
    PowderBlue => (0xb0, 0xe0, 0xe6),
    Purple => (0x80, 0x00, 0x80),
    RebeccaPurple => (0x66, 0x33, 0x99),
    Red => (0xff, 0x00, 0x00),
    RosyBrown => (0xbc, 0x8f, 0x8f),
    RoyalBlue => (0x41, 0x69, 0xe1),
    SaddleBrown => (0x8b, 0x45, 0x13),
    Salmon => (0xfa, 0x80, 0x72),
    SandyBrown => (0xf4, 0xa4, 0x60),
    SeaGreen => (0x2e, 0x8b, 0x57),
    SeaShell => (0xff, 0xf5, 0xee),
    Sienna => (0xa0, 0x52, 0x2d),
    Silver => (0xc0, 0xc0, 0xc0),
    SkyBlue => (0x87, 0xce, 0xeb),
    SlateBlue => (0x6a, 0x5a, 0xcd),
    SlateGray => (0x70, 0x80, 0x90),
    Snow => (0xff, 0xfa, 0xfa),
    SpringGreen => (0x00, 0xff, 0x7f),
    SteelBlue => (0x46, 0x82, 0xb4),
    Tan => (0xd2, 0xb4, 0x8c),
    Teal => (0x00, 0x80, 0x80),
    Thistle => (0xd8, 0xbf, 0xd8),
    Tomato => (0xff, 0x63, 0x47),
    Turquoise => (0x40, 0xe0, 0xd0),
    Violet => (0xee, 0x82, 0xee),
    Wheat => (0xf5, 0xde, 0xb3),
    White => (0xff, 0xff, 0xff),
    WhiteSmoke => (0xf5, 0xf5, 0xf5),
    Yellow => (0xff, 0xff, 0x00),
    YellowGreen => (0x9a, 0xcd, 0x32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_from_str() {
        assert_eq!(Color::from_str("red"), Color::Named(NamedColor::Red));
        assert_eq!(
            Color::from_str("#ff0000"),
            Color::Hex("#ff0000".to_string())
        );
        assert_eq!(Color::from_str("255,0,0"), Color::Rgb(255, 0, 0));
        assert_eq!(Color::from_str("auto"), Color::Auto);
        assert_eq!(Color::from_str("blur"), Color::Blur);
        assert_eq!(Color::from_str("none"), Color::None);
    }

    #[test]
    fn test_color_to_rgb() {
        assert_eq!(Color::Named(NamedColor::Red).to_rgb(), Some((255, 0, 0)));
        assert_eq!(
            Color::Hex("#ff0000".to_string()).to_rgb(),
            Some((255, 0, 0))
        );
        assert_eq!(Color::Rgb(255, 0, 0).to_rgb(), Some((255, 0, 0)));
        assert_eq!(Color::Auto.to_rgb(), None);
        assert_eq!(Color::Blur.to_rgb(), None);
        assert_eq!(Color::None.to_rgb(), None);
    }
}
