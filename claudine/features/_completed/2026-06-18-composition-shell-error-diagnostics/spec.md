---
created: 2026-06-18
reviewed: true
status: ready for planning and implementation
area:
  - darkmatter
  - claudine-cli
depends-on: claudine/features/2026-06-09-improved-descriptions/spec.md
---

# Composition Shell-Error Diagnostics

## Problem

When a `::shell` directive (or a `::shell-block` command, or a frontmatter
`$(...)` command) fails during composition, the error claudine surfaces is a
flat one-liner:

```
CompositionError: composition failed
  compose failed: Shell expansion failed: Command failed (exit 2): 'sniff repo packages --package-area' at line 42
```

This message is actively misleading on three fronts, all of which are
recoverable from data the error variant **already holds**:

1. **The line number is in the wrong coordinate space.** `at line 42` is
   numbered relative to the *body* buffer (frontmatter stripped), not the file
   the author edits. The reader has to mentally add the frontmatter line count
   to find the directive. Nothing in the message signals which coordinate space
   it is.
2. **The captured `stderr` is not reliably surfaced.** The failing command
   printed *why* it exited 2 (here: an unknown/unsupported flag). That text is
   captured into the error but can still be lost at the human-facing diagnostic
   boundary, so the message says "exit 2" and stops exactly where the useful
   information starts.
3. **The composed document state is absent.** The composed frontmatter and the
   offending source line — the context that makes a shell failure diagnosable —
   are reachable through the error's `SourceContext` but are not part of the
   execution-failure diagnostic contract.

## Root cause (verified against current code)

The shell-expansion error type already carries most of the data the message
needs. The execution-failure variant is the important gap because the command's
captured output and composed document state are not part of its stable
diagnostic contract:

```rust
// darkmatter/lib/src/markdown/compose/shell_expansion/types.rs:560
#[error("Command failed (exit {code}): '{command}' at {origin}")]
ExecutionFailed {
    ctx: Box<SourceContext>,   // carried, but not fully rendered  → defect 3
    command: String,
    code: i32,
    stdout: String,            // captured; should be rendered when useful
    stderr: String,            // captured; must reach the diagnostic → defect 2
    origin: ShellCommandOrigin,
}
```

- **Coordinate mismatch (defect 1).** `parse_directives`
  (`shell_expansion/parser.rs:42`) numbers directives `(1..).zip(content.split_inclusive('\n'))`
  over the **body buffer** it is handed. The compose pipeline keeps frontmatter
  in `self.frontmatter()` separate from the body in `self.content`, so a
  `ShellCommandOrigin::Body { line }` is body-relative. Meanwhile
  `SourceContext.content` (`biscuit-terminal/.../errors/source_context.rs`) is
  the **full file** (frontmatter is a byte *range within* `content`), and
  `SourceContext::excerpt_prose(line, …)` treats `line` as 1-based into that
  full file. The two coordinate spaces disagree by exactly the frontmatter line
  count.
- **Rendering helpers already exist (defects 2 & 3 are mostly wiring).**
  `SourceContext` already provides `linked_path_prose()` (OSC 8 file link),
  `frontmatter_prose()` (renders the composed frontmatter as a fenced `yaml`
  block — precisely the "document state" the report is missing), and
  `excerpt_prose(line, context, lang)` (an excerpt centered on a line with a `>`
  gutter). Sibling errors already use this pattern (e.g.
  `FileLinksError::MissingSourceContext` renders a `StatusBlock` at
  `file_links/types.rs:153`). Several `ShellExpansionError` variants already
  render an excerpt, so this feature should not replace the whole shell-error
  renderer. It should make `ExecutionFailed` reach parity with those variants
  and add the missing path/frontmatter/output sections.

> **Reader's note:** Some branches already contain an initial
> `ShellExpansionError::ExecutionFailed` `BlockError` arm that renders
> stdout/stderr and an excerpt. Treat that as partial progress, not as a change
> to this spec's target contract: the final diagnostic must still include the
> linked path, composed frontmatter when present, file-relative coordinates, and
> the claudine boundary behavior described below.

## Goals

1. Report shell-command failures with **file-relative line coordinates** that
   match the file the author edits (the same coordinate space
   `SourceContext.content` / `excerpt_prose` already define).
2. **Surface the captured `stderr`** (and `stdout` when relevant) in the
   diagnostic — the single highest-value change.
3. **Render the carried `SourceContext`** — linked path, the composed
   frontmatter block, and a source excerpt centered on the offending line —
   reusing the existing `SourceContext` helpers rather than adding new ones.
4. Ensure the rich diagnostic **survives the claudine boundary** instead of
   being flattened to a one-line string.

## Non-Goals

