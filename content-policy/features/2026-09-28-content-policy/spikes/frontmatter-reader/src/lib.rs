//! Throwaway spike: strict frontmatter reader + span-targeted renewal edits
//! for 2026-09-28-content-policy. Not production code.

use std::ops::Range;

use biscuit_file::yaml::{YamlPathSegment, YamlRepair, apply_edit_set, locate_yaml_key, locate_yaml_value};
use indexmap::IndexMap;
use serde_json::Value;

pub type Record = IndexMap<String, Value>;

#[derive(Debug, Clone)]
pub struct Frontmatter {
    pub record: Record,
    /// Whole block, opening fence (plus BOM) through closing fence terminator.
    pub block_span: Range<usize>,
    /// YAML text between the fences, terminators intact, in file offsets.
    pub yaml_span: Range<usize>,
    pub yaml_text: String,
}

#[derive(Debug, Clone)]
pub enum ReadOutcome {
    NoFrontmatter,
    /// Starts with `---` but has no closing `---`. `dot_close` says a YAML
    /// `...` end marker line was seen instead.
    Unterminated { dot_close: bool },
    /// First line is a dash-only run of 4+ (Darkmatter's near-miss fence).
    NearMissFence,
    Found(Frontmatter),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReadError {
    NotUtf8,
    Duplicate(String),
    Yaml(String),
    NotMapping(String),
}

#[derive(Clone, Copy)]
struct Line {
    start: usize,
    content_end: usize,
    end: usize,
}

fn lines(source: &str) -> Vec<Line> {
    let bytes = source.as_bytes();
    let (mut out, mut start, mut i) = (Vec::new(), 0, 0);
    while i < bytes.len() {
        let term = match bytes[i] {
            b'\n' => 1,
            b'\r' if bytes.get(i + 1) == Some(&b'\n') => 2,
            b'\r' => 1,
            _ => {
                i += 1;
                continue;
            }
        };
        out.push(Line { start, content_end: i, end: i + term });
        i += term;
        start = i;
    }
    if start < source.len() {
        out.push(Line { start, content_end: source.len(), end: source.len() });
    }
    out
}

/// Parses YAML strictly: one pass, duplicate keys rejected recursively, no
/// fallbacks.
pub fn parse_strict(yaml: &str) -> Result<Record, ReadError> {
    if yaml.trim().is_empty() {
        return Ok(Record::new());
    }
    let value = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(yaml).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("duplicate entry with key") {
            ReadError::Duplicate(msg)
        } else {
            ReadError::Yaml(msg)
        }
    })?;
    match value {
        serde_yaml_ng::Value::Null => Ok(Record::new()),
        serde_yaml_ng::Value::Mapping(_) => serde_yaml_ng::from_value::<Record>(value)
            .map_err(|e| ReadError::NotMapping(e.to_string())),
        other => Err(ReadError::NotMapping(format!("root is {other:?}"))),
    }
}

pub fn read_frontmatter(bytes: &[u8]) -> Result<ReadOutcome, ReadError> {
    let source = std::str::from_utf8(bytes).map_err(|_| ReadError::NotUtf8)?;
    let ls = lines(source);
    let content = |l: Line| &source[l.start..l.content_end];
    let Some(&first) = ls.first() else {
        return Ok(ReadOutcome::NoFrontmatter);
    };
    let first_text = content(first).strip_prefix('\u{feff}').unwrap_or(content(first));
    if first_text.trim() != "---" {
        let t = first_text.trim();
        if t.len() >= 4 && t.bytes().all(|b| b == b'-') {
            return Ok(ReadOutcome::NearMissFence);
        }
        return Ok(ReadOutcome::NoFrontmatter);
    }
    let Some(close) = (1..ls.len()).find(|&i| content(ls[i]).trim() == "---") else {
        let dot_close = ls.iter().skip(1).any(|&l| content(l).trim() == "...");
        return Ok(ReadOutcome::Unterminated { dot_close });
    };
    let yaml_span = ls[1].start.min(ls[close].start)..ls[close].start;
    let yaml_text = source[yaml_span.clone()].to_string();
    // Parse without the last line terminator, as Darkmatter does (it joins
    // lines with `\n` and drops the final one). Otherwise a clip-chomped
    // block scalar that is the last key gains a trailing `\n` Darkmatter's
    // map lacks. Set SPIKE_RAW_PARSE=1 to parse the raw slice instead.
    let parse_text = if std::env::var_os("SPIKE_RAW_PARSE").is_some() {
        yaml_text.as_str()
    } else {
        yaml_text.strip_suffix("\r\n").or_else(|| yaml_text.strip_suffix('\n')).or_else(|| yaml_text.strip_suffix('\r')).unwrap_or(&yaml_text)
    };
    let record = parse_strict(parse_text)?;
    Ok(ReadOutcome::Found(Frontmatter {
        record,
        block_span: 0..ls[close].end,
        yaml_span,
        yaml_text,
    }))
}

