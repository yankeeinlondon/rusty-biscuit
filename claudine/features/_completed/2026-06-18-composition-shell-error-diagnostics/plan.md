---
agent: open_code/zai-coding-plan/glm-5.2
phases: 4
created: 2026-06-18
start_phase: 1
yolo: true
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
  - darkmatter/lib/src/markdown/compose/preflight/collect.rs
  - darkmatter/lib/tests/error_snapshots/shell_expansion.rs
  - darkmatter/cli/src/approval.rs
  - claudine/lib/src/harness/shell.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/mod.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/parser.rs
  - darkmatter/lib/src/markdown/compose/inline/shell_expansion.rs
  - darkmatter/lib/src/markdown/transform/mod.rs
  - darkmatter/lib/src/markdown/compose/preflight/collect.rs
  - darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs
  - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
  - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
  - claudine/lib/src/harness/audit.rs
  - darkmatter/lib/tests/shell_expansion_coordinates.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/lib/src/composition/error.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_code:
  - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
  - darkmatter/lib/src/markdown/compose/preflight/collect.rs
  - darkmatter/lib/tests/error_snapshots/shell_expansion.rs
  - darkmatter/cli/src/approval.rs
  - claudine/lib/src/harness/shell.rs
  - darkmatter/lib/src/markdown/mod.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/parser.rs
  - darkmatter/lib/src/markdown/compose/inline/shell_expansion.rs
  - darkmatter/lib/src/markdown/transform/mod.rs
  - darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs
  - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
  - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
  - claudine/lib/src/harness/audit.rs
  - darkmatter/lib/tests/shell_expansion_coordinates.rs
  - claudine/lib/src/composition/error.rs
documentation: []
packages:
  - darkmatter
  - claudine
---

# Execution Plan — Composition Shell-Error Diagnostics

Derived from [`spec.md`](spec.md). Implements richer, file-relative, stderr-surfacing
diagnostics for failing `::shell` / `::shell-block` / frontmatter `$(...)` commands, and
ensures the rich diagnostic survives the claudine boundary.

Each phase leaves the build green and is independently shippable. Ordering follows the
spec's `Phasing` section: highest-value / lowest-risk first. The `Success criteria` block
at the end maps every spec criterion to a checkpoint.

## Verified anchors (code as of planning)

- Error variant + partial renderer: `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs:560` (`ExecutionFailed`) and `types.rs:726` (its `BlockError` arm, already rendering stdout/stderr + excerpt).
- `truncate_output` is **head-biased** today (`types.rs:772`); spec requires **tail-biased**.
- `ShellCommandOrigin` (`types.rs:16`) + `line_number()` (`types.rs:51`) are **body-relative**.
- `parse_directives` numbers body-relative (`parser.rs:42`, `(1..).zip(content.split_inclusive('\n'))`).
- `parse_directives` call sites: `markdown/transform/mod.rs:420`, `markdown/compose/inline/shell_expansion.rs:22`, `markdown/compose/preflight/collect.rs:247`.
- `Markdown::source_context_for_errors()` (`markdown/mod.rs:184`) builds `SourceContext` from **body-only** `self.content` → `frontmatter_prose()` cannot render today; this is the linchpin coupling Sections 1 & 3.
- `SourceContext` helpers all exist and are reused as-is: `linked_path_prose`, `frontmatter_prose`, `excerpt_prose` (`biscuit-terminal/lib/src/errors/source_context.rs`).
- Boundary: `CompositionError::ShellExpansionFailed` (`claudine/lib/src/composition/error.rs:113`) carries `Box<ShellExpansionError>`; the `_` catch-all (`error.rs:992`) flattens it. Delegation pattern to mirror: `ShellExpansionError::Preflight` (`types.rs:766`). Walker: `claudine/cli/src/output/error_walker.rs`.
- Test conventions: `report_block_error` + `strip_escape_codes` assertions (`error.rs:1662+`), walker tests (`error_walker.rs:73+`).

---

## Phase 1 — Surface stderr/stdout with tail-biased truncation (spec §2)

Biggest payoff, smallest change. No coordinate work. Operates entirely inside the
existing `ExecutionFailed` `BlockError` arm and `truncate_output`.

