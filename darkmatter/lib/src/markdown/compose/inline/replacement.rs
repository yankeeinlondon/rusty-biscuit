//! Text-replacement compose stage.

use super::super::super::Markdown;
use super::super::context::effective_state as state;
use super::super::body_origin::{self, BodyOrigin};
use super::super::replacement;
use super::super::{ComposeOptions, EffectiveState, EffectiveStateBuilder};
use serde_json::Value;
use std::collections::HashMap;
use tracing::debug;

/// Runs the text replacement stage.
///
/// Applies text replacements from the `replace` map in effective state and
/// carries `origin` past them.
/// See [`replacement::apply_replacements`] for algorithm details.
///
/// ## Returns
///
/// The number of replacements applied.
pub(crate) fn run_stage(
    markdown: &mut Markdown,
    state: &EffectiveState,
    options: &ComposeOptions,
    origin: &mut Option<BodyOrigin>,
) -> usize {
    let (new_content, edits) = if let Some(one_off) = &options.one_off_replace {
        let merged_replace = state::merge_replace_maps(state.get_replace_map(), Some(one_off));
        let mut frontmatter = HashMap::new();
        frontmatter.insert("replace".to_string(), Value::Object(merged_replace));
        let scoped_state = EffectiveStateBuilder::new()
            .with_frontmatter(frontmatter)
            .with_context(options.context().clone())
            .build()
            .expect("replace-only state has no user ctx");
        replacement::apply_replacements_with_edits(markdown.content(), &scoped_state)
    } else {
        replacement::apply_replacements_with_edits(markdown.content(), state)
    };
    let count = edits.len();
    if count > 0 {
        body_origin::advance(origin, markdown.content(), &edits, &new_content);
        *markdown.content_mut() = new_content;
    }
    debug!(count, "compose: text replacements applied");
    count
}
