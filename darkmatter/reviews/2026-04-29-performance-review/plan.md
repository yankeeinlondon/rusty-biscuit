---
created: "2026-04-30"
source_review: "review.md"
phases: 6
start_phase: 1
scope:
  - biscuit-terminal/lib
  - tree-hugger/lib
  - darkmatter/lib
  - biscuit-file/lib
source_files_during_phase_1:
  - biscuit-terminal/lib/src/utils/block_constraint.rs
  - biscuit-terminal/lib/src/components/filesystem.rs
  - biscuit-terminal/lib/benches/rendering.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - tree-hugger/lib/src/analysis/mod.rs
  - biscuit-terminal/lib/src/components/filesystem.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/highlighting/grammars.rs
  - darkmatter/lib/src/markdown/highlighting/mod.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
  - biscuit-terminal
  - tree-hugger
  - darkmatter
---

# Performance Review Implementation Plan

## Goal

Implement every recommendation from `review.md` with low behavioral risk, focused regression tests, and targeted benchmarks where the review identifies scaling or allocation risks.

The work spans four workspace packages:

- `biscuit-terminal/lib`: text width/layout and filesystem tree rendering
- `tree-hugger/lib`: symbol ownership lookup during bind analysis
- `darkmatter/lib`: syntax set ownership and markdown terminal renderer allocation behavior
- `biscuit-file/lib`: PDF text normalization

## Constraints And Baseline Checks

- Use workspace metadata, not directory assumptions, when package names are ambiguous.
- Root `just` does not cover every package involved, so use direct package tests where needed.
- Keep public behavior stable unless a performance fix exposes an existing bug.
- Add or update tests before/alongside each optimization so equivalence is checked against current behavior.
- Avoid broad renderer rewrites until the smaller hot-path wins are landed and measurable.

Initial orientation commands:

```bash
cargo metadata --no-deps --format-version 1
cargo test -p biscuit-terminal --lib
cargo test -p tree-hugger --lib
cargo test -p darkmatter --lib
cargo test -p biscuit-file --lib
```

## Phase 1: Low-Risk Hot Path Fixes

### 1.1 Remove Regex Recompilation In `BlockContent::content_length`

Review finding: Finding 1, high severity.

Files:

- `biscuit-terminal/lib/src/utils/block_constraint.rs`
- `biscuit-terminal/lib/benches/rendering.rs`

Implementation:

1. Replace per-line `Regex::new(...)` calls in `BlockContent::content_length` with the existing escape-aware width path.
2. Prefer `visible_width(&line)` over adding new `LazyLock<Regex>` statics because `visible_width` already handles CSI, OSC, APC/Kitty escape sequences, Unicode width, and is used throughout layout code.
3. If any regression appears because callers expect byte length rather than visible width, use static `std::sync::LazyLock<regex::Regex>` as the fallback implementation while preserving the existing return type.

Tests:

