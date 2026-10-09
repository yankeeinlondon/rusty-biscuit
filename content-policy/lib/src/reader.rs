//! The one frontmatter reader shared by evaluation and renewal.
//!
//! It is strict YAML with a single repair: tab-indented frontmatter, which
//! YAML forbids but Darkmatter tolerates, is repaired in memory with Biscuit
//! File's tab-indentation repair, so a document reads the same through both.
//! Every other Darkmatter fallback, such as protecting unquoted `{{ }}`
//! templates, is a named rejection.

use std::fmt;
use std::ops::Range;

use biscuit_file::yaml::{YamlDiagnosticCode, YamlRepair, analyze_yaml, apply_edit_set};
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::model::EvidenceRecord;

/// A line terminator style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineEnding {
    Lf,
    CrLf,
    /// A lone carriage return.
    Cr,
}

impl LineEnding {
    #[allow(missing_docs)]
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
            Self::Cr => "\r",
        }
    }
}

/// A parsed frontmatter block and the source spans renewal edits against.
///
/// All ranges are byte offsets into the whole document.
#[derive(Debug, Clone, PartialEq)]
pub struct Frontmatter {
    record: EvidenceRecord,
    block: Range<usize>,
    yaml: Range<usize>,
    fence_line_ending: LineEnding,
    tab_repair: Vec<YamlRepair>,
}

impl Frontmatter {
    /// The parsed properties, after any tab repair.
    #[must_use]
    pub fn record(&self) -> &EvidenceRecord {
        &self.record
    }

    /// The opening fence (after any byte-order mark) through the closing
    /// fence's line terminator.
    #[must_use]
    pub fn block(&self) -> Range<usize> {
        self.block.clone()
    }

    /// The YAML between the fences, including its final line terminator.
    #[must_use]
    pub fn yaml(&self) -> Range<usize> {
        self.yaml.clone()
    }

    /// The opening fence's line terminator.
    #[must_use]
    pub fn fence_line_ending(&self) -> LineEnding {
        self.fence_line_ending
    }

    /// The tab-indentation repair the reader applied in memory, as edits in
    /// document offsets; empty when the YAML parsed as written.
    #[must_use]
    pub fn tab_repair(&self) -> &[YamlRepair] {
        &self.tab_repair
    }
}

/// The result of reading a document that is not malformed.
#[derive(Debug, Clone, PartialEq)]
pub enum ReadOutcome {
    /// The document does not start with a `---` fence. Evaluation applies
    /// the default policy; renewal creates a block.
    NoFrontmatter,
    Found(Frontmatter),
}

/// Why a document's frontmatter could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    NotUtf8,
    /// An opening `---` with no closing `---`. `dot_close` is `true` when a
    /// YAML `...` end marker appears instead.
    Unterminated { dot_close: bool },
    /// The first line is a run of four or more dashes, such as `----`.
    NearMissFence,
    /// A key is defined twice in one mapping. `key` is the parser's name for
    /// it, when reported.
    DuplicateKey { key: Option<String>, message: String },
    Malformed { cause: MalformedCause, message: String },
}

/// The reason frontmatter is malformed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MalformedCause {
    /// Invalid YAML.
    Yaml,
    /// Invalid YAML because of an unquoted `{{ }}` template, which Darkmatter
    /// reads only through its expression protection.
    ExpressionTemplate,
    /// Valid YAML whose root is not a mapping of string keys.
    NotAMapping,
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotUtf8 => f.write_str("the document is not valid UTF-8"),
            Self::Unterminated { dot_close: false } => {
                f.write_str("the frontmatter block has no closing `---` line")
            }
            Self::Unterminated { dot_close: true } => f.write_str(
                "the frontmatter block has no closing `---` line (a YAML `...` end marker \
                 does not close it)",
            ),
            Self::NearMissFence => f.write_str(
                "the first line is a near-miss fence of four or more dashes; frontmatter opens \
                 with exactly `---`",
            ),
            Self::DuplicateKey { message, .. } => {
                write!(f, "the frontmatter defines a key twice: {message}")
            }
            Self::Malformed { cause, message } => match cause {
                MalformedCause::Yaml => write!(f, "malformed frontmatter: {message}"),
                MalformedCause::ExpressionTemplate => write!(
                    f,
                    "malformed frontmatter: an unquoted `{{{{ }}}}` template expression is not \
                     valid YAML; Darkmatter reads it only through its expression protection, \
                     which content-policy does not apply. Quote the value to fix it ({message})"
                ),
                MalformedCause::NotAMapping => write!(f, "malformed frontmatter: {message}"),
            },
        }
    }
}

