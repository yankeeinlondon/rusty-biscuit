//! The pattern grammar: `[!][reference-prefix]glob`, and the only use of the
//! `globset` crate in biscuit-file.

use globset::{GlobBuilder, GlobMatcher};

use super::error::{GlobReferenceError, RECURSIVE_PREFIX, REMOTE_PREFIX};
use crate::file_reference::{FileReferenceError, ParsedReference, ReferenceKind, TemplateSegment};

/// One authored pattern, parsed but not yet resolved against a context.
#[derive(Debug, Clone)]
pub(crate) struct GlobPattern {
    /// The pattern as authored, `!` included.
    pub authored: String,
    pub negated: bool,
    /// The reference grammar's reading of the pattern after any `!`: its
    /// prefix kind and its payload template.
    pub parsed: ParsedReference,
    /// Whether the glob text after the prefix names no directory (`*.md`,
    /// `^*.md`), so the file-name view applies when it is enabled.
    pub name_only: bool,
}

/// Parse one authored pattern.
///
/// The prefix grammar is [`FileReference`](crate::FileReference)'s, so a
/// prefix it rejects is rejected here with the same error. The glob text is
/// compiled once with each `{{VAR}}` standing for a literal name, so a glob
/// syntax error is reported before any context exists.
pub(super) fn parse_pattern(authored: &str) -> Result<GlobPattern, GlobReferenceError> {
    let (negated, body) = match authored.strip_prefix('!') {
        Some(rest) => (true, rest),
        None => (false, authored),
    };
    let malformed = |source| GlobReferenceError::MalformedPrefix {
        pattern: authored.to_string(),
        source,
    };
    let parsed = crate::file_reference::parse::parse(body).map_err(malformed)?;
    if parsed.recursive {
        return Err(GlobReferenceError::RejectedPrefix {
            pattern: authored.to_string(),
            prefix: RECURSIVE_PREFIX,
            reason: "a glob already searches recursively",
        });
    }
    if matches!(parsed.kind, ReferenceKind::Url(_)) {
        return Err(GlobReferenceError::RejectedPrefix {
            pattern: authored.to_string(),
            prefix: REMOTE_PREFIX,
            reason: "there is no local directory to search",
        });
    }

    let probe: Vec<Piece> = parsed
        .kind
        .template()
        .segments
        .iter()
        .map(|segment| match segment {
            TemplateSegment::Literal(text) => Piece::glob(text),
            TemplateSegment::EnvVar(_) => Piece::literal("x"),
        })
        .collect();
    let split = SplitPattern::new(&probe);
    if split.glob.is_empty() {
        return Err(malformed(FileReferenceError::InvalidSyntax(format!(
            "a glob reference must name files after its prefix: `{body}`"
        ))));
    }
    compile(&split.glob).map_err(|message| GlobReferenceError::InvalidGlob {
        pattern: authored.to_string(),
        message,
    })?;

    let payload = &parsed.authored[parsed.payload_offset..];
    let name_only = !payload.chars().any(std::path::is_separator);
    Ok(GlobPattern {
        authored: authored.to_string(),
        negated,
        parsed,
        name_only,
    })
}

/// One run of pattern text: authored glob syntax, or text that is always a
/// literal name (an interpolated `{{VAR}}` value, or a `%` payload).
#[derive(Debug, Clone)]
pub(crate) struct Piece {
    pub text: String,
    pub glob: bool,
}

impl Piece {
    pub(crate) fn glob(text: &str) -> Self {
        Self {
            text: text.to_string(),
            glob: true,
        }
    }

    pub(crate) fn literal(text: &str) -> Self {
        Self {
            text: text.to_string(),
            glob: false,
        }
    }
}

/// A pattern split at its literal directory prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SplitPattern {
    /// The leading directories that hold no glob syntax, as native text with
    /// their trailing separator; empty when there are none. They narrow the
    /// walk and, for an absolute pattern, are its root.
    pub literal_dir: String,
    /// The rest of the pattern as glob text: `/` separators, literal runs
    /// escaped.
    pub glob: String,
}

impl SplitPattern {
    pub(crate) fn new(pieces: &[Piece]) -> Self {
        let chars: Vec<(char, bool)> = pieces
            .iter()
            .flat_map(|piece| piece.text.chars().map(move |c| (c, piece.glob)))
            .collect();
        let first_meta = chars
            .iter()
            .position(|&(c, glob)| glob && matches!(c, '*' | '?' | '[' | '{'))
            .unwrap_or(chars.len());
        let split_at = chars[..first_meta]
            .iter()
            .rposition(|&(c, _)| std::path::is_separator(c))
            .map_or(0, |separator| separator + 1);

        let literal_dir = chars[..split_at].iter().map(|&(c, _)| c).collect();
        let mut glob = String::new();
        for &(c, is_glob) in &chars[split_at..] {
            if std::path::is_separator(c) {
                glob.push('/');
            } else if is_glob {
                glob.push(c);
            } else {
                glob.push_str(&escape(&c.to_string()));
            }
        }
        Self { literal_dir, glob }
    }
}

/// Compile glob text into a matcher for a `/`-spelled relative path.
///
/// `*` and `?` never cross `/` (`**` does), matching is case-sensitive on
/// every OS, and `\` is always a literal character: `globset` would otherwise
/// read it as an escape on Unix only, so one pattern would mean different
/// things per host. Literal text is escaped with character classes
/// ([`escape`]) instead.
pub(crate) fn compile(glob: &str) -> Result<GlobMatcher, String> {
    GlobBuilder::new(glob)
        .literal_separator(true)
        .case_insensitive(false)
        .backslash_escape(false)
        .build()
        .map(|glob| glob.compile_matcher())
        .map_err(|error| error.kind().to_string())
}

/// Escape `text` so a glob reads it as a literal name.
pub(crate) fn escape(text: &str) -> String {
    globset::escape(text)
}
