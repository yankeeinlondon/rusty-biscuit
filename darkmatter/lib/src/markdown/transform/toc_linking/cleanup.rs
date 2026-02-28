//! Text cleanup services for heading display text.

use super::types::CleanupService;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Matches a leading emoji presentation sequence (including ZWJ compounds) and optional trailing space.
    static ref RE_EMOJI_LEADER: Regex =
        Regex::new(r"^\p{Emoji_Presentation}[\p{Emoji_Component}\p{Emoji_Presentation}\x{200D}\x{FE0F}]*\s*").unwrap();

    /// Matches a trailing emoji presentation sequence (including ZWJ compounds) and optional leading space.
    static ref RE_EMOJI_TRAILING: Regex =
        Regex::new(r"\s*\p{Emoji_Presentation}[\p{Emoji_Component}\p{Emoji_Presentation}\x{200D}\x{FE0F}]*$").unwrap();

    /// Matches any emoji presentation sequence (including ZWJ compounds).
    static ref RE_EMOJI_ALL: Regex =
        Regex::new(r"\p{Emoji_Presentation}[\p{Emoji_Component}\p{Emoji_Presentation}\x{200D}\x{FE0F}]*").unwrap();

    /// Matches a leading numeric index like `1.`, `3.2`, `1.2.3 `.
    static ref RE_NUMBER: Regex =
        Regex::new(r"^[\d]+(?:\.[\d]*)*\.?\s+").unwrap();
}

/// Applies the given cleanup services to the heading text, in order.
pub fn apply_cleanup(text: &str, services: &[CleanupService]) -> String {
    let mut result = text.to_string();
    for service in services {
        result = apply_single(&result, *service);
    }
    result
}

fn apply_single(text: &str, service: CleanupService) -> String {
    match service {
        CleanupService::EmojiLeader => RE_EMOJI_LEADER.replace(text, "").to_string(),
        CleanupService::EmojiTrailing => RE_EMOJI_TRAILING.replace(text, "").to_string(),
        CleanupService::Emoji => {
            let replaced = RE_EMOJI_ALL.replace_all(text, "").to_string();
            // Collapse runs of multiple spaces into one
            let mut prev_space = false;
            let mut out = String::with_capacity(replaced.len());
            for ch in replaced.chars() {
                if ch == ' ' {
                    if !prev_space {
                        out.push(ch);
                    }
                    prev_space = true;
                } else {
                    prev_space = false;
                    out.push(ch);
                }
            }
            out.trim().to_string()
        }
        CleanupService::Number => RE_NUMBER.replace(text, "").to_string(),
        CleanupService::Capitalize => capitalize_first_alnum(text),
    }
}

fn capitalize_first_alnum(text: &str) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    for ch in &mut chars {
        if ch.is_alphanumeric() {
            *ch = ch.to_uppercase().next().unwrap_or(*ch);
            break;
        }
    }
    chars.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_leading_emoji() {
        assert_eq!(
            apply_cleanup("🚀 Getting Started", &[CleanupService::EmojiLeader]),
            "Getting Started"
        );
    }

    #[test]
    fn strips_trailing_emoji() {
        assert_eq!(
            apply_cleanup("Getting Started 🎉", &[CleanupService::EmojiTrailing]),
            "Getting Started"
        );
    }

    #[test]
    fn strips_all_emoji() {
        assert_eq!(
            apply_cleanup("🚀 Fast 🏃 Running 🎉", &[CleanupService::Emoji]),
            "Fast Running"
        );
    }

    #[test]
    fn strips_zwj_sequences() {
        // Family emoji: 👨‍👩‍👧‍👦
        assert_eq!(
            apply_cleanup("👨\u{200D}👩\u{200D}👧\u{200D}👦 Family", &[CleanupService::EmojiLeader]),
            "Family"
        );
    }

    #[test]
    fn strips_leading_number() {
        assert_eq!(
            apply_cleanup("1. Introduction", &[CleanupService::Number]),
            "Introduction"
        );
        assert_eq!(
            apply_cleanup("3.2.1 Deep Section", &[CleanupService::Number]),
            "Deep Section"
        );
        assert_eq!(
            apply_cleanup("42. Answer", &[CleanupService::Number]),
            "Answer"
        );
    }

    #[test]
    fn capitalizes_first_alpha() {
        assert_eq!(
            apply_cleanup("hello world", &[CleanupService::Capitalize]),
            "Hello world"
        );
    }

    #[test]
    fn combined_services() {
        let services = vec![
            CleanupService::EmojiLeader,
            CleanupService::Number,
            CleanupService::Capitalize,
        ];
        assert_eq!(
            apply_cleanup("🔥 3.1 getting started", &services),
            "Getting started"
        );
    }

    #[test]
    fn no_op_when_nothing_matches() {
        assert_eq!(
            apply_cleanup("Normal Heading", &[CleanupService::Number]),
            "Normal Heading"
        );
    }

    #[test]
    fn bare_digit_not_stripped_by_emoji() {
        // ASCII digits should NOT match \p{Emoji_Presentation}
        assert_eq!(
            apply_cleanup("3. Setup", &[CleanupService::EmojiLeader]),
            "3. Setup"
        );
    }
}