- [x] Rewrite `truncate_output` (`types.rs:772`) to be **tail-biased**: keep the final 20 lines AND final 2 KiB (whichever is smaller after UTF-8-safe char-boundary slicing). Emit the spec's truncation marker (e.g. `… output truncated; showing last 20 lines`) only when truncation occurs. Preserve internal newlines exactly; do not shell-escape, quote-rewrite, or colorize.
- [x] Refine the `ExecutionFailed` output-selection logic (`types.rs:726`) per §2: include `stderr` whenever non-empty; include `stdout` only when `stderr` is empty or `stdout` has clearly relevant content and the combined output still fits the budget. Trim trailing whitespace on each captured stream before rendering.
- [x] Add `cfg(test)` asserting a failing command's captured `stderr` reaches the rendered `ExecutionFailed` diagnostic. Use a **portable** cross-platform failing fixture (spawn the test binary / a known helper, or an existing in-repo test shim) — never POSIX-only `sh -c '… >&2; exit 2'` (repo must build/test on macOS, Windows, Linux).
- [x] Add `cfg(test)` for truncation correctness: feed inputs longer than 20 lines and larger than 2 KiB; assert the tail is kept, the marker appears, and no UTF-8 sequence is split.
- [x] Confirm the library representation stays plain-text (no ANSI) — captured output must not be colorized in the library.

**Validation checkpoint (P1):** `just darkmatter test` green (or `cargo test -p darkmatter`); new `truncate_output` + stderr-surfacing tests pass on macOS; build green.

---

## Phase 2 — Render the carried `SourceContext` (spec §3, excerpt correctness deferred to P3)

Wire the existing helpers into the `ExecutionFailed` diagnostic. Per the spec's stated
rationale, the excerpt is wired here but its **line correctness** becomes true only after
Phase 3 lands the coordinate change — so each P2 task must stay green against today's
coordinate space.

- [x] Extend the `ExecutionFailed` `BlockError` arm (`types.rs:726`) to render `ctx.linked_path_prose()` (OSC 8 file link) in the header or first body line.
- [x] Render the composed frontmatter block via `ctx.frontmatter_prose()` when the `SourceContext` carries a frontmatter range; omit cleanly (`None`) when absent. (Note: today's body-only `SourceContext` yields `None`, so this is a no-op until P3 — that is expected and keeps P2 green.)
- [x] Reorder the assembled body to the spec's render order: (1) linked path, (2) excerpt (`ctx.excerpt_prose(origin.line_number(), 1, "markdown")`), (3) frontmatter block, (4) captured output (`stderr`, then optional `stdout`) using the P1 truncation rule.
- [x] Add the frontmatter-origin special rule: when `ShellCommandOrigin::Frontmatter { key }` cannot resolve an exact YAML line, render the linked path + frontmatter block + a plain `Origin: frontmatter.<key>` field, but **omit the excerpt** rather than pointing at line 1 or another misleading fallback.
  - Open Question 1 resolution: attempt **Option B** (carry a file-relative line in `ShellCommandOrigin::Frontmatter`) to match the construction-time coordinate decision for body directives; if the constructor changes prove too invasive, fall back to **Option C** (no frontmatter excerpt) for this feature and track precise frontmatter excerpts separately.
- [x] Add `cfg(test)` asserting the linked path and (when a full-file `SourceContext` is supplied to the test) the frontmatter block appear in the rendered `ExecutionFailed` diagnostic.
- [x] Add `cfg(test)` asserting a `Frontmatter`-origin failure renders the path + `frontmatter.<key>` field and **no** excerpt line gutter.

**Parallelizable:** within this phase, the two new `cfg(test)` blocks can be authored in parallel once the arm edits land. P2 is independent of P1's truncation internals (it consumes P1's truncation rule), so P2 may start as soon as P1's `truncate_output` contract is stable.

**Validation checkpoint (P2):** `just darkmatter test` green; rendering tests pass; the excerpt still renders against today's coordinate space without regressions.

---

## Phase 3 — File-relative line coordinates (spec §1) — the linchpin

The change that touches origin construction. Lands after rendering exists (P2) so the
excerpt and the line number **become correct together**. This is the phase that switches
`SourceContext` to full-file content, which simultaneously makes P2's `frontmatter_prose()`
render real frontmatter and makes the excerpt point at the file line the author edits.

