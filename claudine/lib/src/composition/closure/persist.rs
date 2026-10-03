//! Persisting the agent's frontmatter edit: narrow YAML repair before the
//! closure restores its owned properties, and literal-token encoding after.
//!
//! An inline agent writes YAML by hand. Two things can go wrong with what it
//! writes, and each has one fix here:
//!
//! - **It is not the YAML it meant.** `title: Fix: colons` does not parse, and
//!   `note: see issue #42` parses as `see issue`. [`repair_agent_frontmatter`]
//!   quotes such a value when the agent added or changed it (spec R4).
//! - **It reads as an instruction on the next run.** `summary: fixed {{…}}`
//!   would be scanned as a template. [`encode_agent_values`] stores such a
//!   value as a Darkmatter literal token, which composition decodes to the
//!   exact text as data (spec R3).
//!
//! Both are text edits on exact source bytes; the document is never
//! re-serialized, and nothing here writes to disk.

use biscuit_file::serde_yaml_ng;
use darkmatter::markdown::extract_frontmatter_block;
use darkmatter::markdown::hash::{
    FrontmatterDelta, FrontmatterDeltaEntry, FrontmatterPathSegment, LeafLocateError,
    locate_frontmatter_leaves,
};
use darkmatter::markdown::literal_token::{encode, encode_yaml_scalar};
use serde_json::Value;

use super::CLOSURE_OWNED_PROPERTIES;

/// Why the agent's frontmatter edit cannot be saved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentFrontmatterRejection {
    /// 1-based line in the agent's document, when one line is at fault.
    pub line: Option<usize>,
    /// Dotted path of the value at fault, when there is one.
    pub property: Option<String>,
    /// What is wrong, in the reader's terms.
    pub reason: String,
    /// Whether the pre-run document shows the agent wrote the line at fault.
    /// `false` means the comparison cannot tell, not that the author did.
    pub agent_edit: bool,
}

impl std::fmt::Display for AgentFrontmatterRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(line) = self.line {
            write!(f, "line {line}: ")?;
        }
        if let Some(property) = &self.property {
            write!(f, "`{property}`: ")?;
        }
        f.write_str(&self.reason)?;
        if self.agent_edit {
            f.write_str(" (written by the agent during this run)")?;
        }
        Ok(())
    }
}

/// Why [`encode_agent_values`] could not store the agent's values.
#[derive(Debug)]
pub enum EncodeError {
    /// An agent-owned value cannot be stored as a literal token.
    Rejected(AgentFrontmatterRejection),
    /// The frontmatter is not a block mapping the locator can read.
    Frontmatter(darkmatter::markdown::MarkdownError),
}

/// Quotes agent-written plain scalars that YAML would misread, then checks the
/// whole frontmatter parses.
///
/// Only a **top-level** key the agent added or changed (its source lines
/// differ from `original`'s), whose value is a single-line plain scalar, and
/// which is not a closure-owned property, is ever touched. Its complete source
/// value, `#` text and `: ` included, is wrapped in double quotes when it
/// contains `: ` or ` #`, ends with `:`, or starts with a reserved indicator
/// (`%`, `@`, or a backtick). Numbers, booleans, and nulls (optionally followed
/// by a comment) are left alone, and so is every value that opens another
/// YAML form (quoted or block scalar, flow collection, anchor, alias, tag,
/// comment, `- `, `? `, `: `), valid or not: a malformed one is rejected, never
/// turned into text. Line endings are kept.
///
/// A document without frontmatter is returned unchanged.
///
/// ## Errors
///
/// Returns an [`AgentFrontmatterRejection`] naming the line when the
/// frontmatter has no closing delimiter, or still does not parse as a mapping
/// after the repair: a duplicate key, bad nesting, a malformed quoted scalar or
/// flow collection, or other syntax outside the repair case. Nothing is partially repaired. A near-miss fence is returned
/// unrepaired for the restore step to report.
pub fn repair_agent_frontmatter(
    candidate: &str,
    original: &str,
) -> Result<String, AgentFrontmatterRejection> {
    let original_lines = frontmatter_lines(original);
    // A malformed fence is not a repair case; restoring reports it, typed.
    let Ok(extraction) = extract_frontmatter_block(candidate) else {
        return Ok(candidate.to_string());
    };
    let Some(extraction) = extraction else {
        let opens = candidate
            .lines()
            .next()
            .is_some_and(|line| line.trim_start_matches('\u{feff}').trim() == "---");
        if opens {
            return Err(AgentFrontmatterRejection {
                line: Some(1),
                property: None,
                reason: "the frontmatter has no closing `---` line".to_string(),
                agent_edit: original_lines.is_some(),
            });
        }
        return Ok(candidate.to_string());
    };

    let original_nodes = original_lines
        .as_deref()
        .map(top_level_nodes)
        .unwrap_or_default();
    let mut repaired = candidate.to_string();
    let yaml_start = extraction.yaml_span.start;
    // Descending order keeps earlier offsets valid while splicing.
    for node in top_level_nodes(extraction.yaml).into_iter().rev() {
        if !node.single_line || is_owned(&node.key) {
            continue;
        }
        let unchanged = original_nodes
            .iter()
            .any(|previous| previous.key == node.key && previous.text == node.text);
        if unchanged {
            continue;
        }
        let value = &extraction.yaml[node.value.clone()];
        if let Some(reason) = malformed_structure(value) {
            let line = document_line(candidate, yaml_start + node.value.start);
            return Err(AgentFrontmatterRejection {
                line: Some(line),
                property: Some(semantic_key(&node.key)),
                reason,
                agent_edit: true,
            });
        }
        if needs_quoting(value) {
            let range = yaml_start + node.value.start..yaml_start + node.value.end;
            repaired.replace_range(range, &double_quoted(value));
        }
    }

    reparse(&repaired, original_lines.as_deref())?;
    Ok(repaired)
}

