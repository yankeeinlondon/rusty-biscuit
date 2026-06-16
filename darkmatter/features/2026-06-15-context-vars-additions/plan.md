---
agent: "open_code/zai-coding-plan/glm-5.2"
phases: 7
created: "2026-06-15"
start_phase: 1
yolo: "true"
---

# Execution Plan: Context Variables and Expression Function Additions

Converts `spec.md` into a dependency-ordered, observable task list. Every
function task includes its descriptor registration and unit tests in the same
change because the bidirectional parity tests
(`descriptor_signature_set_equals_dispatchable_signature_set`,
`descriptor_name_set_equals_captured_runtime_key_set`) fail the build until
dispatch and catalog agree.

## Dependency Graph (high level)

```
Phase 1 (shared helpers) ─┬─► Phase 4 (FS path functions)
                          ├─► Phase 5 (link + skill functions)
                          └─► Phase 3 (without_date date matcher)

Phase 2 (Agent ctx group) ── independent of 3/4/5

Phase 3 (pure functions) ──── independent of 2/4/5 (except terminal() uses biscuit-terminal)

Phase 6 (docs) ────────────── after 2-5 (describes finished surface)
Phase 7 (integration gate) ── after 1-6
```

**Parallelizable:** Phases 2, 3, 4, and 5 touch logically distinct code units
and may be developed concurrently once Phase 1 lands. They all append to the
shared `PURE_FUNCTIONS` / `FS_FUNCTIONS` / `EXPRESSION_FUNCTION_DESCRIPTORS`
slices, so merge in any order but expect trivial merge lines in those three
files.

## Key Files

| Concern | File |
|--------|------|
| Context group enum + capture | `darkmatter/lib/src/markdown/compose/context/capture.rs` |
| Context variable descriptors | `darkmatter/lib/src/markdown/compose/context/catalog.rs` |
| Function dispatch + registries | `darkmatter/lib/src/markdown/compose/expression/functions.rs` |
| Function descriptors | `darkmatter/lib/src/markdown/compose/expression/catalog.rs` |
| Resolution context | `darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs` |
| `ctx.*` lazy resolver | `darkmatter/lib/src/markdown/compose/expression/ctx.rs` |
| Coercion helpers (`to_number`, `scalar_string`, `json_number`) | `darkmatter/lib/src/markdown/compose/expression/mod.rs` |
| Compose `find_git_root_from` | `darkmatter/lib/src/markdown/compose/mod.rs` |
| Docs: context variables | `darkmatter/docs/topics/context-variables.md` |
| Docs: expressions | `darkmatter/docs/topics/darkmatter-expressions.md` |
| Skill summary | `.claude/skills/darkmatter/compose.md` (context table) |

## Shared Implementation Contract (applies to every new function)

From spec `## Function Contract`:

- wrong arity → evaluation error
- `null` argument → propagates to `null` (except inspecting predicates: `is_integer`)
- type mismatch → evaluation error
- every function needs a matching `ExpressionFunctionDescriptor`
- function names are canonical snake_case; add underscore-free aliases only where the existing convention already applies

The existing helpers to reuse: `require_args`, `any_null`, `require_string`,
`require_number`, `resolve_arg`, `make_relative`, `is_remote_url`,
`normalize_path_arg`, `to_number`, `json_number`.

---

## Phase 1 — Shared Foundation

Build the reusable helpers that multiple later phases depend on. No public
expression surface changes in this phase; these are internal utilities with
their own unit tests.

- [ ] **1.1** Add indexed-filename grammar parser
  - Add a private helper (e.g. `parse_indexed_stem(stem: &str) -> Option<IndexedName>`) in `functions.rs` implementing the spec's `(?P<base>.+)-(?P<digits>[0-9]+)` rule.
  - `IndexedName` carries `base: String` and `index: u64` (parsed, unpadded).
  - Reject `review1.md`, `review_1.md`, `review-.md`, `review--1.md`; accept `review-1.md`, `review-100.md`, `review-001.md`.
  - Add an extension-splitter helper: extension is everything after the final `.` in the basename; empty when no `.`.
  - Unit-test accept/reject cases from the spec's grammar examples.

