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

This error basically comes down to a file reference NOT being valid but it's so dense in it's explanation that a user has to really focus on what's going on to understand what's wrong.

What we need to do in these situations is:

- report the missing file as the primary error condition and then explain the underlying details after that:

```sh
 MarkdownError: invalid file path
┃
┃ The invalid file reference <orange>features/2026-06-21-opencode-log-fix/spec.md</a></orange> was assigned
┃ to the <inverse>iteration</inverse> Frontmatter property when using the
┃ <blue><a href>{prompt}</a></blue> prompt file.
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

Let's review some of the fundamental differences which this new error provides:

1. Identifies the REAL problem (e.g., an invalid file reference) and makes that the FOCUS of the error message
2. Shows the variables in the underlying schema definition which are relevant
    - rather than show no YAML -- as we did in the original error -- or show ALL the YAML -- as we do in many other cases -- we instead recognize the variables which are involved in the error and focus on those variables!
    - this shows the variables which are relevant not just a big dump of information
    - Note: we not only show the _lines_ which are relevant but we show the parent `$schema` line too so that user can see the "shape/structure" of that the problematic 
3. The prompt file is not just "mentioned" but a OSC8 hyperlink is provided so that the caller can easily get back to prompt file to understand the file and/or make changes to it
4. A missing file is almost always a typo on the callers part and we should help them identify the file "they meant" where ever possible. To do this we will need some string subset and similarity semantics to bring up a small list of suggestions that feel like the most likely intended files

## Context

There is no point in trying to solve this _specific_ problem! We must identify the pattern which problem represents as well as look for similar patterns which are also providing dense, hard to understand error messages. Once we're able to see the patterns of bad reporting we can start to apply strategic solutions.

Having clear error messages is absolutely essential for not only Claudine but also Darkmatter. These two libraries and CLI's provide a powerful toolset and new users WILL make mistakes so it's super important that we help callers to quickly and painlessly **understand** the mistake they've made so it can be fixed without the user having to scour through documentation or blindly trying different options until something works.

The importance of this task means that finding the "right solution" over the "expediant solution" is an absolute requirement. The scope of this improvement must address these issues in Darkmatter and in Claudine.

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

### Scope

- **Both libraries.** Darkmatter owns the diagnostic substrate (typed causes, the focused
  excerpt, file suggestions, the `Diagnostic` facet trait); Claudine *preserves and
  transports* typed errors via `#[from]`/`#[source]`, never flattening them to `String`.
- **Breaking changes are acceptable** where they remove lossy boundaries (no installed user
  base yet); prefer non-breaking where it costs nothing.
- We **extend** the existing `BlockError` / `StatusBlock` / `SourceContext` substrate — no
  `miette` or new diagnostic framework.

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

### Success criteria

(full list in [`integrated-design.md`](./integrated-design.md) §15)

- The reference failure renders with a root-cause headline ("invalid file path"), names the
  receiving frontmatter key, links the prompt file (OSC8 where capable), shows a *focused*
  excerpt (`$schema`/`spec`/`iteration`), and suggests likely files — **identically in
  `md compose` and `claudine compose`**.
- Fatal-vs-warn behavior is provably unchanged by the typing refactor (characterization
  matrix green).
- No new string-only lower-layer error variants; the DM↔Claudine boundary lint passes.
- The win generalizes across `absolute()` / `relative()` / `load_markdown` via the shared
  `FileReferenceDiagnostic`.
- Every handleable error exposes the ratified `category`/`code`/`disposition`/`origin`/
  `detail` facets via `Diagnostic`, projected to `err.*`, so a handler can tap a pattern,
  target a code, or target an instance — with no string-message parsing.

### Non-goals

- The handler/dispatch mechanism and the rendezvous daemon. (The `until: <timestamp>`
  recovery dimension is *noted* in [`integrated-design.md`](./integrated-design.md) §13 and
  [`error-structure.md`](./error-structure.md) §11, but designed elsewhere.)
- The postcondition *checks* that raise the `document.*` / `vcs.*` expectation errors.
