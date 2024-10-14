use std::collections::HashSet;
use std::path::Path;

const UPPER_HEX: &str = "0123456789ABCDEF";

trait SafeChars {
    fn should_escape(&self, c: u8) -> bool;
}

#[derive(Debug, Clone)]
pub enum SafeCharsType {
    Default,
    Custom(HashSet<u8>),
    Noop,
}

impl SafeChars for SafeCharsType {
    fn should_escape(&self, c: u8) -> bool {
        match self {
            SafeCharsType::Default => {
                !(c.is_ascii_alphanumeric()
                    || c == b'/'
                    || c == b'-'
                    || c == b'_'
                    || c == b'.'
                    || c == b'~')
            }
            SafeCharsType::Custom(safe_chars) => {
                !(c.is_ascii_alphanumeric()
                    || c == b'/'
                    || c == b'-'
                    || c == b'_'
                    || c == b'.'
                    || c == b'~'
                    || safe_chars.contains(&c))
            }
            SafeCharsType::Noop => false,
        }
    }
}

pub fn new_safe_chars(safechars: &str) -> SafeCharsType {
    if safechars == "--" {
        SafeCharsType::Noop
    } else if safechars.is_empty() {
        SafeCharsType::Default
    } else {
        SafeCharsType::Custom(safechars.bytes().collect())
    }
}

fn escape<F>(s: &str, should_escape: F) -> String
where
    F: Fn(u8) -> bool,
{
    let mut result = String::with_capacity(s.len());
    for &c in s.as_bytes() {
        if should_escape(c) {
            if c == b' ' {
                result.push('+');
            } else {
                result.push('%');
                result.push(UPPER_HEX.as_bytes()[(c >> 4) as usize] as char);
                result.push(UPPER_HEX.as_bytes()[(c & 15) as usize] as char);
            }
        } else {
            result.push(c as char);
        }
    }
    result
}

pub fn normalize(key: &str, safe_chars: &SafeCharsType) -> String {
    let cleaned = key
        .replace("\r\n", "")
        .replace(['\r', '\n', '\u{000B}', '\u{000C}', '\u{0085}', '\u{2028}', '\u{2029}'], "");

    let cleaned = cleaned.trim_matches('/');
    let path = Path::new(&cleaned).to_str().unwrap_or(cleaned);

    escape(path, |c| safe_chars.should_escape(c))
}
