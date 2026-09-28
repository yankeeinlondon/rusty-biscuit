//! Text-replacement compose stage.

use super::super::super::Markdown;
use super::super::context::effective_state as state;
use super::super::body_origin::{BodyProvenance, EditOrigin};
use super::super::value_origin::{DataPaths, OverrideOrigin, ValuePathSegment};
use super::super::super::types::MarkdownResult;
use super::super::replacement;
use super::super::{ComposeOptions, EffectiveState, EffectiveStateBuilder};
use serde_json::Value;
use std::collections::HashMap;
use tracing::debug;

/// Runs the text replacement stage.
///
/// Applies text replacements from the `replace` map in effective state and
/// carries `body` past them. A replacement writes authored text when its
/// `replace` value is authored and data when the value is data
/// (`frontmatter_data`, an inherited parent value, or a data one-off map).
/// See [`replacement::apply_replacements`] for algorithm details.
///
/// ## Returns
///
/// The number of replacements applied.
pub(crate) fn run_stage(
    markdown: &mut Markdown,
    state: &EffectiveState,
    options: &ComposeOptions,
    body: &mut BodyProvenance,
    frontmatter_data: &DataPaths,
) -> MarkdownResult<usize> {
    let as_edit_origin = |origin: OverrideOrigin| match origin {
        OverrideOrigin::Authored => EditOrigin::Authored,
        OverrideOrigin::Data => EditOrigin::Data,
    };
    let parent_replace = options
        .external_state
        .as_ref()
        .and_then(|state| state.get("replace"))
        .and_then(Value::as_object);
    let origin_of = |key: &str| -> EditOrigin {
        if options.one_off_replace.as_ref().is_some_and(|map| map.contains_key(key)) {
            return as_edit_origin(options.inherited_origin.one_off_replace);
        }
        if options.replace_parent_wins && parent_replace.is_some_and(|map| map.contains_key(key)) {
            return as_edit_origin(options.inherited_origin.external_state);
        }
        let path = [
            ValuePathSegment::Key("replace".to_string()),
            ValuePathSegment::Key(key.to_string()),
        ];
        if frontmatter_data.is_data(&path) { EditOrigin::Data } else { EditOrigin::Authored }
    };
    let (new_content, edits) = if let Some(one_off) = &options.one_off_replace {
        let merged_replace = state::merge_replace_maps(state.get_replace_map(), Some(one_off));
        let mut frontmatter = HashMap::new();
        frontmatter.insert("replace".to_string(), Value::Object(merged_replace));
        let scoped_state = EffectiveStateBuilder::new()
            .with_frontmatter(frontmatter)
            .with_context(options.context().clone())
            .build()
            .expect("replace-only state has no user ctx");
        replacement::apply_replacements_with_edits(markdown.content(), &scoped_state, &origin_of)
    } else {
        replacement::apply_replacements_with_edits(markdown.content(), state, &origin_of)
    };
    let count = edits.len();
    if count > 0 {
        body.advance(markdown.content(), &edits, &new_content)?;
        *markdown.content_mut() = new_content;
    }
    debug!(count, "compose: text replacements applied");
    Ok(count)
}
