//! Comparison keys for supported lexical names; public names remain source text.

use std::borrow::Cow;
use unicode_normalization::UnicodeNormalization;
use crate::shared::ProgrammingLanguage;

pub(crate) fn key(language: ProgrammingLanguage, written: &str) -> Cow<'_, str> {
    match language {
        ProgrammingLanguage::Rust => {
            let bare = written.strip_prefix("r#").unwrap_or(written);
            if bare.is_ascii() { return Cow::Borrowed(bare); }
            let normalized: String = bare.nfc().collect();
            if normalized == written { Cow::Borrowed(written) } else { Cow::Owned(normalized) }
        }
        ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript if written.contains('\\') => {
            Cow::Owned(decode(written).unwrap_or_else(|| written.to_string()))
        }
        _ => Cow::Borrowed(written),
    }
}

pub(crate) fn equal(language: ProgrammingLanguage, a: &str, b: &str) -> bool {
    key(language, a) == key(language, b)
}

pub(crate) fn original_key(language: ProgrammingLanguage, written: &str) -> Cow<'_, str> {
    if matches!(language, ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript)
        && let Some(quote) = written.chars().next().filter(|c| matches!(c, '\'' | '"'))
        && written.ends_with(quote) && written.len() >= 2
    {
        // A string-named export is one name, including any dots it contains.
        let body = &written[1..written.len()-1];
        return Cow::Owned(decode(body).unwrap_or_else(|| body.to_string()));
    }
    let bare = match language {
        ProgrammingLanguage::Rust => written.rsplit("::").next().unwrap_or(written),
        ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript => written.rsplit('.').next().unwrap_or(written),
        _ => super::last_name_segment(written),
    };
    key(language, bare)
}

fn decode(written: &str) -> Option<String> {
    let mut chars = written.chars();
    let mut units = Vec::new();
    while let Some(character) = chars.next() {
        let value = if character != '\\' {
            character as u32
        } else {
            match chars.next()? {
                'u' => {
                    let first = chars.next()?;
                    if first == '{' {
                        let mut hex = String::new();
                        loop {
                            let digit = chars.next()?;
                            if digit == '}' { break; }
                            hex.push(digit);
                        }
                        u32::from_str_radix(&hex, 16).ok()?
                    } else {
                        let mut hex = String::from(first);
                        for _ in 0..3 { hex.push(chars.next()?); }
                        u32::from_str_radix(&hex, 16).ok()?
                    }
                }
                'x' => {
                    let hex: String = [chars.next()?, chars.next()?].into_iter().collect();
                    u32::from_str_radix(&hex, 16).ok()?
                }
                '\n' | '\u{2028}' | '\u{2029}' => continue,
                '\r' => {
                    if chars.clone().next() == Some('\n') { chars.next(); }
                    continue;
                }
                'n' => 10,
                'r' => 13,
                't' => 9,
                'b' => 8,
                'f' => 12,
                'v' => 11,
                '0' => 0,
                other => other as u32,
            }
        };
        if value > 0x10ffff { return None; }
        if value > 0xffff {
            let value = value - 0x10000;
            units.push((0xd800 + (value >> 10)) as u16);
            units.push((0xdc00 + (value & 0x3ff)) as u16);
        } else {
            units.push(value as u16);
        }
    }
    String::from_utf16(&units).ok()
}