- Changing shell-command execution, policy, caching, or timeout behavior. This
  is a diagnostics-rendering change only.
- Changing schema validation ordering. Schema-required values should continue
  to fail before shell expansion, per the existing Darkmatter schema contract;
  this feature only improves diagnostics for shell commands that are actually
  allowed to run and fail.
- Adding new rendering primitives to `biscuit-terminal`. The needed helpers
  (`linked_path_prose`, `frontmatter_prose`, `excerpt_prose`) already exist; if
  one needs a small extension, prefer extending it over a parallel renderer.
- Touching the descriptor-catalog / did-you-mean work in
  [`2026-06-09-improved-descriptions`](../2026-06-09-improved-descriptions/spec.md).
  That feature enriches *unknown-function* and *arity* errors from typed
  catalogs; it shares no machinery with shell-failure diagnostics. The only
  overlap is the error-formatting contract (see below), which this spec adopts
  rather than redefines.

## Error formatting contract (inherited, not redefined)

This spec adopts the same contract Section 4 of
[`2026-06-09-improved-descriptions`](../2026-06-09-improved-descriptions/spec.md)
establishes:

- Darkmatter library errors stay **plain-text / style-free**; claudine restyles
  for stderr using `biscuit-terminal` components.
- Pipeable data and JSON-like outputs must **not** receive ANSI escapes.
- Terminal rendering must use existing `TerminalRenderable` components
  (`StatusBlock`, `Prose`, and existing source-context helpers). Do not build a
  parallel ad hoc ANSI string renderer in either crate.

The richer payload this spec exposes (stderr text, source excerpt, frontmatter
block) flows through that same boundary: structured/plain in the library,
styled only at claudine's stderr renderer.

---

## Section 1 — File-relative line coordinates

Make `ShellCommandOrigin` agree with the coordinate space `SourceContext`
already defines (full-file, 1-based), so any consumer — the `Display` string
*and* `excerpt_prose` — points at the same line the author sees.

- The authoritative space is `SourceContext.content` (the full file). Normalize
  body/shell-block origins to it.
- **Decision: normalize at construction.** When body
  shell-expansion runs, the pipeline knows the frontmatter line count (it owns
  the frontmatter/body split). Carry that offset into directive parsing so
  `ShellCommandOrigin::Body { line }` / `ShellBlock { … }` store **file-relative**
  line numbers from the start. One coordinate space everywhere; no per-consumer
  arithmetic.
- **Alternative (rejected unless construction-time offset is infeasible):**
  keep origins body-relative and add the offset only at render time. Rejected
  because it leaves two coordinate spaces live and every future consumer must
  remember to add the offset.
- Add a `cfg(test)` that composes a fixture whose frontmatter is N lines and
  whose failing `::shell` is on a known file line, then asserts the reported
  origin equals the **file** line (the test would fail today). Cover the
  frontmatter-`$(...)` and `::shell-block` origins too.
- CRLF files must count source lines, not bytes. Tests should include at least
  one CRLF fixture so the offset calculation cannot accidentally depend on LF
  byte assumptions.
- Frontmatter-origin commands already live in file coordinates because their
  authored source is in the frontmatter block. Preserve that behavior and add a
  regression assertion rather than applying the body offset to frontmatter
  origins.

## Section 2 — Surface stderr/stdout

- Include the captured `stderr` in the `ExecutionFailed` diagnostic. Include
  `stdout` only when `stderr` is empty or when `stdout` has clearly relevant
  content and the combined output still fits the budget.
- Trim trailing whitespace; preserve internal newlines exactly. Do not shell
  escape, quote-rewrite, or colorize captured command output in the library.
- **Decision: tail-biased truncation.** Bound each captured stream to the final
  20 lines and 2 KiB, whichever is smaller after UTF-8-safe slicing. Prefix the
  block with a marker such as `... output truncated; showing last 20 lines` when
  truncation occurs. The tail is the right default for compilers, CLIs, and
  stack traces because the actionable error usually appears after progress
  output.
- Keep the library representation plain-text. Claudine renders `stderr` in a
  labeled block on stderr.
- A `cfg(test)` runs a command guaranteed to fail with known stderr and asserts
  the stderr text reaches the rendered diagnostic.
- Use a portable failing fixture. Prefer spawning the current test binary/helper
  or an existing cross-platform test shim over POSIX-only commands such as
  `sh -c 'echo ... >&2; exit 2'`, because this monorepo must compile and test on
  macOS, Windows, and Linux.

## Section 3 — Render the carried `SourceContext`

Reuse the existing helpers; render order:

1. `linked_path_prose()` — the source file as an OSC 8 link in the header or
   first body line.
2. `excerpt_prose(origin_line, context, "markdown")` — the offending line with a
   `>` gutter and a few lines of context. Depends on Section 1 (correct line).