- [x] Introduce a frontmatter line-offset mechanism: carry the frontmatter line count into `parse_directives` so `ShellCommandOrigin::Body { line }` and `ShellBlock { start_line, command_line }` store **file-relative** line numbers from construction (spec decision: normalize at construction, not at render time).
- [x] Update `parse_directives` and its three call sites to supply the offset: `markdown/transform/mod.rs:420`, `markdown/compose/inline/shell_expansion.rs:22`, `markdown/compose/preflight/collect.rs:247`. Compute the offset from the frontmatter/body split the pipeline already owns.
- [x] Switch the `SourceContext` used by the compose/transform shell paths to carry **full-file** content (frontmatter + body) with the frontmatter byte range set, so `excerpt_prose` uses file-relative lines and `frontmatter_prose` renders. Likely a new helper on `Markdown` (full-file reconstruction + range) or an extension of `source_context_for_errors()` (`markdown/mod.rs:184`).
- [x] Preserve the existing file-relative behavior of frontmatter-`$(...)` origins (constructed in `markdown/compose/frontmatter_shell_expansion.rs` and `preflight/collect.rs`); do **not** apply the body offset to frontmatter origins. Add a regression assertion.
- [x] Add `cfg(test)`: a fixture with N frontmatter lines and a failing `::shell` on a known **file** line asserts the reported origin equals the file line (this assertion fails against today's body-relative numbering). Cover the `::shell-block` and frontmatter-`$(...)` origins too.
- [x] Add a CRLF `cfg(test)` fixture proving the offset counts **source lines, not bytes** (no LF byte-length assumption; `detect_frontmatter_range` already handles CRLF — reuse its discipline).
- [x] Reconcile preflight `ShellCommandEntry` line fields (`preflight/collect.rs:249,260,300`) with the now-file-relative origins; update assertions / deny-list entries that previously assumed body-relative lines.

**Parallelizable with care:** the `parse_directives` offset plumbing and the `SourceContext` full-file switch are tightly coupled (both must land together or the excerpt breaks). They may be developed in parallel by two engineers with a single coordinated merge, but must not ship independently.

**Validation checkpoint (P3):** `just darkmatter test` green; the N-frontmatter-lines origin test fails on the old code and passes on the new; `frontmatter_prose()` now renders the real composed frontmatter block; the excerpt gutter points at the file line the author edits.

---

## Phase 4 — Preserve richness across the claudine boundary (spec §4)

Ensure the structured shell-failure diagnostic (path link, excerpt, stderr, frontmatter)
survives the wrapper instead of being `{e}`-flattened, and that piped/JSON output carries
no ANSI.

- [x] Audit `CompositionError::ShellExpansionFailed` rendering (`claudine/lib/src/composition/error.rs:113` + the `_` catch-all at `error.rs:992`). Add an explicit `CompositionError::ShellExpansionFailed { error, .. }` arm in the `BlockError` impl (`error.rs:776`) that delegates to `error.status_block(term)` — mirroring the `ShellExpansionError::Preflight` delegation at `types.rs:766`.
- [x] Only if delegation still cannot surface the inner block: verify whether `error_walker.rs::deepest_block_error` reaches the leaf `ShellExpansionError` via `as_block_error`; if not, update the walker / `as_block_error` discovery so the leaf is reached. Prefer the explicit-arm fix first; do **not** duplicate Darkmatter's shell formatter in claudine.
- [x] Honor the plain-vs-styled contract: when terminal `ColorDepth::None` (piped / `NO_COLOR` / JSON), the rendered diagnostic must contain no escape bytes. Apply the same boundary strip used by `InlineComposeSequenceMismatch` (`error.rs:1036`).
- [x] Add a claudine-side test (Writer-seam / `report_block_error` capture, per `error.rs:1662+` conventions) asserting the rendered failure contains the **file-relative line**, the **stderr text**, and the **source excerpt** — i.e. it is not collapsed to a one-line string.
- [x] Add a claudine-side test asserting piped / `ColorDepth::None` output for a shell failure carries no ANSI (`!rendered.contains('\x1b')`), mirroring `error.rs:1711`.

**Validation checkpoint (P4):** `just claudine test` green; boundary test shows the structured diagnostic survives (file line + stderr + excerpt all present); no-ANSI test passes.

---

## Cross-phase validation & success criteria (maps to spec `Success criteria`)

- [x] After P4: run `just darkmatter test`, `just claudine test`, `just lint`, `just build` (or the repo-equivalent `just` recipes) — all green.
- [x] **Spec criterion — file-relative line:** a failing `::shell` reports the line of the source file the author edits (P3 `cfg(test)` with N frontmatter lines proves it and would fail today). ✔ P3
- [x] **Spec criterion — stderr surfaced:** the failing command's `stderr` appears in the rendered diagnostic. ✔ P1
- [x] **Spec criterion — tail-truncation:** large captured output is tail-truncated with an explicit marker and no UTF-8 corruption. ✔ P1
- [x] **Spec criterion — full SourceContext render:** diagnostic includes linked source path, a source excerpt centered on the offending line, and the composed frontmatter block, reusing existing `SourceContext` helpers. ✔ P2 + P3
- [x] **Spec criterion — boundary fidelity:** claudine renders the full structured diagnostic on stderr; piped/JSON output carries no ANSI. ✔ P4
- [x] **Spec criterion — no behavior change:** schema-validation fail-fast ordering unchanged; no change to shell execution, policy, caching, or timeout behavior (add a regression note / test if the schema-before-shell ordering has an existing guard test). ✔ all phases
- [x] **Manual smoke (macOS):** compose a fixture with frontmatter + a failing `::shell`; confirm the rendered diagnostic shows the file-relative line, stderr, excerpt, frontmatter block, and linked path; confirm `| cat` output has no ANSI.
