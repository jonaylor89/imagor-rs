use url::form_urlencoded;

use super::params::{HAlign, Params, TrimBy, VAlign, F32};

pub fn generate_path(p: &Params) -> String {
    let parts = vec![
        generate_meta(p),
        generate_trim(p),
        generate_crop(p),
        generate_fit_in(p),
        generate_stretch(p),
        generate_size_and_flip(p),
        generate_padding(p),
        generate_valign(p),
        generate_halign(p),
        generate_smart(p),
        generate_filters(p),
        generate_image(p),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<String>>();

    parts.join("/")
}

fn generate_meta(p: &Params) -> Option<String> {
    if p.meta {
        Some("meta".to_string())
    } else {
        None
    }
}

fn generate_trim(p: &Params) -> Option<String> {
    if p.trim || (p.trim_by == TrimBy::TopLeft || p.trim_by == TrimBy::BottomRight) {
        let trims = vec![
            Some("trim".to_string()),
            if p.trim_by == TrimBy::BottomRight {
                Some("bottom-right".to_string())
            } else {
                None
            },
            p.trim_tolerance.map(|t| t.0.to_string()),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<String>>();

        Some(trims.join(":"))
    } else {
        None
    }
}

fn generate_crop(p: &Params) -> Option<String> {
    if p.crop_top.is_some()
        || p.crop_right.is_some()
        || p.crop_left.is_some()
        || p.crop_bottom.is_some()
    {
        Some(format!(
            "{}x{}:{}x{}",
            p.crop_left.unwrap_or(F32(0.0)).0,
            p.crop_top.unwrap_or(F32(0.0)).0,
            p.crop_right.unwrap_or(F32(0.0)).0,
            p.crop_bottom.unwrap_or(F32(0.0)).0
        ))
    } else {
        None
    }
}

fn generate_fit_in(p: &Params) -> Option<String> {
    if p.fit_in {
        Some("fit-in".to_string())
    } else {
        None
    }
}

fn generate_stretch(p: &Params) -> Option<String> {
    if p.stretch {
        Some("stretch".to_string())
    } else {
        None
    }
}

fn generate_size_and_flip(p: &Params) -> Option<String> {
    if p.h_flip
        || p.width.is_some()
        || p.v_flip
        || p.height.is_some()
        || p.padding_left.is_some()
        || p.padding_top.is_some()
    {
        let width = p.width.unwrap_or(0);
        let height = p.height.unwrap_or(0);
        let h_flip = p.h_flip ^ (width < 0);
        let v_flip = p.v_flip ^ (height < 0);

        let h_flip_str = if h_flip { "-" } else { "" };
        let v_flip_str = if v_flip { "-" } else { "" };

        Some(format!(
            "{}{}{}{}",
            h_flip_str,
            width.abs(),
            v_flip_str,
            height.abs()
        ))
    } else {
        None
    }
}

fn generate_padding(p: &Params) -> Option<String> {
    if p.padding_left.is_some()
        || p.padding_top.is_some()
        || p.padding_right.is_some()
        || p.padding_bottom.is_some()
    {
        let left = p.padding_left.unwrap_or(0);
        let top = p.padding_top.unwrap_or(0);
        let right = p.padding_right.unwrap_or(0);
        let bottom = p.padding_bottom.unwrap_or(0);

        if left == right && top == bottom {
            Some(format!("{}x{}", left, top))
        } else {
            Some(format!("{}x{}:{}x{}", left, top, right, bottom))
        }
    } else {
        None
    }
}

fn generate_halign(p: &Params) -> Option<String> {
    if let Some(h_align) = &p.h_align {
        match h_align {
            HAlign::Left | HAlign::Right => Some(h_align.to_string()),
            _ => None,
        }
    } else {
        None
    }
}

fn generate_valign(p: &Params) -> Option<String> {
    if let Some(v_align) = &p.v_align {
        match v_align {
            VAlign::Top | VAlign::Bottom => Some(v_align.to_string()),
            _ => None,
        }
    } else {
        None
    }
}

fn generate_smart(p: &Params) -> Option<String> {
    if p.smart {
        Some("smart".to_string())
    } else {
        None
    }
}

fn generate_filters(p: &Params) -> Option<String> {
    if !p.filters.is_empty() {
        let filters: Vec<String> = p
            .filters
            .iter()
            .filter(|f| !f.is_empty())
            .map(|f| format!("{}({})", f.name.as_ref().unwrap(), f.args.as_ref().unwrap()))
            .collect();
        Some(format!("filters:{}", filters.join(":")))
    } else {
        None
    }
}

fn generate_image(p: &Params) -> Option<String> {
    p.image.as_ref().map(|image| {
        if image.contains('?')
            || image.starts_with("trim/")
            || image.starts_with("meta/")
            || image.starts_with("fit-in/")
            || image.starts_with("stretch/")
            || image.starts_with("top/")
            || image.starts_with("left/")
            || image.starts_with("right/")
            || image.starts_with("bottom/")
            || image.starts_with("center/")
            || image.starts_with("smart/")
        {
            form_urlencoded::Serializer::new(String::new())
                .append_pair("", image)
                .finish()
        } else {
            image.to_string()
        }
    })
}

pub fn generate_unsafe(p: &Params) -> String {
    generate(p, None)
}

pub fn generate(p: &Params, signer: Option<&dyn Signer>) -> String {
    let img_path = generate_path(p);
    if let Some(signer) = signer {
        format!("{}/{}", signer.sign(&img_path), img_path)
    } else {
        format!("unsafe/{}", img_path)
    }
}

pub trait Signer {
    fn sign(&self, path: &str) -> String;
}