impl std::error::Error for ReadError {}

#[derive(Clone, Copy)]
struct Line {
    start: usize,
    content_end: usize,
    end: usize,
}

fn lines(source: &str) -> Vec<Line> {
    let bytes = source.as_bytes();
    let (mut out, mut start, mut index) = (Vec::new(), 0, 0);
    while index < bytes.len() {
        let terminator = match bytes[index] {
            b'\n' => 1,
            b'\r' if bytes.get(index + 1) == Some(&b'\n') => 2,
            b'\r' => 1,
            _ => {
                index += 1;
                continue;
            }
        };
        out.push(Line {
            start,
            content_end: index,
            end: index + terminator,
        });
        index += terminator;
        start = index;
    }
    if start < source.len() {
        out.push(Line {
            start,
            content_end: source.len(),
            end: source.len(),
        });
    }
    out
}

fn line_ending(source: &str, line: Line) -> Option<LineEnding> {
    match &source[line.content_end..line.end] {
        "\n" => Some(LineEnding::Lf),
        "\r\n" => Some(LineEnding::CrLf),
        "\r" => Some(LineEnding::Cr),
        _ => None,
    }
}

/// Reads a document's frontmatter.
///
/// ## Errors
///
/// Returns [`ReadError`] for bytes that are not UTF-8, an unterminated block,
/// a near-miss fence, a duplicate key, or YAML that is malformed even after
/// the tab repair.
pub fn read_frontmatter(bytes: &[u8]) -> Result<ReadOutcome, ReadError> {
    let source = std::str::from_utf8(bytes).map_err(|_| ReadError::NotUtf8)?;
    let all = lines(source);
    let content = |line: Line| &source[line.start..line.content_end];
    let Some(&first) = all.first() else {
        return Ok(ReadOutcome::NoFrontmatter);
    };
    let bom = if source.starts_with('\u{feff}') { 3 } else { 0 };
    let opening = content(first)[bom..].trim();
    if opening != "---" {
        if opening.len() >= 4 && opening.bytes().all(|byte| byte == b'-') {
            return Err(ReadError::NearMissFence);
        }
        return Ok(ReadOutcome::NoFrontmatter);
    }
    let Some(close) = (1..all.len()).find(|&index| content(all[index]).trim() == "---") else {
        let dot_close = all.iter().skip(1).any(|&line| content(line).trim() == "...");
        return Err(ReadError::Unterminated { dot_close });
    };
    let yaml = all[1].start..all[close].start;
    let text = &source[yaml.clone()];
    // Parse without the final line terminator, as Darkmatter does, so a
    // clip-chomped block scalar as the last key reads the same in both.
    let parse_text = text
        .strip_suffix("\r\n")
        .or_else(|| text.strip_suffix('\n'))
        .or_else(|| text.strip_suffix('\r'))
        .unwrap_or(text);
    let (map, tab_repair) = match parse_mapping(parse_text) {
        Ok(map) => (map, Vec::new()),
        Err(error @ ReadError::DuplicateKey { .. }) => return Err(error),
        Err(error) => match repair_tabs(parse_text) {
            Some((repaired, repairs)) => {
                let map = parse_mapping(&repaired)?;
                let shifted = repairs
                    .into_iter()
                    .map(|repair| YamlRepair {
                        span: repair.span.start + yaml.start..repair.span.end + yaml.start,
                        ..repair
                    })
                    .collect();
                (map, shifted)
            }
            None if parse_text.contains("{{") => {
                let ReadError::Malformed { message, .. } = error else {
                    return Err(error);
                };
                return Err(ReadError::Malformed {
                    cause: MalformedCause::ExpressionTemplate,
                    message,
                });
            }
            None => return Err(error),
        },
    };
    Ok(ReadOutcome::Found(Frontmatter {
        record: EvidenceRecord::from(map),
        block: bom..all[close].end,
        yaml,
        fence_line_ending: line_ending(source, first).expect("a later line follows the fence"),
        tab_repair,
    }))
}

