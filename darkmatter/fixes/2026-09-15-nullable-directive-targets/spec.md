---
status: draft
created: 2026-09-15
area: darkmatter
reviewed: true
reviewed_by: codex/gpt-5.6-sol
reviewed_on: 2026-09-15
packages:
  - darkmatter
  - darkmatter-cli
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
    null — not an absent key and not a typo — and materializes it
    as an explicit null in the effective frontmatter. This fix is the
    immediate downstream consequence: once "declared but unset is null" is
    a real and legitimate authoring state, every position that cannot
    accept null needs a defined behavior and a static warning. Its
    reproduction is the same document as this one's,
    `prompts/_implement/implement-plan.md`, for the same underlying reason.
    Neither fix blocks the other. D4 reuses that fix's settled document-
    ownership boundary; read its "Semantics of the materialized null" before
    implementing D4.
  ../2026-08-02-silent-empty-ctx-values/spec.md: >-
    Adjacent failure mode, deliberately not merged into this one. It covers
    a `ctx.*` fact the gatherer never collected rendering as empty with no
    report. This fix covers a *declared* value that is legitimately null.
    Both are instances of "empty is indistinguishable from broken", and
    both were verified to render identically today, but their causes and
    their fixes are different and each deserves its own revert boundary.
  ../2026-09-13-unify-array-rendering/spec.md: >-
    Touches the same context descriptor area that D4 reads, for a different
    reason. No ordering dependency: this fix reuses the existing `required`
    and `default` projections and adds no `ContextValueType` field.
---

# Condition-Blind Preflight Must Tolerate Nullable Directive Targets

## Summary

`::file {{log}}` can abort `md compose` when `log` is null, **even when it
sits inside a `::block` whose condition is false**. The ordinary compose
pipeline is not the culprit: it evaluates page blocks before body
interpolation and transclusion parsing, so a false region never reaches the
runtime transclusion parser. The abort comes from the deliberately
condition-blind shell-approval preflight, which retains false regions so it
can discover commands in every branch, interpolates the body, and then tries
to parse the now-empty directive target.

> **Reader note — review correction:** The original draft proposed adding
> block gating to transclusion parsing. That would duplicate behavior already
> present in terminal composition and would violate preflight's security
> contract: approval discovery must remain condition-blind. This revision
> preserves both contracts and makes a resolved-null target a typed, skippable
> result in the condition-blind analysis instead.

Three defects are stacked here and each is independently worth fixing:

1. **Preflight loses the distinction between an authored-empty target and a
   target whose whole-value interpolation resolved to null.** Its
   condition-blind walk must inspect the branch, but it must not reinterpret a
   legitimate null binding as malformed syntax.
2. **A null interpolated target becomes a fatal parse error**, reported in the
   vocabulary of the directive grammar (`Expected value, found end of
   directive`) rather than of the author's actual mistake.
3. **The error's excerpt points at the wrong lines** — off by exactly the
   frontmatter length, because the line number is body-relative and the
   rendered source context holds the whole file.

And one diagnostic gap, which is what the author should have seen first:

4. **DMLS cannot warn that the target may be null**, and today emits a
   *false* `dm.transclusion.broken_path` on the very construct instead.

## Reproduction

Observed on macOS on 2026-09-15. The original draft attributed the capture to
`6f5b06251`, but that commit predates the offending `::file {{log}}` line; the
line first appears in `63f9052e7`, where `log` also has a computed value. The
captured failure therefore came from an intermediate working-tree state and
cannot honestly be called a clean-commit reproduction. Implementation must
preserve the minimal fixture below as the authoritative regression case and
record the exact revision plus diff used for any live-prompt evidence.

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

In the captured intermediate state, `log` was declared `log: file` — optional,
no default, no computed fallback — and nothing bound it, so `{{log}}`
interpolated to empty and the line became a bare `::file` with no target. The
repository no longer contains that exact state; the minimal fixture below
recreates the relevant contract without depending on prompt history.

Minimal, reproduced independently:

```markdown
---
$schema:
  log: file
---

::block when="file_exists(log)"
::file {{log}}
::end-block
```

