//! One canonical spelling per style field, decided inside one style layer.
//!
//! A `style:` value may spell a field several ways: the page keys are
//! accepted at the top level as well as under `page`, the page spacing has
//! shorthands (`margin`, `mx`, `ml`, ...), and `alignment` reaches every
//! component as well as the page. Normalization rewrites one layer onto the
//! canonical fields of [`StyleFrontmatter`](super::StyleFrontmatter) before
//! that layer meets any other, so that a later layer's broad `margin: 2`
//! beats an earlier layer's `ml: 6`, while inside one layer the most
//! specific spelling wins regardless of key order.
//!
//! ## Rules
//!
//! - A top-level page key (`max-width`, `ml`, `code.theme`, ...) is the same
//!   field as its `page.*` spelling.
//! - Page margin and padding expand all-sides (`margin`), then axis pairs
//!   (`mx`, `my`), then single sides (`ml`, `margin-left`); the most specific
//!   wins.
//! - `alignment` (or `page.alignment`) sets the page alignment and, as the
//!   broadest spelling, the alignment of every [`PageComponent::ALL`]
//!   bucket; a component's own `alignment` in the same layer wins. The
//!   horizontal rule receives `left`, `center`, or `right`, never `full`.
//! - Two different spellings of one field at the same specificity (`ml` and
//!   `page.margin-left`, top-level `max-width` and `page.max-width`) are a
//!   [`SpellingConflict`]; a broad spelling plus a more specific one is not.
//!
//! Component `padding` is not expanded here: the typed component model keeps
//! `padding` beside `padding-left`/`padding-right`, and the layer merge in
//! [`super::layer`] lets a later layer's `padding` replace inherited sides.

use std::collections::HashMap;

use indexmap::IndexMap;
use serde_json::{Map, Value};

use crate::layout::PageComponent;
use crate::style::descriptor::{is_canonical_container, kebabify, leaf_for_canonical, leaf_for_key};

/// The decoded segments of a style path, relative to `style`.
pub(crate) type Segments = Vec<String>;

/// How broadly one spelling sets a field; the higher value wins in a layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Specificity {
    /// `margin`, `padding`, and an `alignment` broadcast to a component.
    Broad,
    /// `mx`, `my`, `px`, `py`.
    Axis,
    /// One field: `ml`, `margin-left`, `table.alignment`, any other key.
    Specific,
}

/// The style bucket a page alignment broadcast reaches for `component`.
pub(crate) fn bucket_of(component: PageComponent) -> &'static str {
    match component {
        PageComponent::Images => "images",
        PageComponent::BlockQuotes => "block-quote",
        PageComponent::Tables => "table",
        PageComponent::CodeBlocks => "code-block",
        PageComponent::Ul => "ul",
        PageComponent::Ol => "ol",
        PageComponent::Li => "li",
        PageComponent::Hyperlinks => "hyperlinks",
        PageComponent::Hr => "hr",
        PageComponent::Disclosure => "disclosure",
    }
}

/// The page fields a page spacing key sets, and how specifically.
fn page_spacing(key: &str) -> Option<(&'static [&'static str], Specificity)> {
    use Specificity::{Axis, Broad, Specific};
    Some(match key {
        "margin" => (&["margin-top", "margin-right", "margin-bottom", "margin-left"], Broad),
        "mx" => (&["margin-left", "margin-right"], Axis),
        "my" => (&["margin-top", "margin-bottom"], Axis),
        "ml" => (&["margin-left"], Specific),
        "mr" => (&["margin-right"], Specific),
        "mt" => (&["margin-top"], Specific),
        "mb" => (&["margin-bottom"], Specific),
        "padding" => (&["padding-top", "padding-right", "padding-bottom", "padding-left"], Broad),
        "px" => (&["padding-left", "padding-right"], Axis),
        "py" => (&["padding-top", "padding-bottom"], Axis),
        "pl" => (&["padding-left"], Specific),
        "pr" => (&["padding-right"], Specific),
        "pt" => (&["padding-top"], Specific),
        "pb" => (&["padding-bottom"], Specific),
        _ => return None,
    })
}

