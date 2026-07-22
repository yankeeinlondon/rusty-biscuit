//! Effective-schema assembly for the frontmatter overlay.
//!
//! Reproduces the compose precedence (design "Extension baselines"): the
//! Darkmatter base baseline by default, extension baselines from
//! [`DmlsConfig`] whose activation globs match the document merged over it
//! (Claudine is pure data — a `claudine.yaml` path plus `.claude/**` globs,
//! zero Claudine-specific code), and the document's own `$schema` on top. The
//! library ([`darkmatter::markdown::schemas`]) remains the semantic authority;
//! this module only decides *which* baselines apply and hands the layered
//! result to [`DarkmatterSchemas::effective_for`].

use std::collections::BTreeSet;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::compose::find_git_root_from;
use darkmatter::markdown::schemas::resolve::{merge_baseline, resolve_schema};
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, EffectiveSchema, PatternKey, PropertyAtom, PropertyDef, SchemaArm,
    SchemaError, SchemaShape, SchemaSourceMap, SchemaSourcePath, SchemaSpanKind, SimplifiedSchema,
    SimplifiedType, StandaloneSchemaDocument, StandaloneSchemaEnvelope, TypeExpr,
    darkmatter_base_json_schema_ref, darkmatter_base_schema, triggers::TriggerRegistry,
};
use globset::{Glob, GlobSetBuilder};
use serde_json::Value;

use crate::config::{DmlsConfig, SchemaExtensionConfig};

use super::{FrontmatterAst, SchemaOutcome};

/// The semantic schema grammar activated for one authored value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaSchemaKind {
    /// One complete SimplifiedSchema property definition.
    TypeDefinition,
    /// One complete `$schema` declaration.
    Schema,
}

/// One frontmatter value whose effective definition activates meta-schema
/// intelligence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontmatterSchemaValue {
    /// RFC 6901 pointer to the authored value.
    pub pointer: String,
    /// Every semantic meta-type arm on the effective property definition.
    pub kinds: Vec<MetaSchemaKind>,
    /// Document-relative span of the complete authored value.
    pub value_span: Range<usize>,
}

/// One complete property definition classified for future semantic consumers.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticTypeRegion {
    /// The semantic role of the complete authored value.
    pub kind: MetaSchemaKind,
    /// Authored property name.
    pub name: String,
    /// Structural path shared with the passive schema source map.
    pub path: SchemaSourcePath,
    /// Exact authored mapping-key span.
    pub key_span: Range<usize>,
    /// Exact authored complete-definition span.
    pub definition_span: Range<usize>,
    /// Parsed definition denoted by the authored value.
    pub definition: PropertyDef,
}

/// Projects every complete definition in a parsed schema as a semantic region.
pub fn semantic_type_regions(
    schema: &SimplifiedSchema,
    source_map: &SchemaSourceMap,
) -> Vec<SemanticTypeRegion> {
    let mut regions = Vec::new();
    match schema {
        SimplifiedSchema::Single(shape) => {
            collect_semantic_type_regions(
                shape,
                &SchemaSourcePath::root(),
                source_map,
                &mut regions,
            );
        }
        SimplifiedSchema::Union(arms) => {
            for (index, arm) in arms.iter().enumerate() {
                if let SchemaArm::Inline(shape) = arm {
                    collect_semantic_type_regions(
                        shape,
                        &SchemaSourcePath::root().union_arm(index),
                        source_map,
                        &mut regions,
                    );
                }
            }
        }
    }
    regions
}

fn collect_semantic_type_regions(
    shape: &SchemaShape,
    parent: &SchemaSourcePath,
    source_map: &SchemaSourceMap,
    regions: &mut Vec<SemanticTypeRegion>,
) {
    for (name, definition) in shape_definitions(shape) {
        let path = parent.property(&name);
        if let (Some(key_span), Some(definition_span)) = (
            source_map.spans(&path, SchemaSpanKind::MappingKey).first(),
            source_map.spans(&path, SchemaSpanKind::Definition).first(),
        ) {
            regions.push(SemanticTypeRegion {
                kind: MetaSchemaKind::TypeDefinition,
                name: name.clone(),
                path: path.clone(),
                key_span: key_span.clone(),
                definition_span: definition_span.clone(),
                definition: definition.clone(),
            });
        }

        for (index, atom) in atoms(definition).iter().enumerate() {
            let TypeExpr::InlineObject(nested) = &atom.ty else {
                continue;
            };
            let nested_path = if matches!(definition, PropertyDef::Union(_)) {
                path.union_arm(index)
            } else {
                path.clone()
            };
            collect_semantic_type_regions(nested, &nested_path, source_map, regions);
        }
    }
}

/// Every complete definition on a shape, paired with the authored mapping key
/// the schema source map indexes it under.
///
/// Literal properties and pattern keys are stored on separate
/// [`SchemaShape`] fields, but the source projector records both under the
/// verbatim authored key, so a region walk that reads only `properties` would
/// silently drop every `<string>` / `<starting::…>` definition.
fn shape_definitions(shape: &SchemaShape) -> Vec<(String, &PropertyDef)> {
    shape
        .properties
        .iter()
        .map(|(name, definition)| (name.clone(), definition))
        .chain(
            shape
                .pattern_keys
                .iter()
                .map(|pattern| (pattern_key_source(&pattern.key), &pattern.def)),
        )
        .collect()
}

/// The `<…>` mapping key a pattern key was authored as.
fn pattern_key_source(key: &PatternKey) -> String {
    match key {
        PatternKey::CatchAll => "<string>".to_string(),
        PatternKey::Starting(prefix) => format!("<starting::{prefix}>"),
        PatternKey::Ending(suffix) => format!("<ending::{suffix}>"),
        PatternKey::Pattern(regex) => format!("<pattern::{regex}>"),
    }
}

/// Whether `offset` falls inside a complete definition of an activated
/// standalone schema document.
///
/// Pattern keys are authored as `<starting::…>`, `<ending::…>`, and
/// `<pattern::…>`, which are also well-formed CommonMark autolinks. The
/// substrate reads a standalone YAML buffer as Markdown like any other, so
/// without this test it claims those keys as links before the schema layer is
/// ever asked.
pub fn standalone_semantic_region_covers(state: &SchemaAuthoringState, offset: usize) -> bool {
    let SchemaAuthoringState::Standalone { model: Some(model), .. } = state else {
        return false;
    };
    let Some(schema) = model.schema() else {
        return false;
    };
    semantic_type_regions(schema, &model.source_map)
        .iter()
        .any(|region| region.key_span.contains(&offset) || region.definition_span.contains(&offset))
}

