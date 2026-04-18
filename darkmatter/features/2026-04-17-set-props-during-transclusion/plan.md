---
phases: 4
created: 2026-04-18
start_phase: 1
source_files_during_phase_1:
    - darkmatter/lib/src/markdown/compose/transclusion/types.rs
    - darkmatter/lib/src/markdown/compose/transclusion/parser.rs
    - darkmatter/lib/src/markdown/compose/parse_utils.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/compose/state.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/cache/hashing.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages_during_phase_2:
    - darkmatter
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/compose/types.rs
    - darkmatter/lib/src/markdown/compose/transclusion/parser.rs
    - darkmatter/lib/src/markdown/compose/transclusion/types.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/cli/src/args.rs
    - darkmatter/cli/src/commands.rs
    - darkmatter/cli/src/output.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
    - darkmatter/lib/src/markdown/compose/transclusion/parser.rs
    - darkmatter/lib/src/markdown/compose/transclusion/types.rs
    - darkmatter/lib/src/markdown/compose/state.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/cli/tests/cli.rs
docs_updated_during_phase_4:
    - darkmatter/docs/transclusion/block-transclusion.md
    - darkmatter/docs/darkmatter-compose-pipeline.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/SKILL.md
packages:
    - darkmatter
    - darkmatter-cli
---

# Execution Plan: Set Frontmatter Properties During Transclusion

**Source documents**

- [spec.md](./spec.md)

## Objective

Let a `::file` transclusion directive override the child document's effective frontmatter via two syntactic forms — `set='{…json5…}'` (object form) and `set.NAME=<json5>` (property form) — with three-layer precedence (child fm → `set=` object → `set.NAME` property), deep-merge semantics for dict values, null-as-literal semantics, duplicate-detection errors, and paired permissive flags on both the library (`ComposeOptions`) and CLI.

## Confidence

High. Grammar additions are isolated to the transclusion directive parser; merge application has a natural integration point in `render_markdown_transclusion` before the child's pipeline runs; the existing `deep_merge` needs a parallel null-override variant rather than a rewrite; warnings reuse existing biscuit-terminal presentation primitives; and CLI plumbing mirrors the established `--allow-shell-timeout` pattern.

## Delivery Constraints

- Do not change existing CLI `--set` semantics or parent-to-child `set_overrides` propagation.
- Keep the existing `deep_merge` behavior (null = "not set") intact for other callers; add a sibling helper for the set-override case.
- Default behavior must be strict: duplicate/invalid cases are errors unless the matching permissive flag is set.
- Warnings (permissive mode) go to stderr via biscuit-terminal's Status + BlockQuote presentation; they do NOT appear in composed output.
- Directive-level set overrides MUST NOT propagate to grandchildren — they apply only to the direct child named in that `::file` directive.
- Cache correctness: compose cache key must incorporate the effective set overlay so two directives with different `set=`/`set.*` do not alias.

## Affected Files (high level)

| File | Change Type |
| --- | --- |
| `darkmatter/lib/src/markdown/compose/transclusion/types.rs` | Extend `BlockOptions`; add error variants |
| `darkmatter/lib/src/markdown/compose/transclusion/parser.rs` | Parse `set=` and `set.NAME=`; emit duplicate/invalid errors |
| `darkmatter/lib/src/markdown/compose/parse_utils.rs` | Add helper to consume dotted sub-identifier after `set` |
| `darkmatter/lib/src/markdown/compose/state.rs` | Add null-aware deep-merge helper + three-layer applier |
| `darkmatter/lib/src/markdown/compose/mod.rs` | Apply overlay before child pipeline; integrate cache key |
| `darkmatter/lib/src/markdown/compose/cache/hashing.rs` | Fold overlay into compose cache key |
| `darkmatter/lib/src/markdown/compose/types.rs` | Add two permissive `ComposeOptions` builders |
| `darkmatter/cli/src/args.rs` | Add two permissive flags to `Compose` subcommand |
| `darkmatter/cli/src/commands.rs` | Wire CLI flags → `ComposeOptions` |
| `darkmatter/cli/src/output.rs` | Warning presentation (Status + BlockQuote) if not already present |
| `darkmatter/cli/tests/cli.rs` | Integration tests covering grammar, flags, and merge outcomes |
| `darkmatter/docs/transclusion/block-transclusion.md` | Document the `set` syntax, precedence, null semantics, flags |
| `darkmatter/docs/darkmatter-compose-pipeline.md` | Cross-reference parent-side interpolation passthrough |
| `.claude/skills/darkmatter/SKILL.md` | Note `::file … set=`/`set.NAME=` feature |