/// Stores each agent-owned string value that could act as an instruction as a
/// Darkmatter literal token.
///
/// `delta` is the agent's semantic change against the pre-run document
/// (closure-owned properties excluded), from the same parse as `restored`.
/// The agent owns every string leaf that is new or changed by value; inside a
/// new container, every string leaf. Of those, only a value containing `{{`
/// or `$(` is encoded: Darkmatter reads frontmatter text as an instruction
/// through those two sequences only (ruling N9). Unchanged values, stored
/// tokens included, keep their bytes; mapping keys are never encoded.
///
/// The token payload is the text composition reads at that leaf, so a later
/// run decodes exactly what the agent wrote.
///
/// ## Errors
///
/// Returns [`EncodeError::Rejected`] naming the value and its line when an
/// owned leaf has no exact source span (an anchor, alias, tag, `<<` merge,
/// plain flow-collection item, or nested sequence), and
/// [`EncodeError::Frontmatter`] when the frontmatter cannot be read as a block
/// mapping. The whole document is never encoded as a fallback.
pub fn encode_agent_values(
    restored: &str,
    delta: &FrontmatterDelta,
) -> Result<String, EncodeError> {
    let mut paths = Vec::new();
    for entry in &delta.entries {
        let (property, value, previous) = match entry {
            FrontmatterDeltaEntry::Addition { property, value } => (property, value, None),
            FrontmatterDeltaEntry::Replacement {
                property,
                previous_value,
                value,
            } => (property, value, Some(previous_value)),
            FrontmatterDeltaEntry::Deletion { .. } => continue,
        };
        let mut path = vec![FrontmatterPathSegment::Key(property.clone())];
        owned_instruction_leaves(value, previous, &mut path, &mut paths);
    }
    if paths.is_empty() {
        return Ok(restored.to_string());
    }

    let mut spans = locate_frontmatter_leaves(restored, &paths).map_err(|error| match error {
        LeafLocateError::Unlocated(unlocated) => EncodeError::Rejected(AgentFrontmatterRejection {
            line: Some(unlocated.line),
            property: Some(unlocated.dotted_path()),
            reason: format!(
                "an agent-written value holding `{{{{` or `$(` must be stored as a literal token, but {}",
                unlocated.reason
            ),
            agent_edit: true,
        }),
        LeafLocateError::Document(error) => EncodeError::Frontmatter(error),
    })?;
    spans.sort_by_key(|span| std::cmp::Reverse(span.range.start));

    let mut encoded = restored.to_string();
    let mut expected = parse_frontmatter(restored);
    for span in &spans {
        encoded.replace_range(span.range.clone(), &encode_yaml_scalar(&span.decoded));
        if let Some(slot) = expected.as_mut().and_then(|tree| value_at_mut(tree, &span.path)) {
            *slot = Value::String(encode(&span.decoded));
        }
    }
    // The splice must change exactly the owned leaves.
    if expected.is_none() || parse_frontmatter(&encoded) != expected {
        return Err(EncodeError::Rejected(AgentFrontmatterRejection {
            line: None,
            property: None,
            reason: "storing the agent's values as literal tokens would change other values"
                .to_string(),
            agent_edit: false,
        }));
    }
    Ok(encoded)
}

/// The encoding gate (ruling N9): the only two sequences through which
/// Darkmatter reads frontmatter text as an instruction.
fn could_instruct(text: &str) -> bool {
    text.contains("{{") || text.contains("$(")
}

