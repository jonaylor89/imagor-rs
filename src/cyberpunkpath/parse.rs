use super::params::{Filter, Params, TrimBy, F32};
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use color_eyre::{
    eyre::{self},
    Result,
};
use lazy_static::lazy_static;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while, take_while1},
    character::complete::{char, digit1},
    combinator::{map, opt, recognize, value},
    error::Error,
    multi::separated_list0,
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    IResult,
};
use regex::{Captures, Regex};
use tracing::info;
use url::Url;

lazy_static! {
    static ref PATH_REGEX: Regex = Regex::new(
        r"/*(?P<params>params/)?(?P<hash>(unsafe/)|([A-Za-z0-9-_=]{8,})/)?(?P<path>.+)?"
    ).unwrap();

    static ref PARAMS_REGEX: Regex = Regex::new(
        r"/*(?P<meta>meta/)?(?P<trim>trim(:(?P<trim_by>top-left|bottom-right))?(:(?P<trim_tolerance>\d+))?/)?(?P<crop>((0?\.)?\\d+)x((0?\.)?\\d+):((0?\.)?\\d+)x((0?\.)?\\d+)/)?(?P<fit_in>fit-in/)?(?P<stretch>stretch/)?(?P<dimensions>(\-?)(\d*)x(\-?)(\d*)/)?(?P<padding>(\d+)x(\d+)(:(\d+)x(\d+))?/)?(?P<h_align>(left|right|center)/)?(?P<v_align>(top|bottom|middle)/)?(?P<smart>smart/)?(?P<rest>.+)?"
    ).unwrap();
}

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

        // parse endpoint
        let (_, params) = parse_path(&path).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Failed to parse params: {}", e),
            )
        })?;

        Ok(params)
    }
}

// pub fn parse(segment: &str) -> Result<Params> {
//     let path_captures = PATH_REGEX.captures(&segment);
//     let params_captures = path_captures
//         .as_ref()
//         .and_then(|caps| caps.name("path"))
//         .and_then(|path_match| PARAMS_REGEX.captures(path_match.as_str()));

//     Ok(Params {
//         params: path_captures
//             .as_ref()
//             .and_then(|caps| caps.name("params"))
//             .is_some(),
//         unsafe_: path_captures
//             .as_ref()
//             .and_then(|caps| caps.name("hash"))
//             .map(|h| h.as_str() == "unsafe/")
//             .unwrap_or(false),
//         hash: path_captures
//             .as_ref()
//             .and_then(|caps| caps.name("hash"))
//             .and_then(|h| {
//                 if h.as_str() != "unsafe/" {
//                     Some(h.as_str().to_string())
//                 } else {
//                     None
//                 }
//             }),
//         path: path_captures
//             .as_ref()
//             .and_then(|caps| caps.name("path"))
//             .map(|p| p.as_str().to_string()),
//         meta: params_captures
//             .as_ref()
//             .and_then(|caps| caps.name("meta"))
//             .is_some(),
//         trim: params_captures
//             .as_ref()
//             .and_then(|caps| caps.name("trim"))
//             .is_some(),
//         trim_by: params_captures
//             .as_ref()
//             .and_then(|caps| caps.name("trim_by"))
//             .map(|m| m.as_str().to_string()),
//         trim_tolerance: params_captures
//             .as_ref()
//             .and_then(|caps| caps.name("trim_tolerance"))
//             .and_then(|m| m.as_str().parse().ok()),
//         crop_left: parse_crop_value(params_captures.as_ref(), 0),
//         crop_top: parse_crop_value(params_captures.as_ref(), 1),
//         crop_right: parse_crop_value(params_captures.as_ref(), 2),
//         crop_bottom: parse_crop_value(params_captures.as_ref(), 3),
//         fit_in: params_captures
//             .as_ref()
//             .and_then(|caps| caps.name("fit_in"))
//             .is_some(),
//         stretch: params_captures
//             .as_ref()
//             .and_then(|caps| caps.name("stretch"))
//             .is_some(),
//         width: parse_dimension(params_captures.as_ref(), 0),
//         height: parse_dimension(params_captures.as_ref(), 1),
//         h_flip: parse_dimension(params_captures.as_ref(), 0)
//             .map(|d| d < 0)
//             .unwrap_or(false),
//         v_flip: parse_dimension(params_captures.as_ref(), 1)
//             .map(|d| d < 0)
//             .unwrap_or(false),
//         padding_left: parse_padding(params_captures.as_ref(), 0),
//         padding_top: parse_padding(params_captures.as_ref(), 1),
//         padding_right: parse_padding(params_captures.as_ref(), 2),
//         padding_bottom: parse_padding(params_captures.as_ref(), 3),
//         h_align: params_captures
//             .as_ref()
//             .and_then(|caps| caps.name("h_align"))
//             .map(|m| m.as_str().trim_end_matches('/').to_string()),
//         v_align: params_captures
//             .as_ref()
//             .and_then(|caps| caps.name("v_align"))
//             .map(|m| m.as_str().trim_end_matches('/').to_string()),
//         smart: params_captures
//             .as_ref()
//             .and_then(|caps| caps.name("smart"))
//             .is_some(),
//         filters: params_captures
//             .as_ref()
//             .and_then(|caps| caps.name("rest"))
//             .and_then(|rest| {
//                 parse_filters(rest.as_str())
//                     .map(|(filters, _)| filters)
//                     .ok()
//             })
//             .unwrap_or_default(),
//         image: params_captures
//             .as_ref()
//             .and_then(|caps| caps.name("rest"))
//             .and_then(|rest| {
//                 parse_filters(rest.as_str())
//                     .map(|(_, img)| Url::parse(&img).map(|u| u.to_string()).unwrap_or(img))
//                     .ok()
//             }),
//         ..Default::default()
//     })
// }

