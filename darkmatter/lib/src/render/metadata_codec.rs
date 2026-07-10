use std::env;

use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MetadataPolicy {
    Inline,
    Strip,
    Lossless,
}

pub(crate) fn metadata_policy(with_inline: bool, env_name: &str) -> MetadataPolicy {
    if with_inline {
        return MetadataPolicy::Inline;
    }

    match env::var(env_name).ok().as_deref().map(str::trim) {
        Some(value) if value.eq_ignore_ascii_case("inline") => MetadataPolicy::Inline,
        Some(value) if value.eq_ignore_ascii_case("strip") => MetadataPolicy::Strip,
        _ => MetadataPolicy::Lossless,
    }
}

pub(crate) fn encode<T: Serialize>(value: &T) -> Option<String> {
    let json = serde_json::to_string(value).ok()?;
    Some(base64_encode(json.as_bytes()))
}

pub(crate) fn decode<T: DeserializeOwned>(value: &str) -> Option<T> {
    let decoded = base64_decode(value.trim())?;
    let json = String::from_utf8(decoded).ok()?;
    serde_json::from_str(&json).ok()
}

fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | b2 as u32;
        out.push(TABLE[((n >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3f) as usize] as char);
        out.push(if chunk.len() > 1 { TABLE[((n >> 6) & 0x3f) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { TABLE[(n & 0x3f) as usize] as char } else { '=' });
    }
    out
}

pub(crate) fn base64_decode(input: &str) -> Option<Vec<u8>> {
    let input = input.trim();
    if input.is_empty() || !input.len().is_multiple_of(4) {
        return None;
    }
    let mut out = Vec::with_capacity(input.len() / 4 * 3);
    for chunk in input.as_bytes().chunks_exact(4) {
        let v0 = base64_value(chunk[0])?;
        let v1 = base64_value(chunk[1])?;
        let v2 = if chunk[2] == b'=' { None } else { Some(base64_value(chunk[2])?) };
        let v3 = if chunk[3] == b'=' { None } else { Some(base64_value(chunk[3])?) };
        let n = ((v0 as u32) << 18) | ((v1 as u32) << 12)
            | ((v2.unwrap_or(0) as u32) << 6) | v3.unwrap_or(0) as u32;
        out.push((n >> 16) as u8);
        if v2.is_some() { out.push((n >> 8) as u8); }
        if v3.is_some() { out.push(n as u8); }
    }
    Some(out)
}

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}