#[derive(Debug, Clone)]
pub enum Target {
    /// A top-level property such as `last_updated`.
    Property(String),
    /// The inline baseline date of `ValidFor(dur, DATE)` in item `index` of
    /// the policy list under `key` (compact string or `rule:` of a mapping).
    RuleDate { key: String, index: usize },
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditError {
    Read(ReadError),
    NoFrontmatter,
    Unterminated { dot_close: bool },
    NearMissFence,
    TargetAbsent,
    NotAList,
    NotADateRule(String),
    /// locate_yaml_value returned None although the value exists.
    NotLocatable(&'static str),
    BlockScalar,
    DoubleQuotedEscapes,
    DateNotFoundInSource,
    AmbiguousDate,
    KeyExists,
    EditRejected(String),
}

fn frontmatter_of(bytes: &[u8]) -> Result<Frontmatter, EditError> {
    match read_frontmatter(bytes).map_err(EditError::Read)? {
        ReadOutcome::Found(fm) => Ok(fm),
        ReadOutcome::NoFrontmatter => Err(EditError::NoFrontmatter),
        ReadOutcome::NearMissFence => Err(EditError::NearMissFence),
        ReadOutcome::Unterminated { dot_close } => Err(EditError::Unterminated { dot_close }),
    }
}

/// Located edit: file-offset span of the bytes to replace and the text.
#[derive(Debug, Clone)]
pub struct PlannedEdit {
    pub span: Range<usize>,
    pub replacement: String,
    /// The value span `locate_yaml_value` returned, in file offsets.
    pub located: Option<Range<usize>>,
}

fn why_unlocatable(fm: &Frontmatter, key: &str) -> &'static str {
    // Lexical sniff of the top-level `key:` line for the refusal reason.
    for line in fm.yaml_text.lines() {
        if let Some(rest) = line.strip_prefix(key).and_then(|r| r.trim_start().strip_prefix(':')) {
            let rest = rest.trim_start();
            if rest.starts_with('[') || rest.starts_with('{') {
                return "flow collection";
            }
            if rest.starts_with('|') || rest.starts_with('>') {
                return "block scalar";
            }
        }
    }
    "multi-line or unrepresentable value"
}

pub fn plan_edit_date(bytes: &[u8], target: &Target, new_date: &str) -> Result<PlannedEdit, EditError> {
    let fm = frontmatter_of(bytes)?;
    let base = fm.yaml_span.start;
    let yaml = fm.yaml_text.as_str();
    match target {
        Target::Property(name) => {
            let current = fm.record.get(name).ok_or(EditError::TargetAbsent)?;
            let path = [YamlPathSegment::Key(name.clone())];
            match locate_yaml_value(yaml, &path) {
                Some(loc) => {
                    let text = &yaml[loc.span.clone()];
                    if text.starts_with('|') || text.starts_with('>') {
                        return Err(EditError::BlockScalar);
                    }
                    let replacement = match text.as_bytes()[0] {
                        b'\'' => format!("'{new_date}'"),
                        b'"' => format!("\"{new_date}\""),
                        _ => new_date.to_string(),
                    };
                    let span = base + loc.span.start..base + loc.span.end;
                    Ok(PlannedEdit { span: span.clone(), replacement, located: Some(span) })
                }
                None if current.is_null() => {
                    // `last_updated:` with no value: insert after the colon.
                    let key = locate_yaml_key(yaml, &path)
                        .ok_or(EditError::NotLocatable("null value, key not locatable"))?;
                    let colon = yaml[key.end..].find(':').ok_or(EditError::NotLocatable("no colon"))?;
                    let at = base + key.end + colon + 1;
                    Ok(PlannedEdit { span: at..at, replacement: format!(" {new_date}"), located: None })
                }
                None => Err(EditError::NotLocatable(why_unlocatable(&fm, name))),
            }
        }
        Target::RuleDate { key, index } => {
            let list = fm.record.get(key).ok_or(EditError::TargetAbsent)?;
            let item = list.as_array().ok_or(EditError::NotAList)?.get(*index).ok_or(EditError::TargetAbsent)?;
            let mut path = vec![YamlPathSegment::Key(key.clone()), YamlPathSegment::Index(*index)];
            let rule = match item {
                Value::String(s) => s.clone(),
                Value::Object(m) => {
                    path.push(YamlPathSegment::Key("rule".into()));
                    m.get("rule").and_then(Value::as_str).ok_or(EditError::NotADateRule(item.to_string()))?.to_string()
                }
                other => return Err(EditError::NotADateRule(other.to_string())),
            };
            let old_date = valid_for_inline_date(&rule).ok_or_else(|| EditError::NotADateRule(rule.clone()))?;
            let loc = locate_yaml_value(yaml, &path).ok_or_else(|| EditError::NotLocatable(why_unlocatable(&fm, key)))?;
            let text = &yaml[loc.span.clone()];
            if text.starts_with('|') || text.starts_with('>') {
                return Err(EditError::BlockScalar);
            }
            if text.starts_with('"') && text.contains('\\') {
                return Err(EditError::DoubleQuotedEscapes);
            }
            let hits: Vec<usize> = text.match_indices(old_date.as_str()).map(|(i, _)| i).collect();
            let at = match hits.as_slice() {
                [] => return Err(EditError::DateNotFoundInSource),
                [one] => *one,
                _ => return Err(EditError::AmbiguousDate),
            };
            let start = base + loc.span.start + at;
            Ok(PlannedEdit {
                span: start..start + old_date.len(),
                replacement: new_date.to_string(),
                located: Some(base + loc.span.start..base + loc.span.end),
            })
        }
    }
}

/// Returns the inline baseline date of `ValidFor(dur, YYYY-MM-DD)`.
pub fn valid_for_inline_date(rule: &str) -> Option<String> {
    let args = rule.trim().strip_prefix("ValidFor(")?.strip_suffix(')')?;
    let (_, baseline) = args.split_once(',')?;
    let baseline = baseline.trim();
    let b = baseline.as_bytes();
    (b.len() == 10 && b[4] == b'-' && b[7] == b'-' && baseline.bytes().filter(u8::is_ascii_digit).count() == 8)
        .then(|| baseline.to_string())
}

fn apply(source: &str, edit: &PlannedEdit) -> Result<Vec<u8>, EditError> {
    let outcome = apply_edit_set(
        source,
        &[YamlRepair { span: edit.span.clone(), replacement: edit.replacement.clone(), explanation: "renew".into() }],
    );
    if let Some(r) = outcome.audit.rejected.first() {
        return Err(EditError::EditRejected(format!("{:?}", r.reason)));
    }
    Ok(outcome.source.into_bytes())
}

pub fn edit_date(bytes: &[u8], target: &Target, new_date: &str) -> Result<Vec<u8>, EditError> {
    let edit = plan_edit_date(bytes, target, new_date)?;
    let source = std::str::from_utf8(bytes).map_err(|_| EditError::Read(ReadError::NotUtf8))?;
    apply(source, &edit)
}

/// Terminator of the line ending at or containing `offset - 1`, i.e. the
/// line an insertion at `offset` follows.
fn terminator_before(source: &str, offset: usize) -> Option<&'static str> {
    let before = &source.as_bytes()[..offset];
    match before {
        [.., b'\r', b'\n'] => Some("\r\n"),
        [.., b'\n'] => Some("\n"),
        [.., b'\r'] => Some("\r"),
        _ => None,
    }
}

