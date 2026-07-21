//! Finding 25's retained cleanup-pass profile (2026-07-15 performance
//! follow-up).
//!
//! F25 asked whether the stage-2 line passes should be fused into one scan. The
//! recorded disposition is **no-win, not implemented**, so there is no
//! candidate to A/B — the evidence is the profile that bounds the prize, plus a
//! cost model that bounds it independently of any equivalence argument.
//!
//! Phase 10 produced that profile with a harness it then deleted, which is what
//! left the claim unreproducible (benchmarks/README.md, "Harnesses are
//! retained, not deleted"). This module is the retained replacement: it emits
//! per-observation vectors that `benchmarks/recompute.ts` reads.
//!
//! ## What bounds the fusion ceiling
//!
//! Every stage-2 line pass has the same shape: walk `output.lines()` and
//! rebuild a fresh `String::with_capacity(output.len())`. Fusing them can
//! remove the *repeated* walk and rebuild; it cannot remove any pass's per-line
//! work, which a fused scan still has to perform. The cost model prices that
//! removable part directly, so the ceiling needs no claim about whether the
//! passes *could* be fused equivalently.
//!
//! The two arms give the **marginal** cost of one additional scan,
//! `(seven − one) / 6`, which is the correct unit: fusion removes scans, and
//! only the first one in a batch pays first-touch page faults. The ceiling is
//! then `(N − 1) × unit`, where `N` counts the stage-2 passes that actually
//! perform a scan/rebuild on these bytes.
//!
//! `N` is read off the retained per-pass vectors rather than assumed: a pass
//! whose *whole* cost is below one unit cannot be performing one. That yields
//! an upper bound on `N`, hence an upper bound on the ceiling — several passes
//! (`restore_list_markers` on an unordered-list-free document, `unescape_*` on
//! text with nothing to unescape) early-out without scanning at all, so
//! charging fusion for all seven would overstate the prize several-fold.
//!
//! ## Notes
//!
//! Nothing here changes cleanup. The stage replicas call the same private
//! functions `cleanup_content_internal` calls, in the same order, on the same
//! bytes.

use std::hint::black_box;
use std::ops::Range;

use pulldown_cmark::{Event, Parser};
use pulldown_cmark_to_cmark::Options as CmarkOptions;

use super::*;
use crate::perf_harness::{Harness, fixture_text};