- Add unit coverage for `BlockContent::content_length` with:
    - plain ASCII
    - CSI color escapes
    - OSC title escapes
    - OSC terminated by ST (`ESC \`) if supported by the lower-level parser
    - Unicode wide characters
- Ensure existing `visible_width_*` tests remain green.

Benchmark:

- Extend or update `bench_strip_escape_codes` in `biscuit-terminal/lib/benches/rendering.rs` to cover multi-line `BlockContent::content_length`.

Validation:

```bash
cargo test -p biscuit-terminal block_constraint --lib
cargo bench -p biscuit-terminal --bench rendering -- strip_escape
```

Acceptance:

- No regex compilation occurs inside `content_length`.
- ANSI/OSC content measures identically or more correctly than before.
- Benchmark shows lower CPU time or unchanged output with no performance regression.

### 1.2 Cache FileSystem Sort Keys

Review finding: Finding 3, medium severity.

Files:

- `biscuit-terminal/lib/src/components/filesystem.rs`

Implementation:

1. Introduce a small local sortable entry type inside `build_tree_recursive`, such as:
   - `is_dir: bool`
   - `sort_name: String`
   - `entry: std::fs::DirEntry`
2. Compute `file_type`, directory classification, and lowercase/case-folded filename once during collection.
3. Sort by `(!is_dir, sort_name)` semantics so directories still come first and alphabetical ordering remains case-insensitive.
4. Preserve current error behavior: entries whose file type cannot be read are skipped or sorted as non-directories only if that matches current behavior.

Tests:

- Add a filesystem unit test that creates mixed-case files and directories and asserts:
    - directories before files
    - case-insensitive alphabetical order
    - stable behavior for existing dotfile/filter settings

Benchmark:

- Add a benchmark case for rendering a temp directory containing hundreds or thousands of entries with mixed-case names.

Validation:

```bash
cargo test -p biscuit-terminal filesystem --lib
cargo bench -p biscuit-terminal --bench rendering -- filesystem
```

Acceptance:

- Lowercase allocation happens once per entry, not once per comparison.
- Rendered ordering is unchanged for representative trees.

## Phase 2: Data-Structure And Caching Fixes

### 2.1 Replace Linear Owner Lookup In `tree-hugger`

Review finding: Finding 2, medium severity.

Files:

- `tree-hugger/lib/src/analysis/mod.rs`
- `tree-hugger/lib/src/analysis/tests.rs`

Implementation:

1. Build an owner index once at the start of `bind_pass` from `index.symbols`.
2. Store symbol spans sorted by `start_byte`, with the original symbol index and span width.
3. Implement lookup as:
   - binary search to the last symbol whose `start_byte <= reference.start_byte`
   - scan backward through candidates whose start can contain the reference
   - choose the smallest containing span to preserve current `min_by_key(width)` behavior
4. Keep a straightforward helper API, for example `OwnerSymbolIndex::new(symbols)` and `OwnerSymbolIndex::find(start_byte, end_byte)`.
5. Do not add a new interval-tree crate unless profiling proves the bounded backward scan is insufficient; the sorted-span index should remove the current full scan in normal nested-code layouts.

Tests:

- Unit tests for:
    - no containing symbol
    - single containing symbol
    - nested symbols where the smallest containing span wins
    - siblings with adjacent spans
    - identical start bytes with different end bytes
- Existing bind pass tests must still pass.

Benchmark:

- Add a `tree-hugger` dev benchmark only if benchmark infrastructure is already acceptable for the area; otherwise add a deterministic ignored stress test or local profiling fixture.
- Synthetic benchmark shape: thousands of symbols plus thousands of references in one file-like span set.

Validation:

```bash
cargo test -p tree-hugger --lib
cargo test -p tree-hugger owner --lib
```

Acceptance:

- `bind_pass` performs one owner-index build per file.
- Reference owner lookup no longer scans every symbol for every reference.
- Existing relation output is unchanged.

### 2.2 Cache UID/GID Resolution In FileSystem Metrics

Review finding: Finding 4, medium severity.

Files:

- `biscuit-terminal/lib/src/components/filesystem.rs`

Implementation:

1. Add an internal metrics context for a tree build, for example:
   - `struct MetricContext { uid_cache: HashMap<u32, Option<String>>, gid_cache: HashMap<u32, Option<String>> }`
2. Create the context in `ensure_tree_built` and pass `&mut MetricContext` through `build_tree_recursive` and `collect_file_metrics`.
3. Cache negative lookups as `None` so missing UID/GID values are not repeatedly queried.
4. Keep the cache scoped to a tree build. This avoids stale long-lived user/group data and keeps `FileSystem` cloning simple.
5. On non-Unix builds, keep the context empty or compile it behind `#[cfg(unix)]`.

Tests:

- Unit-test cache behavior through a small helper where possible.
- If direct libc call counting is awkward, isolate the cache lookup logic into a pure helper and test repeated IDs return the cached value.
- Keep existing metric display tests green.

Validation:

```bash
cargo test -p biscuit-terminal filesystem --lib
```

Acceptance:

- One UID lookup per unique UID per tree build.
- One GID lookup per unique GID per tree build.
- Public `FileSystem` API remains unchanged.

## Phase 3: Syntax Highlighting Memory Fix

### 3.1 Borrow The Global `SyntaxSet`

Review finding: Finding 5, medium severity.

Files:

- `darkmatter/lib/src/markdown/highlighting/grammars.rs`
- `darkmatter/lib/src/markdown/highlighting/mod.rs`
- call sites in `darkmatter/lib/src/markdown/output/*` if required by type changes

Implementation:

1. Change `grammars::load_syntax_set()` to return `&'static SyntaxSet`.
2. Change `CodeHighlighter.syntax_set` from `SyntaxSet` to `&'static SyntaxSet`.
3. Keep `CodeHighlighter` itself lifetime-free by storing only a `'static` reference.
4. Leave `theme` owned unless a separate review identifies theme cloning/loading as a hot path.
5. Update tests that bind `let syntax_set = load_syntax_set();`; method calls should continue to work on the reference.

Tests:

- Existing highlighting tests should cover API behavior.
- Add a small pointer-stability test only if useful, checking two calls return the same static address.

Validation:

```bash
cargo test -p darkmatter highlighting --lib
cargo test -p darkmatter output::terminal --lib
```

Acceptance:

- No `SyntaxSet::clone()` occurs in `CodeHighlighter::new`.
- Public `CodeHighlighter::syntax_set(&self) -> &SyntaxSet` remains compatible.

## Phase 4: Allocation Reductions In Markdown Terminal Rendering

### 4.1 Convert Prose Emission To Append Into Existing Buffers

Review finding: Finding 6, medium severity.

Files:

- `darkmatter/lib/src/markdown/output/terminal.rs`

Implementation:

1. Add an append-style helper:
   - `fn push_prose_text(out: &mut String, text: &str, style: Style, emit_italic: bool, in_strikethrough: bool, in_mark: bool, blockquote_bg: Option<Color>)`
2. Reimplement `emit_prose_text(...) -> String` as a thin test/backward-compatible wrapper around `push_prose_text`, or replace all internal call sites and keep the old helper only for tests if needed.
3. Update `LineWrapper::{emit_word, emit_styled_marker, emit_styled_hyperlink}` and other local call sites to append directly into `self.output`.
4. Replace `format!` calls inside the hot prose path with `write!(&mut String, ...)` where it avoids temporary strings. Import `std::fmt::Write` under an alias if needed to avoid conflict with `std::io::Write`.

Tests:

- Keep all existing `emit_prose_text_*` tests.
- Add at least one test that compares old wrapper output against append output for mark, blockquote background, and combined font styles.

Validation:

```bash
cargo test -p darkmatter emit_prose_text --lib
cargo test -p darkmatter output::terminal --lib
```

Acceptance:

- Hot `LineWrapper` word emission does not allocate a temporary `String` for each word.
- ANSI output remains byte-identical for covered style combinations.

### 4.2 Make `LineWrapper` Flush At Safe Block Boundaries

Review finding: Finding 6, medium severity.

Files:

- `darkmatter/lib/src/markdown/output/terminal.rs`

Implementation:

1. First identify the current flush points. The renderer already writes and resets `LineWrapper` around some block-level outputs such as tables/images.
2. Add an explicit `take_output()` or `flush_into(&mut impl std::io::Write)` method on `LineWrapper`.
3. Flush only at semantically safe boundaries where wrapping state cannot affect future text:
   - after closing paragraphs
   - after headings
   - after lists when the list stack is empty
   - before and after block-level raw outputs
4. Do not flush in the middle of a paragraph, active link, active emphasis span, active table cell, active code block, or active blockquote line.
5. Preserve `current_col`, blockquote state, and prefix behavior across any boundary that still needs state; reset only when the renderer already starts a new independent block.
6. Keep final write path as a fallback for any remaining buffered output.

Tests:

- Add byte-output tests for:
    - two paragraphs with wrapping
    - paragraph followed by table/code/image placeholder path
    - nested blockquote
    - list followed by paragraph
    - long link fallback and OSC8 path if existing tests expose both modes
- Existing terminal snapshot/unit tests must remain green.

Benchmark:

- Add or update a large markdown rendering benchmark after 4.1 so allocation and peak-buffer changes can be measured together.

Validation:

```bash
cargo test -p darkmatter output::terminal --lib
```

Acceptance:

- Large documents do not require buffering the entire rendered document in one `LineWrapper` string.
- Existing rendering output stays byte-identical, except where tests document an intentional fix.

## Phase 5: Single-Pass PDF Normalization

### 5.1 Combine Dehyphenation And Whitespace Collapse

Review finding: Finding 7, low severity.

Files:

- `biscuit-file/lib/src/pdf/backends.rs`

Implementation:

1. Replace the current two-pass `normalize_text` implementation with one state machine over `text.chars().peekable()`.
2. Handle these states in one output buffer:
   - pending whitespace between words
   - hyphen followed by `\n`
   - hyphen followed by `\r\n`
   - spaces/tabs after a dehyphenated line break
   - ordinary whitespace collapse
3. Avoid final `trim().to_string()` by never emitting leading whitespace and by removing one trailing pending space before returning, or by keeping whitespace pending until the next non-whitespace character.
4. Preserve all current public semantics from existing tests.

Tests:

- Existing `normalize_text_*` tests should remain green.
- Add cases for:
    - `word-\r\n   continued`
    - multiple dehyphenations in one input
    - trailing whitespace after a dehyphenated segment
    - whitespace-only input

Benchmark:

- If `biscuit-file` already has accepted benchmark conventions, follow `biscuit-file/docs/testing/benchmark-tests.md` and use `divan` for a large synthetic PDF text normalization benchmark.
- If benchmark setup is out of scope for this change, document a local before/after timing command in the PR notes.

Validation:

```bash
cargo test -p biscuit-file pdf::backends --lib
cargo test -p biscuit-file --lib
```

Acceptance:

- `normalize_text` allocates one output buffer and performs one pass over input.
- Existing normalization behavior is preserved.

## Phase 6: Cross-Package Verification

Run package-specific tests first, then broader workspace checks:

```bash
cargo test -p biscuit-terminal --lib
cargo test -p tree-hugger --lib
cargo test -p darkmatter --lib
cargo test -p biscuit-file --lib
cargo test --workspace --lib
```

Run formatting and linting:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

Run benchmarks/profiling where changed:

```bash
cargo bench -p biscuit-terminal --bench rendering
```

For `darkmatter` renderer profiling, create a generated large markdown fixture outside source control or under a temp directory, then compare render time and allocation profile before and after Phase 4. If adding a committed benchmark, add dev dependency updates and update dependency docs as required by repo drift rules.

## Implementation Order

1. Phase 1.1: `BlockContent::content_length`
2. Phase 1.2: FileSystem sort-key caching
3. Phase 3.1: static `SyntaxSet` borrowing
4. Phase 2.1: `tree-hugger` owner index
5. Phase 2.2: UID/GID cache
6. Phase 5.1: single-pass PDF normalization
7. Phase 4.1: append-style prose emission
8. Phase 4.2: safe-boundary `LineWrapper` flushing

This order lands low-risk, high-impact fixes first and leaves the renderer flushing change until after the byte-level rendering tests are strengthened.

## Risk Register

| Risk | Area | Mitigation |
| --- | --- | --- |
| `content_length` callers expected byte length, not terminal width | `biscuit-terminal` | Add focused tests; fallback to cached regex if needed |
| Owner index returns different owner for equal-width or identical spans | `tree-hugger` | Preserve current tie behavior by sorting with original index and testing identical starts |
| UID/GID cache complicates `FileSystem` cloning | `biscuit-terminal` | Keep cache scoped to `ensure_tree_built`, not stored on `FileSystem` |
| Static syntax-set reference ripples into public API | `darkmatter` | Keep `CodeHighlighter` lifetime-free with `&'static SyntaxSet`; retain existing `syntax_set()` signature |
| Renderer flushing changes byte output | `darkmatter` | Add byte-output tests before flushing; flush only at block boundaries |
| Single-pass PDF normalization mishandles pending whitespace | `biscuit-file` | Drive implementation from existing tests plus new CRLF/trailing-whitespace cases |

## Done Criteria

- All seven findings in `review.md` are addressed in code.
- Tests cover each changed behavior or preserve current public behavior.
- At least the existing `biscuit-terminal` rendering benchmark suite runs after the hot-path changes.
- No public API break is introduced except internal helper signatures.
- Dependency docs are updated if any benchmark or helper dependency is added.
