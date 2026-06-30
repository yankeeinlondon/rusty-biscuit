---
status: "ready for planning and implementation"
reviewed: true
review_iterations: 11
---

## An Example of Poor Error Messages

Today we have errors like this:

```sh
 MarkdownError: transform failed
┃
┃ frontmatter key 'iteration': Interpolation evaluation failed for 'frontmatter(spec, 'review_iterations') ? frontmatter(spec,
┃ 'review_iterations') || 1 : 1': frontmatter() invalid file path: "features/2026-06-21-opencode-log-fix/spec.md"
┃
┃ Review the transform pipeline inputs and any configured rules.
```

This error basically comes down to a file reference not being valid, but the
current report is dense enough that the user has to study implementation detail
before they can see the mistake.

What we need to do in these situations is:

- report the missing file as the primary error condition and then explain the underlying details after that:

```sh
 MarkdownError: invalid file path
┃
┃ The invalid file reference <orange>features/2026-06-21-opencode-log-fix/spec.md</orange> was assigned
┃ to the <inverse>iteration</inverse> Frontmatter property when using the
┃ <blue>{prompt}</blue> prompt file.
┃
┃ ```yaml
┃ $schema: 
┃     spec: "features/2026-06-21-opencode-log-fix/spec.md"
┃     iteration: "frontmatter(spec, 'review_iterations') ? frontmatter(spec,
┃ 'review_iterations') || 1 : 1': frontmatter()"
┃ ```
┃
┃ Did you mean?
┃ 
┃ - `{suggestion-1}`
┃ - `{suggestion-2}`
```

The target report differs from the current one in a few fundamental ways:

1. It identifies the real problem (for example, an invalid file reference) and
   makes that the focus of the error message.
2. It shows the frontmatter properties involved in the failure, not no YAML and
   not the whole frontmatter block. The excerpt includes structural parents such
   as `$schema` so the user can see the relevant shape.
3. It links the prompt file with OSC8 when the terminal supports it, so the user
   can jump back to the file that needs attention.
4. It suggests likely intended files when the missing file looks like a typo,
   using a bounded similarity search over nearby filesystem candidates.

## Context

There is no point in solving only this specific failure. We must identify the
pattern it represents and the similar patterns that also produce dense,
hard-to-understand error messages. Once the bad reporting patterns are visible,
we can apply a structural solution.

Clear error messages are essential for both Claudine and Darkmatter. These
libraries and CLIs provide a powerful toolset, and new users will make mistakes.
The error system must help callers quickly understand the mistake and fix it
without scouring documentation or trying random changes.

The importance of this task means the right solution matters more than the
expedient solution. The scope of this improvement must address these issues in
Darkmatter and in Claudine.

> Note: remember we do not have an installed user base for Claudine and Darkmatter yet so we have the freedom to make breaking changes where necessary to achieve our goals. That doesn't mean we should strive for doing things in a breaking manner but if doing so provides notable benefits then this solution should be considered.

## Task

The fix is **not** a better string for the one failure above — it is a structural change to
how **Darkmatter** and **Claudine** produce, carry, render, and classify errors, so that
every error in the class becomes (a) legible to a human and (b) handleable by a program. The
full design lives in the companion documents below; this section states the goal, scope, and
acceptance criteria, and indexes those documents.

### Two axes

1. **Rendering (legibility).** Report the user's real mistake as the primary error — a
   cause-named headline, a *focused* context excerpt (only the involved keys plus their
   `$schema` parent), OSC8-linked paths, and a cause-specific "did you mean?" hint — never
   the mechanism ("transform failed") that surfaced it. Designed in
   [`integrated-design.md`](./integrated-design.md).
2. **Handleability (classification).** Expose every error through stable facets
   (`category` / `code` / `disposition` / `origin` / `detail`) so an API caller or a prompt
   author can react to it — tap a pattern, target a code, or target an instance. Contract
   **ratified and locked** in [`error-catalog.md`](./error-catalog.md).