## Phase 1: Grammar and Parser

**Goal:** Parse `set='{…}'` and `set.NAME=<json5>` off the `::file` directive line and carry them in `BlockOptions`, with duplicate and JSON5-validation errors raised at parse time by default.

**Depends on:** none

**Parallelizable work:** Steps 1.1 and 1.2 can proceed in parallel (types vs. parser). Step 1.3 depends on both.

### Step 1.1: Extend `BlockOptions` and error variants

**Files**

- `darkmatter/lib/src/markdown/compose/transclusion/types.rs`

**Work**

- Add to `BlockOptions`:
    - `set_object: Option<serde_json::Map<String, serde_json::Value>>` — the parsed object form payload, if present.
    - `set_properties: Vec<(String, serde_json::Value)>` — ordered list of `(name, value)` pairs from the property form (order preserved for deterministic last-write traces under permissive mode).
- Add `TransclusionError` variants:
    - `InvalidFrontmatterAssignment { line: usize, raw: String, reason: String }` — raised when `set=<value>` RHS is not a JSON5 object.
    - `InvalidReassignedFrontmatterProperty { line: usize, name: String }` — raised when `set.NAME` repeats, or when `set=` appears twice on one directive (spec §Property Errors permits one variant to cover both; use `name = "<object>"` for the duplicate-object-form case to keep the error surface small).
- Keep existing variants untouched; no impact on `Display`/`Error` trait impls elsewhere.

**Observable outcome**

- `BlockOptions` can represent the full set-override payload.
- Two new error paths exist but are not yet raised.

### Step 1.2: Add a dotted-identifier cursor helper

**Files**

- `darkmatter/lib/src/markdown/compose/parse_utils.rs`

**Work**

- Add a `try_consume_char(&mut self, expected: char) -> bool` convenience (or a `peek` helper) if not already present.
- Add `read_dotted_suffix(&mut self, line: usize) -> Result<Option<String>, CursorError>` that:
    - Returns `Ok(None)` if the next char is not `.`.
    - Otherwise consumes the `.`, reads one identifier segment via the existing `read_identifier`, and returns `Ok(Some(name))`.
    - After reading the single segment, if the next char is `.`, returns a `CursorError` describing "nested dotted keys are not supported in v1" so `set.author.name="Bob"` becomes a parse error per spec §Acceptance Criteria → Grammar.

**Observable outcome**

- Parser has a clean primitive for `set.NAME` without changing the meaning of `read_identifier` elsewhere.

### Step 1.3: Parse `set` / `set.NAME` in the directive parser

**Files**

- `darkmatter/lib/src/markdown/compose/transclusion/parser.rs`

**Work**

- In the options-loop inside `parse_directive_line`:
    - After reading the leading identifier `key`, if `key == "set"`, call `read_dotted_suffix`:
        - If it returns `Some(name)` → property form. Require `=`, read RHS via the existing `cursor.read_value(...)`, JSON5-parse the RHS, emit `InvalidReassignedFrontmatterProperty` if the name already appears in `options.set_properties`, otherwise push `(name, value)`.
        - If it returns `None` → object form. Require `=`, read RHS, JSON5-parse, require `Value::Object(map)`; otherwise raise `InvalidFrontmatterAssignment`. If `options.set_object.is_some()`, raise `InvalidReassignedFrontmatterProperty { name: "<object>" }` (duplicate object-form).
- Enforce RHS grammar from spec §Grammar:
    - Empty RHS (`set.foo=`) → the existing `cursor.read_value` returns empty; raise `ParseDirective { message: "set.<name>= requires a JSON5 value" }`.
    - Bare `set` (no `=`) → `expect_char('=')` already errors; ensure the resulting message mentions `set` so it's diagnosable.
    - Unquoted bare word (`set.name=Bob`) → JSON5 parse will fail; surface as `ParseDirective` with a message pointing to the RHS and telling the user to quote strings.
- Leave non-`set` keys (`replace`, `quotation`, `when`, etc.) untouched — they continue to flow through `apply_option`.
- Add `biscuit_file::Json5` (already used elsewhere in the CLI) or `json5` crate usage for JSON5 parsing; confirm which is already depended on — prefer `biscuit_file` for consistency.