/// Whether the top-level key `key` is a spelling of a `page` key.
pub(crate) fn is_top_level_page_key(key: &str) -> bool {
    key == "code" || (!is_canonical_container(key) && leaf_for_canonical(&format!("page.{key}")).is_some())
}

/// Two spellings of one field inside one layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpellingConflict {
    /// The canonical field both spell.
    pub(crate) canonical: Segments,
    /// The spelling met first, as written.
    pub(crate) first: Segments,
    /// The other spelling, as written.
    pub(crate) second: Segments,
}

/// One canonical field of a normalized layer.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NormalizedEntry {
    /// The value, a JSON leaf (scalar or array).
    pub(crate) value: Value,
    /// The path that supplied it, as written in the layer.
    pub(crate) written: Segments,
}

/// A layer's leaves on canonical paths. No path is a prefix of another.
pub(crate) type NormalizedLayer = IndexMap<Segments, NormalizedEntry>;

/// The leaves of an object: every non-object, non-null value with its path.
///
/// Null members are absence and empty objects supply nothing, so neither
/// appears. Arrays are leaves.
pub(crate) fn flatten(value: &Value) -> Vec<(Segments, Value)> {
    fn walk(map: &Map<String, Value>, prefix: &mut Segments, out: &mut Vec<(Segments, Value)>) {
        for (key, child) in map {
            prefix.push(key.clone());
            match child {
                Value::Null => {}
                Value::Object(inner) => walk(inner, prefix, out),
                leaf => out.push((prefix.clone(), leaf.clone())),
            }
            prefix.pop();
        }
    }
    let mut out = Vec::new();
    if let Value::Object(map) = value {
        walk(map, &mut Vec::new(), &mut out);
    }
    out
}

/// Rebuilds an object from leaves on paths none of which is a prefix of
/// another.
pub(crate) fn unflatten<'a>(leaves: impl IntoIterator<Item = (&'a Segments, &'a Value)>) -> Value {
    let mut root = Map::new();
    for (path, value) in leaves {
        insert_at(&mut root, path, value.clone());
    }
    Value::Object(root)
}

/// Writes `value` at `path` below `root`, replacing any non-object on the
/// way with an object.
pub(crate) fn insert_at(root: &mut Map<String, Value>, path: &[String], value: Value) {
    let Some((last, parents)) = path.split_last() else {
        return;
    };
    let mut current = root;
    for segment in parents {
        let slot = current.entry(segment.clone()).or_insert_with(|| Value::Object(Map::new()));
        if !slot.is_object() {
            *slot = Value::Object(Map::new());
        }
        current = slot.as_object_mut().expect("just made an object");
    }
    current.insert(last.clone(), value);
}

/// Rewrites the documented snake-case aliases of a document's `style:` value
/// to their kebab-case keys, the way the schema walker reads them.
///
/// A key is rewritten only where the walker accepts it as an alias (a known
/// container, or a leaf whose documented alias spells it); any other key is
/// kept as written. Returns the rewritten value, the raw path of every
/// rewritten path, and the alias pairs that collide with a key already
/// present (`max_width` beside `max-width`).
pub(crate) fn canonicalize_aliases(value: &Value) -> (Value, HashMap<Segments, Segments>, Vec<SpellingConflict>) {
    fn walk(
        map: &Map<String, Value>,
        canonical: &mut Segments,
        raw: &mut Segments,
        raw_of: &mut HashMap<Segments, Segments>,
        conflicts: &mut Vec<SpellingConflict>,
    ) -> Map<String, Value> {
        let mut out = Map::new();
        let mut written_as: HashMap<String, String> = HashMap::new();
        for (key, child) in map {
            let segment = kebabify(key);
            let mut probe = canonical.clone();
            probe.push(segment.clone());
            let dotted = probe.join(".");
            let accepted = leaf_for_key(&dotted, key).is_some() || is_canonical_container(&dotted);
            let output_key = if accepted { segment } else { key.clone() };

            raw.push(key.clone());
            canonical.push(output_key.clone());
            raw_of.insert(canonical.clone(), raw.clone());
            if let Some(previous) = written_as.get(&output_key) {
                let mut first = raw.clone();
                first.pop();
                first.push(previous.clone());
                conflicts.push(SpellingConflict {
                    canonical: canonical.clone(),
                    first,
                    second: raw.clone(),
                });
            } else {
                written_as.insert(output_key.clone(), key.clone());
                let value = match child {
                    // Leaves keep their values; only container keys recurse.
                    Value::Object(inner) if leaf_for_canonical(&canonical.join(".")).is_none() => {
                        Value::Object(walk(inner, canonical, raw, raw_of, conflicts))
                    }
                    other => other.clone(),
                };
                out.insert(output_key, value);
            }
            canonical.pop();
            raw.pop();
        }
        out
    }

    let mut raw_of = HashMap::new();
    let mut conflicts = Vec::new();
    let rewritten = match value {
        Value::Object(map) => Value::Object(walk(map, &mut Vec::new(), &mut Vec::new(), &mut raw_of, &mut conflicts)),
        other => other.clone(),
    };
    (rewritten, raw_of, conflicts)
}

