//! Engine for evaluating and rendering page block regions.

use std::ops::Range;

use super::types::{PageBlockError, PageBlockRegion};
use crate::markdown::compose::conditions;
use crate::markdown::compose::expression::EvaluationLookup;
use crate::markdown::compose::ComposeReport;
use biscuit_terminal::errors::SourceContext;
use tracing::debug;

/// Renders page blocks by evaluating conditions and splicing content.
///
/// Walks the parsed region tree in source order, evaluates each block's
/// `when` condition, and produces output with true blocks' body content
/// preserved and false blocks removed entirely.
pub fn render_page_blocks<L: EvaluationLookup>(
    content: &str,
    regions: &[PageBlockRegion],
    state: &L,
    report: &mut ComposeReport,
    ctx: SourceContext,
) -> Result<String, PageBlockError> {
    debug!(region_count = regions.len(), "page_blocks: rendering");

    let mut output = String::with_capacity(content.len());
    let mut cursor = 0;

    for region in regions {
        // Append literal text before this block
        output.push_str(&content[cursor..region.span.start]);

        let condition = match &region.options.when_expr {
            Some(expr) => {
                for warning in
                    conditions::collect_condition_context_warnings(expr, state, "condition")
                {
                    report.add_warning(warning.at_line(region.start_line));
                }
                conditions::evaluate_condition(expr, state, region.start_line, ctx.clone())?
            }
            None => true, // No when → treated as enabled
        };

        if condition {
            // Render body, recursively processing nested children
            let body = render_body(
                content,
                &region.body_span,
                &region.children,
                state,
                report,
                &ctx,
            )?;
            output.push_str(&body);
            report.page_blocks_rendered += 1;
        } else {
            // False block: append nothing
            report.page_blocks_skipped += 1;
        }

        cursor = region.span.end;
    }

    // Append trailing content after the last block
    output.push_str(&content[cursor..]);

    Ok(output)
}

