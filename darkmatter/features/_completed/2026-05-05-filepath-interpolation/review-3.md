---
ready: true
agent: open_code
model: ""
---

# Review 3: Filepath Interpolation

**Date:** 2026-05-06  
**Scope:** Link Resolve + Link Normalization implementation

---

## Verification Matrix

| Requirement | Strongest observed verification | Assessment |
| --- | --- | --- |
| Markdown links/images resolve to absolute paths during Inline-Pre | Level 1 unit tests in `link_resolve.rs` | Adequate |
| HTML `<a>`, `<img>`, `<video>`, `<audio>`, `<source>`, `<iframe>`, `<link>`, `<script>` resolve | Level 1 unit tests covering all tag types + spaced attributes | Adequate |
| Transcluded child links resolve before insertion and normalize at root | Level 1 integration test + Level 1 CLI test (`test_compose_link_transcluded_child`) | Adequate |
| Same-repo absolute links normalize to relative paths | Level 1 unit tests + Level 1 CLI test (`test_compose_link_relative_same_repo`) | Adequate |
| Home-dir paths normalize to `~/` | Level 1 unit tests (`test_normalize_links_home_dir`, integration) | Adequate |
| Whitelisted ENV paths normalize to `${VAR}` and emit exactly one warning | Level 1 unit tests + Level 1 CLI test (`test_compose_env_var_substitution_one_warning`) | Adequate |
| CLI stdout remains composed markdown only (no debug traces) | Level 1 CLI test asserts no diagnostics in stdout | Adequate |
| HTML spaced attributes (`href = "..."`) are handled | Level 1 unit tests + Level 1 CLI test (`test_compose_html_spaced_attributes`) | Adequate |

---

## Findings

### Low: `find_target_range` can replace wrong attribute when non-target attribute comes first

`find_target_range` searches for the raw target within the record's span and returns the **first** occurrence whose preceding character is `(`, `=`, `"`, `'`, or `<`. If a non-target attribute contains the same path and appears before the target attribute, the wrong attribute is modified.

**Example:** `<img alt="logo.png" src="logo.png">` — the `alt` would be replaced instead of `src`.

**Impact:** Edge case; requires the same path string to appear in multiple attributes with the non-target first. Link Normalization is unaffected because absolute paths are unlikely to collide with attribute values.

**Suggested fix:** Store the attribute name in `ReferenceRecord` and use it to find the correct occurrence, or scan for `attr_name="`/`attr_name='` before the target string.

**Test-rigor classification:** Level 1 unit test would be sufficient to verify the fix.

### Low: HTML entity decoding mismatch in `find_target_range`

`extract_attribute` decodes HTML entities (e.g., `&amp;` → `&`), but `find_target_range` searches for the **decoded** string in the original HTML. A link like `<a href="foo&amp;bar.md">` extracts `foo&bar.md`, which is then not found in the raw HTML.

**Impact:** Rare; file paths containing `&`, `<`, `>`, or `"` are uncommon.

**Suggested fix:** Either skip HTML entity decoding in `extract_attribute` for path attributes, or HTML-encode the target before searching in `find_target_range`.

**Test-rigor classification:** Level 1 unit test would be sufficient.

### Low: ENV-var warning lacks `<blue>`/`<b>` prose markup from spec

The spec requests the warning message:  
> "the path `<blue>{{absolute-filepath}}</blue>` was found to be an offset of the `<b>{ENV}</b>` environment variable and will use this abstraction."

The current warning is plain text. While the CLI renders it through `Status::from_prose`, the specific `<blue>` and `<b>` markup is absent.

**Impact:** Cosmetic; the warning is still visible and correctly classified as `Warning` state.

**Suggested fix:** Add `<blue>` and `<b>` markup to the warning message string before pushing it to `report.warnings`.

**Test-rigor classification:** Level 2 terminal capture would verify the rendered styling, but this is not a behavioral requirement.

### Low: No test for nested `<source>` inside `<video>`/`<audio>`

Standalone `<source>` tags are tested, but a nested structure like `<video><source src="./movie.mp4"></video>` is not explicitly verified. The MDAST parser should surface the `<source>` as a separate `Html` node, so the implementation likely handles this correctly, but it is not proven by tests.

**Suggested fix:** Add a unit test with nested `<video><source src="..."></video>`.

**Test-rigor classification:** Level 1 unit test is sufficient.

---

## Tests Run

- `cargo test -p darkmatter --lib -- link_resolve link_normalization` — 17 passed
- `cargo test -p darkmatter --test link_interpolation_integration` — 4 passed
- `cargo test -p darkmatter-cli --test cli -- test_compose_link_relative_same_repo test_compose_link_transcluded_child test_compose_env_var_substitution_one_warning test_compose_html_spaced_attributes` — 4 passed

---

## Production Readiness

**Ready.**

All review-2 findings have been addressed:
- Debug traces converted to `tracing::trace!` (stdout is clean)
- Spaced HTML attributes are fully supported
- ENV-var warnings are emitted exactly once (library-side `ComposeWarning` + CLI-side `Status` rendering)
- Comprehensive CLI-level test coverage exists

The remaining findings are edge-case bugs in `find_target_range` that are unlikely to affect typical usage, and a cosmetic markup gap in the ENV-var warning. None of these block production deployment.