The two axes share **one taxonomy**: the typed error is the single source of truth for both.
There is no parallel string-matched classification layer — building one would recreate the
control-flow-by-string bug (`is_fatal_eval_error`) this effort exists to remove.

Reader's note: this is a design tightening from review. The existing documents sometimes
talk about compose fatality as if it were fully determined by `disposition`. That would be
too coarse for the current contract: `composition.unknown_function` and
`composition.invalid_file_reference` are both `correctable` author errors, but lenient
compose policy treats both as fatal in body interpolation (see the resolved open question
below — a present file reference that fails to resolve is fatal). The classification facets
remain the single source of truth for handling and rendering; the compose fatal/warn policy
is a separate projection. That projection is held behavior-neutral during the typing refactor
**except** for the one ratified, intentional promotion (file-reference fatality), which is
characterized and tested separately.

### Scope

- **Both libraries.** Darkmatter owns the diagnostic substrate (typed causes, the focused
  excerpt, file suggestions, the `Diagnostic` facet trait); Claudine *preserves and
  transports* typed errors via `#[from]`/`#[source]`, never flattening them to `String`.
- **Breaking changes are acceptable** where they remove lossy boundaries (no installed user
  base yet); prefer non-breaking where it costs nothing.
- We **extend** the existing `BlockError` / `StatusBlock` / `SourceContext` substrate — no
  `miette` or new diagnostic framework.
- **Public contract authority.** [`error-catalog.md`](./error-catalog.md) is the locked
  contract for category, code, disposition, origin, severity, and detail field names. If a
  companion document still has older open-question language about those values, treat the
  catalog as authoritative and update the companion text during implementation rather than
  reopening the ratified choices.
- **Error provenance survives process and crate boundaries.** Any error intended for
  lifecycle `err.*`, API callers, persisted recovery records, or CLI rendering must preserve
  its typed cause chain and serializable detail payload. A `Display` string is presentation
  only and must not be used as the durable representation.
- **Terminal output remains component-based.** New human-facing reports must render through
  the existing `TerminalRenderable`/`BlockError`/`StatusBlock` path, preserving TTY gating,
  `NO_COLOR`/`FORCE_COLOR` behavior, OSC8 capability checks, and non-TTY ANSI stripping.

### Related design documents

- [`error-patterns.md`](./error-patterns.md) — the eleven recurring anti-patterns (the *why*).
- [`design-1.md`](./design-1.md) · [`design-2.md`](./design-2.md) — the two architect proposals.
- [`design-transcript.md`](./design-transcript.md) — how the two were synthesized.
- [`integrated-design.md`](./integrated-design.md) — **the chosen design**: type model,
  rendering contract, the fatality correctness gate, and phasing. *Start here for
  implementation.*
- [`error-structure.md`](./error-structure.md) — the handleability model and the structural
  requirements recovery imposes (absolute-time `reset_at`; `detail` rich enough to author a
  corrective `resume` message now or hand it to a human later).
- [`error-catalog.md`](./error-catalog.md) — the **ratified, locked** contract: the facet
  enums (12 categories · 5 dispositions · 5 origins · 3 severities), the ~38 dotted codes,
  and each code's `detail` schema.

### Delivery

Phased and independently shippable; the sequence and rationale are in
[`integrated-design.md`](./integrated-design.md) §13. Phases 1–3 resolve the literal example
above; 4–6 generalize the rendering to the whole class; 7 adds handleability by implementing
the ratified [`error-catalog.md`](./error-catalog.md); 8 closes the late-binding corners.

Implementation must preserve the existing behavior before improving it: phase 1 adds the
fatal/warn characterization matrix, phase 2 keeps `Display` output behavior-neutral while
typing the engine, and only later phases are allowed to change user-visible rendering. Any
intentional behavior change must be called out as such and tested separately from the typing
refactor. There is exactly one such intentional behavior change in this effort: promoting a
present-but-unresolvable file reference to fatal in lenient body interpolation (the resolved
open question below). The characterization matrix encodes this promotion explicitly, so the
typing refactor is still provably behavior-neutral against the matrix it locks.