fn first_terminator(source: &str) -> &'static str {
    lines(source)
        .first()
        .map(|l| match l.end - l.content_end {
            2 => "\r\n",
            1 if source.as_bytes()[l.content_end] == b'\r' => "\r",
            _ => "\n",
        })
        .unwrap_or("\n")
}

/// Appends `key: value` as the last line of the block, or creates a block.
/// `value_yaml` must already be a YAML scalar in source form.
pub fn insert_property(bytes: &[u8], key: &str, value_yaml: &str) -> Result<Vec<u8>, EditError> {
    let source = std::str::from_utf8(bytes).map_err(|_| EditError::Read(ReadError::NotUtf8))?;
    let edit = match read_frontmatter(bytes).map_err(EditError::Read)? {
        ReadOutcome::Found(fm) => {
            if fm.record.contains_key(key) {
                return Err(EditError::KeyExists);
            }
            let at = fm.yaml_span.end;
            // The newline of the line the new one follows (last YAML line, or
            // the opening fence for an empty block): local style, not global.
            let nl = terminator_before(source, at).unwrap_or("\n");
            PlannedEdit { span: at..at, replacement: format!("{key}: {value_yaml}{nl}"), located: None }
        }
        ReadOutcome::NoFrontmatter => {
            let nl = first_terminator(source);
            // A BOM stays first in the file.
            let at = if source.starts_with('\u{feff}') { 3 } else { 0 };
            PlannedEdit { span: at..at, replacement: format!("---{nl}{key}: {value_yaml}{nl}---{nl}"), located: None }
        }
        ReadOutcome::NearMissFence => return Err(EditError::NearMissFence),
        ReadOutcome::Unterminated { dot_close } => return Err(EditError::Unterminated { dot_close }),
    };
    apply(source, &edit)
}