**Observable outcome**

- Parse-time behavior for all grammar-focused acceptance criteria:
    - `set.name="Bob"` → `set_properties = [("name", "Bob")]`.
    - `set='{a:1}'` → `set_object = Some({a:1})`.
    - `set='{a:1}' set.a="Z"` → both collected, in order.
    - `set=42` → `InvalidFrontmatterAssignment`.
    - `set.name="Bob" set.name="Mary"` → `InvalidReassignedFrontmatterProperty`.
    - `set='{a:1}' set='{b:2}'` → `InvalidReassignedFrontmatterProperty { name: "<object>" }`.
    - `set.author.name="Bob"` → `ParseDirective` (nested dotted key unsupported).
    - `set.name=` / `set.name=Bob` (unquoted) → `ParseDirective`.

### Validation checkpoint

- `cargo test -p darkmatter transclusion::parser`
- Run lexer-adjacent tests to confirm nothing else regressed: `cargo test -p darkmatter compose`
- Confirm unit tests covering `set`/`set.NAME` grammar pass (added in Phase 4 at the latest; may be scaffolded here).

## Phase 2: Merge Engine and Pipeline Integration

**Goal:** Make the parsed set payload actually affect the child document's effective frontmatter, using the three-layer precedence merge with null-as-literal semantics, before any of the child's pre-op stages run. Keep cache correctness intact.

**Depends on:** Phase 1

**Parallelizable work:** Steps 2.1 and 2.2 can proceed in parallel (merge helper vs. cache hashing). Step 2.3 depends on 2.1; step 2.4 depends on 2.1 and 2.2.

### Step 2.1: Null-aware deep-merge helper and three-layer applier

**Files**

- `darkmatter/lib/src/markdown/compose/state.rs`

**Work**

- Add a sibling helper `deep_merge_override(base: &Value, overlay: &Value) -> Value` with the same recursion as `deep_merge` except the `(_, Value::Null) => base.clone()` short-circuit is removed — null in the overlay becomes the effective value. Do not touch `deep_merge`; other callers rely on "null = not set".
- Add a top-level helper:

    ```rust
    pub(crate) fn apply_set_overrides(
        base_fm: &serde_json::Map<String, Value>,
        set_object: Option<&serde_json::Map<String, Value>>,
        set_properties: &[(String, Value)],
    ) -> serde_json::Map<String, Value>
    ```

    - Start from `base_fm` as the base layer.
    - If `set_object` is `Some`, overlay it with `deep_merge_override` (middle layer).
    - For each `(name, value)` in `set_properties`, apply top-layer: treat each as a one-key overlay and `deep_merge_override` into the accumulator. This preserves dict deep-merge for dict-valued properties while hard-overriding leaves.
    - Return the resulting map.
- Unit-test `deep_merge_override` directly with the worked example from spec §Worked Example and the null-semantics acceptance criteria.

**Observable outcome**

- A pure function produces the exact effective frontmatter documented in spec §Merge Semantics, including dict deep-merge, array-as-leaf replacement, and null-as-literal.

### Step 2.2: Fold set overlay into the compose cache key

**Files**

- `darkmatter/lib/src/markdown/compose/cache/hashing.rs`

**Work**

- Add a `fn set_overlay_hash(set_object: Option<&Map>, set_properties: &[(String, Value)]) -> u64` (or similar) that produces a stable hash over the canonicalized payload.
- Export it from the `hashing` module so `render_markdown_transclusion` can include it in its `cache_key` composition.

**Observable outcome**

- Two directives with identical targets but different `set=`/`set.NAME=` produce different cache keys; equivalent payloads alias correctly.

### Step 2.3: Apply the overlay before the child pipeline runs

**Files**

- `darkmatter/lib/src/markdown/compose/mod.rs`

**Work**

- In `render_markdown_transclusion`, inside the `get_or_compute_compose` closure, after `compose_runtime.load_markdown(path)?` and before `child.run_compose_pipeline_internal(...)`:
    - If `directive_options.set_object.is_some()` or `!directive_options.set_properties.is_empty()`, compute the effective frontmatter via `state::apply_set_overrides(child.frontmatter().as_map(), directive_options.set_object.as_ref(), &directive_options.set_properties)` and replace the child's frontmatter map.