/// `value` as composition reads it: each stored literal token replaced by the
/// text it holds. For readers of loaded frontmatter outside composition
/// (schema judgment, sequence sources, displays); never hand the result back
/// to composition as authored text. A malformed token stays raw for
/// composition to report.
pub(crate) fn stored_text(value: &Value) -> Value {
    darkmatter::markdown::literal_token::decode_literal_tokens(value)
        .unwrap_or_else(|_| value.clone())
}

/// Whether `text` begins a stored literal token (`{{!data:`).
pub(crate) fn opens_stored_token(text: &str) -> bool {
    text.starts_with(darkmatter::markdown::literal_token::TOKEN_PREFIX)
}

/// `value`, ready to be written into frontmatter as data: each string leaf
/// that passes the closure's encoding gate becomes a literal token, so a later
/// preparation reads the text back instead of scanning it.
///
/// For writers that serialize a value (the lifecycle frontmatter effects),
/// where every written value is the product of an evaluation. Mapping keys are
/// never encoded.
pub(crate) fn persisted_data(value: &Value) -> Value {
    match value {
        Value::String(text) if could_instruct(text) => Value::String(encode(text)),
        Value::Array(items) => Value::Array(items.iter().map(persisted_data).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, item)| (key.clone(), persisted_data(item)))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn owned_instruction_leaves(
    value: &Value,
    previous: Option<&Value>,
    path: &mut Vec<FrontmatterPathSegment>,
    out: &mut Vec<Vec<FrontmatterPathSegment>>,
) {
    match (value, previous) {
        (Value::String(text), previous) => {
            if previous != Some(value) && could_instruct(text) {
                out.push(path.clone());
            }
        }
        (Value::Object(map), previous) => {
            let previous = previous.and_then(Value::as_object);
            for (key, child) in map {
                path.push(FrontmatterPathSegment::Key(key.clone()));
                owned_instruction_leaves(child, previous.and_then(|p| p.get(key)), path, out);
                path.pop();
            }
        }
        (Value::Array(items), previous) => {
            let previous = previous.and_then(Value::as_array);
            for (index, child) in items.iter().enumerate() {
                path.push(FrontmatterPathSegment::Index(index));
                owned_instruction_leaves(child, previous.and_then(|p| p.get(index)), path, out);
                path.pop();
            }
        }
        _ => {}
    }
}

fn value_at_mut<'a>(tree: &'a mut Value, path: &[FrontmatterPathSegment]) -> Option<&'a mut Value> {
    path.iter().try_fold(tree, |node, segment| match segment {
        FrontmatterPathSegment::Key(key) => node.get_mut(key.as_str()),
        FrontmatterPathSegment::Index(index) => node.get_mut(*index),
    })
}

/// The frontmatter as the closure's other steps read it: the whole YAML block.
fn parse_frontmatter(document: &str) -> Option<Value> {
    let extraction = extract_frontmatter_block(document).ok()??;
    if extraction.yaml.trim().is_empty() {
        return Some(Value::Object(serde_json::Map::new()));
    }
    serde_yaml_ng::from_str(extraction.yaml).ok()
}

fn is_owned(key: &str) -> bool {
    CLOSURE_OWNED_PROPERTIES.contains(&semantic_key(key).as_str())
}

/// The key as YAML reads it (`"prompt"` and `prompt` are the same key).
fn semantic_key(raw: &str) -> String {
    match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(raw) {
        Ok(serde_yaml_ng::Value::String(key)) => key,
        _ => raw.to_string(),
    }
}

/// The frontmatter text between the delimiters, or `None` without frontmatter.
fn frontmatter_lines(document: &str) -> Option<String> {
    extract_frontmatter_block(document)
        .ok()
        .flatten()
        .map(|extraction| extraction.yaml.to_string())
}

/// One top-level `key: value` node, located lexically so it works on text
/// that does not parse.
#[derive(Debug)]
struct TopNode {
    /// The key's source text, trimmed.
    key: String,
    /// The node's lines with line terminators removed, for comparison.
    text: String,
    /// The value's byte range within the YAML text, trailing space excluded.
    value: std::ops::Range<usize>,
    /// Whether no line after the key line belongs to the node.
    single_line: bool,
}