/// One spelling's claim on a canonical field.
struct Claim {
    value: Value,
    written: Segments,
    specificity: Specificity,
    broadcast: bool,
}

/// The canonical claims a written leaf makes.
fn claims_of(written: &Segments, value: &Value) -> Vec<(Segments, Claim)> {
    let mut canonical = written.clone();
    if written.first().is_some_and(|key| is_top_level_page_key(key)) {
        canonical.insert(0, "page".to_string());
    }
    let claim = |path: Segments, value: Value, specificity, broadcast| {
        (
            path,
            Claim {
                value,
                written: written.clone(),
                specificity,
                broadcast,
            },
        )
    };
    if let [page, key] = canonical.as_slice()
        && page == "page"
    {
        if let Some((sides, specificity)) = page_spacing(key) {
            return sides
                .iter()
                .map(|side| claim(vec!["page".into(), (*side).into()], value.clone(), specificity, false))
                .collect();
        }
        if key == "alignment" {
            let mut claims = vec![claim(canonical.clone(), value.clone(), Specificity::Specific, false)];
            if let Some(name) = broadcast_alignment(value) {
                claims.extend(PageComponent::ALL.into_iter().map(|component| {
                    claim(
                        vec![bucket_of(component).into(), "alignment".into()],
                        Value::String(name.into()),
                        Specificity::Broad,
                        true,
                    )
                }));
            }
            return claims;
        }
    }
    vec![claim(canonical, value.clone(), Specificity::Specific, false)]
}

/// The canonical name a page alignment broadcasts, or `None` for a value the
/// page field itself will reject. Every component, the horizontal rule
/// included, receives `left`, `center`, or `right`.
fn broadcast_alignment(value: &Value) -> Option<&'static str> {
    use renderable::layout::Alignment;
    match crate::style::alignment::parse(value.as_str()?).ok()? {
        Alignment::Left => Some("left"),
        Alignment::Center => Some("center"),
        Alignment::Right => Some("right"),
    }
}

