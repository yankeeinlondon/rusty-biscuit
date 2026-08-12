---
agent: claude
phases: 4
created: 2026-07-01
start_phase: 1
yolo: true
clarified: open_code/zai-coding-plan/glm-5.2
review_iterations: 4
---

# Execution Plan — `has_command(cmd)` Host Command Existence Probe

> Source specification preserved below the plan (see **Appendix: Functional
> Specification**). This plan is the executable derivation of that spec. When
> plan and spec disagree, the spec is authoritative — flag the drift and update
> this plan.

## Goal / Definition of Done

A boolean expression function `has_command(cmd)` (alias `hascommand`) that
reports whether `cmd` is a runnable program on the host — via OS-native PATH
search for a bare name, or exists-and-executable check for an absolute path —
by delegating entirely to `which::which(cmd)`. It **never executes** the command
and needs **no whitelisting**. Complete when every Definition-of-Done bullet in
the spec is demonstrably true and the full validation suite (Phase 4) is green
on the host, with cross-platform behavior reasoned through for macOS / Linux /
Windows.

## Key Facts (verified against the current tree)

- `which = "7"` is already in `darkmatter/lib/Cargo.toml:51` — **no new
  dependency**.
- Model the handler on `file_exists_fn`
  (`functions.rs:1110`), **not** `has_skill_fn`. `file_exists_fn` uses
  `require_args_expr("file_exists", args, 1)?` for arity, `any_null` → `false`,
  and `require_string(...)` in a `match` that returns `false` on error.
- `FS_FUNCTIONS` table starts at `functions.rs:2003`; `file_exists` is
  registered at `functions.rs:2006`.
- Dispatch supplies the no-`ResolutionContext` error at the dispatch layer, so
  the handler ignores `ctx` (use `_ctx`). That inherited error is the function's
  **only** error path.
- Catalog: `EXPRESSION_FUNCTION_DESCRIPTORS` (`catalog.rs:45`); the `file_exists`
  descriptor is at `catalog.rs:605` (`category: "Filesystem"`, `order: 3`).
- Parity test `descriptor_signature_set_equals_dispatchable_signature_set`
  (`catalog.rs:967`) requires exact bidirectional descriptor↔dispatch parity.
- Doc table is generated: markers at
  `darkmatter/docs/topics/darkmatter-expressions.md:306` / `:388`; test
  `narrative_doc_function_table_matches_catalog` (`catalog.rs:1106`) enforces an
  exact match. Regenerate with `just darkmatter regen-expr-doc`
  (`darkmatter/justfile:289`).
- Filesystem Helpers prose subsection: `darkmatter-expressions.md:436`.

---

## Phase 1 — Handler + Dispatch Registration

Goal: `has_command` / `hascommand` resolve through `dispatch_fs` with the
`file_exists`-mirrored never-error-on-argument contract.

- [ ] Add `has_command_fn(args: &[Value], _ctx: &ResolutionContext) -> Result<Value, ExpressionError>`
      to `darkmatter/lib/src/markdown/compose/expression/functions.rs`, placed
      immediately after `file_exists_fn` (~line 1139), structured on
      `file_exists_fn`:
      - `require_args_expr("has_command", args, 1)?` (arity guard — the one
        argument-related error case the spec keeps).
      - `any_null(args)` → `Ok(Value::Bool(false))`.
      - `require_string("has_command", &args[0])` inside a `match`; `Err` →
        `Ok(Value::Bool(false))` (non-string → `false`).
      - Empty string → `Ok(Value::Bool(false))` (explicit guard documenting
        intent; `which` would also miss it).
      - Otherwise `Ok(Value::Bool(which::which(raw).is_ok()))` — no PATH
        capture, no CWD/base-dir resolution, no tilde/relative handling
        (delegated to `which`; documented gaps).
- [ ] Write a `///` doc comment noting: pure existence/executability probe,
      never executes, mirrors `file_exists`, delegates path semantics to
      `which`, and the two documented gaps (tilde, relative paths). No `# H1`;
      follow repo rustdoc convention.