fn top_level_nodes(yaml: &str) -> Vec<TopNode> {
    let mut nodes: Vec<TopNode> = Vec::new();
    let mut offset = 0;
    for raw in yaml.split_inclusive('\n') {
        let start = offset;
        offset += raw.len();
        let line = raw.trim_end_matches(['\n', '\r']);
        let top_level = !line.is_empty()
            && !line.starts_with([' ', '\t', '#'])
            && line != "-"
            && !line.starts_with("- ");
        if !top_level {
            if let Some(node) = nodes.last_mut()
                && !line.trim().is_empty()
                && !is_comment_line(line, &yaml[node.value.clone()])
            {
                node.single_line = false;
                node.text.push('\n');
                node.text.push_str(line);
            }
            continue;
        }
        let Some(colon) = mapping_colon(line) else {
            // Not a `key:` line; the reparse reports it.
            continue;
        };
        let after = &line[colon + 1..];
        let leading = after.len() - after.trim_start().len();
        let value_start = start + colon + 1 + leading;
        let value_end = start + line.trim_end().len().max(colon + 1 + leading);
        nodes.push(TopNode {
            key: line[..colon].trim().to_string(),
            text: line.to_string(),
            value: value_start..value_end,
            single_line: true,
        });
    }
    nodes
}

/// Whether `line`, below a node whose key line holds `value`, is a comment
/// rather than part of that value. An indented `#` line is content only when
/// the value opens a quoted or block scalar, which may continue onto it.
fn is_comment_line(line: &str, value: &str) -> bool {
    line.starts_with('#')
        || (line.trim_start().starts_with('#') && !value.starts_with(['"', '\'', '|', '>']))
}

/// The first `:` outside a quoted key that is followed by a space or ends the
/// line; any other `:` is content of a plain key (`a:b: c`).
fn mapping_colon(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut quote = None;
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        match (quote, byte) {
            (Some(b'"'), b'\\') => index += 1,
            (Some(b'\''), b'\'') if bytes.get(index + 1) == Some(&b'\'') => index += 1,
            (Some(active), current) if active == current => quote = None,
            (None, b'"' | b'\'') if index == 0 => quote = Some(byte),
            (None, b':') if matches!(bytes.get(index + 1), None | Some(b' ' | b'\t')) => {
                return Some(index);
            }
            _ => {}
        }
        index += 1;
    }
    None
}

/// Whether an agent-written single-line value must be quoted to read as the
/// text it spells (ruling N12, narrowed by review 1 of the fix).
///
/// A value that opens a structured YAML form is never quoted, valid or not
/// (spec R4): quoting a malformed one would silently turn the structure the
/// agent attempted into a string, so it is left for the reparse to reject.
fn needs_quoting(value: &str) -> bool {
    if value.is_empty() || opens_structure(value) || is_core_scalar(before_comment(value)) {
        return false;
    }
    match value.chars().next() {
        // Reserved indicators: no YAML form starts with one, so the value can
        // only be text.
        Some('%' | '@' | '`') => true,
        _ => {
            value.contains(": ")
                || value.contains(" #")
                || value.contains("\t#")
                || value.ends_with(':')
        }
    }
}

/// Whether `value` starts with an indicator that commits it to a YAML form
/// other than a plain scalar: a quoted or block scalar, a flow collection, an
/// anchor, alias, or tag, a comment, or a block sequence entry, complex key,
/// or mapping value (`- `, `? `, `: `).
fn opens_structure(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some('"' | '\'' | '[' | '{' | '|' | '>' | '&' | '*' | '!' | '#') => true,
        Some('-' | '?' | ':') => matches!(chars.next(), None | Some(' ' | '\t')),
        _ => false,
    }
}

/// Why an agent-written single-line value that opens a structured YAML form
/// cannot be read, or `None` when it reads (or is plain). The value is judged
/// alone, as the closure's reparse would read it, so the rejection names its
/// own line rather than wherever the parser gave up. An alias is not judged:
/// its anchor lives elsewhere in the document, and the reparse reports it.
fn malformed_structure(value: &str) -> Option<String> {
    if !opens_structure(value) || value.starts_with('*') {
        return None;
    }
    let parsed = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&format!("k: {value}"))
        .and_then(serde_yaml_ng::from_value::<Value>);
    parsed.is_err().then(|| {
        let form = match value.chars().next() {
            Some('"') => "a double-quoted string",
            Some('\'') => "a single-quoted string",
            Some('[') => "a flow sequence",
            Some('{') => "a flow mapping",
            Some('|' | '>') => "a block scalar",
            Some('&') => "an anchor",
            Some('!') => "a tag",
            Some('-') => "a sequence entry",
            Some('?') => "a complex key",
            Some(':') => "a mapping value",
            _ => "a YAML structure",
        };
        format!(
            "the value starts {form} that is not valid YAML, so it is not saved as text; \
             complete it, or quote the whole value if it is text"
        )
    })
}

