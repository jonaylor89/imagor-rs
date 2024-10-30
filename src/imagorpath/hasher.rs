use super::{generate::generate_path, params};
use crate::imagorpath::filter::Filter;
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
                    .find_map(|filter| match filter {
                        Filter::Format(format) => Some(format!(".{}", format)),
                        _ => None,
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
                    .find_map(|filter| match filter {
                        Filter::Format(format) => Some(format!(".{}", format)),
                        _ => None,
                    })
                    .unwrap_or_else(|| image[dot_idx..].to_string())
            };
            return format!("{}{}{}", &image[..dot_idx], hash_with_size, ext);
        }
    }
    format!("{}{}", image, hash_with_size)
}

#[tracing::instrument(name = "Verify path hash", skip(expected_path_hash, path_candidate))]
pub fn verify_hash(
    expected_path_hash: SecretString,
    path_candidate: SecretString,
) -> Result<(), AuthError> {
    let expected_path_hash = PasswordHash::new(expected_path_hash.expose_secret())
        .context("Failed to parse hash in PHC string format.")?;

    Argon2::default()
        .verify_password(
            path_candidate.expose_secret().as_bytes(),
            &expected_path_hash,
        )
        .context("Invalid hash")
        .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(name = "Compute path hash", skip(path))]
pub fn compute_hash(path: String) -> Result<SecretString> {
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
    use super::*;
    use crate::imagorpath::{
        filter::{Filter, ImageType},
        params::Params,
        parse::parse_path,
    };

    #[test]
    fn test_digest_result_storage_hasher() {
        let (_, p) = parse_path("fit-in/16x17/foobar").unwrap();
        assert_eq!(
            digest_result_storage_hasher(&p),
            "d5/c2/804e5d81c475bee50f731db17ee613f43262",
        );

        let p_without_path = Params { path: None, ..p };
        assert_eq!(
            digest_result_storage_hasher(&p_without_path),
            "e7/d1/edae4e3a014626605954c8ce09a6d8526ed1"
        );
    }

    #[test]
    fn test_suffix_result_storage_hasher_fit_in() {
        let (_, p) = parse_path("fit-in/16x17/foobar").unwrap();
        assert_eq!(
            suffix_result_storage_hasher(&p),
            "foobar.d5c2804e5d81c475bee5",
        );
    }

    #[test]
    fn test_size_suffix_result_storage_hasher_fit_in() {
        let (_, p) = parse_path("fit-in/16x17/foobar").unwrap();
        assert_eq!(
            size_suffix_result_storage_hasher(&p),
            "foobar.d5c2804e5d81c475bee5_16x17"
        );
    }

    #[test]
    fn test_suffix_result_storage_hasher_fit_in_without_path() {
        let (_, p) = parse_path("fit-in/16x17/foobar").unwrap();
        let p_without_path = Params { path: None, ..p };
        assert_eq!(
            suffix_result_storage_hasher(&p_without_path),
            "foobar.e7d1edae4e3a01462660",
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
    fn test_size_suffix_result_storage_hasher_smart_no_size() {
        let (_, p) = parse_path("smart/example.com/foobar").unwrap();
        assert_eq!(
            size_suffix_result_storage_hasher(&p),
            "example.com/foobar.afa3503c0d76bc49eccd"
        );
    }

    #[test]
    fn test_suffix_result_storage_hasher_smart_no_size() {
        let (_, p) = parse_path("smart/example.com/foobar").unwrap();
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
            // "foobar.45d8ebb31bd4ed80c26e.jpg"
            "foobar.b9bb1f64c760cf298fb6.jpg",
        );
    }

    #[test]
    fn test_suffix_result_storage_hasher_with_format() {
        let p = Params {
            smart: true,
            width: Some(17),
            height: Some(19),
            image: Some("example.com/foobar.jpg".to_string()),
            filters: vec![Filter::Format(ImageType::WEBP)],
            ..Default::default()
        };
        println!("{}", generate_path(&p));
        assert_eq!(
            suffix_result_storage_hasher(&p),
            "example.com/foobar.98c5e02e0ba162bce51d.webp",
        );
        assert_eq!(
            size_suffix_result_storage_hasher(&p),
            "example.com/foobar.98c5e02e0ba162bce51d_17x19.webp",
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
            "example.com/foobar.b56fe819cae010721433.json",
        );
        assert_eq!(
            size_suffix_result_storage_hasher(&p),
            "example.com/foobar.b56fe819cae010721433_17x19.json"
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
            filters: vec![Filter::Format(ImageType::WEBP)],
            ..Default::default()
        };
        println!("{}", generate_path(&p));
        assert_eq!(
            suffix_result_storage_hasher(&p),
            "example.com/foobar.551f72136cd4ce0aaf36.json",
        );
        assert_eq!(
            size_suffix_result_storage_hasher(&p),
            "example.com/foobar.551f72136cd4ce0aaf36_17x19.json"
        );
    }
}