- [ ] Register in `FS_FUNCTIONS` (`functions.rs:2003`), directly after the
      `file_exists` row (`:2006`):
      `FsFunction { canonical: "has_command", aliases: &["hascommand"], signatures: &["has_command(cmd)"], handler: has_command_fn },`

**Checkpoint 1:** `cargo check -p darkmatter` (or `just darkmatter build`)
compiles. Parity test **will still fail** until Phase 2 adds the descriptor —
that is expected and confirms the dispatch surface changed.

---

## Phase 2 — Descriptor Catalog + Generated Doc Table

Goal: descriptor↔dispatch parity restored; generated doc table carries the new
Filesystem row. Depends on Phase 1 (signature string must match exactly).

- [ ] Add an `ExpressionFunctionDescriptor` to
      `EXPRESSION_FUNCTION_DESCRIPTORS` in
      `darkmatter/lib/src/markdown/compose/expression/catalog.rs`, modeled on the
      `file_exists` descriptor (`:605`):
      - `signature: "has_command(cmd)"`
      - `description:` concise, e.g. `"Returns true when the command is found on PATH or is an existing executable path."`
      - `category: "Filesystem"`
      - `order:` a value that groups it with the other Filesystem entries near
        `file_exists` (file_exists is `order: 3`); pick an `order` so it renders
        alongside the existence-probe entries without disturbing existing
        ordering.
      - `example:` an `Example` with an `Executable` verification is risky
        (host-dependent binary presence would make the generated example
        non-deterministic). Prefer `example: None` (renders an empty Example
        cell, like `absolute`/`link`), OR a `DisplayOnly` example if a literal
        illustration is wanted. **Do not** use an `Executable` example that
        probes a real binary.
- [ ] Regenerate the doc table: `just darkmatter regen-expr-doc`. Confirm a new
      `| Filesystem | has_command(cmd) | … |` row appears between the
      `<!-- BEGIN/END GENERATED FUNCTION TABLE -->` markers in
      `darkmatter/docs/topics/darkmatter-expressions.md`.

**Checkpoint 2:** Parity test `descriptor_signature_set_equals_dispatchable_signature_set`
and `narrative_doc_function_table_matches_catalog` both pass
(`just darkmatter test` filtered to the `expression::catalog` module, or full
`just darkmatter test`).

---

## Phase 3 — Tests & User-Facing Prose

Goal: every Definition-of-Done behavior is asserted; documented gaps are written
up. The two task groups below are **parallelizable** (3a code, 3b docs — no
shared files).

### 3a — Unit tests (parallelizable with 3b)

Add to the `functions.rs` test module, alongside the `file_exists` tests
(~line 2649). Use `ResolutionContext::new(tempdir)` and `tempfile` fixtures as
the existing tests do.

- [ ] `has_command` **found**: `has_command_fn(&[json!("ls")], &ctx)` (or `echo`)
      → `true`. Prefer a near-universal Unix binary; if targeting `rg`, gate on
      its presence rather than asserting unconditionally. On Windows CI a
      `PATHEXT` binary (e.g. `cmd`) exercises the extension-resolution path.
- [ ] **Not found**: `has_command_fn(&[json!("definitely-not-a-real-bin-zzz")], &ctx)` → `false`.
- [ ] **null** → `false`; **number** (`json!(42)`) → `false`; **array**
      (`json!([])`) → `false`; **object** → `false`; **bool** → `false`.
- [ ] **Empty string** (`json!("")`) → `false`.
- [ ] **Absolute path to real executable** → `true`. Build the fixture from a
      known system binary path resolved via `which::which` (portable) OR create
      a temp file and `chmod +x` it on Unix (`#[cfg(unix)]`).
- [ ] **Absolute path to non-existent file** → `false`.
- [ ] `#[cfg(unix)]` **absolute path to existing non-executable file** →
      `false` (write a temp file with no executable bit).