/// Normalizes one layer's leaves (see the module docs).
///
/// ## Errors
///
/// Every [`SpellingConflict`] in the layer.
pub(crate) fn normalize_leaves(leaves: Vec<(Segments, Value)>) -> Result<NormalizedLayer, Vec<SpellingConflict>> {
    let mut claims: IndexMap<Segments, Vec<Claim>> = IndexMap::new();
    for (written, value) in &leaves {
        for (path, claim) in claims_of(written, value) {
            claims.entry(path).or_default().push(claim);
        }
    }

    let mut conflicts: Vec<SpellingConflict> = Vec::new();
    let mut record = |conflict: SpellingConflict| {
        let seen = conflicts
            .iter()
            .any(|known| known.first == conflict.first && known.second == conflict.second);
        if !seen {
            conflicts.push(conflict);
        }
    };
    let mut layer = NormalizedLayer::new();
    let mut broadcast: Vec<Segments> = Vec::new();
    for (path, candidates) in claims {
        let best = candidates.iter().map(|claim| claim.specificity).max().expect("a claimed path has a claim");
        let mut winners = candidates.into_iter().filter(|claim| claim.specificity == best);
        let winner = winners.next().expect("the best specificity has a claim");
        for other in winners {
            record(SpellingConflict {
                canonical: path.clone(),
                first: winner.written.clone(),
                second: other.written,
            });
        }
        if winner.broadcast {
            broadcast.push(path.clone());
        }
        layer.insert(
            path,
            NormalizedEntry {
                value: winner.value,
                written: winner.written,
            },
        );
    }

    // A path and its own descendant can only both be claimed through two
    // spellings of one container (`code: x` beside `page.code.theme`).
    let paths: Vec<&Segments> = layer.keys().filter(|path| !broadcast.contains(path)).collect();
    for ancestor in &paths {
        for descendant in &paths {
            if descendant.len() > ancestor.len() && descendant.starts_with(ancestor) {
                record(SpellingConflict {
                    canonical: (*ancestor).clone(),
                    first: layer[*ancestor].written.clone(),
                    second: layer[*descendant].written.clone(),
                });
            }
        }
    }

    if conflicts.is_empty() { Ok(layer) } else { Err(conflicts) }
}

/// A document's `style:` value on canonical fields, for the typed parser.
///
/// Aliases are rewritten ([`canonicalize_aliases`]) and the layer is
/// normalized. A non-object is returned unchanged so the typed parser
/// reports its shape.
///
/// ## Errors
///
/// Every [`SpellingConflict`], alias collisions first.
pub(crate) fn normalize_document_value(value: &Value) -> Result<Value, Vec<SpellingConflict>> {
    if !value.is_object() {
        return Ok(value.clone());
    }
    let (rewritten, _, alias_conflicts) = canonicalize_aliases(value);
    if !alias_conflicts.is_empty() {
        return Err(alias_conflicts);
    }
    let layer = normalize_leaves(flatten(&rewritten))?;
    Ok(unflatten(layer.iter().map(|(path, entry)| (path, &entry.value))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn normalized(value: Value) -> Value {
        normalize_document_value(&value).expect("no conflict")
    }

    #[test]
    fn specific_side_beats_all_sides_in_either_order() {
        for value in [json!({"margin": 2, "ml": 6}), json!({"ml": 6, "margin": 2})] {
            assert_eq!(
                normalized(value)["page"],
                json!({"margin-top": 2, "margin-right": 2, "margin-bottom": 2, "margin-left": 6})
            );
        }
    }

    #[test]
    fn axis_beats_all_sides_and_side_beats_axis() {
        let page = &normalized(json!({"page": {"padding": 1, "px": 3, "pr": 5}}))["page"];
        assert_eq!(page, &json!({"padding-top": 1, "padding-right": 5, "padding-bottom": 1, "padding-left": 3}));
    }

    #[test]
    fn two_spellings_of_one_field_conflict() {
        let conflicts = normalize_document_value(&json!({"ml": 1, "page": {"margin-left": 2}})).unwrap_err();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].canonical, ["page", "margin-left"]);
    }

    #[test]
    fn alignment_broadcasts_below_component_alignment() {
        let value = normalized(json!({"alignment": "centered", "table": {"alignment": "left"}}));
        assert_eq!(value["page"]["alignment"], "centered");
        assert_eq!(value["table"]["alignment"], "left");
        assert_eq!(value["hr"]["alignment"], "center");
        assert_eq!(value["code-block"]["alignment"], "center");
    }

    #[test]
    fn invalid_alignment_is_not_broadcast() {
        let value = normalized(json!({"alignment": "diagonal"}));
        assert_eq!(value, json!({"page": {"alignment": "diagonal"}}));
    }

    #[test]
    fn aliases_rewrite_and_collide() {
        assert_eq!(normalized(json!({"block_quote": {"max_width": 4}})), json!({"block-quote": {"max-width": 4}}));
        assert!(normalize_document_value(&json!({"page": {"max_width": 4, "max-width": 5}})).is_err());
    }

    #[test]
    fn nulls_and_empty_objects_disappear() {
        assert_eq!(normalized(json!({"page": {"color": null}, "table": {}})), json!({}));
    }
}