The normal runtime page-block stage suppresses this directive. The
condition-blind preflight walk does not, and currently aborts before the
runtime stage gets that opportunity.

## Why This Matters Now

`::file {{var}}` is the only way to transclude a document chosen at compose
time, and guarding it with `::block when="file_exists(var)"` is the obvious
and correct way to write it. Every such site can become a preflight abort when
shell approval is enabled. The pattern is not exotic: it is how a prompt
conditionally includes a log, a plan, or a prior phase's output.

The prompt gained the optional `log` parameter in `4b6e8944f`; the guarded
transclusion first appears in `63f9052e7`. Because the recorded failure came
from an intermediate state, this spec does not claim that either clean commit
reproduces it. Binding a computed `log` repairs that prompt instance, but does
not fix the condition-blind preflight class.

## Background

### Where the abort comes from

`parse_directives` (`darkmatter/lib/src/markdown/compose/transclusion/parser.rs`)
walks an already-interpolated body line by line. A line is a candidate when
`is_block_directive_line` sees `::file`, `::code`, or `::url` at its start;
code regions are excluded. The line is then handed to
`parse_directive_line`, which calls `Cursor::read_value` for the target. With
the target interpolated away, the cursor is at end of input and returns
`Expected value, found end of directive`.

There are two materially different callers:

- `run_transclusion_phase` parses content only after `PageBlocks` and
  `Interpolation` have run. A false page block has already been removed here.
- `compose/preflight/collect.rs` intentionally prepares interpolation and text
  replacement **without** page blocks, because shell approval must discover
  commands in every branch. Its recursive transclusion scan then calls
  `parse_directives(...)?`; this is the failing call site.

The fix must not make preflight condition-aware. A condition can depend on
state that changes before execution, and omitting a reachable child's shell
commands from the approval set would weaken the invariant that no command can
execute unless it was approved from the preflight graph.

### Why the excerpt is wrong

`parse_directives` numbers lines from 1 over `Markdown::content`, which is the
body only. The failing preflight call supplies
`full_source_context_for_errors()`, whose excerpt includes frontmatter, but
does not add `frontmatter_line_count()` to the directive line. The same module
already passes this offset to its shell-directive parser; the recursive
transclusion parser does not. The two halves of that diagnostic are therefore
in different coordinate spaces. Ordinary `run_transclusion_phase` instead
uses the body-only `source_context_for_errors()` and is internally consistent.

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

The guard is valid runtime protection. An author writing

```markdown
::block when="file_exists(log)"
::file {{log}}
::end-block
```

has expressed the correct intent in the language the language provides. The
document aborts only because the earlier, condition-blind security analysis
cannot represent a null target without first turning it into invalid directive
text. Nothing in the error mentions `log`, preflight, or nullability, and the
excerpt points 96 lines away.

## Design

### D1. Preserve runtime gating and condition-blind preflight

The two walks have different, intentional contracts:

- **Terminal composition is condition-aware.** `PageBlocks` remains before
  `Interpolation` and transclusion. A false block contributes no output, no
  transclusion, and no runtime diagnostic from its body.
- **Shell-approval preflight is condition-blind.** It continues to scan every
  branch and recursively inspect every concrete transclusion target so a child
  command cannot evade approval merely because the branch is false in the
  preflight snapshot.

Do not move page-block evaluation into preflight and do not add a second block
evaluator to the transclusion parser. The incident is fixed by representing
target evaluation separately from directive grammar (D2), not by weakening or
duplicating either pipeline's gating semantics.

A syntactically malformed authored directive in a false block therefore has
two deliberate outcomes: terminal composition ignores it because the whole
region is removed, while condition-blind preflight reports it because the
branch remains part of the approval surface. A resolved-null target is not
such a syntax error.

### D2. A null or empty interpolated target is a typed, non-fatal outcome

For `::file`, `::code`, and `::url`, a whole-value target expression that
evaluates to JSON null or the empty string stops being a grammar error. Both
are established absence sentinels for an optional `file` value; preserving the
typed reason still matters for diagnostics and future policy.

