# Darkmatter Pipeline Technical Review

Date: 2026-02-14
Reviewer: Codex (GPT-5)

## Scope

Reviewed:

- `darkmatter/lib` transform pipeline (replacement, interpolation, transclusion)
- `darkmatter/cli` `compose` command behavior and tests
- Functional docs used as requirements baseline:
  - `darkmatter/docs/darkmatter-pipeline.md`
  - `darkmatter/docs/transclusion/block-transclusion.md`
  - `darkmatter/docs/transclusion/fm-transclusion.md`
  - `darkmatter/docs/transclusion/code-transclusion.md`
  - `darkmatter/docs/text-replacement.md`
  - `darkmatter/design/interpolation.md`

Validation run:

- `cargo test -p darkmatter -p darkmatter-cli` (all passing)

## Findings (Ordered by Severity)

### 1. [P0] `replace=true` does not invert parent/child precedence as documented

- Requirement says `replace=true` should give parent replace map precedence over child (`darkmatter/docs/transclusion/block-transclusion.md:85`).
- Current implementation applies parent replace map only after child pipeline is finished (`darkmatter/lib/src/markdown/transform/mod.rs:636`).
- Child still computes its own replacement stage with child-wins merge semantics, so precedence is not actually inverted.

Observed repro:

```bash
tmpdir=$(mktemp -d)
cat > "$tmpdir/root.md" <<'MD'
---
replace:
  TOKEN: parent
---
::file ./child.md replace=true
MD
cat > "$tmpdir/child.md" <<'MD'
---
replace:
  TOKEN: child
---
TOKEN
MD
cargo run -q -p darkmatter-cli -- compose "$tmpdir/root.md"
# Output: child
```

Recommendation:

- Apply `ParentWins` at state construction time for the direct child (before child Stage 1 replacement/interpolation), not as a post-transform text pass.

### 2. [P0] One-off `replace={...}` values incorrectly propagate to grandchildren

- Requirement says one-off replace values are non-inheriting (`darkmatter/docs/transclusion/block-transclusion.md:91`).
- Current `build_child_external_state` merges one-off values into child inherited state (`darkmatter/lib/src/markdown/transform/mod.rs:769`), which then becomes normal inherited state for descendants.

Observed repro:

```bash
tmpdir=$(mktemp -d)
cat > "$tmpdir/root.md" <<'MD'
---
replace:
  A: root
---
::file ./child.md replace={"ONE":"oneoff"}
MD
cat > "$tmpdir/child.md" <<'MD'
Child: ONE A
::file ./grand.md
MD
cat > "$tmpdir/grand.md" <<'MD'
Grand: ONE A
MD
cargo run -q -p darkmatter-cli -- compose "$tmpdir/root.md"
# Output includes: Grand: oneoff root
```

Recommendation:

- Keep one-off replace map out of propagated state; apply it only for the immediate transclusion node.

### 3. [P1] Frontmatter transclusion misclassifies bare filenames as inline text

- Requirements say frontmatter string values should be classified as local file reference / URL / inline string / invalid (`darkmatter/docs/transclusion/fm-transclusion.md:9`).
- `render_frontmatter_reference` treats values as inline unless `is_file_like_reference` returns true (`darkmatter/lib/src/markdown/transform/mod.rs:543`, `darkmatter/lib/src/markdown/transform/transclusion/resolver.rs:207`).
- `intro.md` is considered inline text because it has no slash/prefix.

Observed repro:

```bash
tmpdir=$(mktemp -d)
cat > "$tmpdir/root.md" <<'MD'
---
prologue: intro.md
---
Body
MD
cat > "$tmpdir/intro.md" <<'MD'
Intro text
MD
cargo run -q -p darkmatter-cli -- compose "$tmpdir/root.md"
# Output begins with literal 'intro.md' instead of file contents
```

Recommendation:

- For frontmatter refs, attempt file resolution first (relative to source) before classifying as inline string.

### 4. [P1] Code transclusion fallback language is never used for unknown extensions

- Requirement says unknown language should fallback to `txt` (`darkmatter/docs/transclusion/code-transclusion.md:17`).
- `infer_language()` returns extension in both branches (`darkmatter/lib/src/markdown/transform/transclusion/code.rs:17`).

Observed repro:

```bash
tmpdir=$(mktemp -d)
cat > "$tmpdir/root.md" <<'MD'
::code ./sample.weird
MD
cat > "$tmpdir/sample.weird" <<'MD'
hello
MD
cargo run -q -p darkmatter-cli -- compose "$tmpdir/root.md"
# Fence is ```weird, not ```txt
```

Recommendation:

- Return `fallback` when `SYNTAX_SET.find_syntax_by_extension(ext)` is `None`.
- Add test for unknown extension fallback.

### 5. [P1] `fail_fast` is documented but not enforced for interpolation failures

- `TransformOptions` says fail-fast returns an error on first failure (`darkmatter/lib/src/markdown/transform/types.rs:57`).
- Interpolation stage only logs warnings and continues on parse/eval errors (`darkmatter/lib/src/markdown/transform/mod.rs:288`), including TODO acknowledging this.

Recommendation:

- Convert interpolation stage to return `MarkdownResult<usize>` and propagate parse/eval errors when `fail_fast = true`.
- Record non-fatal interpolation warnings in `TransformReport` when `fail_fast = false`.

### 6. [P2] Replacement scanner is O(n^2) and allocates per cursor step

- In `scan_and_replace`, each loop rebuilds a prefix string to compute remaining slice (`darkmatter/lib/src/markdown/transform/replacement.rs:167`).
- This is allocation-heavy and quadratic for long documents.

Recommendation:

- Track byte offset directly (precompute char->byte mapping or iterate with `char_indices`) and slice by byte index without per-iteration allocations.
- Add a perf regression benchmark for large docs + large replace maps.

### 7. [P2] `compose --state` accepts non-object JSON silently

- CLI says `--state` is key/value pairs (`darkmatter/cli/src/lib.rs:209`).
- Non-object JSON is accepted (`darkmatter/cli/src/main.rs:240`) but discarded by state builder (`darkmatter/lib/src/markdown/transform/state.rs:255`).

Recommendation:

- Validate `--state` is a JSON object and return a user-facing error otherwise.

### 8. [P3] CLI docs are out of sync with implemented flags

- README still documents removed top-level flags like `--clean`, `--clean-save`, `--fm-merge-with`, `--fm-defaults` (`darkmatter/cli/README.md:79`).
- Integration tests correctly verify these removed flags fail.

Recommendation:

- Update README to match current `read/clean/compose/toc/delta` subcommand model.

## Test Coverage Gaps

1. Add Stage 2 tests for `replace=true` precedence inversion and one-off non-inheritance.
2. Add tests for frontmatter transclusion of bare file names (`prologue: intro.md`).
3. Add tests for unknown code extension fallback to `code_fallback_language`.
4. Add fail-fast interpolation tests asserting hard error behavior.
5. Add recursion depth-limit tests (`MaxDepthExceeded`) and runtime stack integrity checks.
6. Extend CLI compose tests beyond stdin happy path:
   - file-based compose with relative transclusion
   - non-object `--state` validation
   - compose `--show` behavior across markdown/html/json outputs

## Additional Performance/Ergonomics Opportunities

1. Expose key transclusion controls on CLI compose (`--max-depth`, `--ignore-invalid`, optional `--fail-fast`) for safer production workflows.
2. Avoid cloning full content in interpolation stage unless at least one replacement succeeds.
3. Consider transclusion caching for repeated includes in large document graphs.