- [ ] **Directory on PATH named like the probe** → `false` (create a temp dir,
      prepend to a scoped `PATH`, or assert a well-known directory name is not
      resolved). Keep this robust — `which` rejects directories, so a simpler
      assertion that a directory path argument yields `false` is acceptable.
- [ ] **Tilde gap**: `has_command_fn(&[json!("~/bin/x")], &ctx)` → `false`.
- [ ] **Relative gap**: `has_command_fn(&[json!("./mytool")], &ctx)` → `false`.
- [ ] **Alias + dispatch**: assert `dispatch_fs("has_command", …, &ctx)` and
      `dispatch_fs("hascommand", …, &ctx)` both resolve (return `Some(..)`),
      mirroring the existing `dispatch_fs` alias tests in this module.
- [ ] Guard any test that depends on a specific host binary so it stays green on
      macOS, Linux, and Windows CI (choose per-OS binaries under `cfg`, or probe
      via `which` first).

### 3b — Documentation prose (parallelizable with 3a)

- [ ] In `darkmatter/docs/topics/darkmatter-expressions.md`, add prose to the
      `#### Filesystem Helpers` subsection (~line 436), near `file_exists`:
      - One row/paragraph describing `has_command(cmd)` as a PATH/executable
        existence probe that never executes and needs no whitelisting.
      - Explicitly document both **gaps**: tilde (`~`) is not expanded; relative
        paths (`./foo`, `bin/foo`) are not resolved — both return `false` by
        design (never-error contract), addressable later without an API change.
      - Note the Windows `PATHEXT` / Unix executable-bit behavior and that
        directories and symlinked executables behave as the spec states.
      - Keep the "Remote URL arg?" column semantics consistent: `has_command`
        takes **no** remote URL argument.

**Checkpoint 3:** `just darkmatter test` and `just darkmatter test-l2` (if the
new tests touch L2 surfaces — they should not) pass locally. Doc prose edits do
**not** live inside the generated-table markers, so they do not affect the
table-match test.

---

## Phase 4 — Validation & Cross-Platform Verification

Goal: prove the Definition of Done end-to-end and confirm no regression.

- [ ] `just darkmatter lint` — clippy clean (watch for unused `ctx`; use `_ctx`,
      not an `#[allow]`).
- [ ] `just darkmatter test` — full unit suite green, including the parity and
      doc-table-match tests.
- [ ] Manual smoke via the compose/expression surface: verify
      `has_command("ls")` (or a present binary) evaluates `true` and
      `has_command("definitely-not-a-real-bin-zzz")` evaluates `false` through a
      real resolution context (e.g. a fixture doc or the existing dispatch test
      harness).
- [ ] Confirm the no-`ResolutionContext` path still yields the standard
      "requires a document resolution context, which is unavailable here" error
      (inherited from dispatch) — no bespoke error added.
- [ ] Cross-platform reasoning pass (host is macOS-only): confirm the
      implementation contains **no** OS-specific branches — all platform
      behavior (`PATHEXT`, executable bit, symlinks, directory rejection) is
      delegated to `which`. Ensure host-binary-dependent tests are `cfg`-gated
      or probe-guarded so Windows/Linux CI stay green.
- [ ] Re-read the spec's **Definition of Done** list and tick each bullet
      against a passing test or a manual observation.

**Final checkpoint:** All Phase 4 boxes checked; `git diff` shows changes only
in `functions.rs`, `catalog.rs`, and `darkmatter-expressions.md` (plus this
plan). No `cargo fmt` run. No unrelated edits (Rule 3 Surgical Changes).

---

## Risks & Notes

- **Non-determinism in tests/examples** — never assert an unconditional `true`
  for a binary that may be absent in a given CI image; gate on `which` or choose
  per-OS near-universal binaries. Keep the catalog `Example` non-executable to
  avoid a host-dependent generated doc table.
- **Parity ordering** — Phase 1 and Phase 2 must both land before the parity
  test can pass; do not treat the interim red parity test in Checkpoint 1 as a
  failure.