- In terminal composition the directive is **skipped** — no transclusion, no
  output, no abort — and one typed `ComposeWarning` names the directive kind,
  source location, and expression root when available.
- In condition-blind preflight it contributes no graph edge and discovers no
  child commands. This is safe only for a value already resolved to null in
  the preflight state; a target that depends on a still-pending frontmatter
  shell value is rejected before approval as a dynamic command shape, using
  the existing fail-closed policy for body commands whose approved and
  executed shapes could differ.
- The `TransclusionError::ParseDirective` path stays for targets that are
  genuinely malformed as *authored*. An empty target produced by
  interpolation is no longer routed to it.

A directive whose authored target is literally empty (`::file` with nothing
after it) remains an error. The distinction is whether the emptiness came
from the author or from evaluation. Parse the authored directive once through
the shared `directives_api`, retain its target span, and evaluate that target
through Darkmatter's existing interpolation parser/evaluator. Do not infer
provenance by comparing an interpolated line with a second text scan.

An authored quoted empty string (`::file ""`) is also authored-empty and
remains an error. A mixed target such as `::file "{{dir}}/log.md"` is a string
target, not a nullable whole value; its normal path-resolution behavior is
unchanged.

The body-interpolation stage must not erase this type information before the
decision is made. For the current pipeline, the simplest design is a shared
directive-target rewrite at the start of body interpolation: parse the current
body with `directives_api`, evaluate whole-value target spans to typed values,
replace concrete results, and remove null directive lines while recording the
warning. Then run the ordinary interpolation rewrite over the remaining text.
Terminal composition invokes it after page blocks; preflight invokes it on the
condition-blind body. A single span-aware rewrite avoids retaining offsets
across edits elsewhere in the body. Pending-shell dependency detection runs on
the authored target before this rewrite.

Skipping rather than erroring is the correct runtime default because a null
optional is a legitimate state. A guard remains recommended because it both
documents intent and suppresses the runtime warning.

### D3. The parse error reports file-relative lines

`TransclusionError::ParseDirective` and every sibling error that carries a
body-relative line into a full-file `SourceContext` must agree on one
coordinate space. The fix is to report file-relative lines, because that is
what the excerpt renderer, the editor, and the author's `:140` jump all
speak.

`SourceContext` already distinguishes the two: `source_context_for_errors()`
contains the body and accepts body-relative lines;
`full_source_context_for_errors()` contains the reconstructed file and accepts
file-relative lines. A caller using the latter must add
`frontmatter_line_count()` exactly once.

Audit every transclusion-parser call site, especially both calls in
`compose/preflight/collect.rs`, for the context/line convention. Do not change
the already-consistent body-only call in `run_transclusion_phase` merely to
make all callers look alike. An assertion that a reported line resolves to a
line actually containing the offending construct belongs in the test suite.

### D4. Darkmatter owns nullability analysis

Add the analysis alongside the expression lint introduced by the
[nested-span fix](../../../claudine/fixes/2026-09-13-better-static-analysis/spec.md).
Darkmatter owns both the effectful target evaluation used by composition and
the passive nullability classification used by DMLS; consumers must not
reimplement expression recognition or schema semantics.

```rust
pub enum EvaluatedDirectiveTarget {
    Concrete(String),
    Absent {
        reason: AbsentTargetReason,
        root: Option<ExpressionPath>,
    },
}

pub enum AbsentTargetReason {
    Null,
    EmptyString,
}

pub enum TargetNullability {
    NonNullable,
    Nullable { root: ExpressionPath },
    Unknown,
}
```

The names are illustrative; the contract is not. Runtime evaluation receives
the parsed target span and the request's existing `EvaluationLookup`, so it
preserves typed null/empty-string results instead of first stringifying them.
Passive analysis
receives the parsed whole-value expression, the document's assembled
`EffectiveSchema`, and static frontmatter. It returns `Unknown` rather than
guessing when the schema is unavailable, a union cannot be resolved, or the
expression is more complex than a supported property path.