- [ ] **1.2** Generalize the path display policy into a reusable helper
  - The existing `make_relative(abs, base_dir)` produces the display path; extract/extend so path-component functions (`basename`, `dir`, `parent_dir`, `file_trailing`, `dir_leading`) can split the *resolved absolute path* with `std::path` and then render the relevant component using `/` separators.
  - Add a helper `display_path_with_forward_slashes(path: &Path) -> String` that joins components with `/` for stable Markdown output (per spec: "Use platform path semantics from `std::path` for splitting, but return `/` as the separator in composed strings").
  - Ensure `make_relative` continues to honor repo-root relative → base_dir relative → `~`-aliased → absolute, unchanged.

- [ ] **1.3** Add skill-root discovery abstraction (injectable for tests)
  - Create a small struct/trait (e.g. `SkillRoots`) that resolves user-scoped and local-scoped skill roots.
  - User-scoped roots derived from an injectable home dir:
    - Claude: `~/.claude/skills`
    - OpenCode: `~/.config/opencode/skill`
    - Codex: `~/.codex/skills`
  - Local-scoped roots resolved from the nearest git root (reuse `find_git_root_from`) or `ResolutionContext.base_dir`:
    - `.claude/skills`, `.opencode/skill`, `.codex/skills`, `.agents/skills`
  - Agent name → alias normalization: `claude`/`claude_code`/`claude-code`; `opencode`/`open_code`/`open-code`; `codex`.
  - Unknown agents check only `.agents/skills` and `.codex/skills`.
  - Must NOT depend on the developer's real home directory — accept an injected home path so tests are hermetic.
  - Unit-test alias normalization and root selection with a temp-dir home.

- [ ] **1.4** Add strict calendar-date substring matcher (for `without_date`)
  - A helper that scans a string for `YYYY-MM-DD` substrings and removes only those that parse as real `NaiveDate` values (reuse `parse_iso_date` already in `functions.rs`).
  - `2026-02-30` is NOT removed (not a real date). Full datetimes keep only their valid date substring removed.
  - Do not collapse leftover whitespace/punctuation (spec: callers compose further cleanup).
  - Unit-test real vs. invalid dates and datetime substring cases.

- [ ] **Checkpoint 1** — `just check darkmatter` compiles; new helper unit tests pass (`cargo nextest run -p darkmatter` filtered to the new test module).

---

## Phase 2 — Agent Context Group

Add the demand-driven `Agent` capture group with `ctx.agent` and `ctx.model`.
This phase is independent of Phases 3-5 and may proceed in parallel.

- [ ] **2.1** Add `ContextGroup::Agent` variant
  - In `capture.rs`: add the variant to the `ContextGroup` enum.
  - Update `ContextGroup::all()` to return 9 groups (add `Self::Agent`).
  - Update `ContextGroup::for_key()` to map `"agent" | "model" => Some(Self::Agent)`.
  - No new I/O: this group reads `std::env` with trimming + defaults.

- [ ] **2.2** Implement `populate_agent(values: &mut Map)`
  - Read `AGENT` env var: trim ASCII whitespace; missing/empty → `"unknown"`.
  - Read `MODEL` env var: trim ASCII whitespace; missing/empty → `"default"`.
  - No model allowlist (spec Non-Goal).
  - Both keys always inserted as `Value::String`.
  - Wire into `capture_runtime_context_for_groups` under the `ContextGroup::Agent` branch.

- [ ] **2.3** Add context variable descriptors
  - In `catalog.rs` `CONTEXT_VARIABLE_DESCRIPTORS`: add a new `// ── Agent ──` section with two descriptors (`agent`, `model`), `ContextValueType::String`, category `"Agent"`.
  - This satisfies the `descriptor_name_set_equals_captured_runtime_key_set` parity test.