/// The tab-indentation repair for `text` and the repaired text, when Biscuit
/// File proposes one.
fn repair_tabs(text: &str) -> Option<(String, Vec<YamlRepair>)> {
    let analysis = analyze_yaml(text);
    let repairs: Vec<YamlRepair> = analysis
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == YamlDiagnosticCode::TabIndentation)
        .flat_map(|diagnostic| diagnostic.repairs.iter().cloned())
        .collect();
    if repairs.is_empty() {
        return None;
    }
    let outcome = apply_edit_set(text, &repairs);
    outcome
        .audit
        .rejected
        .is_empty()
        .then_some((outcome.source, repairs))
}

fn parse_mapping(text: &str) -> Result<Map<String, Value>, ReadError> {
    let yaml = biscuit_file::Yaml::from_str(text).map_err(|error| {
        let message = error.to_string();
        if message.contains("duplicate entry") {
            ReadError::DuplicateKey {
                key: duplicate_key_name(&message),
                message,
            }
        } else {
            ReadError::Malformed {
                cause: MalformedCause::Yaml,
                message,
            }
        }
    })?;
    let value = yaml.value();
    if value.is_null() {
        return Ok(Map::new());
    }
    if !value.is_mapping() {
        return Err(ReadError::Malformed {
            cause: MalformedCause::NotAMapping,
            message: "the frontmatter is not a mapping of properties".to_string(),
        });
    }
    Map::<String, Value>::deserialize(value.clone()).map_err(|error| ReadError::Malformed {
        cause: MalformedCause::NotAMapping,
        message: error.to_string(),
    })
}