// fn parse_crop_value(captures: Option<&Captures>, index: usize) -> Option<F32> {
//     captures
//         .and_then(|caps| caps.name("crop"))
//         .and_then(|crop| crop.as_str().split(|c| c == 'x' || c == ':').nth(index))
//         .and_then(|val| val.parse().ok())
// }

// fn parse_dimension(captures: Option<&Captures>, index: usize) -> Option<i32> {
//     captures
//         .and_then(|caps| caps.name("dimensions"))
//         .and_then(|dims| dims.as_str().split('x').nth(index))
//         .and_then(|val| val.trim_start_matches('-').parse::<i32>().ok())
//         .map(|val| {
//             if index == 0 && val.to_string().starts_with('-') {
//                 -val
//             } else {
//                 val
//             }
//         })
// }

// fn parse_padding(captures: Option<&Captures>, index: usize) -> Option<i32> {
//     captures
//         .and_then(|caps| caps.name("padding"))
//         .and_then(|padding| padding.as_str().split(|c| c == 'x' || c == ':').nth(index))
//         .and_then(|val| val.parse().ok())
// }

// fn parse_filters(s: &str) -> Result<(Vec<Filter>, String)> {
//     if !s.starts_with("filters:") {
//         return Ok((vec![], s.to_string()));
//     }

//     let (filters_str, image) = s[8..].split_once('/').unwrap_or((s[8..].trim(), ""));
//     dbg!("{} and {}", filters_str, image);
//     let filters = parse_filter_string(filters_str)?;

//     Ok((filters, image.to_string()))
// }

// fn parse_filter_string(s: &str) -> Result<Vec<Filter>> {
//     let mut filters = Vec::new();
//     let mut current_filter = Filter::default();
//     let mut depth = 0;

//     for c in s.chars() {
//         match (c, depth) {
//             ('(', 0) => {
//                 depth += 1;
//             }
//             (')', 1) => {
//                 filters.push(current_filter.clone());
//                 current_filter = Filter::default();
//                 depth -= 1;
//             }
//             (':', 0) => {
//                 if !current_filter.is_empty() {
//                     filters.push(current_filter.clone());
//                     current_filter = Filter::default();
//                 }
//             }
//             _ => {
//                 if depth > 0 {
//                     current_filter.args.get_or_insert_with(String::new).push(c);
//                 } else {
//                     current_filter.name.get_or_insert_with(String::new).push(c);
//                 }
//                 if c == '(' {
//                     depth += 1;
//                 }
//                 if c == ')' {
//                     depth -= 1;
//                 }
//             }
//         }
//     }

//     if !current_filter.is_empty() {
//         filters.push(current_filter.clone());
//     }

//     Ok(filters)
// }

fn parse_meta(input: &str) -> IResult<&str, bool> {
    value(true, tag("meta/"))(input)
}