- [ ] **2.4** Context capture tests
  - `AGENT` set → captured value matches (after trim).
  - `MODEL` set → captured value matches (after trim).
  - `AGENT` unset/empty → `"unknown"`.
  - `MODEL` unset/empty → `"default"`.
  - Both keys always present when group captured.
  - Descriptor parity test stays green (it captures against the real environment, so assert presence not specific values where the env var is unset).
  - Use `serial_test` if mutating env vars to avoid cross-test interference.

- [ ] **Checkpoint 2** — `just check darkmatter`; `cargo nextest run -p darkmatter` context module tests green; descriptor parity test green.

---

## Phase 3 — Pure Expression Functions

Register new pure functions in `PURE_FUNCTIONS` and their descriptors in
`EXPRESSION_FUNCTION_DESCRIPTORS`. Each function ships with success, null,
type-mismatch, and arity tests.

- [ ] **3.1** `is_positive(val)` and `is_negative(val)`
  - Use `to_number()` coercion (the same one `number()` accepts).
  - `is_positive`: `true` only when coerced value `> 0`; error when coercion fails; `0` is neither.
  - `is_negative`: `true` only when coerced value `< 0`; error when coercion fails.
  - These DO null-propagate? Spec says `-> boolean | Error` and "Returns an error when coercion fails." `to_number(None)` is `None` → treat as coercion failure → error. Confirm: spec lists them under "Type Predicate Functions" but signature is `boolean | Error`, not inspecting. Follow the explicit spec: coercion failure errors; `null` coercion fails → error. (Double-check against spec lines 168-175.)
  - Add descriptors in a new/extended "Numeric Predicates" category.
  - Tests: positive/negative/zero numbers, numeric strings, booleans (`true`→1 positive), `null` (error per coercion rule), non-numeric string (error), arity.

- [ ] **3.2** `is_integer(val)`
  - Inspecting predicate: never errors, does NOT null-propagate.
  - `true` only for JSON numbers whose value has no fractional component.
  - `false` for numberlike strings, booleans, arrays, objects, `null`.
  - Add descriptor.
  - Tests: integers, floats (`1.5`→false, `1.0`→true if represented as integer-valued number), numeric strings (false), booleans (false), null (false), arity.

- [ ] **3.3** `without_date(string)`
  - Requires JSON string; uses the Phase 1.4 date-substring matcher.
  - Null-propagates; type mismatch (non-string) errors.
  - Add descriptor in "String Mutations" category.
  - Tests: removes valid dates, leaves invalid dates (`2026-02-30`), datetime substring only removes the date part, no whitespace collapse, null propagation, type mismatch, arity.

- [ ] **3.4** `ensure_leading(var, prefix)` and `ensure_trailing(var, postfix)`
  - `var`/`prefix`/`postfix` may be JSON strings or JSON numbers.
  - `null` argument propagates to `null`.
  - Arrays, objects, booleans → error.
  - If string form of `var` already starts (ends) with string form of `prefix` (`postfix`), return `var` unchanged preserving its JSON type.
  - If `var` is a JSON number and the modifier is a JSON number or numberlike string, prepend/append and return a JSON number when representable; otherwise return a JSON string.
  - Cover all four spec examples exactly: `ensure_leading("foobar","foo")→"foobar"`, `ensure_leading("bar","foo")→"foobar"`, `ensure_leading(123,4)→4123`, `ensure_leading("123",4)→"4123"`.
  - Add descriptors.
  - Tests: the four examples, already-prefixed preservation (number stays number), null propagation, boolean/array/object error, arity.

- [ ] **3.5** `terminal(string)`
  - Requires JSON string; render through `biscuit_terminal::components::prose::Prose`.
  - Use `Prose::new(content).render_optimistic(None)` — deterministic, non-interactive, no live terminal probe (spec: "do not probe or mutate the user's live terminal").
  - Return the rendered string including ANSI SGR sequences.
  - Treat argument as Prose markup, not literal text (spec: callers escape angle brackets before calling).
  - Null-propagates; non-string errors.
  - Add descriptor (new "Rendering" category or under "String Mutations").
  - Tests: `<bold>x</bold>` renders with SGR codes, literal text passes through, null propagation, type mismatch, arity. Assert the output contains the expected ANSI sequence (`\x1b[1m`) rather than exact full bytes to stay robust to Prose internals.