/// Renders the body of a block, recursively processing nested child blocks.
fn render_body<L: EvaluationLookup>(
    content: &str,
    body_span: &Range<usize>,
    children: &[PageBlockRegion],
    state: &L,
    report: &mut ComposeReport,
    ctx: &SourceContext,
) -> Result<String, PageBlockError> {
    if children.is_empty() {
        return Ok(content[body_span.clone()].to_string());
    }

    let mut output = String::new();
    let mut cursor = body_span.start;

    for child in children {
        // Append text between cursor and child block
        output.push_str(&content[cursor..child.span.start]);

        let condition = match &child.options.when_expr {
            Some(expr) => {
                for warning in
                    conditions::collect_condition_context_warnings(expr, state, "condition")
                {
                    report.add_warning(warning.at_line(child.start_line));
                }
                conditions::evaluate_condition(expr, state, child.start_line, ctx.clone())?
            }
            None => true,
        };

        if condition {
            let body = render_body(
                content,
                &child.body_span,
                &child.children,
                state,
                report,
                ctx,
            )?;
            output.push_str(&body);
            report.page_blocks_rendered += 1;
        } else {
            report.page_blocks_skipped += 1;
        }

        cursor = child.span.end;
    }

    // Append remaining body content after the last child
    output.push_str(&content[cursor..body_span.end]);

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::page_blocks::parser::parse_page_blocks;
    use crate::markdown::compose::{ComposeContext, EffectiveState, EffectiveStateBuilder};
    use serde_json::json;
    use std::collections::HashMap;

    fn state_with(data: serde_json::Value) -> EffectiveState {
        let fm: HashMap<String, serde_json::Value> = match data {
            serde_json::Value::Object(map) => map.into_iter().collect(),
            _ => HashMap::new(),
        };
        EffectiveStateBuilder::new()
            .with_frontmatter(fm)
            .with_context(ComposeContext::fixed_for_testing())
            .build()
            .unwrap()
    }

    fn render(content: &str, state: &EffectiveState) -> (String, ComposeReport) {
        use std::path::PathBuf;
        use std::sync::Arc;
        let source = SourceContext::new(
            PathBuf::from("/test.md"),
            PathBuf::from("test.md"),
            Arc::from(content),
        );
        let regions = parse_page_blocks(content, source.clone()).unwrap();
        let mut report = ComposeReport::default();
        let output = render_page_blocks(content, &regions, state, &mut report, source).unwrap();
        (output, report)
    }

    #[test]
    fn when_true_preserves_body() {
        let content = "before\n::block when=\"flag\"\nbody\n::end-block\nafter\n";
        let state = state_with(json!({"flag": true}));
        let (output, report) = render(content, &state);
        assert_eq!(output, "before\nbody\nafter\n");
        assert_eq!(report.page_blocks_rendered, 1);
        assert_eq!(report.page_blocks_skipped, 0);
    }

    #[test]
    fn when_false_removes_block() {
        let content = "before\n::block when=\"flag\"\nbody\n::end-block\nafter\n";
        let state = state_with(json!({"flag": false}));
        let (output, report) = render(content, &state);
        assert_eq!(output, "before\nafter\n");
        assert_eq!(report.page_blocks_rendered, 0);
        assert_eq!(report.page_blocks_skipped, 1);
    }

    #[test]
    fn when_omitted_treated_as_true() {
        let content = "::block\nbody\n::end-block\n";
        let state = state_with(json!({}));
        let (output, report) = render(content, &state);
        assert_eq!(output, "body\n");
        assert_eq!(report.page_blocks_rendered, 1);
    }

    #[test]
    fn frontmatter_variable_resolution() {
        let content = "::block when=\"state == 'foo'\"\nfoo content\n::end-block\n";
        let state = state_with(json!({"state": "foo"}));
        let (output, _) = render(content, &state);
        assert_eq!(output, "foo content\n");
    }

    #[test]
    fn env_variable_resolution() {
        let content = "::block when=\"env.AGENT\"\nagent content\n::end-block\n";
        let mut ctx = ComposeContext::fixed_for_testing();
        ctx.env_mut()
            .insert("AGENT".to_string(), "claude".to_string());
        let state = EffectiveStateBuilder::new()
            .with_frontmatter(HashMap::new())
            .with_context(ctx)
            .build()
            .unwrap();

        let source = SourceContext::new(
            std::path::PathBuf::from("/test.md"),
            std::path::PathBuf::from("test.md"),
            std::sync::Arc::from(content),
        );
        let regions = parse_page_blocks(content, source.clone()).unwrap();
        let mut report = ComposeReport::default();
        let output = render_page_blocks(content, &regions, &state, &mut report, source).unwrap();
        assert_eq!(output, "agent content\n");
    }

    #[test]
    fn true_outer_false_inner() {
        let content = "::block when=\"outer\"\nouter body\n::block when=\"inner\"\ninner body\n::end-block\nouter tail\n::end-block\n";
        let state = state_with(json!({"outer": true, "inner": false}));
        let (output, report) = render(content, &state);
        assert_eq!(output, "outer body\nouter tail\n");
        assert_eq!(report.page_blocks_rendered, 1);
        assert_eq!(report.page_blocks_skipped, 1);
    }

    #[test]
    fn false_outer_skips_everything() {
        let content = "::block when=\"outer\"\nouter body\n::block when=\"inner\"\ninner body\n::end-block\n::end-block\n";
        let state = state_with(json!({"outer": false, "inner": true}));
        let (output, report) = render(content, &state);
        assert_eq!(output, "");
        assert_eq!(report.page_blocks_rendered, 0);
        // Only the outer block is counted as skipped; inner is never evaluated
        assert_eq!(report.page_blocks_skipped, 1);
    }

    #[test]
    fn content_preserved_byte_for_byte() {
        let content = "aaa\n::block when=\"x\"\nbbb\n::end-block\nccc\n::block when=\"y\"\nddd\n::end-block\neee\n";
        let state = state_with(json!({"x": true, "y": true}));
        let (output, _) = render(content, &state);
        assert_eq!(output, "aaa\nbbb\nccc\nddd\neee\n");
    }

    #[test]
    fn report_counters_incremented() {
        let content = "::block when=\"a\"\nA\n::end-block\n::block when=\"b\"\nB\n::end-block\n::block when=\"c\"\nC\n::end-block\n";
        let state = state_with(json!({"a": true, "b": false, "c": true}));
        let (_, report) = render(content, &state);
        assert_eq!(report.page_blocks_rendered, 2);
        assert_eq!(report.page_blocks_skipped, 1);
    }

    #[test]
    fn condition_parse_error_propagates() {
        let content = "::block when=\"==\"\nbody\n::end-block\n";
        let state = state_with(json!({}));
        let source = SourceContext::new(
            std::path::PathBuf::from("/test.md"),
            std::path::PathBuf::from("test.md"),
            std::sync::Arc::from(content),
        );
        let regions = parse_page_blocks(content, source.clone()).unwrap();
        let mut report = ComposeReport::default();
        let result = render_page_blocks(content, &regions, &state, &mut report, source);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PageBlockError::Condition(_)));
    }

    // ── Edge case regression tests ──────────────────────────────────────

    #[test]
    fn empty_block_body_true() {
        let content = "before\n::block when=\"x\"\n::end-block\nafter\n";
        let state = state_with(json!({"x": true}));
        let (output, report) = render(content, &state);
        assert_eq!(output, "before\nafter\n");
        assert_eq!(report.page_blocks_rendered, 1);
    }

    #[test]
    fn block_at_start_of_file() {
        let content = "::block when=\"x\"\nfirst\n::end-block\nrest\n";
        let state = state_with(json!({"x": true}));
        let (output, _) = render(content, &state);
        assert_eq!(output, "first\nrest\n");
    }

    #[test]
    fn block_at_end_of_file() {
        let content = "start\n::block when=\"x\"\nlast\n::end-block";
        let state = state_with(json!({"x": true}));
        let (output, _) = render(content, &state);
        assert_eq!(output, "start\nlast\n");
    }

    #[test]
    fn adjacent_blocks_mixed_conditions() {
        let content = "::block when=\"a\"\nA\n::end-block\n::block when=\"b\"\nB\n::end-block\n::block when=\"c\"\nC\n::end-block\n";
        let state = state_with(json!({"a": false, "b": true, "c": false}));
        let (output, report) = render(content, &state);
        assert_eq!(output, "B\n");
        assert_eq!(report.page_blocks_rendered, 1);
        assert_eq!(report.page_blocks_skipped, 2);
    }

    #[test]
    fn deeply_nested_three_levels_all_true() {
        let content =
            "::block\nL1\n::block\nL2\n::block\nL3\n::end-block\n::end-block\n::end-block\n";
        let state = state_with(json!({}));
        let (output, report) = render(content, &state);
        assert_eq!(output, "L1\nL2\nL3\n");
        assert_eq!(report.page_blocks_rendered, 3);
    }

    #[test]
    fn no_blocks_returns_content_unchanged() {
        let content = "just plain text\nno blocks here\n";
        let state = state_with(json!({}));
        let (output, report) = render(content, &state);
        assert_eq!(output, content);
        assert_eq!(report.page_blocks_rendered, 0);
        assert_eq!(report.page_blocks_skipped, 0);
    }
}