/// Parsed schema-authoring state attached to a [`super::DocumentOverlay`].
#[derive(Debug, Clone)]
pub enum SchemaAuthoringState {
    /// No effective semantic type or standalone content envelope activates.
    Inactive,
    /// Semantic meta-type values in Markdown frontmatter.
    Frontmatter(Vec<FrontmatterSchemaValue>),
    /// A standalone pure or tagged SimplifiedSchema authoring document.
    Standalone {
        /// Envelope claimed by the current buffer.
        envelope: StandaloneSchemaEnvelope,
        /// Fresh or retained last-good semantic model and structural source map.
        model: Option<Arc<StandaloneSchemaDocument>>,
        /// Whether `model` belongs to an earlier valid buffer version.
        stale: bool,
        /// Parse failure owned by the current buffer.
        error: Option<Arc<SchemaError>>,
    },
}

/// Builds typed frontmatter activation from the effective schema.
pub fn frontmatter_authoring(
    ast: Option<&FrontmatterAst>,
    outcome: &SchemaOutcome,
) -> SchemaAuthoringState {
    let Some(ast) = ast else {
        return SchemaAuthoringState::Inactive;
    };
    let shape = effective_shape(outcome);
    let mut values = Vec::new();
    for (index, entry) in ast.entries().iter().enumerate() {
        // The authored key chain, not `dotted.split('.')`: a property named
        // `build.target` is one key, and splitting it would look up a nested
        // `build` → `target` path that does not exist, silently dropping the
        // property's declared semantic meta-type.
        let path = ast.key_path_at(index);
        if path.len() > 1 && path[0] == "$schema" {
            continue;
        }
        let kinds = if path == ["$schema"] {
            vec![MetaSchemaKind::Schema]
        } else {
            def_at_path(&shape, &path)
                .map(|definition| meta_schema_kinds(&definition))
                .unwrap_or_default()
        };
        if !kinds.is_empty() {
            values.push(FrontmatterSchemaValue {
                pointer: entry.pointer.clone(),
                kinds,
                value_span: entry.value_span.clone(),
            });
        }
    }
    if values.is_empty() {
        SchemaAuthoringState::Inactive
    } else {
        SchemaAuthoringState::Frontmatter(values)
    }
}

/// Returns the standalone envelope lexically claimed by the current text.
///
/// This recognizes only top-level YAML mapping entries, in either block or
/// flow presentation — the authoritative parser
/// ([`parse_standalone_schema_document`](darkmatter::markdown::schemas::parse_standalone_schema_document))
/// accepts a mapping regardless of presentation, so a claim that saw only
/// block style would drop the retained model of a flow-authored document at
/// the first malformed keystroke.
///
/// A real YAML parse is unavailable here by construction: this runs on text
/// that is *already known* to be unparseable, on every keystroke. The
/// recognizer is therefore lexical — one linear scan, no backtracking — and
/// tolerant of truncated or otherwise broken input. It still refuses ordinary
/// YAML and raw JSON Schema, which carry top-level keys beyond a sole
/// `$schema` and never carry `kind: schema`.
pub fn standalone_envelope_claim(text: &str) -> Option<StandaloneSchemaEnvelope> {
    let top_level = match flow_mapping_start(text) {
        Some(start) => flow_top_level_entries(&text[start + 1..]),
        None => block_top_level_entries(text),
    };
    if top_level.iter().any(|(key, value)| {
        key == "kind" && lexical_scalar(value).as_deref() == Some("schema")
    }) {
        return Some(StandaloneSchemaEnvelope::Tagged);
    }
    (top_level.len() == 1 && top_level[0].0 == "$schema")
        .then_some(StandaloneSchemaEnvelope::Pure)
}

/// Top-level implicit or explicit pairs of a block mapping.
fn block_top_level_entries(text: &str) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    let mut quote: Option<char> = None;
    let mut explicit_key = None;
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        // A line typed while a top-level quoted scalar is still open is that
        // scalar's continuation, not a mapping entry: `description: 'multi`
        // followed by a literal `kind: schema` line is one ordinary key, and
        // reading the continuation as structure would tag the document as an
        // envelope the authoritative parser refuses. Only such an already-open
        // continuation advances cross-line quote state.
        if quote.is_some() {
            advance_quote_state(line, &mut quote);
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed == "---" || trimmed == "..." {
            continue;
        }
        // An indented line is nested payload, never a top-level entry. With no
        // top-level quote open it must not advance quote state either: a quote
        // inside the nested value (`  title: foo-"bar`) is that value's own
        // business, and letting it open top-level quote state would swallow the
        // following top-level key and drop a genuine envelope's retained model.
        if line.chars().next().is_some_and(char::is_whitespace) {
            continue;
        }
        if let Some(key) = explicit_key.take() {
            let Some(value) = explicit_indicator_content(line, ':') else {
                continue;
            };
            advance_quote_state(value, &mut quote);
            entries.push((key, value.trim().to_string()));
            continue;
        }
        if let Some(key) = explicit_indicator_content(line, '?') {
            explicit_key = lexical_scalar(key.trim());
            continue;
        }
        // A top-level line advances quote state so its own value may open a
        // multi-line quoted scalar (`description: "multi`) whose following
        // continuation lines close it.
        advance_quote_state(line, &mut quote);
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let Some(key) = lexical_scalar(key.trim()) else {
            continue;
        };
        entries.push((key, value.trim().to_string()));
    }
    entries
}

fn explicit_indicator_content(source: &str, indicator: char) -> Option<&str> {
    let rest = source.strip_prefix(indicator)?;
    (rest.is_empty() || rest.starts_with(char::is_whitespace)).then_some(rest.trim_start())
}

/// Advances quoted-scalar state by one character taken from inside an active
/// quoted scalar.
///
/// Two YAML rules a "the same quote character closes it" scan gets wrong, both
/// of which let an authored quote escape its scalar and expose the commas and
/// colons the recognizer treats as structure:
///
/// - inside a double-quoted scalar `\` escapes the next character, so `\"` is
///   not a terminator;
/// - inside a single-quoted scalar `\` is *not* an escape, but `''` denotes a
///   literal `'`.
///
/// ## Returns
///
/// Whether `next` was absorbed as the second half of a `''` pair. A caller
/// echoing its input must then consume and echo that character too, so the
/// scalar is kept verbatim.
fn step_quoted(
    quote: &mut Option<char>,
    escaped: &mut bool,
    ch: char,
    next: Option<char>,
) -> bool {
    let Some(open) = *quote else {
        return false;
    };
    if *escaped {
        *escaped = false;
    } else if open == '"' && ch == '\\' {
        *escaped = true;
    } else if ch == open {
        if open == '\'' && next == Some('\'') {
            return true;
        }
        *quote = None;
    }
    false
}