- [ ] **Checkpoint 3** — `just check darkmatter`; `cargo nextest run -p darkmatter` new pure-function tests green; descriptor signature parity test green (every new signature has a descriptor and vice versa).

---

## Phase 4 — Filesystem Path Functions

Register as context-aware `FS_FUNCTIONS`. Depends on Phase 1.1 (indexed
grammar) and 1.2 (display path helper). All share the spec's "shared path
rules": resolve through `FileReference` + magic paths + `resolve_from`,
display via the `relative(file)` policy, no `Path::exists()`, HTTP(S) → error,
`/` separators in output.

- [ ] **4.1** Shared FS-path validator helper
  - Add a helper that takes the `file` argument + `ResolutionContext`, rejects HTTP(S) URLs with an error, calls `resolve_arg`, and returns the resolved absolute `PathBuf` (error on `Ok(None)` or parse/resolution failure).
  - Reuse for every function in this phase so the shared rules are encoded once.

- [ ] **4.2** Indexed-file functions
  - `is_indexed_file(file) -> boolean` — true when basename stem matches the indexed grammar.
  - `file_index(file) -> number` — parsed index; `-1` when non-indexed.
  - `increment_file_index(file) -> string` — `review-1.md`→`review-2.md`; `review-001.md`→`review-002.md`; non-indexed starts at index `2` (`review.md`→`review-2.md`); added indexes use no zero padding.
  - `decrement_file_index(file) -> string` — decrement clamped at `0`; `review-001.md`→`review-000.md`; non-indexed starts at `0` (`review.md`→`review-0.md`).
  - All return display-path-shaped strings where a path is returned.
  - Add 4 descriptors ("Filesystem" category).
  - Tests: indexed/non-indexed detection, zero-padded increment preserves width, non-indexed increment/decrement starts, clamp at 0, null propagation, HTTP(S) rejection, missing files do NOT error (existence not checked), arity.

- [ ] **4.3** Path-component functions
  - `basename(file)` — final component including extension.
  - `basename_without_index(file)` — remove indexed suffix from stem: `foo/review-1.md`→`review.md`; non-indexed unchanged.
  - `dir(file)` — directory portion of the display path.
  - `ext(file)` — final extension without `.`; `""` when none.
  - `parent_dir(file)` — directory immediately above basename: `foo/bar/baz/test.md`→`baz`; `""` when none.
  - `file_trailing(file)` — last dir segment + basename: `foo/bar/baz/test.md`→`baz/test.md`; basename when no directory.
  - `dir_leading(file)` — directory path before the last segment: `foo/bar/baz/test.md`→`foo/bar`; `""` when no leading directory.
  - Add 7 descriptors.
  - Tests: the three spec example paths (`foo/bar/baz/test.md`, `foo/review-1.md`, extensionless names), display-path rendering (repo-relative vs `~`-aliased), magic-path resolution, null propagation, HTTP(S) rejection, invalid-ref error, arity.

- [ ] **4.4** `join(left, right)`
  - `left`/`right` must be JSON strings; `left` may be relative/absolute/magic path.
  - Strip leading separators from `right` before joining; collapse duplicate separators; emit `/`.
  - Validate the joined result through the shared FS-path rules (Phase 4.1) — i.e. resolve through `FileReference`.
  - Reject HTTP(S) arguments; do not check existence.
  - Example: `join("foo/bar/", "/baz/bax.md")` → `foo/bar/baz/bax.md`.
  - Add descriptor.
  - Tests: spec example, leading/trailing separator stripping, duplicate-separator collapse, absolute `left`, magic-path `left`, HTTP(S) rejection on either arg, null propagation, arity, type mismatch.

- [ ] **Checkpoint 4** — `just check darkmatter`; full FS-function test module green; descriptor parity green.

