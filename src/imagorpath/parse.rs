use super::params::{
    Color, Filter, Fit, FocalParams, HAlign, ImageType, LabelParams, LabelPosition, Params,
    RoundedCornerParams, TrimBy, VAlign, WatermarkParams, WatermarkPosition, F32,
};
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use color_eyre::Result;
use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_while1, take_while_m_n},
    character::complete::{alphanumeric1, char, digit1},
    combinator::{map, opt, recognize, value},
    error::{context, ErrorKind, VerboseError, VerboseErrorKind},
    multi::{many1, separated_list0, separated_list1},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    AsChar, IResult,
};
use tracing::info;

#[derive(Debug)]
pub struct CyberpunkPath {
    pub path: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for Params
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    #[tracing::instrument(skip(parts, _state))]
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Access the URI and perform your custom parsing logic
        let uri = &parts.uri;
        let path = uri.path();

        info!("Parsing path: {}", path);

        // TODO: check auth of imagorpath

        let (_, params) = parse_path(path).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Failed to parse params: {}", e),
            )
        })?;

        Ok(params)
    }
}

impl TryFrom<&str> for Params {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let (_, path) = parse_path(value).map_err(|e| format!("Failed to parse path: {}", e))?;
        Ok(path)
    }
}

fn parse_unsafe(input: &str) -> IResult<&str, bool, VerboseError<&str>> {
    value(false, tag("unsafe/"))(input)
}

fn parse_meta(input: &str) -> IResult<&str, bool, VerboseError<&str>> {
    value(true, tag("meta/"))(input)
}

fn parse_trim(
    input: &str,
) -> IResult<&str, (bool, Option<TrimBy>, Option<F32>), VerboseError<&str>> {
    terminated(
        opt(tuple((
            value(true, tag("trim")),
            opt(preceded(
                char(':'),
                alt((
                    value(TrimBy::TopLeft, tag("top-left")),
                    value(TrimBy::BottomRight, tag("bottom-right")),
                )),
            )),
            opt(preceded(
                char(':'),
                map(digit1, |s: &str| F32(s.parse().unwrap())),
            )),
        ))),
        char('/'),
    )(input)
    .map(|(next_input, result)| (next_input, result.unwrap_or((false, None, None))))
}

fn parse_crop(input: &str) -> IResult<&str, (F32, F32, F32, F32), VerboseError<&str>> {
    terminated(
        separated_pair(
            separated_pair(parse_f32, char('x'), parse_f32),
            char(':'),
            separated_pair(parse_f32, char('x'), parse_f32),
        ),
        char('/'),
    )(input)
    .map(|(next_input, ((left, top), (right, bottom)))| (next_input, (left, top, right, bottom)))
}

fn parse_f32(input: &str) -> IResult<&str, F32, VerboseError<&str>> {
    map(
        recognize(tuple((
            opt(char('-')),
            digit1,
            opt(preceded(char('.'), digit1)),
        ))),
        |s: &str| F32(s.parse().unwrap()),
    )(input)
}

fn parse_dimensions(
    input: &str,
) -> IResult<&str, (Option<i32>, Option<i32>, bool, bool), VerboseError<&str>> {
    terminated(
        tuple((
            map(opt(recognize(pair(opt(char('-')), digit1))), |d| {
                d.map(|s: &str| s.parse::<i32>().unwrap())
            }),
            preceded(
                char('x'),
                map(opt(recognize(pair(opt(char('-')), digit1))), |d| {
                    d.map(|s: &str| s.parse::<i32>().unwrap())
                }),
            ),
        )),
        char('/'),
    )(input)
    .map(|(next_input, (width, height))| {
        (
            next_input,
            (
                width.map(|w| w.abs()),
                height.map(|h| h.abs()),
                width.map_or(false, |w| w < 0),
                height.map_or(false, |h| h < 0),
            ),
        )
    })
}