fn duplicate_key_name(message: &str) -> Option<String> {
    let rest = &message[message.find("with key")? + "with key".len()..];
    let rest = rest.trim_start().strip_prefix('"')?;
    Some(rest[..rest.find('"')?].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(bytes: &[u8]) -> Frontmatter {
        match read_frontmatter(bytes).unwrap() {
            ReadOutcome::Found(frontmatter) => frontmatter,
            ReadOutcome::NoFrontmatter => panic!("no frontmatter"),
        }
    }

    #[test]
    fn spans_cover_the_block_under_lf_crlf_cr_and_bom() {
        for (source, line_ending) in [
            ("---\na: 1\n---\nbody\n", LineEnding::Lf),
            ("---\r\na: 1\r\n---\r\nbody\r\n", LineEnding::CrLf),
            ("---\ra: 1\r---\rbody\r", LineEnding::Cr),
        ] {
            let frontmatter = found(source.as_bytes());
            let nl = line_ending.as_str();
            assert_eq!(frontmatter.fence_line_ending(), line_ending);
            assert_eq!(&source[frontmatter.yaml()], format!("a: 1{nl}"));
            assert_eq!(&source[frontmatter.block()], format!("---{nl}a: 1{nl}---{nl}"));
            assert_eq!(frontmatter.record().get("a"), Some(&Value::from(1)));
        }
        let bom = "\u{feff}---\na: 1\n---\n";
        let frontmatter = found(bom.as_bytes());
        assert_eq!(frontmatter.block(), 3..bom.len());
        assert_eq!(&bom[frontmatter.yaml()], "a: 1\n");
    }

    #[test]
    fn no_frontmatter_differs_from_an_empty_block() {
        assert_eq!(read_frontmatter(b"# Title\n").unwrap(), ReadOutcome::NoFrontmatter);
        assert_eq!(read_frontmatter(b"").unwrap(), ReadOutcome::NoFrontmatter);
        assert_eq!(read_frontmatter("\u{feff}# T\n".as_bytes()).unwrap(), ReadOutcome::NoFrontmatter);
        for empty in ["---\n---\n", "---\n\n---\n", "---\n# only a comment\n---\n"] {
            let frontmatter = found(empty.as_bytes());
            assert_eq!(frontmatter.record(), &EvidenceRecord::new(), "{empty:?}");
        }
    }

    #[test]
    fn fence_failures_are_errors() {
        assert_eq!(
            read_frontmatter(b"---\na: 1\n").unwrap_err(),
            ReadError::Unterminated { dot_close: false }
        );
        assert_eq!(
            read_frontmatter(b"---\na: 1\n...\nbody\n").unwrap_err(),
            ReadError::Unterminated { dot_close: true }
        );
        assert_eq!(read_frontmatter(b"----\na: 1\n----\n").unwrap_err(), ReadError::NearMissFence);
        assert_eq!(read_frontmatter(b"\xff\xfe").unwrap_err(), ReadError::NotUtf8);
    }

    #[test]
    fn duplicate_keys_are_rejected_anywhere_in_the_block() {
        let top = read_frontmatter(b"---\nlast_updated: 2026-01-01\nlast_updated: 2026-02-01\n---\n");
        assert!(
            matches!(&top, Err(ReadError::DuplicateKey { key: Some(key), .. }) if key == "last_updated"),
            "{top:?}"
        );
        let nested = read_frontmatter(
            b"---\ncontent_policy:\n  - rule: ValidFor(3mo)\n    rule: Evergreen\n    action: refresh\n---\n",
        );
        assert!(matches!(nested, Err(ReadError::DuplicateKey { .. })), "{nested:?}");
    }

    #[test]
    fn clip_chomped_block_scalar_as_last_key_has_no_trailing_newline() {
        let frontmatter = found(b"---\ndescription: >\n  folded text\n---\nbody\n");
        assert_eq!(frontmatter.record().get("description"), Some(&Value::from("folded text")));
        let crlf = found(b"---\r\ndescription: >\r\n  folded text\r\n---\r\n");
        assert_eq!(crlf.record().get("description"), Some(&Value::from("folded text")));
    }

    #[test]
    fn tab_indentation_is_repaired_in_memory() {
        let source = "---\nupdate_policy:\n\t- Duration(6mo)\nprompt: |-\n\tLine one\n\t\tLine two\nlast_updated: 2026-02-27\n---\n";
        let frontmatter = found(source.as_bytes());
        assert_eq!(frontmatter.record().get("prompt"), Some(&Value::from("Line one\n  Line two")));
        assert_eq!(
            frontmatter.record().get("update_policy"),
            Some(&serde_json::json!(["Duration(6mo)"]))
        );
        assert_eq!(frontmatter.tab_repair().len(), 3);
        for repair in frontmatter.tab_repair() {
            assert!(source[repair.span.clone()].contains('\t'), "{repair:?}");
        }
        let repaired = apply_edit_set(source, frontmatter.tab_repair()).source;
        assert!(!repaired.contains('\t'));
        assert!(found(repaired.as_bytes()).tab_repair().is_empty());
    }

    #[test]
    fn expression_templates_are_named_in_the_rejection() {
        let error = read_frontmatter(b"---\nprompt: {{ name }} is here\n---\n").unwrap_err();
        assert!(
            matches!(error, ReadError::Malformed { cause: MalformedCause::ExpressionTemplate, .. }),
            "{error:?}"
        );
        assert!(error.to_string().contains("expression protection"));
    }

    #[test]
    fn non_mapping_roots_are_malformed() {
        for source in ["---\n- a\n---\n", "---\njust text\n---\n"] {
            assert!(
                matches!(
                    read_frontmatter(source.as_bytes()),
                    Err(ReadError::Malformed { cause: MalformedCause::NotAMapping, .. })
                ),
                "{source:?}"
            );
        }
        assert!(matches!(
            read_frontmatter(b"---\na: [1\n---\n"),
            Err(ReadError::Malformed { cause: MalformedCause::Yaml, .. })
        ));
    }
}
