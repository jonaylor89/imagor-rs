use super::{generate::generate_path, params};
use argon2::{
    password_hash::SaltString, Algorithm, Argon2, Params, PasswordHash, PasswordHasher,
    PasswordVerifier, Version,
};

use color_eyre::{
    eyre::{Context, Error},
    Result,
};
use hex;
use secrecy::{ExposeSecret, SecretBox, SecretString};
use sha1::{Digest, Sha1};

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials(#[source] Error),

    #[error(transparent)]
    UnexpectedError(#[from] Error),
}

fn hex_digest_path(path: &str) -> String {
    let digest = Sha1::digest(path.as_bytes());
    let hash = hex::encode(digest);
    format!("{}/{}/{}", &hash[..2], &hash[2..4], &hash[4..])
}

pub fn digest_storage_hasher(image: &str) -> String {
    hex_digest_path(image)
}

pub fn digest_result_storage_hasher(p: &params::Params) -> String {
    let path = p.path.clone().unwrap_or_else(|| generate_path(p));
    hex_digest_path(&path)
}

pub fn suffix_result_storage_hasher(p: &params::Params) -> String {
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

pub fn size_suffix_result_storage_hasher(p: &params::Params) -> String {
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

#[tracing::instrument(
    name = "Verify password hash",
    skip(expected_password_hash, password_candidate)
)]
pub fn verify_hash(
    expected_password_hash: SecretString,
    password_candidate: SecretString,
) -> Result<(), AuthError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format.")?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid password.")
        .map_err(AuthError::InvalidCredentials)
}

fn compute_hash(path: String) -> Result<SecretString> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let hash_password = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(15_000, 2, 1, None).unwrap(),
    )
    .hash_password(path.as_bytes(), &salt);

    let password_hash = hash_password?.to_string();

    Ok(SecretBox::from(password_hash))
}

#[cfg(test)]
mod tests {
    use params::Filter;

    use super::*;
    use crate::imagorpath::{params::Params, parse::parse_path};

    #[test]
    fn test_digest_result_storage_hasher() {
        let (_, p) = parse_path("fit-in/16x17/foobar").unwrap();
        assert_eq!(
            digest_result_storage_hasher(&p),
            "d5/c2/804e5d81c475bee50f731db17ee613f43262"
        );

        let p_without_path = Params { path: None, ..p };
        assert_eq!(
            digest_result_storage_hasher(&p_without_path),
            "d5/c2/804e5d81c475bee50f731db17ee613f43262"
        );
    }

    #[test]
    fn test_suffix_result_storage_hasher_fit_in() {
        let (_, p) = parse_path("fit-in/16x17/foobar").unwrap();
        assert_eq!(
            suffix_result_storage_hasher(&p),
            "foobar.d5c2804e5d81c475bee5"
        );
        assert_eq!(
            size_suffix_result_storage_hasher(&p),
            "foobar.d5c2804e5d81c475bee5_16x17"
        );

        let p_without_path = Params { path: None, ..p };
        assert_eq!(
            suffix_result_storage_hasher(&p_without_path),
            "foobar.d5c2804e5d81c475bee5"
        );
    }

    #[test]
    fn test_suffix_result_storage_hasher_smart() {
        let (_, p) = parse_path("17x19/smart/example.com/foobar").unwrap();
        assert_eq!(
            suffix_result_storage_hasher(&p),
            "example.com/foobar.ddd349e092cda6d9c729"
        );
        assert_eq!(
            size_suffix_result_storage_hasher(&p),
            "example.com/foobar.ddd349e092cda6d9c729_17x19"
        );
    }

    #[test]
    fn test_suffix_result_storage_hasher_smart_no_size() {
        let (_, p) = parse_path("smart/example.com/foobar").unwrap();
        assert_eq!(
            size_suffix_result_storage_hasher(&p),
            "example.com/foobar.afa3503c0d76bc49eccd"
        );
        assert_eq!(
            suffix_result_storage_hasher(&p),
            "example.com/foobar.afa3503c0d76bc49eccd"
        );
    }

    #[test]
    fn test_suffix_result_storage_hasher_with_extension() {
        let (_, p) = parse_path("166x169/top/foobar.jpg").unwrap();
        assert_eq!(
            suffix_result_storage_hasher(&p),
            "foobar.45d8ebb31bd4ed80c26e.jpg"
        );
    }

    #[test]
    fn test_size_suffix_result_storage_hasher_with_extension() {
        let (_, p) = parse_path("166x169/top/foobar.jpg").unwrap();
        assert_eq!(
            size_suffix_result_storage_hasher(&p),
            "foobar.45d8ebb31bd4ed80c26e_166x169.jpg"
        );
    }

    #[test]
    fn test_suffix_result_storage_hasher_with_extension_no_path() {
        let (_, p) = parse_path("166x169/top/foobar.jpg").unwrap();
        let p_without_path = Params { path: None, ..p };
        assert_eq!(
            suffix_result_storage_hasher(&p_without_path),
            "foobar.45d8ebb31bd4ed80c26e.jpg"
        );
    }

    #[test]
    fn test_suffix_result_storage_hasher_with_format() {
        let p = Params {
            smart: true,
            width: Some(17),
            height: Some(19),
            image: Some("example.com/foobar.jpg".to_string()),
            filters: vec![Filter {
                name: Some("format".to_string()),
                args: Some("webp".to_string()),
            }],
            ..Default::default()
        };
        println!("{}", generate_path(&p));
        assert_eq!(
            suffix_result_storage_hasher(&p),
            "example.com/foobar.8aade9060badfcb289f9.webp"
        );
        assert_eq!(
            size_suffix_result_storage_hasher(&p),
            "example.com/foobar.8aade9060badfcb289f9_17x19.webp"
        );
    }

    #[test]
    fn test_suffix_result_storage_hasher_with_meta() {
        let p = Params {
            meta: true,
            smart: true,
            width: Some(17),
            height: Some(19),
            image: Some("example.com/foobar.jpg".to_string()),
            ..Default::default()
        };
        println!("{}", generate_path(&p));
        assert_eq!(
            suffix_result_storage_hasher(&p),
            "example.com/foobar.d72ff6ef20ba41fa570c.json"
        );
        assert_eq!(
            size_suffix_result_storage_hasher(&p),
            "example.com/foobar.d72ff6ef20ba41fa570c_17x19.json"
        );
    }

    #[test]
    fn test_suffix_result_storage_hasher_with_meta_and_format() {
        let p = Params {
            meta: true,
            smart: true,
            width: Some(17),
            height: Some(19),
            image: Some("example.com/foobar.jpg".to_string()),
            filters: vec![Filter {
                name: Some("format".to_string()),
                args: Some("webp".to_string()),
            }],
            ..Default::default()
        };
        println!("{}", generate_path(&p));
        assert_eq!(
            suffix_result_storage_hasher(&p),
            "example.com/foobar.c80ab0faf85b35a140a8.json"
        );
        assert_eq!(
            size_suffix_result_storage_hasher(&p),
            "example.com/foobar.c80ab0faf85b35a140a8_17x19.json"
        );
    }
}