For an unset document parameter, nullability comes from
`EffectiveSchema.simplified` plus `origins`: a top-level, document-owned or
referenced-file SimplifiedSchema property without `required`. This is close to,
but intentionally not identical to, optional-binding materialization.
`default(...)` is JSON Schema metadata and Darkmatter does not apply it, so it
does **not** prove a target non-null. Raw JSON Schema, baseline-only properties,
trigger payloads, root unions, and nested properties remain outside this rule.
A concrete non-null frontmatter scalar at the path suppresses the warning; an
explicit null does not.

For `ctx.*`, reuse `context_variable_descriptors()`. Its
`ContextVariableDescriptor` already exposes `required` and `default`; do not
add a duplicate nullable field to `ContextValueType`. This is also why the
[array-rendering fix](../2026-09-13-unify-array-rendering/spec.md) has no data
model dependency on this work.

**Narrowing is the load-bearing half.** Without it the rule fires on the
minimal guarded fixture — correctly written code — and a diagnostic that is
wrong on the idiomatic solution will be turned off. `::block when="…"` is a
narrowing guard in the ordinary control-flow sense: inside it, `log` is
`file`, not `file | null`.

Walk Darkmatter's parsed condition AST and return narrowed property paths; do
not implement a second string recognizer. The recognized forms are a small
closed set. A false narrowing would hide a real warning, while an unrecognized
valid guard produces a false positive, so every added form requires truth-table
tests against the expression evaluator:

| Condition | Narrows |
| --- | --- |
| `file_exists(x)` | `x` |
| `x` | `x` |
| `!!x` | `x` |
| `x != null` / `x != ''` | `x` when that comparison evaluates true |
| `a && b` | union of each side's narrowing |
| anything else | nothing |