fn parse_fit(input: &str) -> IResult<&str, Option<Fit>, VerboseError<&str>> {
    let (input, fit) = opt(alt((
        value(Fit::FitIn, tag("fit-in/")),
        value(Fit::Stretch, tag("stretch/")),
    )))(input)?;

    // Check if both fit-in and stretch are present
    let (input, both_present) = opt(pair(tag("fit-in/"), tag("stretch/")))(input)?;

    match (fit, both_present) {
        (Some(_), Some(_)) => Ok((input, Some(Fit::FitIn))), // Default to FitIn if both are present
        (Some(f), None) => Ok((input, Some(f))),
        (None, Some(_)) => Ok((input, Some(Fit::FitIn))), // Default to FitIn if both are present
        (None, None) => Ok((input, None)),
    }
}

fn parse_alignment(
    input: &str,
) -> IResult<&str, (Option<HAlign>, Option<VAlign>), VerboseError<&str>> {
    tuple((
        opt(terminated(
            alt((
                value(HAlign::Left, tag("left")),
                value(HAlign::Right, tag("right")),
                value(HAlign::Center, tag("center")),
            )),
            char('/'),
        )),
        opt(terminated(
            alt((
                value(VAlign::Top, tag("top")),
                value(VAlign::Bottom, tag("bottom")),
                value(VAlign::Middle, tag("middle")),
            )),
            char('/'),
        )),
    ))(input)
}

fn parse_smart(input: &str) -> IResult<&str, bool, VerboseError<&str>> {
    value(true, tag("smart/"))(input)
}

fn take_until_unbalanced(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    let mut depth = 0;
    let mut chars = input.char_indices().peekable();

    for (idx, ch) in chars {
        match ch {
            '(' => depth += 1,
            ')' => {
                if depth == 0 {
                    return Ok((&input[idx..], &input[..idx]));
                }
                depth -= 1;
            }
            _ => {}
        }
    }

    Err(nom::Err::Error(VerboseError {
        errors: vec![(input, VerboseErrorKind::Nom(ErrorKind::TakeUntil))],
    }))
}

fn parse_color(input: &str) -> IResult<&str, Color, VerboseError<&str>> {
    alt((
        map(tag_no_case("auto"), |_| Color::Auto),
        map(tag_no_case("blur"), |_| Color::Blur),
        map(tag_no_case("none"), |_| Color::None),
        map(
            tuple((
                nom::character::complete::u8,
                char(','),
                nom::character::complete::u8,
                char(','),
                nom::character::complete::u8,
            )),
            |(r, _, g, _, b)| Color::Rgb(r, g, b),
        ),
        map(
            preceded(char('#'), take_while_m_n(6, 6, |c: char| c.is_hex_digit())),
            |hex: &str| Color::Hex(hex.to_string()),
        ),
        map(
            take_while1(|c: char| c.is_alphabetic() || c == '_'),
            |name: &str| Color::Named(name.to_string()),
        ),
    ))(input)
}

fn parse_focal_point(input: &str) -> IResult<&str, FocalParams, VerboseError<&str>> {
    alt((
        // Parse Region
        map(
            tuple((
                parse_f32,
                char('x'),
                parse_f32,
                char(':'),
                parse_f32,
                char('x'),
                parse_f32,
            )),
            |(left, _, top, _, right, _, bottom)| FocalParams::Region {
                top_left: (left, top),
                bottom_right: (right, bottom),
            },
        ),
        // Parse Point
        map(tuple((parse_f32, char('x'), parse_f32)), |(x, _, y)| {
            FocalParams::Point(x, y)
        }),
    ))(input)
}