/// Advances cross-line quoted-scalar state by one physical line of a block
/// mapping.
///
/// A quote character opens a scalar only where a scalar may begin — line start,
/// or immediately after a structural indicator for the active presentation
/// (see [`is_scalar_boundary`]). Elsewhere it is ordinary text: the plain
/// scalars `don't`, `foo-"bar`, and `foo{ "bar` must not open a quote that
/// swallows every following line and drops a genuine envelope's retained
/// model. A flow collection opened at a block node boundary switches to flow
/// semantics until its matching delimiter closes.
fn advance_quote_state(line: &str, quote: &mut Option<char>) {
    let mut chars = line.chars().peekable();
    let mut escaped = false;
    let mut at_scalar_start = true;
    let mut previous_space = true;
    let mut presentation = ScalarPresentation::Block;
    let mut flow_depth = 0usize;
    while let Some(ch) = chars.next() {
        if quote.is_some() {
            if step_quoted(quote, &mut escaped, ch, chars.peek().copied()) {
                chars.next();
            }
            continue;
        }
        match ch {
            '#' if previous_space => return,
            '\'' | '"' if at_scalar_start => *quote = Some(ch),
            '{' | '[' if presentation == ScalarPresentation::Flow || at_scalar_start => {
                presentation = ScalarPresentation::Flow;
                flow_depth += 1;
            }
            '}' | ']' if presentation == ScalarPresentation::Flow => {
                flow_depth = flow_depth.saturating_sub(1);
                if flow_depth == 0 {
                    presentation = ScalarPresentation::Block;
                }
            }
            _ => {}
        }
        previous_space = ch.is_whitespace();
        at_scalar_start =
            is_scalar_boundary(ch, at_scalar_start, chars.peek().copied(), presentation)
                || (at_scalar_start && previous_space);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScalarPresentation {
    Block,
    Flow,
}

/// Whether `ch` is a structural indicator that begins a fresh scalar position,
/// judged by the current scalar position and the character `next` that follows
/// it.
///
/// YAML indicators are context-sensitive. `-` is structural only when it is
/// already at a scalar boundary and is followed by whitespace or end of line;
/// a mid-token `-` remains plain-scalar content even when whitespace follows
/// it. `:` followed by whitespace or end of line is a mapping separator. `[`,
/// `{`, and `,` begin scalar positions only in flow presentation; in block
/// presentation they are content once a plain scalar has begun.
fn is_scalar_boundary(
    ch: char,
    at_scalar_start: bool,
    next: Option<char>,
    presentation: ScalarPresentation,
) -> bool {
    match ch {
        '[' | '{' | ',' => presentation == ScalarPresentation::Flow,
        '-' => at_scalar_start && next.is_none_or(char::is_whitespace),
        ':' => next.is_none_or(char::is_whitespace),
        _ => false,
    }
}

/// Byte offset of the `{` opening a whole-document flow mapping, skipping
/// leading blank lines, comments, and document markers. `None` when the first
/// content is anything else, which leaves block scanning in charge.
fn flow_mapping_start(text: &str) -> Option<usize> {
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed == "---" || trimmed == "..." {
            offset += line.len();
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        return trimmed.starts_with('{').then_some(offset + indent);
    }
    None
}

/// Top-level `key: value` entries of a flow mapping. `inner` starts
/// immediately after the opening `{`.
///
/// Only depth-0 `:` and `,` delimit entries, so nested flow collections and
/// quoted scalars (which is where a raw JSON Schema hides its `://` and its
/// commas) cannot be mistaken for top-level structure. A quote opens such a
/// scalar only at a YAML scalar boundary; a quote embedded in plain content is
/// inert. Unterminated input yields whatever entries were complete, which is
/// exactly what a mid-edit buffer needs.
fn flow_top_level_entries(inner: &str) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    let mut in_comment = false;
    let mut at_scalar_start = true;
    let mut previous_space = true;
    let mut key: Option<String> = None;
    let mut token = String::new();
    let mut escaped = false;
    let mut chars = inner.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_comment {
            if ch == '\n' {
                in_comment = false;
                previous_space = true;
            }
            continue;
        }
        if quote.is_some() {
            token.push(ch);
            if step_quoted(&mut quote, &mut escaped, ch, chars.peek().copied())
                && let Some(doubled) = chars.next()
            {
                token.push(doubled);
            }
            at_scalar_start = false;
            previous_space = false;
            continue;
        }
        let mut structural_value_boundary = false;
        match ch {
            '\'' | '"' if at_scalar_start => {
                token.push(ch);
                quote = Some(ch);
                escaped = false;
            }
            '#' if previous_space => in_comment = true,
            '{' | '[' => {
                depth += 1;
                token.push(ch);
            }
            '}' | ']' if depth > 0 => {
                depth -= 1;
                token.push(ch);
            }
            // The outermost `}` closes the mapping; anything after it belongs
            // to a second document and is not this envelope's business.
            '}' => {
                push_flow_entry(&mut entries, &mut key, &mut token);
                break;
            }
            ':' if depth == 0 && key.is_none() => {
                key = Some(std::mem::take(&mut token));
                structural_value_boundary = true;
            }
            ',' if depth == 0 => push_flow_entry(&mut entries, &mut key, &mut token),
            _ => token.push(ch),
        }
        previous_space = ch.is_whitespace();
        at_scalar_start = structural_value_boundary
            || is_scalar_boundary(
                ch,
                at_scalar_start,
                chars.peek().copied(),
                ScalarPresentation::Flow,
            )
            || (at_scalar_start && previous_space);
    }
    push_flow_entry(&mut entries, &mut key, &mut token);
    entries
}

/// Records one flow entry and resets the scanner's per-entry state. A fragment
/// with no `:` separator, or whose key is not a readable scalar, is dropped
/// rather than guessed at — over-claiming here would activate DMLS on text the
/// authoritative parser would refuse.
fn push_flow_entry(
    entries: &mut Vec<(String, String)>,
    key: &mut Option<String>,
    token: &mut String,
) {
    let value = std::mem::take(token);
    let Some(key) = key.take() else {
        return;
    };
    if let Some(key) = lexical_scalar(key.trim()) {
        entries.push((key, value.trim().to_string()));
    }
}

fn lexical_scalar(source: &str) -> Option<String> {
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(source).ok()?;
    value.as_str().map(str::to_string)
}

/// A cursor's structural position inside a YAML **flow** collection.
///
/// Block presentation hands a cursor its key and its ancestry through line
/// indentation. A flow collection has neither: `{$schema: {title: string}}` is
/// one line at column zero. The authoritative parser accepts both
/// presentations, so the capabilities that locate a cursor must too.
pub(crate) struct FlowCursor {
    /// Document byte offset of the delimiter opening the *outermost* flow
    /// collection. A flow value can be the value of an ordinary block key, so
    /// the block ancestry above this offset remains the caller's to resolve
    /// with the same block helpers it already uses.
    pub(crate) root_start: usize,
    /// Mapping keys enclosing the cursor *within* the flow collection,
    /// outermost first. Sequences are transparent: like a block `- ` item they
    /// contribute nesting but no key.
    pub(crate) ancestors: Vec<String>,
    /// The key whose value the cursor is authoring.
    pub(crate) key: String,
    /// Document byte offset where that value's authored text begins.
    pub(crate) value_start: usize,
}