Parentheses preserve the inner result. `||` and a single `!` narrow nothing.
Nested blocks compose: the innermost region carries the union of every
enclosing block's narrowed set. Paths, not only bare root strings, are retained
so `doc.log` and `ctx.some_optional_value` can be compared consistently; v1 may
emit only for top-level document parameters and cataloged `ctx.*` paths.

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
span that D4 classifies as nullable and that no enclosing `::block` narrows.
Range the diagnostic on the target expression, not the whole directive.

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
Guard with `::block when="file_exists(log)"`, bind a non-null value, or make
the parameter required.
```

**DMLS has no nullability classification for expression results today.** It
already assembles the document's effective schema and Darkmatter already
projects `ctx.*` descriptors; D4 joins those authorities without introducing
a second schema resolver in the DSL provider. D5 is the first consumer. This is the same
Darkmatter-owns-the-rule shape the nested-span fix's Invariant 2
establishes, for the same reason: two implementations of "is this nullable"
would desync.

### D6. Documentation

- `darkmatter/docs/inline/interpolation.md`: a null whole-value span in a
  directive target skips the directive, and a false `::block` suppresses the
  runtime warning.
- `darkmatter/docs/topics/darkmatter-expressions.md`: `default(...)` is schema
  metadata, not an applied value, so it does not make an optional expression
  non-null.
- `darkmatter/dmls/docs/diagnostics.md`: the new code, its severity under
  the ladder, and the `broken_path` skip for interpolated targets.
- `darkmatter/docs/topics/simplified-schemas.md`: that an optional parameter is
  nullable at an unbound use site even when it carries default metadata,
  cross-linking [`optional-params`](../2026-08-12-optional-params/spec.md).
- `.claude/skills/darkmatter/compose.md`: the runtime-versus-preflight contract
  and guard idiom. Hash via `md hash`.
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
- The audit of every transclusion-parser call that combines a body-relative
  line with a full-file `SourceContext` (D3).
- Condition-blind preflight recursion, including its pending-shell-value
  fail-closed boundary.

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
  consumes its settled document-ownership semantics but also handles an
  optional property carrying unapplied `default(...)` metadata.
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
  composes successfully and transcludes nothing through ordinary terminal
  composition, with no nullable-target warning (D1, D2).
- The same document completes condition-blind preflight without a parse error,
  contributes no child edge for the null target, and does not weaken scanning
  of concrete sibling branches. **This is the incident and the primary
  regression test.**
- The same directive with no enclosing block composes successfully, emits
  one `ComposeWarning` naming `x`, and does not abort (D2).
- A whole-value expression resolving to `""` has the same skip behavior and
  reports the empty-string reason rather than null.
- Authored-empty `::file` and `::file ""` both still error (D2 boundary).
- A syntactically malformed directive inside a false block is ignored by
  terminal composition but rejected by condition-blind preflight (D1).
- A transclusion target depending on a pending frontmatter shell value is
  rejected before approval; it is never omitted from preflight and then
  allowed to introduce unapproved child commands at execution time.
- The shared narrowing analysis covers each row of D4's table, including
  parentheses and `a && b` union,
  and asserting that `||` and `!` narrow nothing.
- Passive target classification is nullable for `log: file` and
  `x: string(default('a'))`, non-nullable for `plan: file(required)`, and
  suppressed by a concrete non-null frontmatter value.
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
- `log: file(default('fallback.md'))` still produces `nullable_target` when
  the document does not bind `log`, because schema defaults are not applied.

### Darkmatter CLI L1 (`darkmatter/cli`)

- Run the minimal guarded fixture through the normal `md compose` entry point
  with shell expansion enabled, proving that the real preflight lifecycle no
  longer aborts. Use `CliProcessFixture`; do not hand-build the process.
- Run a fixture whose pending target could reveal a child `::shell` command
  after preflight and assert the command is rejected before approval.

### Passive corpus

- Extend the shipped-artifact corpus so every checked-in directive target is
  parsed by the shared authored-directive API. This guards the parser change
  without performing composition, filesystem resolution, shell execution, or
  network access.

### Evidence

- The minimal probe from Reproduction, composed before and after, captured
  verbatim.
- If live-prompt evidence is retained, capture the exact commit and working
  diff that contain both the nullable `log` state and offending directive;
  do not attribute it to `6f5b06251`.
- An executed DMLS diagnostic listing for a document containing
  `::file {{log}}`, before and after, since the false positive is currently
  established by reading rather than by running.

## Acceptance Criteria

1. The minimal guarded fixture succeeds through the normal `md compose`
   command with shell preflight enabled, with `log` unset, and transcludes
   nothing where `::file {{log}}` sits.
2. A malformed directive's reported line resolves to a line in the file
   that actually contains it, for at least three distinct frontmatter
   lengths.
3. `::file {{log}}` produces no `broken_path` in DMLS, and this is shown by
   an executed diagnostic run.
4. `::file {{log}}` produces one `nullable_target` warning unguarded and
   none inside `::block when="file_exists(log)"`.
5. Terminal composition remains condition-aware, preflight remains
   condition-blind, and tests lock both outcomes for malformed syntax in a
   false block.
6. No existing transclusion or DMLS test changes behavior, except any whose
   expectations encode the body-relative line numbers D3 corrects — each of
   those re-cut deliberately and recorded as such.
7. A target that can change only after preflight cannot introduce an
   unapproved transcluded shell command; the pending-target case fails closed
   before approval.
8. `default(...)` is not presented or tested as a runtime fallback.
9. An evaluated empty string is skipped, while an authored empty or quoted-
   empty target remains a syntax error.

## Sequencing

D1 is an invariant, not a new gating feature. D2 and D3 unblock the incident.
D2 must include the pending-shell-value guard so the first commit is already
security-complete. D4 consumes the settled ownership rules from
[`optional-params`](../2026-08-12-optional-params/spec.md), and D5 depends on
D4 — but D5's `broken_path` skip depends on neither and need not wait.

A defensible first commit is D1–D3 plus the `broken_path` skip: it ends the
abort without weakening condition-blind approval, fixes the misleading
excerpt, and removes a false warning. D4–D6 can follow as the passive
authoring diagnostic.

## Open Questions

None. The review resolves the original gating choice in favor of preserving
the existing runtime/preflight split, and it resolves default handling by
following Darkmatter's established rule that schema defaults are metadata.