fn parse_filter(input: &str) -> IResult<&str, Filter, VerboseError<&str>> {
    let (input, name) = take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)?;
    let (input, args) = opt(delimited(char('('), take_while1(|c| c != ')'), char(')')))(input)?;

    match name.to_lowercase().as_str() {
        "backgroundcolor" => {
            let (_, color) = parse_color(args.unwrap_or(""))?;
            Ok((input, Filter::BackgroundColor(color)))
        }
        "blur" => map(parse_f32, Filter::Blur)(args.unwrap_or("")),
        "brightness" => map(nom::character::complete::i32, Filter::Brightness)(args.unwrap_or("")),
        "contrast" => map(nom::character::complete::i32, Filter::Contrast)(args.unwrap_or("")),
        "fill" => {
            let (_, color) = parse_color(args.unwrap_or(""))?;
            Ok((input, Filter::Fill(color)))
        }
        "focal" => {
            let (_, focal_point) = parse_focal_point(args.unwrap_or(""))?;
            Ok((input, Filter::Focal(focal_point)))
        }
        "format" => {
            let image_type = match args.unwrap_or("").to_uppercase().as_str() {
                "GIF" => ImageType::GIF,
                "jpeg" => ImageType::JPEG,
                "png" => ImageType::PNG,
                "magick" => ImageType::MAGICK,
                "pdf" => ImageType::PDF,
                "svg" => ImageType::SVG,
                "tiff" => ImageType::TIFF,
                "webp" => ImageType::WEBP,
                "heif" => ImageType::HEIF,
                "bmp" => ImageType::BMP,
                "avif" => ImageType::AVIF,
                "jp2k" => ImageType::JP2K,
                _ => {
                    return Err(nom::Err::Error(VerboseError {
                        errors: vec![(input, VerboseErrorKind::Context("Unknown image format"))],
                    }))
                }
            };
            Ok((input, Filter::Format(image_type)))
        }
        "grayscale" => Ok((input, Filter::Grayscale)),
        "hue" => map(nom::character::complete::i32, Filter::Hue)(args.unwrap_or("")),
        "label" => map(parse_label_params, Filter::Label)(args.unwrap_or("")),
        "maxbytes" => map(nom::character::complete::u64, |v| {
            Filter::MaxBytes(v as usize)
        })(args.unwrap_or("")),
        "maxframes" => map(nom::character::complete::u64, |v| {
            Filter::MaxFrames(v as usize)
        })(args.unwrap_or("")),
        "orient" => map(nom::character::complete::i32, Filter::Orient)(args.unwrap_or("")),
        "page" => {
            map(nom::character::complete::u64, |v| Filter::Page(v as usize))(args.unwrap_or(""))
        }
        "dpi" => map(nom::character::complete::u32, Filter::Dpi)(args.unwrap_or("")),
        "proportion" => map(parse_f32, Filter::Proportion)(args.unwrap_or("")),
        "quality" => map(nom::character::complete::u8, Filter::Quality)(args.unwrap_or("")),
        "rgb" => map(parse_rgb, |(r, g, b)| Filter::Rgb(r, g, b))(args.unwrap_or("")),
        "rotate" => map(nom::character::complete::i32, Filter::Rotate)(args.unwrap_or("")),
        "roundcorner" => map(parse_rounded_corner_params, Filter::RoundCorner)(args.unwrap_or("")),
        "saturation" => map(nom::character::complete::i32, Filter::Saturation)(args.unwrap_or("")),
        "sharpen" => map(parse_f32, Filter::Sharpen)(args.unwrap_or("")),
        "stripexif" => Ok((input, Filter::StripExif)),
        "stripicc" => Ok((input, Filter::StripIcc)),
        "stripmetadata" => Ok((input, Filter::StripMetadata)),
        "upscale" => Ok((input, Filter::Upscale)),
        "watermark" => map(parse_watermark_params, Filter::Watermark)(args.unwrap_or("")),
        _ => Err(nom::Err::Error(VerboseError {
            errors: vec![(input, VerboseErrorKind::Context("Unknown filter"))],
        })),
    }
}

fn parse_filters(input: &str) -> IResult<&str, Vec<Filter>, VerboseError<&str>> {
    preceded(
        tag("filters:"),
        terminated(separated_list0(char(':'), parse_filter), opt(char('/'))),
    )(input)
}

fn parse_rgb(input: &str) -> IResult<&str, (i32, i32, i32), VerboseError<&str>> {
    let (input, rgb) = separated_list1(char(','), nom::character::complete::i32)(input)?;
    if rgb.len() != 3 {
        Err(nom::Err::Error(VerboseError {
            errors: vec![(input, VerboseErrorKind::Context("RGB requires 3 values"))],
        }))
    } else {
        Ok((input, (rgb[0], rgb[1], rgb[2])))
    }
}

fn parse_label_params(input: &str) -> IResult<&str, LabelParams, VerboseError<&str>> {
    let (input, (text, x, y, size, color, alpha, font)) = tuple((
        take_while1(|c| c != ','),
        preceded(char(','), parse_label_position),
        preceded(char(','), parse_label_position),
        preceded(char(','), nom::character::complete::u32),
        preceded(char(','), parse_color),
        opt(preceded(char(','), nom::character::complete::u8)),
        opt(preceded(char(','), take_while1(|c| c != ','))),
    ))(input)?;

    Ok((
        input,
        LabelParams {
            text: text.to_string(),
            x,
            y,
            size,
            color,
            alpha,
            font: font.map(|s| s.to_string()),
        },
    ))
}

