//! The suffix grammar that follows a frontmatter `$( … )` value's closing
//! parenthesis.
//!
//! One parser serves execution, the post-expansion leak guard, and the language
//! server, so the five suffixes and their rules are defined here only. A suffix
//! is never part of the command's approved bytes: approving `$(cmd)` covers
//! `$(cmd)::ok`.

use crate::markdown::span::{SourceSpan, Spanned};

/// A recognized trailing suffix on a frontmatter `$(...)` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontmatterShellSuffix {
    /// `::timeout:N` — a per-directive timeout in whole seconds.
    Timeout(u64),
    /// `::no-cache` — bypass the per-compose command cache.
    NoCache,
    /// `::ok` — the value is a boolean: the command exited `0`.
    Ok,
    /// `::exit-code` — the value is the exit status, or null for an allowed
    /// timeout.
    ExitCode,
    /// `::result` — the value is `{ ok, code, stdout, stderr }`.
    Result,
}

/// What a result suffix turns a command's outcome into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellResultKind {
    /// `::ok`
    Ok,
    /// `::exit-code`
    ExitCode,
    /// `::result`
    Result,
}

impl FrontmatterShellSuffix {
    /// The result this suffix asks for, when it is one of the three result
    /// suffixes.
    pub fn result_kind(&self) -> Option<ShellResultKind> {
        match self {
            Self::Ok => Some(ShellResultKind::Ok),
            Self::ExitCode => Some(ShellResultKind::ExitCode),
            Self::Result => Some(ShellResultKind::Result),
            Self::Timeout(_) | Self::NoCache => None,
        }
    }

    /// The suffix as an author writes it, e.g. `::timeout:5`.
    pub fn spelling(&self) -> String {
        match self {
            Self::Timeout(seconds) => format!("::timeout:{seconds}"),
            Self::NoCache => "::no-cache".to_string(),
            Self::Ok => "::ok".to_string(),
            Self::ExitCode => "::exit-code".to_string(),
            Self::Result => "::result".to_string(),
        }
    }

    fn same_kind(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

/// Author-facing description of one suffix, shared by completion and hover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellSuffixDescriptor {
    /// The suffix as listed, e.g. `::timeout:<seconds>`.
    pub label: &'static str,
    /// The text a completion inserts, e.g. `::timeout:`.
    pub insert_text: &'static str,
    /// One sentence describing what the suffix does.
    pub summary: &'static str,
}

/// Every frontmatter shell suffix, result suffixes first.
pub const FRONTMATTER_SHELL_SUFFIXES: [ShellSuffixDescriptor; 5] = [
    ShellSuffixDescriptor {
        label: "::ok",
        insert_text: "::ok",
        summary: "The value is a boolean: `true` when the command exited 0. A non-zero exit is a value, not a failure.",
    },
    ShellSuffixDescriptor {
        label: "::exit-code",
        insert_text: "::exit-code",
        summary: "The value is the exit status as a number, or `null` for a timeout allowed by `--allow-shell-timeout`. A non-zero exit is a value, not a failure.",
    },
    ShellSuffixDescriptor {
        label: "::result",
        insert_text: "::result",
        summary: "The value is an object `{ ok, code, stdout, stderr }` with trimmed streams. A non-zero exit is a value, not a failure.",
    },
    ShellSuffixDescriptor {
        label: "::timeout:<seconds>",
        insert_text: "::timeout:",
        summary: "Overrides the shell timeout for this command, in whole seconds greater than zero.",
    },
    ShellSuffixDescriptor {
        label: "::no-cache",
        insert_text: "::no-cache",
        summary: "Runs the command fresh: the per-compose command cache is neither read nor written.",
    },
];

/// The descriptor for a parsed suffix.
pub fn describe_suffix(suffix: &FrontmatterShellSuffix) -> &'static ShellSuffixDescriptor {
    let index = match suffix {
        FrontmatterShellSuffix::Ok => 0,
        FrontmatterShellSuffix::ExitCode => 1,
        FrontmatterShellSuffix::Result => 2,
        FrontmatterShellSuffix::Timeout(_) => 3,
        FrontmatterShellSuffix::NoCache => 4,
    };
    &FRONTMATTER_SHELL_SUFFIXES[index]
}