/// One enclosing flow collection.
struct FlowFrame {
    /// Byte offset of the `{` or `[` that opened it.
    start: usize,
    /// Whether it is a mapping (`{`) rather than a sequence (`[`).
    mapping: bool,
    /// The key this collection is the value of. `None` at the flow root and for
    /// a collection authored as a sequence element.
    label: Option<String>,
    /// The key whose value is currently being authored, once its `:` was read.
    key: Option<String>,
    /// Where that value's text begins, once a non-space character was typed.
    value_start: Option<usize>,
}

/// Locates a cursor inside a flow collection, or `None` when `offset` is in
/// ordinary block context (where indentation already answers the question).
///
/// A real parse is unavailable by construction — this runs on every keystroke,
/// including on text that does not parse — so the walk is lexical, linear, and
/// shares its quoted-scalar rules with [`standalone_envelope_claim`]'s scanner.
/// A delimiter only opens a collection where a YAML scalar may begin, so a `{`
/// inside a plain scalar (`title: a {b`) is text rather than structure.
///
/// The cursor's own value text is deliberately *not* interpreted here: the
/// innermost enclosing mapping entry is reported whole, so the union arms,
/// inline objects, and constraint lists inside it stay the business of the
/// shared tolerant type-expression cursor authority.
pub(crate) fn flow_value_cursor(text: &str, offset: usize) -> Option<FlowCursor> {
    let prefix = text.get(..offset)?;
    let mut frames: Vec<FlowFrame> = Vec::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut in_comment = false;
    let mut at_scalar_start = true;
    let mut previous_space = true;
    let mut token = String::new();
    let mut chars = prefix.char_indices().peekable();

    while let Some((index, ch)) = chars.next() {
        if in_comment {
            if ch == '\n' {
                in_comment = false;
                previous_space = true;
                at_scalar_start = true;
            }
            continue;
        }
        if quote.is_some() {
            token.push(ch);
            let next = chars.peek().map(|(_, ch)| *ch);
            if step_quoted(&mut quote, &mut escaped, ch, next)
                && let Some((_, doubled)) = chars.next()
            {
                token.push(doubled);
            }
            previous_space = false;
            at_scalar_start = false;
            continue;
        }
        match ch {
            '#' if previous_space => in_comment = true,
            '\'' | '"' if at_scalar_start => {
                note_flow_value_start(&mut frames, index);
                quote = Some(ch);
                escaped = false;
                token.push(ch);
            }
            '{' | '[' if at_scalar_start => {
                note_flow_value_start(&mut frames, index);
                let label = frames
                    .last()
                    .filter(|frame| frame.mapping)
                    .and_then(|frame| frame.key.clone());
                frames.push(FlowFrame {
                    start: index,
                    mapping: ch == '{',
                    label,
                    key: None,
                    value_start: None,
                });
                token.clear();
            }
            '}' | ']' => {
                frames.pop();
                token.clear();
            }
            ':' if frames.last().is_some_and(|frame| frame.mapping && frame.key.is_none()) => {
                let key = lexical_scalar(token.trim())
                    .unwrap_or_else(|| token.trim().to_string());
                let frame = frames.last_mut().expect("guarded above");
                frame.key = Some(key);
                frame.value_start = None;
                token.clear();
            }
            ',' if !frames.is_empty() => {
                let frame = frames.last_mut().expect("guarded above");
                frame.key = None;
                frame.value_start = None;
                token.clear();
            }
            _ => {
                if !ch.is_whitespace() {
                    note_flow_value_start(&mut frames, index);
                }
                token.push(ch);
            }
        }
        previous_space = ch.is_whitespace();
        at_scalar_start =
            matches!(ch, ':' | ',' | '-' | '[' | '{') || (at_scalar_start && previous_space);
    }

    let root_start = frames.first()?.start;
    // Sequences carry no key of their own, so a cursor in one is authoring the
    // value of the nearest enclosing mapping entry — the same reading block
    // presentation gives `key: [a, b`.
    let mapping = frames.iter().rposition(|frame| frame.mapping)?;
    let frame = &frames[mapping];
    Some(FlowCursor {
        root_start,
        ancestors: frames[..=mapping].iter().filter_map(|f| f.label.clone()).collect(),
        key: frame.key.clone()?,
        value_start: frame.value_start.unwrap_or(offset),
    })
}

fn note_flow_value_start(frames: &mut [FlowFrame], index: usize) {
    if let Some(frame) = frames.last_mut()
        && frame.key.is_some()
        && frame.value_start.is_none()
    {
        frame.value_start = Some(index);
    }
}

fn effective_shape(outcome: &SchemaOutcome) -> SchemaShape {
    let mut shape = match darkmatter_base_schema() {
        SimplifiedSchema::Single(shape) => shape,
        SimplifiedSchema::Union(_) => SchemaShape::default(),
    };
    let SchemaOutcome::Ready(Some(bundle)) = outcome else {
        return shape;
    };
    for extension in &bundle.extension_shapes {
        overlay_shape(&mut shape, extension);
    }
    match &bundle.effective.simplified {
        Some(SimplifiedSchema::Single(document)) => overlay_shape(&mut shape, document),
        Some(SimplifiedSchema::Union(arms)) => overlay_shape(&mut shape, &merged_root_shape(arms)),
        None => {}
    }
    shape
}

fn overlay_shape(base: &mut SchemaShape, overlay: &SchemaShape) {
    for (name, definition) in &overlay.properties {
        base.properties.insert(name.clone(), definition.clone());
    }
}

fn merged_root_shape(arms: &[SchemaArm]) -> SchemaShape {
    let mut merged = SchemaShape::default();
    for arm in arms {
        if let SchemaArm::Inline(shape) = arm {
            merge_shape(&mut merged, shape);
        }
    }
    merged
}

fn merge_shape(base: &mut SchemaShape, incoming: &SchemaShape) {
    for (name, definition) in &incoming.properties {
        let definition = match base.properties.get(name) {
            Some(existing) => merge_defs(existing, definition),
            None => definition.clone(),
        };
        base.properties.insert(name.clone(), definition);
    }
}

fn def_at_path(root: &SchemaShape, path: &[&str]) -> Option<PropertyDef> {
    let (leaf, ancestors) = path.split_last()?;
    let mut shape = root.clone();
    for ancestor in ancestors {
        let definition = shape.properties.get(*ancestor)?;
        shape = merged_inline_shape(definition)?;
    }
    shape.properties.get(*leaf).cloned()
}

fn merged_inline_shape(definition: &PropertyDef) -> Option<SchemaShape> {
    let mut merged = None;
    for atom in atoms(definition) {
        let TypeExpr::InlineObject(shape) = &atom.ty else {
            continue;
        };
        merge_shape(merged.get_or_insert_with(SchemaShape::default), shape);
    }
    merged
}