/// Renewal of a property: edit in place when present, insert when absent.
pub fn set_date_property(bytes: &[u8], key: &str, new_date: &str) -> Result<Vec<u8>, EditError> {
    match edit_date(bytes, &Target::Property(key.into()), new_date) {
        Err(EditError::TargetAbsent | EditError::NoFrontmatter) => insert_property(bytes, key, new_date),
        other => other,
    }
}

/// Minimal byte diff: (old changed range, new changed range).
pub fn byte_diff(old: &[u8], new: &[u8]) -> (Range<usize>, Range<usize>) {
    let prefix = old.iter().zip(new).take_while(|(a, b)| a == b).count();
    let max_suffix = old.len().min(new.len()) - prefix;
    let suffix = old.iter().rev().zip(new.iter().rev()).take(max_suffix).take_while(|(a, b)| a == b).count();
    (prefix..old.len() - suffix, prefix..new.len() - suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_only_the_date_under_crlf() {
        let src = b"---\r\nlast_updated: '2026-01-01' # c\r\n---\r\nbody\r\n";
        let out = edit_date(src, &Target::Property("last_updated".into()), "2026-09-28").unwrap();
        assert_eq!(out, b"---\r\nlast_updated: '2026-09-28' # c\r\n---\r\nbody\r\n");
    }

    #[test]
    fn rule_date_in_rule_mapping() {
        let src = b"---\ncontent_policy:\n  - rule: ValidFor(3mo, 2026-01-01)\n    action: archive\n---\n";
        let target = Target::RuleDate { key: "content_policy".into(), index: 0 };
        let out = edit_date(src, &target, "2026-09-28").unwrap();
        assert_eq!(out, b"---\ncontent_policy:\n  - rule: ValidFor(3mo, 2026-09-28)\n    action: archive\n---\n");
    }

    #[test]
    fn duplicate_keys_rejected() {
        assert!(matches!(read_frontmatter(b"---\na: 1\na: 2\n---\n"), Err(ReadError::Duplicate(_))));
    }

    /// Pins observed `locate_yaml_value` behavior on shapes its docs say
    /// yield `None` (multi-line) or that the scanner cannot parent
    /// (zero-indent sequences).
    #[test]
    fn locate_probe() {
        use biscuit_file::yaml::YamlPathSegment::{Index, Key};
        let k = |s: &str| Key(s.to_string());
        let loc = |src: &str, path: &[YamlPathSegment]| locate_yaml_value(src, path).map(|l| src[l.span].to_string());
        // Multi-line plain mapping value: first line only is returned.
        assert_eq!(loc("a: one\n  two\n", &[k("a")]).as_deref(), Some("one"));
        // Multi-line plain sequence entry: first line only.
        assert_eq!(loc("p:\n  - V(3mo,\n    x)\n", &[k("p"), Index(0)]).as_deref(), Some("V(3mo,"));
        // Multi-line double-quoted: first line fragment.
        let dq = loc("a: \"one\n  two\"\n", &[k("a")]);
        eprintln!("multi-line double-quoted -> {dq:?}");
        // Zero-indent (compact) sequence under a key: not found.
        assert_eq!(loc("p:\n- V(3mo)\n", &[k("p"), Index(0)]), None);
        assert_eq!(loc("p:\n- rule: V(3mo)\n  action: a\n", &[k("p"), Index(0), k("rule")]), None);
        // ...and it is reported at the root-sequence path instead.
        assert_eq!(loc("p:\n- V(3mo)\n", &[Index(0)]).as_deref(), Some("V(3mo)"));
        // Block scalar indicator is returned as the value.
        assert_eq!(loc("a: >-\n  text\n", &[k("a")]).as_deref(), Some(">-"));
    }
}