/// The five suffixes as a readable list for diagnostics.
pub fn expected_suffixes() -> String {
    let labels: Vec<String> = FRONTMATTER_SHELL_SUFFIXES
        .iter()
        .map(|descriptor| format!("`{}`", descriptor.label))
        .collect();
    format!(
        "{}, or {}",
        labels[..labels.len() - 1].join(", "),
        labels[labels.len() - 1]
    )
}

/// Why a suffix tail was rejected, with the byte span of the offending text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellSuffixError {
    /// Byte span of the rejected suffix or trailing text.
    pub span: SourceSpan,
    /// The diagnostic message.
    pub message: String,
}

/// Parses the text after a `$( … )` value's closing parenthesis.
///
/// `base` is the byte offset of `after_close` in the caller's coordinates;
/// returned spans are shifted by it. Suffixes combine in any order. The rules:
/// at most one result suffix (`::ok`, `::exit-code`, `::result`), no suffix
/// twice, and nothing that is not a suffix — a recognized suffix followed by
/// other text is rejected rather than read as that suffix.
///
/// ## Errors
///
/// A [`ShellSuffixError`] naming the first offending suffix or text.
pub fn parse_frontmatter_shell_suffixes(
    after_close: &str,
    base: usize,
) -> Result<Vec<Spanned<FrontmatterShellSuffix>>, ShellSuffixError> {
    let mut suffixes: Vec<Spanned<FrontmatterShellSuffix>> = Vec::new();
    let mut rest = after_close;
    let mut offset = base;
    while !rest.is_empty() {
        let (suffix, consumed) = next_suffix(rest, offset)?;
        if let Some(earlier) = suffixes.iter().find(|earlier| earlier.value.same_kind(&suffix.value)) {
            return Err(ShellSuffixError {
                span: suffix.span.clone(),
                message: format!(
                    "Duplicate suffix `{}`: `{}` already appears on this shell expression",
                    suffix.value.spelling(),
                    earlier.value.spelling()
                ),
            });
        }
        if suffix.value.result_kind().is_some()
            && let Some(earlier) = suffixes
                .iter()
                .find(|earlier| earlier.value.result_kind().is_some())
        {
            return Err(ShellSuffixError {
                span: suffix.span.clone(),
                message: format!(
                    "At most one result suffix per shell expression: `{}` and `{}` both appear",
                    earlier.value.spelling(),
                    suffix.value.spelling()
                ),
            });
        }
        offset += consumed;
        rest = &rest[consumed..];
        suffixes.push(suffix);
    }
    Ok(suffixes)
}

/// The suffixes a passive reader can recover, in source order: parsing stops
/// at the first text [`parse_frontmatter_shell_suffixes`] would reject, so a
/// half-typed value still yields the suffixes before it.
pub(crate) fn scan_frontmatter_shell_suffixes(
    after_close: &str,
    base: usize,
) -> Vec<Spanned<FrontmatterShellSuffix>> {
    let mut suffixes = Vec::new();
    let mut rest = after_close;
    let mut offset = base;
    while !rest.is_empty() {
        let Ok((suffix, consumed)) = next_suffix(rest, offset) else {
            break;
        };
        offset += consumed;
        rest = &rest[consumed..];
        suffixes.push(suffix);
    }
    suffixes
}