fn parse_label_position(input: &str) -> IResult<&str, LabelPosition, VerboseError<&str>> {
    alt((
        value(LabelPosition::Left, tag("left")),
        value(LabelPosition::Right, tag("right")),
        value(LabelPosition::Center, tag("center")),
        value(LabelPosition::Top, tag("top")),
        value(LabelPosition::Bottom, tag("bottom")),
        map(nom::character::complete::i32, LabelPosition::Pixels),
        map(parse_f32, LabelPosition::Percentage),
    ))(input)
}

// fn parse_color(input: &str) -> IResult<&str, Color, VerboseError<&str>> {
//     alt((
//         map(
//             preceded(tag("rgb("), terminated(parse_rgb, char(')'))),
//             |(r, g, b)| Color::Rgb(r as u8, g as u8, b as u8),
//         ),
//         map(
//             preceded(tag("#"), take_while1(|c: char| c.is_ascii_hexdigit())),
//             |hex: &str| Color::Hex(hex.to_string()),
//         ),
//         value(Color::Auto, tag("auto")),
//         value(Color::Blur, tag("blur")),
//         value(Color::None, tag("none")),
//         map(
//             take_while1(|c: char| c.is_alphanumeric() || c == '_'),
//             |name: &str| Color::Named(name.to_string()),
//         ),
//     ))(input)
// }

fn parse_rounded_corner_params(
    input: &str,
) -> IResult<&str, RoundedCornerParams, VerboseError<&str>> {
    let (input, (rx, ry, color)) = tuple((
        nom::character::complete::u32,
        opt(preceded(char(','), nom::character::complete::u32)),
        opt(preceded(char(','), parse_color)),
    ))(input)?;

    Ok((input, RoundedCornerParams { rx, ry, color }))
}

fn parse_watermark_params(input: &str) -> IResult<&str, WatermarkParams, VerboseError<&str>> {
    let (input, (image, x, y, alpha, w_ratio, h_ratio)) = tuple((
        take_while1(|c| c != ','),
        preceded(char(','), parse_watermark_position),
        preceded(char(','), parse_watermark_position),
        preceded(char(','), nom::character::complete::u8),
        opt(preceded(char(','), parse_f32)),
        opt(preceded(char(','), parse_f32)),
    ))(input)?;

    Ok((
        input,
        WatermarkParams {
            image: image.to_string(),
            x,
            y,
            alpha,
            w_ratio,
            h_ratio,
        },
    ))
}

fn parse_watermark_position(input: &str) -> IResult<&str, WatermarkPosition, VerboseError<&str>> {
    alt((
        value(WatermarkPosition::Left, tag("left")),
        value(WatermarkPosition::Right, tag("right")),
        value(WatermarkPosition::Center, tag("center")),
        value(WatermarkPosition::Top, tag("top")),
        value(WatermarkPosition::Bottom, tag("bottom")),
        value(WatermarkPosition::Repeat, tag("repeat")),
        map(nom::character::complete::i32, WatermarkPosition::Pixels),
        map(parse_f32, WatermarkPosition::Percentage),
    ))(input)
}

fn parse_image(input: &str) -> IResult<&str, String, VerboseError<&str>> {
    recognize(many1(alt((
        alphanumeric1,
        tag("."),
        tag("-"),
        tag("_"),
        tag("/"),
        tag(":"),
    ))))(input)
    .map(|(next_input, result)| (next_input, result.to_string()))
}

