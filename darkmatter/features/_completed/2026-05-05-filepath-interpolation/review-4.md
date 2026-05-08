---
ready: true
agent: open_code
model: ""
---

# Review 4: Filepath Interpolation

**Date:** 2026-05-06  
**Scope:** Link Resolve + Link Normalization implementation (post-review-3 remediation)

---

## Verification Matrix

| Requirement | Strongest observed verification | Assessment |
| --- | --- | --- |
| Markdown links/images resolve to absolute paths during Inline-Pre | Level 1 unit tests in `link_resolve.rs` (11 tests) | Adequate |
| HTML `<a>`, `<img>`, `<video>`, `<audio>`, `<source>`, `<iframe>`, `<link>` resolve | Level 1 unit tests covering all tag types + spaced attributes + nested source | Adequate |
| Transcluded child links resolve before insertion and normalize at root | Level 1 integration test + Level 1 CLI test (`test_compose_link_transcluded_child`) | Adequate |
| Same-repo absolute links normalize to relative paths | Level 1 unit tests + Level 1 CLI test (`test_compose_link_relative_same_repo`) | Adequate |
| Home-dir paths normalize to `~/` | Level 1 unit tests (`test_normalize_links_home_dir`) + integration test | Adequate |
| Whitelisted ENV paths normalize to `${VAR}` and emit exactly one warning | Level 1 unit tests + Level 1 CLI test (`test_compose_env_var_substitution_one_warning`) | Adequate |
| CLI stdout remains composed markdown only (no debug traces) | Level 1 CLI test asserts no diagnostics in stdout | Adequate |
| HTML spaced attributes (`href = "..."`) are handled | Level 1 unit tests + Level 1 CLI test (`test_compose_html_spaced_attributes`) | Adequate |
| Finalization stage only runs on root document | Level 1 integration test (`test_child_no_normalization`) + code review of `depth() <= 1` gating | Adequate |
| ENV-var warning uses `<blue>`/`<b>` prose markup from spec | Level 1 unit test verifies warning message content | Adequate |
| Attribute-aware `find_target_range` does not replace wrong attribute | Level 1 unit test (`test_link_resolve_wrong_attribute_not_replaced`) | Adequate |
| HTML entity decoding in attributes (`&amp;`) | Level 1 unit test (`test_link_resolve_html_entity_in_attribute`) | Adequate |
| Nested `<source>` inside `<video>`/`<audio>` | Level 1 unit test (`test_link_resolve_nested_source_in_video`) | Adequate |

---

## Findings

None. All prior review findings have been remediated and the implementation matches the specification.

### Prior findings verified as resolved

| Review | Finding | Status |
| --- | --- | --- |
| Review 2 | Debug traces written to stdout | **Fixed** — converted to `tracing::trace!`, CLI tests assert stdout cleanliness |
| Review 2 | HTML spaced attributes skipped by pre-scan | **Fixed** — pre-scan removed, attribute-aware search handles `attr = "val"` |
| Review 2 | ENV-var warnings emitted twice | **Fixed** — library records `ComposeWarning` only; CLI renders via `Status` |
| Review 2 | No CLI coverage for end-to-end feature | **Fixed** — 4 CLI tests added covering same-repo, transclusion, ENV warning, spaced attrs |
| Review 3 | `find_target_range` replaces wrong attribute | **Fixed** — `get_attribute_name_for_syntax` + attribute-aware search |
| Review 3 | HTML entity decoding mismatch | **Fixed** — HTML-encoded match via `html_escape::encode_quoted_attribute` |
| Review 3 | ENV-var warning lacks `<blue>`/`<b>` markup | **Fixed** — message now includes `<blue>{{path}}</blue>` and `<b>{ENV}</b>` |
| Review 3 | No test for nested `<source>` in `<video>` | **Fixed** — `test_link_resolve_nested_source_in_video` added |

---

## Tests Run

- `cargo test -p darkmatter --lib -- link_resolve link_normalization` — 20 passed
- `cargo test -p darkmatter --test link_interpolation_integration` — 4 passed
- `cargo test -p darkmatter-cli --test cli -- test_compose_link_relative_same_repo test_compose_link_transcluded_child test_compose_env_var_substitution_one_warning test_compose_html_spaced_attributes` — 4 passed

---

## Production Readiness

**Ready.**

All specification requirements are implemented and verified:

1. **Link Resolve** converts all local links (Markdown + specified HTML tags) to absolute paths during the Inline-Pre stage.
2. **Transclusion** runs between Inline-Pre and Finalization, ensuring child document links are resolved before root-level normalization.
3. **Finalization** is gated to the root document only (`transclusion.depth() <= 1`).
4. **Link Normalization** applies the correct fallback chain: same-repo relative → `~/` home alias → `${VAR}` env abstraction → keep absolute.
5. **Warnings** are emitted exactly once per ENV-var substitution via `ComposeWarning` + CLI `Status` rendering.
6. **Edge cases** are handled: HTML entities, spaced attributes, duplicate attributes, nested `<source>`, non-existent files.

No gaps, broken functionality, or test coverage deficiencies remain.