fn merge_defs(existing: &PropertyDef, incoming: &PropertyDef) -> PropertyDef {
    let mut merged = atoms(existing).to_vec();
    for atom in atoms(incoming) {
        if !merged.contains(atom) {
            merged.push(atom.clone());
        }
    }
    if merged.len() == 1 {
        PropertyDef::Single(merged.remove(0))
    } else {
        PropertyDef::Union(merged)
    }
}

fn meta_schema_kinds(definition: &PropertyDef) -> Vec<MetaSchemaKind> {
    let mut kinds = Vec::new();
    for atom in atoms(definition) {
        let kind = match atom.ty {
            TypeExpr::Primitive(SimplifiedType::TypeDefinition) => MetaSchemaKind::TypeDefinition,
            TypeExpr::Primitive(SimplifiedType::Schema) => MetaSchemaKind::Schema,
            _ => continue,
        };
        if !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }
    kinds
}

fn atoms(definition: &PropertyDef) -> &[PropertyAtom] {
    match definition {
        PropertyDef::Single(atom) => std::slice::from_ref(atom),
        PropertyDef::Union(atoms) => atoms,
    }
}

/// The assembled schema for one document: the effective schema plus the
/// frontmatter JSON it validates (both derived from the same parsed document,
/// so diagnostics never re-parse).
#[derive(Clone)]
pub struct SchemaBundle {
    /// The layered effective schema (base + extensions + document `$schema`).
    pub effective: EffectiveSchema,
    /// The document's frontmatter as JSON, with the `$schema` control key
    /// stripped (it is not document data).
    pub frontmatter_json: Value,
    /// The SimplifiedSchema shapes of the matched extension baselines.
    ///
    /// `effective.simplified` carries only the document's own `$schema`; the
    /// extension shapes are kept separately so completion/hover can offer
    /// extension-declared keys (e.g. Claudine's `provider`/`model`) even when
    /// the document has no `$schema` of its own.
    pub extension_shapes: Vec<SchemaShape>,
    /// Extension-baseline dependency files (each matched extension's own path
    /// plus its imports/examples), sorted and deduplicated. These are NOT on
    /// `effective.dependencies()` because extension baselines are merged into the
    /// baseline JSON schema *before* the effective schema is assembled, so their
    /// edges never reach [`EffectiveSchema::dependencies`]. The overlay cache
    /// content-hashes them alongside the effective dependencies so editing a
    /// configured extension baseline (or a type it imports) invalidates the
    /// cached bundle.
    pub extension_dependencies: Vec<PathBuf>,
}

/// Assembles the effective schema for a document.
///
/// ## Returns
///
/// `Ok(Some(bundle))` for every document with frontmatter (the base baseline
/// always applies); `Ok(None)` only when the document has no frontmatter map.
///
/// ## Errors
///
/// Propagates [`SchemaError`] from extension loading, baseline merging, `$schema`
/// resolution, or validator construction so the caller can range a
/// `dm.schema.prepare` / `dm.schema.invalid_schema_shape` diagnostic.
pub fn assemble(
    doc_path: &Path,
    document_text: &str,
    config: &DmlsConfig,
    workspace_roots: &[PathBuf],
    trigger_registry: Option<TriggerRegistry>,
) -> Result<Option<SchemaBundle>, SchemaError> {
    let combined = combined_baseline(doc_path, config, workspace_roots)?;

    let md: Markdown = document_text.into();
    let md = md.with_source(ComposeSource::File(doc_path.to_path_buf()));
    if md.frontmatter().as_map().is_empty() && md.frontmatter().raw_source().is_none() {
        return Ok(None);
    }

    let CombinedBaseline {
        baseline,
        extension_shapes,
        dependencies,
    } = combined;
    let mut schemas = match baseline {
        Some(baseline) => DarkmatterSchemas::new().with_baseline_json_schema(baseline),
        None => DarkmatterSchemas::new().with_darkmatter_baseline_json_schema(),
    }?;
    if let Some(registry) = trigger_registry {
        schemas = schemas.with_trigger_registry(registry);
    }
    if let Some(dir) = doc_path.parent() {
        schemas = schemas.with_file_ref_fallback_dir(dir.to_path_buf());
    }

    match schemas.effective_for(&md)? {
        Some(effective) => Ok(Some(SchemaBundle {
            effective,
            frontmatter_json: frontmatter_json(&md),
            extension_shapes,
            extension_dependencies: dependencies,
        })),
        None => Ok(None),
    }
}

/// Selects the trigger-discovery boundary for a document.
///
/// The nearest containing workspace folder is authoritative. When the document
/// is inside a Git repository whose root is at or below that folder, the
/// repository root narrows the boundary. Documents outside all workspace
/// folders intentionally return `None` and never discover triggers.
pub fn trigger_boundary(doc_path: &Path, workspace_roots: &[PathBuf]) -> Option<PathBuf> {
    let workspace = workspace_roots
        .iter()
        .filter(|root| doc_path.starts_with(root))
        .max_by_key(|root| root.components().count())?;
    let start = doc_path.parent().unwrap_or(doc_path);
    match find_git_root_from(start) {
        Some(repo) if repo.starts_with(workspace) => Some(repo),
        _ => Some(workspace.clone()),
    }
}

/// The Darkmatter base baseline with matching extension baselines merged over
/// it, plus the matched extensions' SimplifiedSchema shapes and dependency
/// files.
struct CombinedBaseline {
    /// `None` retains the shared built-in baseline; `Some` is materialized only
    /// when an extension must be merged over it.
    baseline: Option<Value>,
    extension_shapes: Vec<SchemaShape>,
    dependencies: Vec<PathBuf>,
}

/// The Darkmatter base baseline with every matching extension baseline merged
/// over it (extension wins over base; a later extension wins over an earlier),
/// alongside the matched extensions' SimplifiedSchema shapes.
fn combined_baseline(
    doc_path: &Path,
    config: &DmlsConfig,
    workspace_roots: &[PathBuf],
) -> Result<CombinedBaseline, SchemaError> {
    let mut baseline: Option<Value> = None;
    let mut shapes = Vec::new();
    let mut dependencies: BTreeSet<PathBuf> = BTreeSet::new();
    for extension in config.schema.extensions.values() {
        if !extension_matches(doc_path, extension, workspace_roots) {
            continue;
        }
        let resolved = load_extension_schema(extension, workspace_roots)?;
        if let Some(SimplifiedSchema::Single(shape)) = &resolved.simplified {
            shapes.push(shape.clone());
        }
        // The extension file's own path (via `referenced_files`, since
        // `load_extension_schema` resolves a `Value::String(path)`) plus the
        // types it imports and examples it references are dependency edges the
        // overlay cache must hash — they never reach `effective.dependencies()`
        // because the extension is merged into the baseline before assembly.
        dependencies.extend(resolved.referenced_files.iter().cloned());
        dependencies.extend(resolved.imports.iter().cloned());
        dependencies.extend(resolved.examples.iter().cloned());
        // `merge_baseline(under, over)` lets `over` win — the extension
        // overrides the base, matching compose's layering.
        let lower = baseline
            .as_ref()
            .unwrap_or_else(|| darkmatter_base_json_schema_ref());
        baseline = Some(merge_baseline(lower, resolved.json_schema)?);
    }
    Ok(CombinedBaseline {
        baseline,
        extension_shapes: shapes,
        dependencies: dependencies.into_iter().collect(),
    })
}

