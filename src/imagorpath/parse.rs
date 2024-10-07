use super::params::{Filter, HAlign, Params, TrimBy, VAlign, F32};
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use color_eyre::Result;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{alphanumeric1, char, digit1},
    combinator::{map, opt, recognize, success, value},
    error::{context, ErrorKind, VerboseError, VerboseErrorKind},
    multi::{many1, separated_list0},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    IResult,
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

        let (_, params) = parse_path(&path).map_err(|e| {
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

fn parse_fit_in(input: &str) -> IResult<&str, bool, VerboseError<&str>> {
    value(true, tag("fit-in/"))(input)
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

    while let Some((idx, ch)) = chars.next() {
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

fn parse_filters(input: &str) -> IResult<&str, Vec<Filter>, VerboseError<&str>> {
    alt((
        preceded(
            tag("filters:"),
            terminated(
                separated_list0(
                    char(':'),
                    alt((
                        map(
                            tuple((
                                take_while1(|c: char| c.is_alphanumeric() || c == '_'),
                                delimited(char('('), take_until_unbalanced, char(')')),
                            )),
                            |(name, args)| Filter {
                                name: Some(name.to_string()),
                                args: if args.is_empty() {
                                    None
                                } else {
                                    Some(args.to_string())
                                },
                            },
                        ),
                        map(
                            take_while1(|c: char| c.is_alphanumeric() || c == '_'),
                            |name: &str| Filter {
                                name: Some(name.to_string()),
                                args: None,
                            },
                        ),
                    )),
                ),
                opt(char('/')),
            ),
        ),
        value(vec![], success(())),
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
                context("parse_fit_in", opt(parse_fit_in)),
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
                fit_in,
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
                    fit_in: fit_in.unwrap_or_default(),
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
    use crate::imagorpath::params::{HAlign, TrimBy, VAlign};
    use nom::error::convert_error;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_parse_generate_non_url_image() {
        let uri = "meta/trim/10x11:12x13/fit-in/-300x-200/left/top/smart/filters:some_filter()/img";
        let expected_params = Params {
            path: Some(
                "meta/trim/10x11:12x13/fit-in/-300x-200/left/top/smart/filters:some_filter()/img"
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
            fit_in: true,
            filters: vec![Filter {
                name: Some("some_filter".to_string()),
                args: None,
            }],
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
            filters: vec![Filter { name: Some("fill".to_string()), args: Some("cyan".to_string()) }],
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
        let uri = "meta/trim:bottom-right:100/10x11:12x13/fit-in/-300x-200/left/top/smart/filters:some_filter()/s.glbimg.com/es/ge/f/original/2011/03/29/orlandosilva_60.jpg";
        let expected_params = Params {
            path: Some("meta/trim:bottom-right:100/10x11:12x13/fit-in/-300x-200/left/top/smart/filters:some_filter()/s.glbimg.com/es/ge/f/original/2011/03/29/orlandosilva_60.jpg".to_string()),
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
            fit_in: true,
            filters: vec![Filter { name: Some("some_filter".to_string()), args: None }],
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
                Filter { name: Some("watermark".to_string()), args: Some("s.glbimg.com/filters:label(abc):watermark(aaa.com/fit-in/filters:aaa(bbb))/aaa.jpg,0,0,0".to_string()) },
                Filter { name: Some("brightness".to_string()), args: Some("-50".to_string()) },
                Filter { name: Some("grayscale".to_string()), args: None },
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
                Filter { name: Some("watermark".to_string()), args: Some("s.glbimg.com/filters:label(abc):watermark(aaa.com/fit-in/filters:aaa(bbb))/aaa.jpg,0,0,0".to_string()) },
                Filter { name: Some("brightness".to_string()), args: Some("-50".to_string()) },
                Filter { name: Some("grayscale".to_string()), args: None },
            ],
        );
        let result = parse_filters(input).unwrap();
        assert_eq!(result, expected);
    }
}