/// Reads one suffix from the start of `rest`, returning it with the number of
/// bytes it spans.
fn next_suffix(
    rest: &str,
    offset: usize,
) -> Result<(Spanned<FrontmatterShellSuffix>, usize), ShellSuffixError> {
    let Some(body) = rest.strip_prefix("::") else {
        return Err(ShellSuffixError {
            span: offset..offset + rest.len(),
            message: format!(
                "Unexpected trailing content `{rest}` after the frontmatter shell expression; \
                 a suffix starts with `::` and is one of {}",
                expected_suffixes()
            ),
        });
    };
    let item_len = body.find("::").unwrap_or(body.len());
    let item = &body[..item_len];
    let consumed = 2 + item_len;
    let span = offset..offset + consumed;
    let reject = |message: String| ShellSuffixError {
        span: span.clone(),
        message,
    };
    let suffix = match item {
        "" => {
            return Err(reject(format!(
                "Empty suffix `::` on the frontmatter shell expression; expected one of {}",
                expected_suffixes()
            )));
        }
        "ok" => FrontmatterShellSuffix::Ok,
        "exit-code" => FrontmatterShellSuffix::ExitCode,
        "result" => FrontmatterShellSuffix::Result,
        "no-cache" => FrontmatterShellSuffix::NoCache,
        _ if item == "timeout" || item.starts_with("timeout:") => {
            let digits = item.strip_prefix("timeout:").unwrap_or("");
            match digits.parse::<u64>() {
                Ok(0) => {
                    return Err(reject(
                        "Frontmatter shell timeout must be greater than zero".to_string(),
                    ));
                }
                Ok(seconds) if digits.bytes().all(|byte| byte.is_ascii_digit()) => {
                    FrontmatterShellSuffix::Timeout(seconds)
                }
                _ => {
                    return Err(reject(format!(
                        "Invalid ::timeout value `{digits}` in frontmatter shell expression; \
                         expected a positive integer number of seconds"
                    )));
                }
            }
        }
        other => {
            return Err(reject(format!(
                "Unrecognized suffix `::{other}` on the frontmatter shell expression; expected one of {}",
                expected_suffixes()
            )));
        }
    };
    Ok((Spanned::new(suffix, span), consumed))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str) -> Vec<FrontmatterShellSuffix> {
        parse_frontmatter_shell_suffixes(text, 0)
            .unwrap()
            .into_iter()
            .map(|spanned| spanned.value)
            .collect()
    }

    fn message(text: &str) -> String {
        parse_frontmatter_shell_suffixes(text, 0).unwrap_err().message
    }

    #[test]
    fn suffixes_combine_in_any_order() {
        use FrontmatterShellSuffix::*;
        assert_eq!(kinds("::ok::timeout:5::no-cache"), [Ok, Timeout(5), NoCache]);
        assert_eq!(kinds("::no-cache::timeout:5::result"), [NoCache, Timeout(5), Result]);
        assert_eq!(kinds("::exit-code"), [ExitCode]);
        assert!(kinds("").is_empty());
    }

    #[test]
    fn two_result_suffixes_name_both() {
        let text = message("::ok::result");
        assert!(text.contains("`::ok`") && text.contains("`::result`"), "{text}");
    }

    #[test]
    fn any_repeated_suffix_is_rejected() {
        let text = message("::timeout:5::timeout:9");
        assert!(text.contains("`::timeout:9`") && text.contains("`::timeout:5`"), "{text}");
        assert!(message("::no-cache::no-cache").contains("Duplicate"));
    }

    #[test]
    fn unrecognized_empty_and_trailing_text_list_all_five() {
        for bad in ["::bogus", "::ok::bogus", "::", "::ok trailing", " ::ok", "x"] {
            let text = message(bad);
            for label in ["::ok", "::exit-code", "::result", "::timeout:<seconds>", "::no-cache"] {
                assert!(text.contains(label), "`{bad}` → {text}");
            }
        }
    }

    #[test]
    fn a_rejected_suffix_carries_its_span() {
        let error = parse_frontmatter_shell_suffixes("::ok::bogus", 10).unwrap_err();
        assert_eq!(error.span, 14..21);
    }

    #[test]
    fn scanning_stops_at_the_first_rejected_suffix() {
        let scanned = scan_frontmatter_shell_suffixes("::ok::bog", 0);
        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].span, 0..4);
    }
}