- Do NOT forward the directive's set payload through `child_options` — it must not propagate to grandchildren. Overlay is applied exactly once on the direct child.
- Re-canonicalize the child's frontmatter representation after mutation if needed (match existing patterns that go through `frontmatter_mut().as_map_mut()`).

**Observable outcome**

- The child's pre-ops (frontmatter interpolation, frontmatter shell expansion, replacement, page blocks, interpolation, body shell expansion) all see the overridden values — matching spec §Pipeline Integration (overlay-first).
- Grandchildren referenced by `::file` inside the child do NOT inherit the parent-applied overlay.

### Step 2.4: Wire cache hash

**Files**

- `darkmatter/lib/src/markdown/compose/mod.rs`

**Work**

- Extend the `cache_key` construction in `render_markdown_transclusion` to include `set_overlay_hash(…)`. Add the hash as an extra 16-hex component at the end of the existing key format so existing entries are invalidated cleanly.
- Mirror the same extra input inside `persistent_ctx` (or an equivalent OperationPersistentContext field) if the closure-hash path needs it for persistent-cache freshness to stay `Strict` under new payloads.

**Observable outcome**

- No cache collisions between distinct overlay payloads, and no spurious cache misses for the no-overlay path (the hash component is stable and neutral when both `set_object` and `set_properties` are empty).

### Validation checkpoint