3. `frontmatter_prose()` — the composed frontmatter as a fenced `yaml` block,
   so the reader sees the document state the command ran against. Include it
   when present; omit cleanly when absent.
4. Captured command output (`stderr`, then optional `stdout`) in labeled blocks
   using the Section 2 truncation rule.

Model the assembled diagnostic on the `StatusBlock` pattern already used by
`FileLinksError` so shell failures render consistently with sibling
composition errors.

Frontmatter commands need a slightly different excerpt rule: if
`ShellCommandOrigin::Frontmatter { key }` cannot resolve to an exact YAML source
line, render the linked path, the frontmatter block, and a plain `Origin:
frontmatter.<key>` field, but omit the excerpt rather than pointing at line 1 or
another misleading fallback.

## Section 4 — Preserve richness across the claudine boundary

The screenshot shows claudine collapsing the darkmatter error to a single line,
so the boundary is dropping structure even where it exists today.

- Audit claudine's `CompositionError` rendering path. Ensure it preserves the
  structured shell-failure diagnostic (path link, excerpt, stderr, frontmatter)
  instead of `{e}`-flattening it.
- Prefer carrying `ShellExpansionError` as a typed source through
  `CompositionError::ShellExpansionFailed`, as the current claudine error type
  already intends, and update the top-level error walker only if it still cannot
  discover the inner `BlockError`. Do not solve this by duplicating Darkmatter's
  shell diagnostic formatter in claudine.
- `ShellExpansionError::Preflight` already exists specifically so a wrapped
  `MarkdownError` "renders identically to the `compose_with` path rather than
  flattening it into an opaque policy error"
  (`shell_expansion/types.rs:576`). Apply the same fidelity expectation to the
  execution-failure path.
- A claudine-side test (Writer-seam or L2 capture, per existing conventions)
  asserts the rendered failure contains the file-relative line, the stderr
  text, and the source excerpt — and that piped/JSON output carries no ANSI.

---

## Phasing

Each phase leaves the build green and is independently shippable; ordered so the
highest-value, lowest-risk change lands first.

1. **stderr/stdout surfacing** (Section 2) — biggest payoff, smallest change;
   no coordinate work required.
2. **`SourceContext` rendering** (Section 3) — path link + frontmatter block +
   excerpt, reusing existing helpers.
3. **File-relative coordinates** (Section 1) — the change that touches
   origin construction; lands after rendering exists so the excerpt and the
   line number become correct together.
4. **claudine boundary fidelity** (Section 4) — ensure the wrapper renders the
   full diagnostic and respects the plain-vs-styled contract.

## Open questions

1. **Exact source line for frontmatter keys.** The useful frontmatter diagnostic
   is the key that owns the failing command; an excerpt is useful only if it can
   point at the actual YAML line for that key.

   - **Option A — key-line lookup during rendering.** Scan
     `SourceContext.frontmatter` for a top-level key matching the origin and use
     that file-relative line for `excerpt_prose`.
     - Pros: best author experience; no origin type expansion.
     - Cons: YAML edge cases (quoted keys, nested keys, duplicate keys) can make
       a lightweight scan wrong.
   - **Option B — carry a line in `ShellCommandOrigin::Frontmatter`.** Capture
     the frontmatter key's file-relative line when parsing shell expansion.
     - Pros: precise and cheap at render time; consistent with body origins.
     - Cons: expands the origin type and requires updates at every constructor.
   - **Option C — no frontmatter excerpt.** Render the linked path,
     frontmatter block, and `frontmatter.<key>` origin only.
     - Pros: avoids misleading excerpts; smallest implementation.
     - Cons: less convenient for large frontmatter blocks.

   **Recommendation:** Option B. This matches the construction-time coordinate
   decision for body directives and avoids a second YAML-ish parser in the
   renderer. If the constructor changes prove too invasive, fall back to Option C
   for this feature and track precise frontmatter excerpts separately.

## Success criteria

- A failing `::shell` reports the line number **of the source file the author
  edits**; a `cfg(test)` with N frontmatter lines proves the file-relative
  origin (and would fail against today's body-relative numbering).
- The failing command's `stderr` appears in the rendered diagnostic.
- Large captured output is tail-truncated with an explicit truncation marker and
  no UTF-8 corruption.
- The diagnostic includes the linked source path, a source excerpt centered on
  the offending line, and the composed frontmatter block, reusing the existing
  `SourceContext` helpers.
- Claudine renders the full structured diagnostic on stderr; piped/JSON output
  carries no ANSI escapes.
- Existing schema-validation fail-fast behavior is unchanged: invalid or
  missing schema-required inputs still fail before shell commands run.
- No change to shell execution, policy, caching, or timeout behavior.