/// Serializes an event stream exactly as `cleanup_content_internal` does.
fn cmark_serialize(events: &[Event<'_>]) -> String {
    let mut output = String::new();
    let options = CmarkOptions {
        code_block_token_count: 3,
        increment_ordered_list_bullets: true,
        ..Default::default()
    };
    let borrowed: Vec<_> = events.iter().map(std::borrow::Cow::Borrowed).collect();
    pulldown_cmark_to_cmark::cmark_with_options(borrowed.into_iter(), &mut output, options)
        .expect("fixture serializes");
    output
}

/// The scan-and-rebuild skeleton every stage-2 line pass shares, with all
/// per-line work removed.
///
/// Idempotent, which is what lets the cost model's two arms be equivalence-gated
/// against each other.
fn scan_rebuild(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Times one stage-2 pass, including the `String` clone each sample needs to
/// restore its input. `f25-<label>-stage2-string-clone` measures that clone
/// alone so it can be subtracted from the retained vectors.
fn profile_line_pass(harness: &Harness, name: &str, input: &str, mut pass: impl FnMut(&mut String)) {
    harness.single(name, || {
        let mut working = input.to_string();
        pass(&mut working);
        black_box(working);
    });
}

/// Profiles one fixture's cleanup stages and its fusion ceiling.
fn profile_fixture(harness: &Harness, label: &str, raw: &str) {
    println!("f25: {label} ({} bytes)", raw.len());

    harness.single(&format!("f25-{label}-cleanup-content"), || {
        black_box(cleanup_content(black_box(raw)));
    });
    harness.single(&format!("f25-{label}-strip-incidental-newlines"), || {
        black_box(strip_incidental_newlines(black_box(raw)));
    });

    // From here the replica follows `cleanup_content_internal` step by step on
    // the bytes it would actually see.
    let content = strip_incidental_newlines(raw);
    harness.single(&format!("f25-{label}-stage1-parse"), || {
        let events: Vec<(Event, Range<usize>)> = Parser::new_ext(&content, cleanup_parser_options())
            .into_offset_iter()
            .collect();
        black_box(events);
    });
    let events_with_ranges: Vec<(Event, Range<usize>)> =
        Parser::new_ext(&content, cleanup_parser_options())
            .into_offset_iter()
            .collect();
    let opaque_body_lines = opaque_directive_body_lines(&content);

    harness.single(&format!("f25-{label}-stage1-extract-list-markers"), || {
        black_box(extract_list_markers(
            &content,
            &events_with_ranges,
            &opaque_body_lines,
        ));
    });
    let list_markers = extract_list_markers(&content, &events_with_ranges, &opaque_body_lines);
    let list_item_contexts = extract_list_item_contexts(&events_with_ranges, &opaque_body_lines);
    let additional_paragraph_contexts =
        extract_additional_paragraph_contexts(&content, &events_with_ranges, &opaque_body_lines);
    let protected_markers =
        extract_indented_code_markers(&events_with_ranges, &opaque_body_lines);
    let preferred_style = get_preferred_emphasis_style();
    harness.single(&format!("f25-{label}-stage1-preserve-emphasis"), || {
        black_box(preserve_original_emphasis(
            &content,
            &events_with_ranges,
            preferred_style,
        ));
    });
    let events = preserve_original_emphasis(&content, &events_with_ranges, preferred_style);

    // `add_text_language_to_empty_code_blocks` and `align_tables_in_stream`
    // consume their stream, so each sample must clone it back;
    // `f25-<label>-stage1-events-clone` isolates that clone.
    harness.single(&format!("f25-{label}-stage1-events-clone"), || {
        black_box(events.clone());
    });
    harness.single(&format!("f25-{label}-stage1-add-text-language"), || {
        black_box(add_text_language_to_empty_code_blocks(events.clone()));
    });
    let with_text_lang = add_text_language_to_empty_code_blocks(events.clone());
    harness.single(&format!("f25-{label}-stage1-align-tables"), || {
        black_box(align_tables_in_stream(with_text_lang.clone()));
    });
    let processed = align_tables_in_stream(with_text_lang.clone());
    harness.single(&format!("f25-{label}-stage1-cmark-serialize"), || {
        black_box(cmark_serialize(&processed));
    });

    // Stage-2 inputs: each pass sees the previous pass's output.
    let serialized = cmark_serialize(&processed);
    let mut after_restore_emphasis = serialized.clone();
    restore_emphasis_placeholders(&mut after_restore_emphasis);
    let mut after_unescape_emphasis = after_restore_emphasis.clone();
    unescape_emphasis_chars(&mut after_unescape_emphasis);
    let mut after_normalize_spacing = after_unescape_emphasis.clone();
    normalize_list_spacing(
        &mut after_normalize_spacing,
        ListSpacingMode::Normal,
        &protected_markers,
    );
    let mut after_blockquote = after_normalize_spacing.clone();
    fix_blockquote_formatting(&mut after_blockquote, &protected_markers);
    let mut after_restore_markers = after_blockquote.clone();
    restore_list_markers(
        &mut after_restore_markers,
        &list_markers,
        &protected_markers,
    );
    let mut after_fix_indentation = after_restore_markers.clone();
    fix_list_indentation(
        &mut after_fix_indentation,
        DEFAULT_INDENT,
        &list_item_contexts,
        &additional_paragraph_contexts,
        &protected_markers,
    );

    harness.single(&format!("f25-{label}-stage2-string-clone"), || {
        black_box(after_unescape_emphasis.clone());
    });
    profile_line_pass(
        harness,
        &format!("f25-{label}-stage2-restore-emphasis-placeholders"),
        &serialized,
        restore_emphasis_placeholders,
    );
    profile_line_pass(
        harness,
        &format!("f25-{label}-stage2-unescape-emphasis-chars"),
        &after_restore_emphasis,
        unescape_emphasis_chars,
    );
    profile_line_pass(
        harness,
        &format!("f25-{label}-stage2-normalize-list-spacing"),
        &after_unescape_emphasis,
        |working| {
            normalize_list_spacing(
                working,
                ListSpacingMode::Normal,
                &protected_markers,
            )
        },
    );
    profile_line_pass(
        harness,
        &format!("f25-{label}-stage2-fix-blockquote-formatting"),
        &after_normalize_spacing,
        |working| fix_blockquote_formatting(working, &protected_markers),
    );
    profile_line_pass(
        harness,
        &format!("f25-{label}-stage2-restore-list-markers"),
        &after_blockquote,
        |working| {
            restore_list_markers(working, &list_markers, &protected_markers)
        },
    );
    profile_line_pass(
        harness,
        &format!("f25-{label}-stage2-fix-list-indentation"),
        &after_restore_markers,
        |working| {
            fix_list_indentation(
                working,
                DEFAULT_INDENT,
                &list_item_contexts,
                &additional_paragraph_contexts,
                &protected_markers,
            )
        },
    );
    profile_line_pass(
        harness,
        &format!("f25-{label}-stage2-unescape-brackets"),
        &after_fix_indentation,
        unescape_brackets,
    );
    profile_line_pass(
        harness,
        &format!("f25-{label}-stage2-trailing-trim"),
        &after_fix_indentation,
        |working| {
            let mut normalized = working.trim_start_matches('\n').to_string();
            if !normalized.is_empty() {
                normalized.truncate(normalized.trim_end_matches('\n').len());
                normalized.push('\n');
            }
            *working = normalized;
        },
    );

    // The cost model. Both arms are byte-equivalent (scan_rebuild is
    // idempotent), so the gate below is a real assertion rather than a comment.
    let model_input = &after_unescape_emphasis;
    let seven = || {
        let mut text = scan_rebuild(model_input);
        for _ in 1..7 {
            text = scan_rebuild(&text);
        }
        text
    };
    let one = || scan_rebuild(model_input);
    assert_eq!(
        seven(),
        one(),
        "the cost model's two arms must agree, or the difference is not pure scan/rebuild overhead",
    );
    harness.interleaved_pair(
        &format!("f25-{label}-costmodel-seven-scan-rebuild"),
        || {
            black_box(seven());
        },
        &format!("f25-{label}-costmodel-one-scan-rebuild"),
        || {
            black_box(one());
        },
    );
}

/// Emits F25's retained per-observation vectors for `toc_large` and
/// `replace_heavy`.
#[test]
#[ignore = "measurement harness; run explicitly with DM_PERF_RAW_DIR set"]
fn f25_cleanup_profile_raw_samples() {
    let Some(harness) = Harness::from_env(50, 10) else {
        return;
    };
    profile_fixture(&harness, "toc-large", &fixture_text("toc_large"));
    profile_fixture(&harness, "replace-heavy", &fixture_text("replace_heavy"));
}