/// Loads one extension's SimplifiedSchema (or JSON Schema) file.
fn load_extension_schema(
    extension: &SchemaExtensionConfig,
    workspace_roots: &[PathBuf],
) -> Result<darkmatter::markdown::schemas::resolve::ResolvedSchema, SchemaError> {
    let path = resolve_extension_path(&extension.path, workspace_roots);
    let base_dir = path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
    // Drive resolution through the same file-reference path a `$schema` string
    // uses, so YAML/JSON disambiguation matches document references.
    let value = Value::String(path.to_string_lossy().into_owned());
    resolve_schema(&value, &base_dir)
}

/// Resolves an extension's configured path: relative paths anchor on the first
/// workspace root.
fn resolve_extension_path(path: &Path, workspace_roots: &[PathBuf]) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    workspace_roots
        .first()
        .map(|root| root.join(path))
        .unwrap_or_else(|| path.to_path_buf())
}

/// Whether `doc_path` matches an extension's activation globs. An extension
/// with no globs never auto-activates.
fn extension_matches(
    doc_path: &Path,
    extension: &SchemaExtensionConfig,
    workspace_roots: &[PathBuf],
) -> bool {
    if extension.globs.is_empty() {
        return false;
    }
    let relative = relative_to_root(doc_path, workspace_roots);
    let mut builder = GlobSetBuilder::new();
    for pattern in &extension.globs {
        if let Ok(glob) = Glob::new(pattern) {
            builder.add(glob);
        }
    }
    match builder.build() {
        Ok(set) => set.is_match(&relative),
        Err(_) => false,
    }
}

/// The document path relative to its first ancestor workspace root (else the
/// path itself), for glob matching.
fn relative_to_root(doc_path: &Path, workspace_roots: &[PathBuf]) -> PathBuf {
    for root in workspace_roots {
        if let Ok(relative) = doc_path.strip_prefix(root) {
            return relative.to_path_buf();
        }
    }
    doc_path.to_path_buf()
}