---

## Phase 5 — Link and Skill Functions

Two remaining function families. `link` is context-aware `FS_FUNCTIONS`;
`has_skill`/`has_local_skill` are context-aware functions that read the
filesystem and need `base_dir`. Depends on Phase 1.2 (path helpers) for `link`
and Phase 1.3 (skill-root discovery) for the skill functions.

- [ ] **5.1** `link(file)` (one-argument, file-only)
  - Resolve file through shared FS-path rules (Phase 4.1).
  - Description = `relative(file)` style output (reuse existing `relative_fn` / `make_relative`).
  - Destination = resolved absolute path.
  - HTTP(S) URL strings error (spec: URL links require explicit description).
  - Use the same destination escaping as the two-argument form.
  - Add descriptor.

- [ ] **5.2** `link(target, desc)` (two-argument)
  - `desc` must be a JSON string.
  - Accept HTTP(S) URL string OR local file reference (shared FS-path rules for local).
  - HTTP(S) destinations emitted exactly as supplied after `url::Url::parse` confirms valid HTTP(S).
  - Output: `[desc](destination)` Markdown link syntax.
  - Escape `[` and `]` in link text.
  - Emit CommonMark-safe destination (angle-bracket or percent-encoding) when spaces, `)`, `<`, `>` would break parsing.
  - Add descriptor.
  - Tests for both link forms: one-arg file link, two-arg file link, two-arg HTTP(S) link, link-text escaping (`[`/`]`), destination escaping (spaces/parentheses), null propagation, HTTP(S) one-arg rejection, arity, type mismatch.

- [ ] **5.3** `has_skill(name)` and `has_local_skill(name)`
  - `name` must be a JSON string; reject names with path separators or `..` (basename-only lookup).
  - `has_skill`: checks direct child directory with that basename in any known user-scoped OR local-scoped skill root for the executing agent.
  - `has_local_skill`: local-scoped roots only.
  - Agent derived from `ctx.agent` when available, else `AGENT` env with the same defaulting rules (Phase 1.3 abstraction).
  - Only direct child directories count; nested dirs/files do not.
  - Missing roots return `false`, not an error.
  - These are context-aware: register in `FS_FUNCTIONS` (need `ResolutionContext.base_dir` + git root for local roots). The handler receives `ctx.agent` via the resolution context or env — decide and document the lookup path; the spec says "Derive the agent from `ctx.agent` when available."
  - Add 2 descriptors ("Context" or "Skills" category).
  - Tests using temporary directory roots (inject home dir from Phase 1.3); never depend on the developer's real home. Cover: known agent aliases, user-scoped hit, local-scoped hit, `has_local_skill` excludes user roots, path-separator/`..` rejection, missing root → false, nested directory does not count, null propagation, arity.

- [ ] **Checkpoint 5** — `just check darkmatter`; `cargo nextest run -p darkmatter` link + skill tests green; descriptor parity green.

---

## Phase 6 — Documentation Updates

Author-facing docs must reflect the finished surface. Do this after the
function surface is stable so docs do not describe behavior that then changes.

- [ ] **6.1** Update `darkmatter/docs/topics/context-variables.md`
  - Add the **Agent** capture group to the "Capture Groups" table (I/O = env var read; properties = `agent`, `model`).
  - Add an "Agent" section with the two variables, their defaults (`"unknown"` / `"default"`), trimming, and the no-allowlist rule.

- [ ] **6.2** Update `darkmatter/docs/topics/darkmatter-expressions.md`
  - Add the new functions to the appropriate existing sections (Type Predicates, String Mutations, Read-Side Functions / Filesystem) and add new subsections as needed (Numeric Predicates, Path Helpers, Link, Skills, Rendering).
  - Document the shared path rules briefly (resolve through `FileReference`, no existence check, HTTP(S) rejected by path helpers, `/` separators).
  - Document the indexed-filename grammar.
  - Document `terminal()` Prose-markup semantics (not literal text; escape angle brackets).
  - Update the "Authoring a New Expression Function" count references if it names a specific function count.