#[tracing::instrument]
pub fn parse_path(input: &str) -> IResult<&str, Params, VerboseError<&str>> {
    context(
        "parse_path",
        map(
            tuple((
                opt(char('/')),
                context("parse_unsafe", opt(parse_unsafe)),
                context("parse_meta", opt(parse_meta)),
                context("parse_trim", opt(parse_trim)),
                context("parse_crop", opt(parse_crop)),
                context("parse_fit", opt(parse_fit)),
                context("parse_dimensions", opt(parse_dimensions)),
                context("parse_alignment", opt(parse_alignment)),
                context("parse_smart", opt(parse_smart)),
                context("parse_filters", opt(parse_filters)),
                context("parse_image", opt(parse_image)),
            )),
            |(
                _,
                unsafe_,
                meta,
                trim_details,
                crop,
                fit,
                dimensions,
                alignment,
                smart,
                filters,
                image,
            )| {
                Params {
                    unsafe_: unsafe_.unwrap_or_default(),
                    path: Some(input.to_string()),
                    image,
                    trim: trim_details.as_ref().map(|t| t.0).unwrap_or_default(),
                    trim_by: trim_details.as_ref().and_then(|t| t.1).unwrap_or_default(),
                    trim_tolerance: trim_details.as_ref().and_then(|t| t.2),
                    crop_left: crop.map(|(left, _, _, _)| left),
                    crop_top: crop.map(|(_, top, _, _)| top),
                    crop_right: crop.map(|(_, _, right, _)| right),
                    crop_bottom: crop.map(|(_, _, _, bottom)| bottom),
                    width: dimensions.and_then(|(width, _, _, _)| width),
                    height: dimensions.and_then(|(_, height, _, _)| height),
                    meta: meta.unwrap_or_default(),
                    h_flip: dimensions
                        .map(|(_, _, h_flip, _)| h_flip)
                        .unwrap_or_default(),
                    v_flip: dimensions
                        .map(|(_, _, _, v_flip)| v_flip)
                        .unwrap_or_default(),
                    h_align: alignment
                        .as_ref()
                        .and_then(|(h_align, _)| h_align.to_owned()),
                    v_align: alignment
                        .as_ref()
                        .and_then(|(_, v_align)| v_align.to_owned()),
                    smart: smart.unwrap_or_default(),
                    fit: fit.unwrap_or_default(),
                    filters: filters.unwrap_or_default(),
                    ..Default::default()
                }
            },
        ),
    )(input)
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::imagorpath::params::{Fit, HAlign, TrimBy, VAlign};
    use nom::error::convert_error;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_parse_generate_non_url_image() {
        let uri = "meta/trim/10x11:12x13/fit-in/-300x-200/left/top/smart/filters:grayscale()/img";
        let expected_params = Params {
            path: Some(
                "meta/trim/10x11:12x13/fit-in/-300x-200/left/top/smart/filters:grayscale()/img"
                    .to_string(),
            ),
            image: Some("img".to_string()),
            trim: true,
            trim_by: TrimBy::TopLeft,
            crop_left: Some(F32(10.0)),
            crop_top: Some(F32(11.0)),
            crop_right: Some(F32(12.0)),
            crop_bottom: Some(F32(13.0)),
            width: Some(300),
            height: Some(200),
            meta: true,
            h_flip: true,
            v_flip: true,
            h_align: Some(HAlign::Left),
            v_align: Some(VAlign::Top),
            smart: true,
            fit: Some(Fit::FitIn),
            filters: vec![Filter::Grayscale],
            ..Default::default()
        };

        let parser_results = parse_path(uri);
        match parser_results {
            Ok((_, result)) => {
                assert_eq!(result, expected_params, "Failed test: non url image");
            }
            Err(nom::Err::Error(e) | nom::Err::Failure(e)) => {
                println!("Parser error:");
                println!("{}", convert_error(uri, e));
                panic!("Parser failed");
            }
            Err(nom::Err::Incomplete(_)) => {
                println!("Parser error: Incomplete input");
                panic!("Parser failed");
            }
        }
    }

    #[test]
    fn test_parse_generate_real_example() {
        let uri = "unsafe/30x40:100x150/filters:fill(cyan)/raw.githubusercontent.com/cshum/imagor/master/testdata/dancing-banana.gif";
        let expected_params = Params {
            path: Some("unsafe/30x40:100x150/filters:fill(cyan)/raw.githubusercontent.com/cshum/imagor/master/testdata/dancing-banana.gif".to_string()),
            image: Some("raw.githubusercontent.com/cshum/imagor/master/testdata/dancing-banana.gif".to_string()),
            trim: false,
            trim_by: TrimBy::TopLeft,
            crop_left: Some(F32(30.0)),
            crop_top: Some(F32(40.0)),
            crop_right: Some(F32(100.0)),
            crop_bottom: Some(F32(150.0)),
            filters: vec![
                Filter::Fill(Color::Named("cyan".to_string()))
            ],
            ..Default::default()
        };

        let parser_results = parse_path(uri);
        match parser_results {
            Ok((_, result)) => {
                assert_eq!(result, expected_params, "Failed test: real image");
            }
            Err(nom::Err::Error(e) | nom::Err::Failure(e)) => {
                println!("Parser error:");
                println!("{}", convert_error(uri, e));
                panic!("Parser failed");
            }
            Err(nom::Err::Incomplete(_)) => {
                println!("Parser error: Incomplete input");
                panic!("Parser failed");
            }
        }
    }

    #[test]
    fn test_parse_generate_url_image() {
        let uri = "meta/trim:bottom-right:100/10x11:12x13/fit-in/-300x-200/left/top/smart/filters:grayscale()/s.glbimg.com/es/ge/f/original/2011/03/29/orlandosilva_60.jpg";
        let expected_params = Params {
            path: Some("meta/trim:bottom-right:100/10x11:12x13/fit-in/-300x-200/left/top/smart/filters:grayscale()/s.glbimg.com/es/ge/f/original/2011/03/29/orlandosilva_60.jpg".to_string()),
            image: Some("s.glbimg.com/es/ge/f/original/2011/03/29/orlandosilva_60.jpg".to_string()),
            trim: true,
            trim_by: TrimBy::BottomRight,
            trim_tolerance: Some(F32(100.0)),
            crop_left: Some(F32(10.0)),
            crop_top: Some(F32(11.0)),
            crop_right: Some(F32(12.0)),
            crop_bottom: Some(F32(13.0)),
            width: Some(300),
            height: Some(200),
            meta: true,
            h_flip: true,
            v_flip: true,
            h_align: Some(HAlign::Left),
            v_align: Some(VAlign::Top),
            smart: true,
            fit: Some(Fit::FitIn),
            filters: vec![
                Filter::Grayscale,
            ],
            ..Default::default()
        };

        let parser_results = parse_path(uri);
        match parser_results {
            Ok((_, result)) => {
                assert_eq!(result, expected_params, "Failed test: url image");
            }
            Err(nom::Err::Error(e) | nom::Err::Failure(e)) => {
                println!("Parser error:");
                println!("{}", convert_error(uri, e));
                panic!("Parser failed");
            }
            Err(nom::Err::Incomplete(_)) => {
                println!("Parser error: Incomplete input");
                panic!("Parser failed");
            }
        }
    }

    #[test]
    fn test_parse_filters_with_image() {
        let input = "filters:watermark(s.glbimg.com/filters:label(abc):watermark(aaa.com/fit-in/filters:aaa(bbb))/aaa.jpg,0,0,0):brightness(-50):grayscale()/some/example/img";
        let expected = (
            "some/example/img",
            vec![
                Filter::Watermark(WatermarkParams {
                    image: "s.glbimg.com/filters:label(abc):watermark(aaa.com/fit-in/filters:aaa(bbb))/aaa.jpg".to_string(),
                    x: WatermarkPosition::Pixels(0),
                    y: WatermarkPosition::Pixels(0),
                    alpha: 0,
                    w_ratio: None,
                    h_ratio: None,
                }),
                Filter::Brightness(-50),
                Filter::Grayscale,
            ],
        );
        let result = parse_filters(input).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_filters_without_image() {
        let input = "filters:watermark(s.glbimg.com/filters:label(abc):watermark(aaa.com/fit-in/filters:aaa(bbb))/aaa.jpg,0,0,0):brightness(-50):grayscale()";
        let expected = (
            "",
            vec![
                Filter::Watermark(
                    WatermarkParams {
                        image: "s.glbimg.com/filters:label(abc):watermark(aaa.com/fit-in/filters:aaa(bbb))/aaa.jpg".to_string(),
                        x: WatermarkPosition::Pixels(0),
                        y: WatermarkPosition::Pixels(0),
                        alpha: 0,
                        w_ratio: None,
                        h_ratio: None,
                    },
                ),
                Filter::Brightness(-50),
                Filter::Grayscale,
            ],
        );
        let result = parse_filters(input).unwrap();
        assert_eq!(result, expected);
    }
}