/// The document frontmatter as JSON, with the `$schema` control key removed.
fn frontmatter_json(md: &Markdown) -> Value {
    let map = md.frontmatter().as_map();
    let mut object = serde_json::Map::with_capacity(map.len());
    for (key, value) in map {
        if key == "$schema" {
            continue;
        }
        object.insert(key.clone(), value.clone());
    }
    Value::Object(object)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_extension(globs: &[&str], path: &str) -> DmlsConfig {
        let mut config = DmlsConfig::default();
        config.schema.extensions.insert(
            "claudine".to_string(),
            SchemaExtensionConfig {
                path: PathBuf::from(path),
                globs: globs.iter().map(|g| g.to_string()).collect(),
            },
        );
        config
    }

    #[test]
    fn test_base_baseline_applies_without_schema() {
        let bundle = assemble(
            Path::new("/w/doc.md"),
            "---\ntitle: Hello\n---\n\nbody\n",
            &DmlsConfig::default(),
            &[PathBuf::from("/w")],
            None,
        )
        .expect("assembles")
        .expect("has frontmatter");
        // A plain title validates cleanly against the base baseline (which
        // allows additional properties).
        let report = bundle.effective.validate(&bundle.frontmatter_json);
        assert!(report.valid, "unexpected problems: {:?}", report.problems);
    }

    #[test]
    fn test_no_frontmatter_yields_none() {
        let bundle = assemble(
            Path::new("/w/doc.md"),
            "# Just a body\n",
            &DmlsConfig::default(),
            &[PathBuf::from("/w")],
            None,
        )
        .expect("assembles");
        assert!(bundle.is_none());
    }

    #[test]
    fn test_extension_glob_gates_activation() {
        // The extension path does not exist, so activation must be attempted
        // only when the glob matches (a non-matching doc never loads it).
        let config = config_with_extension(&[".claude/**"], "missing-schema.yaml");
        let outside = assemble(
            Path::new("/w/notes/x.md"),
            "---\ntitle: A\n---\n\nbody\n",
            &config,
            &[PathBuf::from("/w")],
            None,
        );
        // Outside `.claude/**`: extension never loaded, assembly succeeds.
        assert!(outside.is_ok());
    }

    #[test]
    fn test_relative_to_root() {
        assert_eq!(
            relative_to_root(Path::new("/w/.claude/p.md"), &[PathBuf::from("/w")]),
            PathBuf::from(".claude/p.md")
        );
        assert_eq!(
            relative_to_root(Path::new("/other/p.md"), &[PathBuf::from("/w")]),
            PathBuf::from("/other/p.md")
        );
    }

    #[test]
    fn test_extension_matches_respects_globs() {
        let extension = SchemaExtensionConfig {
            path: PathBuf::from("claudine.yaml"),
            globs: vec![".claude/**".to_string()],
        };
        let roots = [PathBuf::from("/w")];
        assert!(extension_matches(Path::new("/w/.claude/p.md"), &extension, &roots));
        assert!(!extension_matches(Path::new("/w/notes/p.md"), &extension, &roots));
        let no_globs = SchemaExtensionConfig {
            path: PathBuf::from("x.yaml"),
            globs: Vec::new(),
        };
        assert!(!extension_matches(Path::new("/w/.claude/p.md"), &no_globs, &roots));
    }

    #[test]
    fn semantic_type_regions_project_existing_activation_state() {
        let source = concat!(
            "$schema:\n",
            "  scalar: 'string(required)'\n",
            "  native:\n",
            "    nested: number\n",
            "  union:\n",
            "    - string\n",
            "    - item: boolean\n",
        );
        let model = darkmatter::markdown::schemas::parse_standalone_schema_document(
            source,
            Path::new("/w/schema.yaml"),
        )
        .expect("classification")
        .expect("standalone schema");

        let schema = model.schema().expect("mapping payload is an inline schema");
        let regions = semantic_type_regions(schema, &model.source_map);
        let names: Vec<&str> = regions.iter().map(|region| region.name.as_str()).collect();
        assert!(names.contains(&"scalar"), "{regions:#?}");
        assert!(names.contains(&"native"), "{regions:#?}");
        assert!(names.contains(&"union"), "{regions:#?}");
        assert!(names.contains(&"nested"), "{regions:#?}");
        assert!(names.contains(&"item"), "{regions:#?}");
        assert!(regions.iter().all(|region| region.kind == MetaSchemaKind::TypeDefinition));
        for region in regions {
            assert!(!region.definition_span.is_empty(), "{region:#?}");
            assert!(!region.key_span.is_empty(), "{region:#?}");
            assert_eq!(&source[region.key_span.clone()], region.name);
        }
    }

    /// The claim must track the authoritative parser, which accepts a mapping
    /// in either presentation. A block-only claim silently drops the retained
    /// model of every flow-authored envelope the moment it is mid-edit.
    #[test]
    fn envelope_claim_recognizes_block_and_flow_presentation() {
        for pure in [
            "$schema:\n  title: string\n",
            "? $schema\n:\n  title: string\n",
            "{\"$schema\":{\"title\":\"string\"}}",
            "{$schema: {title: string}}\n",
            "---\n{ \"$schema\": { \"title\": \"str\" } }\n",
            // Truncated mid-edit: the closing braces are not typed yet.
            "{\"$schema\": {\"title\": \"str",
        ] {
            assert_eq!(
                standalone_envelope_claim(pure),
                Some(StandaloneSchemaEnvelope::Pure),
                "{pure:?}"
            );
        }

        for tagged in [
            "kind: schema\ntypes:\n  title: string\n",
            "? kind\n: schema\n? types\n:\n  title: string\n",
            "{kind: schema, types: {title: string}}",
            "{\"types\": {\"title\": \"str\"}, \"kind\": \"schema\"}",
        ] {
            assert_eq!(
                standalone_envelope_claim(tagged),
                Some(StandaloneSchemaEnvelope::Tagged),
                "{tagged:?}"
            );
        }

        for inert in [
            "title: ordinary yaml\n",
            "{\"title\": \"ordinary flow yaml\"}",
            // Raw JSON Schema: `$schema` is present but not the sole key, and
            // its `://` and `,` are inside quoted scalars.
            "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"type\":\"object\"}",
            "{kind: config, types: {title: string}}",
            "[1, 2, 3]\n",
        ] {
            assert_eq!(standalone_envelope_claim(inert), None, "{inert:?}");
        }
    }

    /// An escaped quote must not end the scanner's quoted region early: a
    /// prematurely closed scalar re-opens on the real closing quote and
    /// swallows the top-level `,` that separates the document's other keys,
    /// collapsing ordinary YAML into a sole-`$schema` pure claim.
    ///
    /// The claim must agree with the authoritative parser in both directions,
    /// so each case is asserted against
    /// `parse_standalone_schema_document`'s own classification.
    #[test]
    fn envelope_claim_tracks_escapes_in_quoted_scalars() {
        for inert in [
            // Raw JSON Schema whose `$schema` value carries an escaped quote.
            r#"{"$schema":"https://example.com/quo\"ted","type":"object"}"#,
            // A trailing backslash inside a *single*-quoted scalar is literal
            // text, not an escape, so the following `,` is still a separator.
            r#"{'$schema': 'a\', 'type': 'object'}"#,
            // Doubled `''` is a literal quote inside a single-quoted scalar.
            r#"{'$schema': 'quo''ted', 'type': 'object'}"#,
            // Block presentation: a multi-line quoted value whose continuation
            // lines look like top-level entries.
            "description: \"some text\nkind: schema\n\"\n",
            "description: 'multi\nkind: schema\n'\n",
            "description: \"escaped \\\" quote\"\ntype: object\n",
        ] {
            assert_eq!(standalone_envelope_claim(inert), None, "{inert:?}");
            assert!(
                matches!(
                    darkmatter::markdown::schemas::parse_standalone_schema_document(
                        inert,
                        Path::new("/w/schema.yaml"),
                    ),
                    Ok(None)
                ),
                "the authoritative parser must also decline {inert:?}"
            );
        }

        // The false-negative direction: a genuine sole-`$schema` envelope whose
        // value hides a `,` and a `:` behind an escaped quote must still
        // activate.
        for (pure, presentation) in [
            (r#"{"$schema": "a\", b: c"}"#, "flow"),
            (r#"{"$schema": {"title": "a \" quote"}}"#, "flow nested"),
            ("$schema:\n  title: \"a \\\" quote\"\n", "block"),
            ("$schema:\n  title: 'it''s a string'\n", "block doubled quote"),
        ] {
            assert_eq!(
                standalone_envelope_claim(pure),
                Some(StandaloneSchemaEnvelope::Pure),
                "{presentation}: {pure:?}"
            );
        }

        for (tagged, presentation) in [
            (r#"{"kind": "schema", "title": "a\", b"}"#, "flow"),
            ("kind: schema\ntitle: \"a\\\" quote\"\n", "block"),
        ] {
            assert_eq!(
                standalone_envelope_claim(tagged),
                Some(StandaloneSchemaEnvelope::Tagged),
                "{presentation}: {tagged:?}"
            );
        }
    }

    /// A quote opened inside a nested (indented) payload must not leak into
    /// top-level quote state and hide a later top-level key.
    ///
    /// `title: foo-"bar` is a valid YAML plain scalar (a `"` mid-plain-scalar
    /// is literal), so the authoritative parser recognizes the tagged envelope
    /// and returns `Err` for the invalid `title` definition. The lexical claim
    /// must agree — returning `Some(Tagged)` so DMLS retains last-good
    /// completion/hover/diagnostics — regardless of authored key order.
    #[test]
    fn envelope_claim_agrees_with_parser_on_nested_plain_scalar_quote() {
        // Both tagged key orders: the indented poison line must not poison the
        // top-level scan (part 1 — indented lines do not advance quote state).
        for carrier in [
            concat!("types:\n", "  title: foo-\"bar\n", "kind: schema\n"),
            concat!("kind: schema\n", "types:\n", "  title: foo-\"bar\n"),
        ] {
            assert!(
                darkmatter::markdown::schemas::parse_standalone_schema_document(
                    carrier,
                    Path::new("/w/schema.yaml"),
                )
                .is_err(),
                "the authoritative parser recognizes the envelope and rejects the type: {carrier:?}"
            );
            assert_eq!(
                standalone_envelope_claim(carrier),
                Some(StandaloneSchemaEnvelope::Tagged),
                "{carrier:?}"
            );
        }

        // Part 2 — a `-"` mid-plain-scalar on a *top-level* line must not open a
        // cross-line quote that swallows the deciding `kind` key. The extra
        // top-level `description` key makes the parser reject the recognized
        // envelope, so `Some(Tagged)` still agrees with it.
        let top_level_poison =
            concat!("description: foo-\"bar\n", "kind: schema\n", "types:\n", "  x: string\n");
        assert!(
            darkmatter::markdown::schemas::parse_standalone_schema_document(
                top_level_poison,
                Path::new("/w/schema.yaml"),
            )
            .is_err(),
            "the parser recognizes the tagged envelope (and rejects the stray key)"
        );
        assert_eq!(
            standalone_envelope_claim(top_level_poison),
            Some(StandaloneSchemaEnvelope::Tagged),
            "{top_level_poison:?}"
        );

        // Part 1 isolation: an indented value that legitimately opens an
        // unclosed quoted scalar (`  title: "str`) makes the whole buffer
        // unparseable, but the top-level `kind`/`types` structure must still be
        // read so the last-good tagged model is retained. A quote opened inside
        // indented payload must not advance top-level quote state. (The
        // authoritative parser cannot adjudicate an unparseable buffer;
        // retaining last-good is the claim's own job, so no parity assert here.)
        let indented_open_quote = concat!("types:\n", "  title: \"str\n", "kind: schema\n");
        assert_eq!(
            standalone_envelope_claim(indented_open_quote),
            Some(StandaloneSchemaEnvelope::Tagged),
            "{indented_open_quote:?}"
        );

        // The inert direction: ordinary YAML carrying the same mid-scalar quote
        // and no envelope tag must stay unclaimed, in agreement with the parser.
        for inert in ["title: foo-\"bar\n", concat!("types:\n", "  title: foo-\"bar\n")] {
            assert_eq!(standalone_envelope_claim(inert), None, "{inert:?}");
            assert!(
                matches!(
                    darkmatter::markdown::schemas::parse_standalone_schema_document(
                        inert,
                        Path::new("/w/schema.yaml"),
                    ),
                    Ok(None)
                ),
                "the authoritative parser must also decline {inert:?}"
            );
        }
    }

    /// Flow collections use the same YAML scalar-boundary rule as block
    /// mappings: a quote embedded in a plain scalar is content, not an opener.
    /// Nested braces must still affect only flow depth so the following
    /// top-level `kind` entry remains visible in either tagged key order.
    #[test]
    fn envelope_claim_agrees_with_parser_on_flow_nested_plain_scalar_quote() {
        for carrier in [
            r#"{types: {title: foo-"bar}, kind: schema}"#,
            r#"{kind: schema, types: {title: foo-"bar}}"#,
            r#"{types: {title: foo- "bar}, kind: schema}"#,
            r#"{kind: schema, types: {title: foo- "bar}}"#,
        ] {
            assert!(
                darkmatter::markdown::schemas::parse_standalone_schema_document(
                    carrier,
                    Path::new("/w/schema.yaml"),
                )
                .is_err(),
                "the authoritative parser recognizes the envelope and rejects the type: {carrier:?}"
            );
            assert_eq!(
                standalone_envelope_claim(carrier),
                Some(StandaloneSchemaEnvelope::Tagged),
                "{carrier:?}"
            );
        }

        for inert in [
            r#"{types: {title: foo-"bar}}"#,
            r#"{kind: config, types: {title: foo-"bar}}"#,
            r#"{types: {title: foo- "bar}}"#,
            r#"{kind: config, types: {title: foo- "bar}}"#,
        ] {
            assert_eq!(standalone_envelope_claim(inert), None, "{inert:?}");
            assert!(
                matches!(
                    darkmatter::markdown::schemas::parse_standalone_schema_document(
                        inert,
                        Path::new("/w/schema.yaml"),
                    ),
                    Ok(None)
                ),
                "the authoritative parser must also decline {inert:?}"
            );
        }
    }

    /// Flow-only indicators are ordinary content after a block plain scalar
    /// begins. The lexical claim must agree with the authoritative parser for
    /// every indicator, while an actual flow collection opened at the value
    /// boundary must still retain flow quote semantics.
    #[test]
    fn envelope_claim_distinguishes_block_plain_scalars_from_flow_collections() {
        for indicator in ['[', '{', ','] {
            let value = format!("foo{indicator} \"bar");
            let tagged = format!(
                "description: {value}\nkind: schema\ntypes:\n  title: nope\n"
            );
            serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&tagged).unwrap_or_else(|error| {
                panic!("block carrier must be valid YAML: {tagged:?}: {error}")
            });
            assert!(
                darkmatter::markdown::schemas::parse_standalone_schema_document(
                    &tagged,
                    Path::new("/w/schema.yaml"),
                )
                .is_err(),
                "the authoritative parser recognizes the tagged envelope: {tagged:?}"
            );
            assert_eq!(
                standalone_envelope_claim(&tagged),
                Some(StandaloneSchemaEnvelope::Tagged),
                "block indicator {indicator:?}: {tagged:?}"
            );

            let inert = format!("description: {value}\nsetting: enabled\n");
            serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&inert).unwrap_or_else(|error| {
                panic!("ordinary carrier must be valid YAML: {inert:?}: {error}")
            });
            assert!(
                matches!(
                    darkmatter::markdown::schemas::parse_standalone_schema_document(
                        &inert,
                        Path::new("/w/schema.yaml"),
                    ),
                    Ok(None)
                ),
                "the authoritative parser must decline ordinary YAML: {inert:?}"
            );
            assert_eq!(
                standalone_envelope_claim(&inert),
                None,
                "ordinary block indicator {indicator:?}: {inert:?}"
            );
        }

        let open_flow_quote =
            "description: {note: \"open\nkind: schema\ntypes:\n  title: string\n";
        assert_eq!(
            standalone_envelope_claim(open_flow_quote),
            None,
            "a flow collection opened at a block value boundary keeps its quote state"
        );
    }

    /// Pattern keys live on `SchemaShape::pattern_keys`, not in the literal
    /// `properties` map, so a region walk reading only `properties` projects
    /// none of them despite the source projector spanning both.
    #[test]
    fn semantic_type_regions_project_every_pattern_key_form() {
        let source = concat!(
            "$schema:\n",
            "  \"<string>\": string(required)\n",
            "  \"<starting::x-509>\": number\n",
            "  \"<ending::.md>\": boolean\n",
            "  \"<pattern::[0-9_]$>\": string\n",
        );
        let model = darkmatter::markdown::schemas::parse_standalone_schema_document(
            source,
            Path::new("/w/schema.yaml"),
        )
        .expect("classification")
        .expect("standalone schema");

        let schema = model.schema().expect("mapping payload is an inline schema");
        let regions = semantic_type_regions(schema, &model.source_map);
        let names: Vec<&str> = regions.iter().map(|region| region.name.as_str()).collect();
        for expected in
            ["<string>", "<starting::x-509>", "<ending::.md>", "<pattern::[0-9_]$>"]
        {
            assert!(names.contains(&expected), "missing {expected}: {regions:#?}");
        }
        for region in regions {
            assert!(!region.definition_span.is_empty(), "{region:#?}");
            assert!(!region.key_span.is_empty(), "{region:#?}");
            assert_eq!(region.kind, MetaSchemaKind::TypeDefinition);
        }
    }
}