- [ ] **6.3** Update `.claude/skills/darkmatter/compose.md`
  - Add `ctx.agent` and `ctx.model` rows to the context-values table.

- [ ] **6.4** Update the descriptor catalogs (context + expression)
  - These were populated per-task in Phases 2-5; this task is a final review pass to confirm ordering, categories, and descriptions are consistent and the static catalogs read cleanly. No parity-test impact beyond confirming green.

- [ ] **Checkpoint 6** — docs render cleanly (`md compose` if applicable, or visual review); no broken cross-links.

---

## Phase 7 — Integration Testing & Validation Gate

Prove the new surface works end-to-end and that the whole package still builds
and passes every gate.

- [ ] **7.1** End-to-end compose tests
  - Representative functions work in `{{ ... }}` interpolation (e.g. `{{ basename(doc.path) }}`, `{{ terminal("<bold>x</bold>") }}`, `{{ ctx.agent }}`).
  - Representative functions work in `when="..."` conditions (e.g. `when="is_indexed_file(doc.path)"`, `when="has_skill('foo')"`).
  - `ctx.agent` / `ctx.model` resolve through the lazy `CtxLookup` (Phase 2 wiring).
  - Demand-driven capture: referencing `ctx.agent` captures only the Agent group (assert via the existing capture-group test pattern).

- [ ] **7.2** Claudine anti-drift regression test
  - Add/extend the test in the catalog test module that proves the new descriptor entries appear in the exported expression catalog consumed by Claudine (`claudine context --expressions`).
  - Must not add a Claudine-only hardcoded list — assert against the shared `EXPRESSION_FUNCTION_DESCRIPTORS`.

- [ ] **7.3** Full local gate (run from `darkmatter/`)
  - [ ] `just sanity` (fast confidence subset, lib + cli)
  - [ ] `just lint` (both crates)
  - [ ] `just doctest` (both crates)
  - [ ] `just test` (Level-1, both crates)
  - [ ] `just check` (clean compile, both crates)

- [ ] **7.4** Final parity confirmation
  - `descriptor_signature_set_equals_dispatchable_signature_set` green (every new overload matched).
  - `descriptor_name_set_equals_captured_runtime_key_set` green (agent + model descriptors present).
  - `every_descriptor_overload_is_dispatchable_at_its_declared_arity` green (every new overload dispatchable).

- [ ] **Checkpoint 7 — FEATURE COMPLETE** — all gates green; spec acceptance criteria met:
  - Context capture tests for `AGENT`, `MODEL`, missing values, descriptor parity ✔
  - Unit tests for every new function's success, null, type-mismatch, arity ✔
  - File helper tests (relative paths, magic paths, missing files, invalid refs, extensionless, zero-padded, non-indexed, join separators, display-path, remote URL rejection) ✔
  - Link tests (one-arg file, two-arg file, two-arg HTTP(S), text escaping, destination escaping) ✔
  - Skill tests with temp-dir roots (no real-home dependency) ✔
  - End-to-end compose tests (interpolation + `when=`) ✔
  - CLI documentation anti-drift regression ✔

---

## Notes for Implementers

- **Descriptor parity is a build gate, not a final task.** Add each descriptor in the same task as its function registration; the parity tests fail otherwise.
- **`is_integer` is an inspecting predicate** (never errors, no null propagation); `is_positive`/`is_negative` are coercing predicates that error on coercion failure. Do not unify their contracts.
- **Skill discovery must be hermetic.** Inject the home directory; never call `dirs::home_dir()` directly in the skill-root resolver — wrap it so tests pass a temp dir.
- **No `cargo fmt`.** Per repo rules, do not run `cargo fmt` unless explicitly asked.
- **Comment discipline.** Follow `AGENTS.md`: no HOW-narration, no tautological docs, fix drifted comments in any symbol whose behavior changes. Use `## H2` rustdoc sections, no `# H1`.
- **US English** for all symbol names and documentation.