### Success criteria

(full list in [`integrated-design.md`](./integrated-design.md) §15)

- The reference failure renders with a root-cause headline ("invalid file path"), names the
  receiving frontmatter key, links the prompt file (OSC8 where capable), shows a *focused*
  excerpt (`$schema`/`spec`/`iteration`), and suggests likely files — **identically in
  `md compose` and `claudine compose`**.
- Fatal-vs-warn behavior is provably unchanged by the typing refactor (characterization
  matrix green), with the single ratified exception of file-reference fatality, which the
  matrix encodes and tests as its own intentional cell.
- No new string-only lower-layer error variants; the DM↔Claudine boundary lint passes.
- The win generalizes across `absolute()` / `relative()` / `load_markdown` via the shared
  `FileReferenceDiagnostic`.
- Every handleable error exposes the ratified `category`/`code`/`disposition`/`origin`/
  `detail` facets via `Diagnostic`, projected to `err.*`, so a handler can tap a pattern,
  target a code, or target an instance — with no string-message parsing.
- The legacy lifecycle fields `err.kind`, `err.variant`, and `err.msg` remain available
  during migration, with `err.kind` and `err.variant` treated as deprecated aliases for the
  new `err.category` and `err.code` fields. New documentation and examples must use the new
  faceted names.

### Resolved Decisions

#### Missing file references are fatal in lenient compose mode (RESOLVED)

The prior behavior treated an unknown expression function as fatal in lenient mode but turned
a present-but-unresolvable file reference into a warning. The new diagnostics make file
reference failures much clearer; whether to also change warn/fatal behavior was a product
decision, not a side effect of the typing refactor.

**Decision (ratified):** a file reference that is *present* but fails to resolve — malformed,
not-found, or found-elsewhere — is **fatal**, in both `fail_fast` and lenient body
interpolation, alongside the always-fatal unknown function. This generalizes the existing
whole-value frontmatter strictness rule (which already treats referenced files as required
executable input) to body interpolation: an evaluated reference that misses is almost always
a real authoring mistake, not a tolerated absence. Authors who genuinely want to tolerate an
absent file guard the reference with `file_exists`/a ternary so no resolution is attempted; a
reference that *is* evaluated and misses is surfaced rather than silently demoted to a warning
that leaves the literal `{{ … }}` in place. `RemoteNotEnabled` is excluded — it is a v1
capability gap governed by its own "remote not supported" policy, not a reference error.

This is the **one intentional behavior change** in this effort (a superset of the previously
recommended option 3, extended to body interpolation per ratified direction). It is
characterized and tested separately from the behavior-neutral typing refactor: the fatality
characterization matrix
(`darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs`) encodes
the fatal cells explicitly, and the gate itself lives in
`ExpressionError::is_authoring_fatal`
(`darkmatter/lib/src/markdown/compose/expression/error.rs`).

The options considered (preserve warn behavior; promote everywhere; promote only whole-value
frontmatter) are retained in the design history; the chosen direction makes present, evaluated
file references fatal wherever they occur.

### Non-goals

- The handler/dispatch mechanism and the rendezvous daemon. (The `until: <timestamp>`
  recovery dimension is *noted* in [`integrated-design.md`](./integrated-design.md) §13 and
  [`error-structure.md`](./error-structure.md) §11, but designed elsewhere.)
- The postcondition *checks* that raise the `document.*` / `vcs.*` expectation errors.
- Rewriting unrelated existing diagnostics that are already typed, readable, and outside the
  lossy Darkmatter/Claudine boundary. Those can adopt `Diagnostic` facets when touched, but
  they are not blockers for the reference failure.