- `cargo test -p darkmatter state::apply_set_overrides` (via the module's test module).
- `cargo test -p darkmatter transclusion` to confirm end-to-end directive → overlay → composed body flows.
- Sanity-run with fail-fast on a small doc that exercises `set.name="Bob"` and `set='{x:1}'` to verify effective frontmatter visible to child interpolation.

## Phase 3: Permissive Flags and Warning Presentation

**Goal:** Expose library-level toggles that downgrade the two error conditions to warnings (with partial application — sibling valid setters still apply), add the matching CLI flags, and render warning messages using biscuit-terminal's Status + BlockQuote (WARN state, orange vertical bar) per spec §Warning Messages.

**Depends on:** Phase 2 (core happy path must work before the permissive paths layer on top)

**Parallelizable work:** Step 3.1 (library flags) and Step 3.5 (warning presentation) can proceed in parallel. Steps 3.2 / 3.3 / 3.4 chain serially after 3.1.

### Step 3.1: Add permissive flags to `ComposeOptions`

**Files**

- `darkmatter/lib/src/markdown/compose/types.rs`

**Work**

- Add two crate-visible fields (match the `allow_ctx_override` pattern):
    - `pub(crate) allow_invalid_frontmatter_assignment: bool` (default `false`).
    - `pub(crate) allow_reassigned_frontmatter_property: bool` (default `false`).
- Add matching builders:
    - `pub fn with_allow_invalid_frontmatter_assignment(mut self, allow: bool) -> Self`.
    - `pub fn with_allow_reassigned_frontmatter_property(mut self, allow: bool) -> Self`.
- Include both in the manual `Debug` impl's `.field(...)` chain.
- Initialize both to `false` in `new_with_context`.

**Observable outcome**

- The library exposes the two toggles with the canonical builder pattern and they flow through `Clone` + `Debug` correctly.

### Step 3.2: Thread permissive behavior into the parser

**Files**

- `darkmatter/lib/src/markdown/compose/transclusion/parser.rs`
- `darkmatter/lib/src/markdown/compose/transclusion/types.rs`

**Work**

- The parser does not currently receive `ComposeOptions`. Two viable paths:
    1. Carry raw "pending" clauses in `BlockOptions` and defer the error-vs-warning decision until the engine stage (where `ComposeOptions` is in scope), OR
    2. Add a `ParseConfig` parameter to `parse_directives` / `parse_directive_line` and thread it from the caller.
- Prefer path (1) — it keeps `parse_directives`'s signature narrow and keeps parser unit tests stable. To implement:
    - Keep `set_object` / `set_properties` on `BlockOptions` as the parsed-successfully slots.
    - Add `deferred_set_errors: Vec<DeferredSetError>` to `BlockOptions`, where `DeferredSetError` enumerates `InvalidAssignment { raw, reason }` and `ReassignedProperty { name }`.
    - When the parser encounters an `InvalidFrontmatterAssignment` or `InvalidReassignedFrontmatterProperty` condition, push the appropriate deferred error AND drop the offending clause from the payload — matching the spec's "sibling setters on the same directive line that are independently valid STILL apply" requirement.
    - When strict (engine-side), a non-empty `deferred_set_errors` produces the corresponding hard error (first deferred error wins, or chain all — implementer's call; prefer surfacing the first with the rest in the error's context).
    - When permissive, the engine emits a `ComposeWarning` per deferred error and continues.
- Under the duplicate-property case in permissive mode, use the rightmost assignment — update the vec-insertion logic so the second encounter overwrites the first value AND records a deferred error.

**Observable outcome**

- Parser never conditionally emits errors based on flags (flags aren't in scope), but the structure encodes the problems faithfully for the engine.
- Sibling-valid clauses survive in permissive mode.

### Step 3.3: Engine-side application of permissive behavior

**Files**

- `darkmatter/lib/src/markdown/compose/mod.rs`

**Work**

- Before the overlay is applied in `render_markdown_transclusion` (Step 2.3 integration point), iterate `directive_options.deferred_set_errors`:
    - Strict mode (either flag `false` for the relevant error kind) → return `MarkdownError::from(TransclusionError::Invalid*)` with line metadata from the directive.
    - Permissive mode (flag `true`) → emit a `ComposeWarning` per deferred error using the existing `report.add_warning(ComposeWarning::new("transclusion", message).at_line(directive.line))` pattern, then continue to apply whichever set clauses survived.
- Gate the strict/permissive branch separately per error kind — the two flags are independent.

**Observable outcome**

- `set=42 set.name="Bob"` under `--allow-invalid-frontmatter-assignment` emits a warning for `set=42`, applies `name: "Bob"`, and composes successfully (spec §Flag Behavior).
- `set.name="Bob" set.name="Mary"` under `--allow-reassigned-frontmatter-property` emits a warning, uses `"Mary"`, composes successfully.

### Step 3.4: Add CLI flags

**Files**

- `darkmatter/cli/src/args.rs`
- `darkmatter/cli/src/commands.rs`

**Work**

- In `args.rs`, on the `Compose` subcommand (mirror `allow_shell_timeout`):
    - `allow_invalid_frontmatter_assignment: bool` with `long = "allow-invalid-frontmatter-assignment"`.
    - `allow_reassigned_frontmatter_property: bool` with `long = "allow-reassigned-frontmatter-property"`.
- In `commands.rs`, in `run_compose`, forward both into `ComposeOptions` via the new builders.
- Do not add an alias or short flag — keep surface area minimal and consistent with existing long-only `--allow-*` flags.

**Observable outcome**

- `md compose doc.md --allow-invalid-frontmatter-assignment` parses and passes through; behavior is validated in Phase 4 tests.

### Step 3.5: Warning presentation via biscuit-terminal Status + BlockQuote

**Files**

- `darkmatter/cli/src/output.rs` (or whichever module currently emits compose warnings to stderr)

**Work**

- Locate the existing compose-warning emission path in the CLI. If it already uses `biscuit_terminal::Status` + `BlockQuote`, reuse it verbatim for the new warning messages. If not, add a small helper (private to the CLI) that renders a warning with:
    - `Status` in WARN state showing `<b>{error-name}</b>` where `{error-name}` is `InvalidFrontmatterAssignment` or `InvalidReassignedFrontmatterProperty`.
    - A `BlockQuote` body containing:
        - A short description sentence.
        - A fenced code block showing the offending directive line with one line of context before and after (pulled from the source file when `--state`/source is known; otherwise just the directive line).
        - A blank line.
        - `- this occurred in the <blue><a href={abs-path}>{relative-path}</a></blue> file`
        - `- because of possible transclusion the line number may not be reliable but before transclusion it was on line <yellow>{N}</yellow>`
- Route the new warnings from the library's `ComposeWarning` collection into this presenter when `perf`-style routing already exists; otherwise render at the top of the stderr tail the CLI already emits after compose.
- Do NOT render anything extra to stdout; composed output must remain unaffected.

**Observable outcome**

- Running `md compose doc.md --allow-invalid-frontmatter-assignment` on a document whose `::file` uses `set=42 set.name="Bob"` prints the styled warning to stderr with the directive line highlighted and a hyperlink back to the source file, while stdout contains the composed markdown.

### Validation checkpoint

- `cargo test -p darkmatter-cli` (CLI parser tests for the two new flags).
- `cargo test -p darkmatter compose::transclusion` (permissive vs. strict paths exercised via library tests).
- Manual spot check rendering the warning style in a real terminal to confirm the orange bar and WARN status show up.

## Phase 4: Tests, Documentation, and Release Validation

**Goal:** Lock in every acceptance criterion with automated coverage; update user-facing documentation; sync the repo skill; run release-level validation.

**Depends on:** Phase 3

**Parallelizable work:** Steps 4.1 and 4.2 and 4.3 can proceed in parallel once Phase 3 lands. Step 4.4 is the final gate.

### Step 4.1: Library test coverage for grammar, merge, and pipeline integration

**Files**

- `darkmatter/lib/src/markdown/compose/transclusion/parser.rs` (tests module)
- `darkmatter/lib/src/markdown/compose/state.rs` (tests module)
- `darkmatter/lib/src/markdown/compose/mod.rs` (tests module)

**Work**

- Parser tests (spec §Acceptance Criteria → Grammar):
    - `set.name="Bob"` parses.
    - `set.name=Bob` parse error.
    - `set.name=` parse error.
    - `set` (bare) parse error.
    - `set.author.name="Bob"` parse error (nested dotted key).
    - `set=42` produces `InvalidFrontmatterAssignment`.
    - `set='{a:1}' set='{b:2}'` produces `InvalidReassignedFrontmatterProperty { name: "<object>" }`.
    - `set.name="Bob" set.name="Mary"` produces `InvalidReassignedFrontmatterProperty { name: "name" }`.
    - Each JSON5 type parses for property form: `set.age=42`, `set.tags=[1,2,3]`, `set.meta={x:1}`, `set.x=null`, `set.ok=true`.
    - Deferred-error path: `set=42 set.name="Bob"` produces one deferred error with `set.name` surviving in `set_properties`.
- State tests (spec §Acceptance Criteria → Null/Merge):
    - Null semantics at top level and inside object-form.
    - Three-layer precedence: `set='{name: "Carol"}' set.name="Bob"` → `"Bob"` wins.
    - Dict deep-merge: `{a: {x: 1}}` with `set.a='{y:2}'` → `{x: 1, y: 2}`.
    - Leaf override: `{name: "Alice"}` with `set.name="Bob"` → `"Bob"`.
    - Arrays as leaves: `{tags: ["a","b"]}` with `set.tags=["c"]` → `["c"]`.
- Pipeline integration tests (spec §Acceptance Criteria → Pipeline Integration):
    - Child `::block when="role == 'admin'"` renders when parent passes `set.role="admin"` despite child fm saying `role: "guest"`.
    - Child `{{ fm.name }}` renders as `Bob` when parent passes `set.name="Bob"`.
    - Child `replace:` rule sees overridden value.
    - `::shell` directive in child receives overridden frontmatter in its environment (assert via captured stdout).
    - Grandchild isolation: parent `::file child.md set.role="admin"`, and `child.md` contains `::file grandchild.md` — grandchild must NOT see `role: "admin"` from the parent-applied overlay (unless grandchild's own fm already has it).
- Parent-side interpolation passthrough: parent fm `dictionary: { name: "Bob" }`, directive `set={{dictionary}}` behaves identically to `set='{name: "Bob"}'`. This exercises the fact that parent's stage-5 interpolation resolves the RHS to a literal JSON5 value before transclusion runs.

**Observable outcome**

- Every bullet under spec §Acceptance Criteria is covered by at least one automated test.

### Step 4.2: CLI integration tests

**Files**

- `darkmatter/cli/tests/cli.rs`

**Work**

- Strict (default) mode:
    - `md compose parent.md` where parent contains `::file child.md set=42 set.name="Bob"` exits with a non-zero status and surfaces `InvalidFrontmatterAssignment` to stderr.
    - Same parent with `set.name="Bob" set.name="Mary"` exits with a non-zero status and surfaces `InvalidReassignedFrontmatterProperty`.
- Permissive mode:
    - `--allow-invalid-frontmatter-assignment` with `set=42 set.name="Bob"` succeeds; stdout contains composed body; stderr contains a WARN-styled `InvalidFrontmatterAssignment` block with the directive line quoted and a hyperlink back to the parent file; effective child frontmatter contains `name: "Bob"`.
    - `--allow-reassigned-frontmatter-property` with duplicate property form succeeds; stderr contains a WARN-styled `InvalidReassignedFrontmatterProperty`; effective value is the right-most (`"Mary"`).
- Flags are independent — setting one does not downgrade the other error kind.
- Golden path coverage: `::file child.md set='{author:{handle:"@bob"},tags:["blue"]}' set.name="Bob"` against the spec's worked example, asserting the composed body reflects `name: "Bob"`, `author.name: "Alice"`, `author.handle: "@bob"`, `tags: ["blue"]`.

**Observable outcome**

- CLI behavior matches spec §Acceptance Criteria end-to-end on the observable surface (stdout content, stderr warnings, exit codes).

### Step 4.3: Documentation and skill updates

**Files**

- `darkmatter/docs/transclusion/block-transclusion.md`
- `darkmatter/docs/darkmatter-compose-pipeline.md`
- `.claude/skills/darkmatter/SKILL.md`

**Work**

- `block-transclusion.md`: add a new `#### Setting Frontmatter Properties` subsection under "Options and Conditionals" that covers both forms, the three-layer precedence, null semantics, the duplicate-detection errors, and the two permissive flags. Include the worked example from spec §Merge Semantics.
- `darkmatter-compose-pipeline.md`: add a note that `set={{dictionary}}` on a parent `::file` directive is resolved by the parent's stage-5 interpolation before transclusion runs, cross-linking back to the new subsection in `block-transclusion.md`.
- `.claude/skills/darkmatter/SKILL.md`: extend the transclusion bullet list (or the compose-pipeline section as appropriate) with a one-line note that `::file` accepts `set='{…}'` and `set.NAME=<value>` to override child frontmatter.

**Observable outcome**

- A user reading the docs can author a working `set`/`set.NAME` directive without consulting the spec; the feature is discoverable from both the transclusion doc and the compose-pipeline doc.

### Step 4.4: Release validation

**Work**

- `cargo fmt -p darkmatter -p darkmatter-cli`
- `cargo test -p darkmatter`
- `cargo test -p darkmatter-cli`
- `cargo build -p darkmatter -p darkmatter-cli`
- Targeted doctest check: `cargo test -p darkmatter --doc` (the new documentation examples must compile or be marked `text`/`md`).
- Confirm the skill `SKILL.md` changes pass the repo's skill validators if any (grep for any CI guard; otherwise visually inspect against the existing SKILL style).

**Observable outcome**

- Branch is in a releasable state: tests green, no formatter diffs, docs and skill consistent with implementation.

### Validation checkpoint

- Full test suite green on both packages.
- Sample compose run against a parent/child pair exercising the worked example emits the expected effective frontmatter end-to-end.
- Permissive-mode sample surfaces warnings with the prescribed Status + BlockQuote styling.

## Dependency Summary

1. **Phase 1 → Phase 2**: the merge engine needs the parsed payload shape from `BlockOptions`.
2. **Phase 2 → Phase 3**: permissive flags gate behaviors that only make sense once the strict happy path and the cache-correct overlay exist.
3. **Phase 3 → Phase 4**: tests and docs codify final behavior; running them before Phase 3 would require repeat rewrites.

## Parallel Work Map

- Within Phase 1: Steps 1.1 (types) and 1.2 (cursor helper) are independent.
- Within Phase 2: Steps 2.1 (merge helper) and 2.2 (cache hash) are independent.
- Within Phase 3: Step 3.1 (library flags) and Step 3.5 (warning presenter) are independent.
- Within Phase 4: Steps 4.1, 4.2, and 4.3 are independent; Step 4.4 is the final gate.

## Done Criteria

- All acceptance criteria in spec §Acceptance Criteria are covered by automated tests and pass.
- Default behavior is strict: duplicate and non-object-RHS conditions are errors.
- `--allow-invalid-frontmatter-assignment` and `--allow-reassigned-frontmatter-property` (and their `ComposeOptions` counterparts) downgrade errors to styled stderr warnings and keep sibling-valid clauses applied.
- The overlay is applied to the child document's frontmatter before any of its pre-op stages run; grandchildren do NOT inherit it.
- Cache correctness is maintained: distinct overlays produce distinct cache keys; identical overlays alias correctly.
- `darkmatter/docs/transclusion/block-transclusion.md`, `darkmatter/docs/darkmatter-compose-pipeline.md`, and the repo skill describe the feature accurately and consistently.