/// The 1-based line of byte `offset` in `document`.
fn document_line(document: &str, offset: usize) -> usize {
    document[..offset].matches('\n').count() + 1
}

/// The part of a plain value before a ` #` comment, trimmed.
fn before_comment(value: &str) -> &str {
    let end = [" #", "\t#"]
        .iter()
        .filter_map(|marker| value.find(marker))
        .min()
        .unwrap_or(value.len());
    value[..end].trim_end()
}

/// YAML 1.2 core-schema null, boolean, integer, and float spellings.
fn is_core_scalar(text: &str) -> bool {
    const WORDS: &[&str] = &[
        "~", "null", "Null", "NULL", "true", "True", "TRUE", "false", "False", "FALSE", ".nan",
        ".NaN", ".NAN",
    ];
    if WORDS.contains(&text) {
        return true;
    }
    let unsigned = text.strip_prefix(['-', '+']).unwrap_or(text);
    if matches!(unsigned, ".inf" | ".Inf" | ".INF") {
        return true;
    }
    if let Some(octal) = text.strip_prefix("0o") {
        return !octal.is_empty() && octal.bytes().all(|b| (b'0'..=b'7').contains(&b));
    }
    if let Some(hex) = text.strip_prefix("0x") {
        return !hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit());
    }
    let (mantissa, exponent) = match unsigned.find(['e', 'E']) {
        Some(at) => (&unsigned[..at], Some(&unsigned[at + 1..])),
        None => (unsigned, None),
    };
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    let mantissa_ok = match mantissa.split_once('.') {
        Some((whole, fraction)) => {
            digits(whole) && digits(fraction) && !(whole.is_empty() && fraction.is_empty())
        }
        None => !mantissa.is_empty() && digits(mantissa),
    };
    let exponent_ok = exponent.is_none_or(|exponent| {
        let exponent = exponent.strip_prefix(['-', '+']).unwrap_or(exponent);
        !exponent.is_empty() && digits(exponent)
    });
    mantissa_ok && exponent_ok
}

fn double_quoted(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\t' => out.push(ch),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Parses the repaired frontmatter as the closure will, attributing a failure
/// to its line.
fn reparse(document: &str, original_yaml: Option<&str>) -> Result<(), AgentFrontmatterRejection> {
    let Ok(Some(extraction)) = extract_frontmatter_block(document) else {
        return Ok(());
    };
    let yaml_first_line = document[..extraction.yaml_span.start].matches('\n').count() + 1;
    let reject = |line: Option<usize>, reason: String| {
        let written_by_agent = line.and_then(|line| {
            let text = document.lines().nth(line - 1)?.trim_end_matches('\r');
            let original = original_yaml?;
            Some(!original.lines().any(|old| old.trim_end_matches('\r') == text))
        });
        AgentFrontmatterRejection {
            line,
            property: None,
            reason,
            agent_edit: written_by_agent.unwrap_or(false),
        }
    };
    if extraction.yaml.trim().is_empty() {
        return Ok(());
    }
    // A duplicate top-level key is located here: the parser reports only the
    // mapping that holds it.
    let mut seen = std::collections::HashSet::new();
    let mut offset = 0;
    for raw in extraction.yaml.split_inclusive('\n') {
        let line_start = offset;
        offset += raw.len();
        if raw.starts_with([' ', '\t', '#', '-']) {
            continue;
        }
        if let Some(colon) = mapping_colon(raw.trim_end_matches(['\n', '\r']))
            && !seen.insert(semantic_key(raw[..colon].trim()))
        {
            let line = yaml_first_line + extraction.yaml[..line_start].matches('\n').count();
            return Err(reject(
                Some(line),
                format!("the key `{}` appears more than once", semantic_key(raw[..colon].trim())),
            ));
        }
    }
    // `serde_yaml_ng::Value` refuses a duplicate key; a JSON map would keep
    // the last one.
    let as_yaml = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(extraction.yaml);
    match as_yaml.and_then(|yaml| serde_yaml_ng::from_value::<Value>(yaml.clone()).map(|_| yaml)) {
        Ok(serde_yaml_ng::Value::Mapping(_) | serde_yaml_ng::Value::Null) => Ok(()),
        Ok(_) => Err(reject(
            Some(yaml_first_line),
            "the frontmatter is not a mapping of `key: value` properties".to_string(),
        )),
        Err(error) => {
            let line = error
                .location()
                .map(|location| yaml_first_line + location.line().saturating_sub(1));
            Err(reject(line, format!("the frontmatter is not valid YAML: {error}")))
        }
    }
}

#[cfg(test)]
mod tests;
