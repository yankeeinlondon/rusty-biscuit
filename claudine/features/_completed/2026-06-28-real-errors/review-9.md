---
ready: false
implemented: true
agent: "codex/default"
created: "2026-06-29T11:50:38"
---

# Review 9

## Findings

### High - The transport guard allowlists string-collapsed boundary errors that the design requires typed

The feature's transport rule is explicit: errors crossing the Darkmatter/Claudine boundary should travel by `#[from]`/`#[source]`, not by `to_string()` or `format!("{e}")`, and the design calls out `resolve.rs` and `sequence.rs` as conversion targets. The current implementation still preserves several of those boundary failures as formatted strings:

- `CompositionError::MarkdownLoad(String)` remains the sink for non-frontmatter Markdown load/read failures (`claudine/lib/src/composition/error.rs`:58-60).
- `resolve_composition_source` formats both read failures and non-frontmatter `MarkdownError`s into `MarkdownLoad` (`claudine/lib/src/composition/resolve.rs`:12-23, `claudine/lib/src/composition/resolve.rs`:68-76).
- `CompositionError::SequenceExternalLoad(String)` remains the external sequence load sink (`claudine/lib/src/composition/error.rs`:838-840).
- `resolve_sequence_reference` and `load_external_sequence` format `FileReferenceError`, YAML load errors, JSON conversion errors, and synthetic not-found/home-dir failures into that string variant (`claudine/lib/src/composition/sequence.rs`:83-118, `claudine/lib/src/composition/sequence.rs`:214-227).

The lint does not fail because these collapses are explicitly allowlisted as "not-yet-converted exceptions" (`scripts/check-error-transport.allow`:14-33). That may be an acceptable migration note, but it is not the production contract described by the feature. A programmatic handler still cannot inspect the original source type for these cases; it only gets the prose projection.

Verification level present: Level 1 grep guard, but it proves only "no un-allowlisted collapses," not "the design-listed boundary collapses are gone." The allowlist also means new reviewers can see a green `lint-transport` while required conversion work remains.

Required verification level: Level 1 is sufficient. Convert these variants to typed enum arms with concrete `#[source]` fields or honest typed sub-variants for heterogeneous cases, then tighten the guard so these exact lines are no longer allowed. Add representative tests proving the typed source survives for Markdown read/load failures and external sequence reference/load failures.

## Notes

The previous review's blockers appear addressed: lifecycle aliases now mirror `category`/`code` for classifiable errors, `err.severity` is projected, and public lifecycle docs use faceted fields outside the deprecated-alias section.

The flagship invalid-file-reference rendering path has appropriate Level 2 coverage in both CLIs: `darkmatter/cli/tests/level2_errors.rs` and `claudine/cli/tests/level2_invalid_file_reference_capture.rs` assert the root-cause headline, focused excerpt, OSC8 link, and did-you-mean suggestions through real tmux capture. No Level 3 coverage is required for this feature because it does not depend on OS keyboard encoding.

Checks run:

- `env -u CDPATH scripts/check-error-transport.sh` passed, with the caveat above about allowlisted exceptions.
- `env -u CDPATH scripts/check-lifecycle-doc-facets.sh` passed.

I did not run the full unit, lint, or Level 2 suites in this review pass.

Production ready: **no**.
