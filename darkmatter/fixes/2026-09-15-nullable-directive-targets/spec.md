---
status: draft
created: 2026-09-15
area: darkmatter
packages:
  - darkmatter
  - dmls
related:
  - ../../../claudine/fixes/2026-09-13-better-static-analysis/spec.md
  - ../2026-08-12-optional-params/spec.md
  - ../2026-08-02-silent-empty-ctx-values/spec.md
  - ../2026-09-13-unify-array-rendering/spec.md
  - ../../docs/topics/simplified-schemas.md
  - ../../dmls/docs/diagnostics.md
references:
  ../../../claudine/fixes/2026-09-13-better-static-analysis/spec.md: >-
    Sibling, not dependency. That fix and this one are the two halves of
    "make expression defects visible before anything runs", and they were
    found the same way — an authoring defect that survived every static
    layer and surfaced as a runtime abort. Its subject is a *nested span
    inside a string literal*; this one's subject is a *declared-nullable
    value reaching a position that rejects null*. They share no rule and
    neither blocks the other, but three things connect them and the
    implementation of either must not break the other. First, its
    Invariant 2 — one recognizer, one surface classifier, owned by
    Darkmatter and merely consumed by Claudine and DMLS — is the layering
    this spec follows for its own analysis (D4). Second, its D6 frontmatter
    sequence descent is what puts `initialize.stack[].action[]` on the
    editor's map, and it uses `prompts/_implement/implement-plan.md` — the
    document that produced this fix's incident — as its cost benchmark
    (32 to 70 entries); that same file carries two unrelated latent
    expression defects at lines 48 and 52 which only D6 can surface.
    Third, its D7 severity ladder ("a warning means it *might* be wrong; an
    error means it *will never* work") is the policy D5 below classifies
    its new diagnostic under, rather than inventing a second one. Read its
    "Why DMLS missed it" section before implementing D5: the reason the
    editor was blind there is structural, and partly the same structure.
  ../2026-08-12-optional-params/spec.md: >-
    The upstream cause. It establishes that a `$schema`-declared optional
    parameter the caller did not supply is a *binding* whose value is
    null — not an absent key and not a typo — and proposes materializing it
    as an explicit null in the effective frontmatter. This fix is the
    immediate downstream consequence: once "declared but unset is null" is
    a real and legitimate authoring state, every position that cannot
    accept null needs a defined behavior and a static warning. Its
    reproduction is the same document as this one's,
    `prompts/_implement/implement-plan.md`, for the same underlying reason.
    Neither fix blocks the other, but D4's nullability source is whatever
    that spec settles, so read its "Semantics of the materialized null"
    before implementing D4.
  ../2026-08-02-silent-empty-ctx-values/spec.md: >-
    Adjacent failure mode, deliberately not merged into this one. It covers
    a `ctx.*` fact the gatherer never collected rendering as empty with no
    report. This fix covers a *declared* value that is legitimately null.
    Both are instances of "empty is indistinguishable from broken", and
    both were verified to render identically today, but their causes and
    their fixes are different and each deserves its own revert boundary.
  ../2026-09-13-unify-array-rendering/spec.md: >-
    Touches the same `ContextValueType` descriptor that D4 extends, for a
    different reason. No ordering dependency; noted so whoever implements
    second does not treat the other's field addition as a conflict.
---

# A Guarded Directive Must Not Abort, and a Nullable Target Must Warn

## Summary

`::file {{log}}` aborts compose when `log` is null, **even when it sits
inside a `::block` whose condition is false**. The abort is a directive
*parse* error, raised before block gating has run, and it reports itself at
the wrong line.

Three defects are stacked here and each is independently worth fixing:

1. **Block conditions do not gate directive parsing.** A directive inside a
   false `::block` is parsed anyway, so a guard that reads as protection
   provides none.
2. **A null interpolated target is a fatal parse error**, reported in the
   vocabulary of the directive grammar (`Expected value, found end of
   directive`) rather than of the author's actual mistake.
3. **The error's excerpt points at the wrong lines** — off by exactly the
   frontmatter length, because the line number is body-relative and the
   rendered source context holds the whole file.

And one diagnostic gap, which is what the author should have seen first:

4. **DMLS cannot warn that the target may be null**, and today emits a
   *false* `dm.transclusion.broken_path` on the very construct instead.

## Reproduction

Verified on 2026-09-15 at `6f5b06251`, macOS.

```sh
md compose prompts/_implement/implement-plan.md \
   spec='fixes/2026-09-14-cicd-improvements/spec.md'
```

````text
TransclusionError: directive parse failed

Directive parsing failed here:

```md
  43 │         - when: "frontmatter(log, 'message_to_agent')"
> 44 │           action:
  45 │               - stderr: |-
```

Gutter: Column 7 is near the error.

Error: Expected value, found end of directive
Check syntax: ::file path="..."
````

The excerpt is frontmatter. The real offending line is
`prompts/_implement/implement-plan.md:140`:

```markdown
::block when="file_exists(log)"
…
::block when="phase > 1"
…
::file {{log}}
::end-block
::end-block
```

`log` is declared `log: file` — optional, no default, no computed
fallback — and nothing binds it, so `{{log}}` interpolates to empty and the
line becomes a bare `::file` with no target.

Minimal, reproduced independently:

```markdown
---
title: probe
---

::block when="file_exists(log)"
::file {{log}}
::end-block
```

Composing this fails identically. **The false `::block` does not suppress
it.** That is the primary finding and it is not inferable from the
authoring surface.

## Why This Matters Now

`::file {{var}}` is the only way to transclude a document chosen at compose
time, and guarding it with `::block when="file_exists(var)"` is the obvious
and correct way to write it. Every such site in the repository is a latent
abort. The pattern is not exotic: it is how a prompt conditionally includes
a log, a plan, or a prior phase's output.

The document that produced this incident had just gained its `log`
parameter (`4b6e8944f`, "feat(prompts): add log-based phasing, phase
messaging, and human-in-the-loop hooks"). Adding a *correctly guarded*
optional transclusion broke the prompt for every caller who did not pass
that optional parameter. Declaring the missing `log:` computed property
fixes that document; it does not touch the class.

## Background

### Where the abort comes from

`parse_directives` (`darkmatter/lib/src/markdown/compose/transclusion/parser.rs:31`)
walks the **already-interpolated** body line by line. A line is a candidate
when `is_block_directive_line` (same file, line 15) sees `::file`, `::code`,
or `::url` at its start; code regions are excluded, nothing else is. The
line is then handed to `parse_directive_line`, which reads the kind and
calls `Cursor::read_value` for the target. With the target interpolated
away, the cursor is at end of input, and
`darkmatter/lib/src/markdown/compose/parse_utils.rs:190` returns
`Expected value, found end of directive`.

The call site is `pipeline/phases.rs:194`, inside
`run_transclusion_phase`, and it uses `?` — the first bad directive aborts
the whole compose. `::block` conditions are evaluated downstream of this,
in the transclusion engine, on the directive set that parsing produced.
Parsing therefore cannot be gated by them as the code stands.

### Why the excerpt is wrong

`parse_directives` numbers lines from 1 over `Markdown::content`, which is
the body only — frontmatter is a separate field
(`darkmatter/lib/src/markdown/mod.rs:98-102`). The rendered excerpt comes
from `SourceContext::excerpt_prose`
(`biscuit-terminal/lib/src/errors/source_context.rs:127`), a plain 1-based
index into whatever content that context holds, and by the time the error
is printed that content includes the frontmatter. The two halves of the
diagnostic are in different coordinate spaces.

Measured on three documents; the offset equals the frontmatter length every
time:

| Document | Frontmatter lines | Directive at file line | Reported line |
| --- | ---: | ---: | ---: |
| probe (3-line frontmatter) | 3 | 6 | 3 |
| probe (6-line frontmatter) | 6 | 9 | 3 |
| `implement-plan.md` | 96 | 140 | 44 |

The column is correct in every case (`::file` is six characters; column 7).
Only the line is wrong, which is the worst shape for this bug: the excerpt
looks authoritative and points at real, unrelated code.

### Why "declared but unset" is a legitimate state

`log: file` in `$schema` declares an optional parameter. Per
[`optional-params`](../2026-08-12-optional-params/spec.md), the absence of
a caller-supplied value is a meaningful signal, not a typo, and should
materialize as an explicit null binding. This fix takes that as given: the
author is entitled to write `::file {{log}}`, and the language owes them a
defined behavior and a warning, not an abort.

Verified today: a declared-optional-unset root and a wholly undeclared root
both interpolate to empty in a document body, with no diagnostic from
either. Body interpolation is permissive; only subtree compose runs the
strict-root check.

## The Defect

The guard reads as protection and is not. An author writing

```markdown
::block when="file_exists(log)"
::file {{log}}
::end-block
```

has expressed the correct intent in the language the language provides, and
the document aborts anyway. Nothing in the error text mentions `log`,
`::block`, or nullability, and the excerpt points 96 lines away.

## Design

### D1. Block conditions gate directive resolution

A `::file` / `::code` / `::url` directive inside a `::block` whose condition
is false must not be resolved, and must not be able to abort the compose.

Two implementations are available and the plan must choose one and say why:

- **Gate before parse.** Evaluate block conditions first and exclude the
  enclosed spans from the directive scan. Most faithful to author intent,
  and it makes a *syntactically* malformed directive inside a dead block
  harmless too. It is the larger change: block evaluation currently lives
  downstream in the transclusion engine, and moving it ahead of parsing
  reorders the phase.
- **Gate before resolve.** Keep the scan where it is, but demote a parse
  failure to a deferred error carried on the directive, raised only if the
  directive survives block gating. Smaller and lower-risk; it keeps a dead
  block's syntax errors invisible, which is a behavior change in the same
  direction.

The second is the recommended starting point on risk grounds, and D2 makes
it sufficient for the incident. The determination is required because the
two differ in what happens to a genuinely malformed directive inside a dead
block, and that difference must be a decision rather than a side effect.

### D2. A null or empty interpolated target is a typed, non-fatal outcome

Independent of D1, `::file` with a target that interpolated to empty stops
being a grammar error.

- The directive is **skipped** — no transclusion, no output, no abort.
- A `ComposeWarning` is emitted naming the directive kind, the source span,
  and, when the target text was a single whole-value span, the variable
  that resolved to null.
- The `TransclusionError::ParseDirective` path stays for targets that are
  genuinely malformed as *authored*. An empty target produced by
  interpolation is no longer routed to it.

A directive whose authored target is literally empty (`::file` with nothing
after it) remains an error. The distinction is whether the emptiness came
from the author or from interpolation, which requires the pre-interpolation
text to be available at the scan. The plan must say how it reaches that —
carrying the raw span alongside the interpolated line, or scanning before
interpolation — because a parallel second walk is the shape that drifts.

Skipping rather than erroring is the correct default even without D1: a
null optional is a legitimate state, and the alternative is that every
`::file {{var}}` must be defensively guarded by a mechanism that, per D1,
does not currently work.

### D3. The parse error reports file-relative lines

`TransclusionError::ParseDirective` and every sibling error that carries a
body-relative line into a full-file `SourceContext` must agree on one
coordinate space. The fix is to report file-relative lines, because that is
what the excerpt renderer, the editor, and the author's `:140` jump all
speak.

`SourceContext` already distinguishes the two: `SourceContext::new` detects
the frontmatter range, and `Markdown::full_source_context_for_errors`
(`darkmatter/lib/src/markdown/mod.rs:214`) documents file-relative lines as
"the coordinate space shell-expansion errors report into". Transclusion
should join it rather than invent a third convention.

This is not confined to `::file`. Every error constructed inside
`run_transclusion_phase` from a `&self.content` line number has the same
drift and must be audited in one pass. An assertion that a reported line
resolves to a line actually containing the offending construct belongs in
the test suite, so this cannot regress silently.

### D4. Darkmatter owns nullability analysis

Add, alongside the expression lint the
[nested-span fix](../../../claudine/fixes/2026-09-13-better-static-analysis/spec.md)
introduces and following the same layering rule — Darkmatter owns the
analysis; Claudine and DMLS consume it:

```rust
/// Whether a whole-value span's declared type admits null on this
/// document: the root is `$schema`-declared without `required` and
/// without `default(…)`, or is a declared-nullable `ctx.*` value.
pub fn span_admits_null(source: &str, schema: &SimplifiedSchema) -> Option<NullableSpan>;

/// The roots an enclosing `::block when=` condition proves non-null for
/// the region it encloses.
pub fn narrowed_roots(condition: &str) -> BTreeSet<String>;
```

**Nullability is not projected today.** `ContextValueType`
(`darkmatter/lib/src/markdown/compose/context/catalog.rs:34`) carries
`base`, `is_array`, and `integer` — there is no optional or nullable bit.
Either that descriptor gains one, or the analysis reads `required` and
`default(…)` from the `SimplifiedSchema` directly. The plan must choose;
the descriptor is the better home if the
[array-rendering fix](../2026-09-13-unify-array-rendering/spec.md) is
touching it anyway.

**Narrowing is the load-bearing half.** Without it the rule fires on
`implement-plan.md:140` — which is *correctly written code* — and a
diagnostic that is wrong on the one site in the repository that handles the
case properly will be turned off. `::block when="…"` is a narrowing guard
in the ordinary control-flow sense: inside it, `log` is `file`, not
`file | null`.

The recognized narrowing forms are a small closed set, and the set is
deliberately small because a wrong *widening* is a false positive while a
missed narrowing is only a missed warning:

| Condition | Narrows |
| --- | --- |
| `file_exists(x)` | `x` |
| `x` | `x` |
| `!!x` | `x` |
| `x != null` / `x != ''` | `x` |
| `a && b` | union of each side's narrowing |
| anything else | nothing |

`||` and `!` narrow nothing. Nested blocks compose: the innermost region
carries the union of every enclosing block's narrowed set.

### D5. DMLS stops the false positive and warns for the right reason

**The false positive first.** `transclusion_diagnostics`
(`darkmatter/dmls/src/providers/dsl.rs:611`) takes `directive.target`,
splits on `#`, and calls `resolve_local_path` with no interpolation guard.
For `::file {{log}}` it therefore resolves `{{log}}` as a literal filename,
finds nothing, and emits

```text
dm.transclusion.broken_path  WARNING
broken transclusion: no file matches `{{log}}`
```

— a warning that sends the author looking for a missing file. No test in
`darkmatter/dmls/` covers a directive target containing an interpolation
span. *(Established by reading `dsl.rs:611-636`; DMLS is a pure LSP server
with no CLI diagnostics dump, so this was not confirmed by execution. An
executed confirmation is an acceptance criterion, not an assumption.)*

A target containing a `{{ … }}` span is skipped by `broken_path`
unconditionally. Path existence is not knowable for an interpolated target
and must not be guessed.

**Then the real diagnostic.** Add
`code::TRANSCLUSION_NULLABLE_TARGET = "dm.transclusion.nullable_target"`,
source `darkmatter.compose`, on a directive whose target is a whole-value
span that `span_admits_null` reports and that no enclosing `::block`
narrows.

Severity **`WARNING`** under the
[nested-span fix's D7 ladder](../../../claudine/fixes/2026-09-13-better-static-analysis/spec.md):
the ladder reads "a warning means the construct *might* be wrong; an error
means it *will never* work", and this one might be right — the caller may
supply the value. Classifying it here rather than inventing a second policy
is deliberate; if that fix has not landed, this diagnostic still lands at
`WARNING` and the ladder ratifies it retroactively.

Message:

```text
`log` may be null here; a null `::file` target transcludes nothing.
Guard with `::block when="file_exists(log)"` or declare a default.
```

**DMLS has no type awareness of expression results today** — zero
references to `ContextValueType` anywhere under `darkmatter/dmls/`. It has
`dm.schema.type_mismatch` for frontmatter *values* against the schema, but
nothing that types the *result* of an expression. D4's exports are what
close that, and D5 is their first consumer. This is the same
Darkmatter-owns-the-rule shape the nested-span fix's Invariant 2
establishes, for the same reason: two implementations of "is this nullable"
would desync.

### D6. Documentation

- `darkmatter/docs/inline/interpolation.md`: a null whole-value span in a
  directive target skips the directive, and what `::block` narrowing means.
- `darkmatter/dmls/docs/diagnostics.md`: the new code, its severity under
  the ladder, and the `broken_path` skip for interpolated targets.
- `darkmatter/docs/topics/simplified-schemas.md`: that an optional
  parameter without a default is nullable at every use site, cross-linking
  [`optional-params`](../2026-08-12-optional-params/spec.md).
- `.claude/skills/darkmatter/`: the guard idiom, one paragraph. Hash via
  `md hash`.
- `darkmatter/README.md` only if it enumerates diagnostic codes today.

## Relationship to the Nested-Span Fix

Stated here as well as in the frontmatter, because the two fixes will be
read together and the boundary matters.

They are **siblings, not dependencies**. Neither blocks the other and they
may land in either order. What they share:

- **Layering.** That fix's Invariant 2 — one recognizer and one surface
  classifier, owned by Darkmatter, consumed by Claudine and DMLS — is the
  rule D4 and D5 follow. Its D1 lint module is the natural neighbor for
  D4's exports.
- **A document.** Its D6 uses `prompts/_implement/implement-plan.md` as the
  frontmatter-descent cost benchmark. That is this fix's incident document,
  and it carries two further latent defects — unbalanced parentheses in
  `initialize.stack[].action[].stderr` at lines 48 and 52 — that **only**
  D6 can surface, because `lower_mapping`
  (`darkmatter/dmls/src/overlay/frontmatter.rs:413`) recurses into
  `Node::Mapping` only and nothing under a `- item` line has a frontmatter
  entry at all. Correcting those two lines belongs to that fix's live-
  instance work, not to this one.
- **A severity policy.** D5 classifies under its D7 ladder rather than
  writing a second one.

What they do **not** share: no rule, no diagnostic code, no error variant.
Its subject is a nested span inside a string literal on a non-rescanning
surface. This one's is a declared-nullable value reaching a position that
rejects null. An implementer of either should not need to read the other's
design to finish.

## Scope

In scope:

- D1 through D6.
- The audit of every body-relative line number constructed in
  `run_transclusion_phase` (D3), not only the `::file` path.
- `prompts/_implement/implement-plan.md`'s missing `log:` computed
  property, as a regression fixture rather than only as a repair. Ken added
  the declaration on 2026-09-15; the fixture is captured from the state
  that reproduced, not from the working tree.

Out of scope:

- The two unbalanced-parenthesis defects at
  `prompts/_implement/implement-plan.md:48` and `:52`, and the undefined
  `message_to_agent` root at `:54`. They belong to the nested-span fix's
  frontmatter descent.
- Making body interpolation of an undeclared root an error. That is
  [`silent-empty-ctx-values`](../2026-08-02-silent-empty-ctx-values/spec.md)
  and it is a different decision with a different blast radius.
- Materializing optional parameters as explicit nulls.
  [`optional-params`](../2026-08-12-optional-params/spec.md) owns that; D4
  consumes whatever it settles.
- Nullability analysis for anything other than a whole-value span in a
  directive target. A mixed target (`::file "{{dir}}/log.md"`) is not
  covered: it is not a whole-value span, and the useful rule there is path
  shape, not nullability.
- An `md lint` / `md validate` CLI surface. Ruled out by the nested-span
  fix and not reopened here.

## Testing

Every new test must fail on the commit before its fix, and the verification
record must say how that was shown.

### Darkmatter L1 (`darkmatter/lib`)

- A `::file {{x}}` inside `::block when="file_exists(x)"` with `x` null
  composes successfully and transcludes nothing (D1, D2). **This is the
  incident and the primary regression test.**
- The same directive with no enclosing block composes successfully, emits
  one `ComposeWarning` naming `x`, and does not abort (D2).
- An authored-empty `::file` with no interpolation still errors (D2
  boundary).
- A syntactically malformed directive inside a *false* block behaves as the
  D1 determination says it should, asserted explicitly either way.
- `narrowed_roots` over each row of D4's table, including `a && b` union,
  and asserting that `||` and `!` narrow nothing.
- `span_admits_null` is true for `log: file`, false for
  `plan: file(required)`, false for `x: string(default('a'))`.
- **Line-number correctness**: for a document with an *n*-line frontmatter
  and a malformed directive at file line *m*, the reported line is *m*.
  Parameterized over at least three frontmatter lengths, because a single
  case passes under both the buggy and the correct implementation when
  *n* = 0 (D3).

### DMLS L1 (`darkmatter/dmls`)

- `::file {{log}}` produces **no** `dm.transclusion.broken_path`. Executed,
  closing the read-only gap noted in D5.
- `::file {{log}}` with `log: file` unguarded produces exactly one
  `dm.transclusion.nullable_target` at `WARNING`, ranged on the span.
- The same inside `::block when="file_exists(log)"` produces **none**. This
  is the false-positive guard and the reason the rule is usable.
- The same inside a nested block whose outer condition narrows and whose
  inner does not — still none.
- `::file "real.md"` next to a missing file still produces
  `broken_path`, so the skip did not disable the existing rule.
- `log: file(required)` produces no `nullable_target`.

### Evidence

- The minimal probe from Reproduction, composed before and after, captured
  verbatim.
- `prompts/_implement/implement-plan.md` at the reproducing state, composed
  with only `spec=`, before and after.
- An executed DMLS diagnostic listing for a document containing
  `::file {{log}}`, before and after, since the false positive is currently
  established by reading rather than by running.

## Acceptance Criteria

1. Composing the Reproduction command succeeds, with `log` unset, and
   transcludes nothing where `::file {{log}}` sits.
2. A malformed directive's reported line resolves to a line in the file
   that actually contains it, for at least three distinct frontmatter
   lengths.
3. `::file {{log}}` produces no `broken_path` in DMLS, and this is shown by
   an executed diagnostic run.
4. `::file {{log}}` produces one `nullable_target` warning unguarded and
   none inside `::block when="file_exists(log)"`.
5. The D1 determination is recorded in the plan with its reasoning, and the
   behavior of a malformed directive inside a dead block is asserted by a
   test either way.
6. No existing transclusion or DMLS test changes behavior, except any whose
   expectations encode the body-relative line numbers D3 corrects — each of
   those re-cut deliberately and recorded as such.

## Sequencing

D2 and D3 are independent of everything else and unblock the incident on
their own. D1 depends on the raw-span determination in D2. D4 depends on
whatever [`optional-params`](../2026-08-12-optional-params/spec.md) settles
for the nullability source, and D5 depends on D4 — but D5's first half, the
`broken_path` skip, depends on nothing and should not wait behind it.

A defensible first commit is D2 + D3 + the `broken_path` skip: it ends the
abort, fixes the misleading excerpt, and removes a false warning, with no
new analysis and no new diagnostic code.