- **`ctx` unused** — the handler takes `&ResolutionContext` only to satisfy the
  `FsFunction` handler type; ambient `PATH` is intentionally used (spec
  rationale: `PATH` is never mutated by the compose process, unlike CWD).
- **Non-interactive session** — `just`/`cargo` may prompt or be denied; run
  one-shot with explicit non-interactive flags. Do not run signing/credential
  commands. Report and route around any denied tool call.

---

## Appendix: Functional Specification (verbatim source)

<!-- Original spec preserved for traceability; git history also retains it. -->

### `has_command(cmd)` — Host Command Existence Probe

**Purpose.** Add a boolean expression function `has_command(cmd)` that reports
whether `cmd` is a runnable program on the host: found via an OS-native PATH
search for a bare command name, or verified to exist-and-be-executable when
given as an absolute path. The function is a pure existence/executability
probe — it **never executes the command** and requires **no whitelisting**.

**Dispatch category and PATH source.** Registered in `FS_FUNCTIONS` (real I/O,
not pure). Reads ambient `PATH` via the `which` crate (`which = "7"`, already a
dependency). Captured-`PATH` plumbing was rejected as unjustified overhead:
`PATH` is inherited from the launching shell and never mutated by compose.
Handler signature `(args: &[Value], ctx: &ResolutionContext)`, threaded through
`dispatch_fs`; the no-`ResolutionContext` error is inherited from the dispatch
layer and is the function's only error path.

**Argument/null/type contract.** Mirrors `file_exists`. Return type is always
`Value::Bool`. `null` → `false`; non-string → `false`; empty string → `false`;
unfound → `false`; non-executable → `false`. Never errors on argument type or
value. Diverges from `has_skill`/`has_local_skill` (which null-propagate and
error on type). Keep the `require_args_expr(..., 1)` arity check from
`file_exists_fn` — arity errors are separate from the "never errors on its
argument value/type" guarantee.

**Path argument semantics (delegated to `which::which`).** Bare name → PATH
search. Absolute path → exists AND executable. Windows respects `PATHEXT`. Unix
requires the executable bit. Symlinks are followed. Directories are rejected.

**Documented gaps (not bugs).** Tilde (`~`) is not expanded →
`has_command("~/bin/mytool")` = `false`. Relative paths (`./mytool`, `bin/foo`)
are not resolved → `false` (not resolved against PATH, base dir, or CWD).
Both addressable later without an API change.

**Categorization.** Filesystem category (not Context) — a host filesystem
existence probe over `PATH`, like `file_exists`. Applies in the descriptor
catalog and the docs.

**Naming/alias.** Canonical `has_command`; alias `hascommand`.

**Implementation surfaces.**
1. Handler + dispatch registration — `functions.rs` (`has_command_fn` modeled on
   `file_exists_fn` ~line 1110; register in `FS_FUNCTIONS` ~line 2003 next to
   `file_exists`).
2. Descriptor catalog — `catalog.rs` (`EXPRESSION_FUNCTION_DESCRIPTORS` line 45,
   modeled on the `file_exists` descriptor line 607, `category: "Filesystem"`;
   keep the parity test at line 951 green).
3. Documentation — `darkmatter/docs/topics/darkmatter-expressions.md`
   (generated table between markers lines 306/388 regenerated from the catalog;
   prose in `#### Filesystem Helpers` line 436 documenting both gaps).

**Non-functional.** Does not execute the target; no whitelisting; consistent
across macOS, Windows, Linux (Windows `PATHEXT`, Unix executable bit).

**Definition of Done.** See the plan's Goal and Phase 4; the spec enumerates:
present-binary → `true` and bogus → `false`; `null`/`42`/`[]`/non-string →
`false`; `""` → `false`; absolute executable → `true`; absolute non-existent →
`false`; absolute non-executable (Unix) → `false`; PATH directory named like the
probe → `false`; `~/bin/x` → `false`; `./mytool` → `false`; registered as
`has_command` with `hascommand` alias, both via `dispatch_fs`; parity test still
passes; consistent across macOS/Linux/Windows (Windows exercising `PATHEXT`).
