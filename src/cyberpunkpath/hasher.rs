use crate::cyberpunkpath::params::Params;
use hex;
use sha1::{Digest, Sha1};

use super::generate::generate_path;

fn hex_digest_path(path: &str) -> String {
    let digest = Sha1::digest(path.as_bytes());
    let hash = hex::encode(digest);
    format!("{}/{}/{}", &hash[..2], &hash[2..4], &hash[4..])
}

pub fn digest_storage_hasher(image: &str) -> String {
    hex_digest_path(image)
}

pub fn digest_result_storage_hasher(p: &Params) -> String {
    let path = p.path.clone().unwrap_or_else(|| generate_path(p));
    hex_digest_path(&path)
}

pub fn suffix_result_storage_hasher(p: &Params) -> String {
    let path = p.path.clone().unwrap_or_else(|| generate_path(p));
    let digest = Sha1::digest(path.as_bytes());
    let hash = format!(".{}", hex::encode(&digest[..10]));

    let image = p.image.as_ref().unwrap();
    let dot_idx = image.rfind('.');
    let slash_idx = image.rfind('/');

    if let Some(dot_idx) = dot_idx {
        if slash_idx.map_or(true, |idx| idx < dot_idx) {
            let ext = if p.meta {
                ".json".to_string()
            } else {
                p.filters
                    .iter()
                    .find_map(|filter| {
                        if filter.name.as_deref() == Some("format") {
                            Some(format!(".{}", filter.args.as_ref().unwrap()))
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| image[dot_idx..].to_string())
            };
            return format!("{}{}{}", &image[..dot_idx], hash, ext);
        }
    }
    format!("{}{}", image, hash)
}

pub fn size_suffix_result_storage_hasher(p: &Params) -> String {
    let path = p.path.clone().unwrap_or_else(|| generate_path(p));
    let digest = Sha1::digest(path.as_bytes());
    let hash_base = format!(".{}", hex::encode(&digest[..10]));

    let hash_with_size = if p.width.is_some() || p.height.is_some() {
        format!(
            "{}_{}x{}",
            hash_base,
            p.width.unwrap_or(0),
            p.height.unwrap_or(0)
        )
    } else {
        hash_base
    };

    let image = p.image.as_ref().unwrap();
    let dot_idx = image.rfind('.');
    let slash_idx = image.rfind('/');

    if let Some(dot_idx) = dot_idx {
        if slash_idx.map_or(true, |idx| idx < dot_idx) {
            let ext = if p.meta {
                ".json".to_string()
            } else {
                p.filters
                    .iter()
                    .find_map(|filter| {
                        if filter.name.as_deref() == Some("format") {
                            Some(format!(".{}", filter.args.as_ref().unwrap()))
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| image[dot_idx..].to_string())
            };
            return format!("{}{}{}", &image[..dot_idx], hash_with_size, ext);
        }
    }
    format!("{}{}", image, hash_with_size)
}