fn parse_trim(input: &str) -> IResult<&str, (bool, Option<TrimBy>, Option<F32>)> {
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

fn parse_crop(input: &str) -> IResult<&str, (F32, F32, F32, F32)> {
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

fn parse_f32(input: &str) -> IResult<&str, F32> {
    map(
        recognize(tuple((
            opt(char('-')),
            digit1,
            opt(preceded(char('.'), digit1)),
        ))),
        |s: &str| F32(s.parse().unwrap()),
    )(input)
}

fn parse_dimensions(input: &str) -> IResult<&str, (Option<i32>, Option<i32>, bool, bool)> {
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

fn parse_fit_in(input: &str) -> IResult<&str, bool> {
    value(true, tag("fit-in/"))(input)
}

fn parse_alignment(input: &str) -> IResult<&str, (Option<String>, Option<String>)> {
    tuple((
        opt(terminated(
            alt((
                value("left".to_string(), tag("left")),
                value("right".to_string(), tag("right")),
                value("center".to_string(), tag("center")),
            )),
            char('/'),
        )),
        opt(terminated(
            alt((
                value("top".to_string(), tag("top")),
                value("bottom".to_string(), tag("bottom")),
                value("middle".to_string(), tag("middle")),
            )),
            char('/'),
        )),
    ))(input)
}

fn parse_smart(input: &str) -> IResult<&str, bool> {
    value(true, tag("smart/"))(input)
}

fn parse_filter_args(input: &str) -> IResult<&str, Option<String>> {
    let (input, args) = take_until(")")(input)?;
    let mut nested_level = 0;
    let mut result = String::new();

    for c in args.chars() {
        match c {
            '(' => {
                nested_level += 1;
                result.push(c);
            }
            ')' => {
                if nested_level == 0 {
                    break;
                }
                nested_level -= 1;
                result.push(c);
            }
            _ => result.push(c),
        }
    }

    if !result.is_empty() {
        return Ok((input, Some(result)));
    }

    Ok((input, None))
}

fn parse_filter(input: &str) -> IResult<&str, Filter> {
    alt((
        // Filter with arguments (possibly nested)
        map(
            tuple((
                take_while1(|c: char| c.is_alphanumeric() || c == '_'),
                delimited(char('('), parse_filter_args, char(')')),
            )),
            |(name, args)| Filter {
                name: Some(name.to_string()),
                args,
            },
        ),
        // Filter without arguments
        map(
            take_while1(|c: char| c.is_alphanumeric() || c == '_'),
            |name: &str| Filter {
                name: Some(name.to_string()),
                args: None,
            },
        ),
    ))(input)
}

fn parse_filters(input: &str) -> IResult<&str, Vec<Filter>> {
    preceded(
        tag("filters:"),
        terminated(separated_list0(char(':'), parse_filter), char('/')),
    )(input)
}

fn parse_image(input: &str) -> IResult<&str, String> {
    take_while1(|c: char| c != '/')(input)
        .map(|(next_input, result)| (next_input, result.to_string()))
}

fn parse_path(input: &str) -> IResult<&str, Params> {
    map(
        tuple((
            parse_meta,
            parse_trim,
            opt(parse_crop),
            parse_fit_in,
            parse_dimensions,
            parse_alignment,
            parse_smart,
            opt(parse_filters),
            parse_image,
        )),
        |(
            meta,
            (trim, trim_by, trim_tolerance),
            crop,
            fit_in,
            (width, height, h_flip, v_flip),
            (h_align, v_align),
            smart,
            filters,
            image,
        )| {
            Params {
                path: Some(input.to_string()),
                image: Some(image),
                trim,
                trim_by: trim_by.unwrap_or(TrimBy::TopLeft),
                trim_tolerance,
                crop_left: crop.map(|(left, _, _, _)| left),
                crop_top: crop.map(|(_, top, _, _)| top),
                crop_right: crop.map(|(_, _, right, _)| right),
                crop_bottom: crop.map(|(_, _, _, bottom)| bottom),
                width,
                height,
                meta,
                h_flip,
                v_flip,
                h_align,
                v_align,
                smart,
                fit_in,
                filters: filters.unwrap_or_default(),
                ..Default::default()
            }
        },
    )(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cyberpunkpath::params::{TrimBy, H_ALIGN_LEFT, V_ALIGN_TOP};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_parse_generate() {
        let tests = vec![
             (
                 "non url image",
                 "meta/trim/10x11:12x13/fit-in/-300x-200/left/top/smart/filters:some_filter()/img",
                 Params {
                     path: Some("meta/trim/10x11:12x13/fit-in/-300x-200/left/top/smart/filters:some_filter()/img".to_string()),
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
                     h_align: Some(H_ALIGN_LEFT.to_string()),
                     v_align: Some(V_ALIGN_TOP.to_string()),
                     smart: true,
                     fit_in: true,
                     filters: vec![Filter { name: Some("some_filter".to_string()), args: None }],
                     ..Default::default()
                 },
             ),
             (
                 "url image",
                 "meta/trim:bottom-right:100/10x11:12x13/fit-in/-300x-200/left/top/smart/filters:some_filter()/s.glbimg.com/es/ge/f/original/2011/03/29/orlandosilva_60.jpg",
                 Params {
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
                     h_align: Some(H_ALIGN_LEFT.to_string()),
                     v_align: Some(V_ALIGN_TOP.to_string()),
                     smart: true,
                     fit_in: true,
                     filters: vec![Filter { name: Some("some_filter".to_string()), args: None }],
                     ..Default::default()
                 },
             ),
         ];

        for (name, uri, expected_params) in tests {
            let (_, result) = parse_path(uri).unwrap();
            assert_eq!(result, expected_params, "Failed test: {}", name);
        }
    }

    #[test]
    fn test_parse_filters() {
        let test_cases = vec![
            (
                "filters:watermark(s.glbimg.com/filters:label(abc):watermark(aaa.com/fit-in/filters:aaa(bbb))/aaa.jpg,0,0,0):brightness(-50):grayscale()/some/example/img",
                (
                    "some/example/img",
                    vec![
                        Filter { name: Some("watermark".to_string()), args: Some("s.glbimg.com/filters:label(abc):watermark(aaa.com/fit-in/filters:aaa(bbb))/aaa.jpg,0,0,0".to_string()) },
                        Filter { name: Some("brightness".to_string()), args: Some("-50".to_string()) },
                        Filter { name: Some("grayscale".to_string()), args: None },
                    ],
                )
            ),
            (
                "some/example/img",
                ("some/example/img", vec![]),
            ),
            (
                "filters:watermark(s.glbimg.com/filters:label(abc):watermark(aaa.com/fit-in/filters:aaa(bbb))/aaa.jpg,0,0,0):brightness(-50):grayscale()",
                (
                    "",
                    vec![
                        Filter { name: Some("watermark".to_string()), args: Some("s.glbimg.com/filters:label(abc):watermark(aaa.com/fit-in/filters:aaa(bbb))/aaa.jpg,0,0,0".to_string()) },
                        Filter { name: Some("brightness".to_string()), args: Some("-50".to_string()) },
                        Filter { name: Some("grayscale".to_string()), args: None },
                    ],
                )
            ),
        ];

        for (input, expected) in test_cases {
            let result = parse_filters(input).unwrap();
            assert_eq!(result, expected);
        }
    }

    // #[test]
    // fn test_parse_filter_string() {
    //     let test_cases = vec![
    //             (
    //                 "watermark(s.glbimg.com/filters:label(abc):watermark(aaa.com/fit-in/filters:aaa(bbb))/aaa.jpg,0,0,0):brightness(-50):grayscale()",
    //                 vec![
    //                     Filter { name: Some("watermark".to_string()), args: Some("s.glbimg.com/filters:label(abc):watermark(aaa.com/fit-in/filters:aaa(bbb))/aaa.jpg,0,0,0".to_string()) },
    //                     Filter { name: Some("brightness".to_string()), args: Some("-50".to_string()) },
    //                     Filter { name: Some("grayscale".to_string()), args: None },
    //                 ]
    //             ),
    //             (
    //                 "label(哈哈,1,2,3):brightness(-50):grayscale()",
    //                 vec![
    //                     Filter { name: Some("label".to_string()), args: Some("哈哈,1,2,3".to_string()) },
    //                     Filter { name: Some("brightness".to_string()), args: Some("-50".to_string()) },
    //                     Filter { name: Some("grayscale".to_string()), args: None },
    //                 ]
    //             ),
    //         ];

    //     for (input, expected) in test_cases {
    //         let result = parse_filter_string(input);
    //         assert_eq!(result, expected);
    //     }
    // }
}
