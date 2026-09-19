---
$schema: feature-review.yaml
ready: false
human_review: false
reviewed_by: codex/default
created: 2026-09-16T19:39:15-07:00
spec: 2026-09-13-better-static-analysis/spec.md
log: claudine/fixes/2026-09-13-better-static-analysis/implementation-log.md
implemented: true
implemented_by: codex/default
next: 2026-09-13-better-static-analysis/review-2.md
description: A **fix** review of `2026-09-13-better-static-analysis/spec.md`
fix: 2026-09-13-better-static-analysis/review-1.md
findings:
    - "[high] Expression-literal decoding reverses escaped-opener parity after the hard lint"
    - "[high] Ambiguous proxy overlay paths can bypass prepare-time validation"
---

# Review 1: Make Expression Defects Visible Before Anything Runs

## Verdict

The fix is **not ready for production**. The main implementation is broad and
well tested at the correct boundary: Darkmatter owns the lint and escape
scanner, Claudine rejects ordinary defective lifecycle surfaces before launch,
and DMLS now reaches sequence-nested frontmatter with precise diagnostics and
typed quick fixes.

Two edge cases break the central contract. A valid escaped opener can be
rejected because the static lint counts backslashes before expression-literal
decoding, while a real defect in `proxy.with` can evade the same lint because
two distinct overlay paths collapse to one dotted string. Both need focused
Level 1 regressions and implementation fixes.

Human review is not required. The specification already decides both policies:
escape parity is evaluated after the owning syntax decodes its text, and
authored source must remain attached to the canonical lifecycle surface rather
than being recovered through an ambiguous parallel path walk.

## Findings

### 1. Expression-literal decoding reverses escaped-opener parity after the hard lint (high)

`flag_literal` scans the raw bytes between an expression string literal's
quotes (`darkmatter/lib/src/markdown/compose/expression/lint.rs:213-225`). That
is normally necessary for lossless rewrites, but it is not the representation
on which D8 defines escape parity. The expression lexer subsequently decodes
`\\` to `\`, so an authored literal containing two backslashes immediately
before `{{` has an even run during static linting and an odd run in the value
that reaches runtime interpolation.

For example, this valid lifecycle expression is rejected by the hard lint:

```yaml
success:
    say: |-
        {{ '\\{{ name }}' }}
```

The raw literal slice contains `\\{{ name }}`, so `find_all_plain` reports a
nested span. Evaluating the expression produces `\{{ name }}`, which D8 says is
an escaped opener and which the runtime scanner correctly ignores. The result
is an editor `ERROR` and prepare-time refusal for text that would safely retain
the CommonMark escape. This is not merely theoretical: quoting another template
language is the use case D8 was introduced to support.

The existing quiet case uses one raw backslash
(`lint.rs:579-590`), which stays odd before decoding and therefore cannot catch
the parity reversal. The implementation log records this edge but dismisses it
instead of reconciling it with D8.

Required change: classify opener escaping against the decoded literal value
while retaining a decoded-to-authored map for diagnostic ranges and rewrites,
or otherwise make the raw scan reproduce decoded parity. Add Level 1 tests at
all three consumers: the Darkmatter lint must stay quiet, Claudine preparation
must accept the lifecycle value, and DMLS must emit no nested-span diagnostic.
Include odd/even runs that change length during expression-literal decoding.

### 2. Ambiguous proxy overlay paths can bypass prepare-time validation (high)

`LifecycleSourceMap` independently walks raw lifecycle data and keys authored
values by display strings such as `success.stack[0].action[0].with.a.b`
(`claudine/lib/src/composition/lifecycle/source_map.rs:36-85,180-193`). The
canonical parsed-surface walk constructs the same unescaped strings
(`claudine/lib/src/composition/lifecycle/validate.rs:315-339`), and validation
then looks up source solely by that string (`validate.rs:28-50`).

`proxy.with` keys are arbitrary static YAML strings; `ProxyWith::new` rejects
interpolation in a key but permits dots and bracket text. Consequently these
distinct values have the same source-map identity:

```yaml
with:
    a:
        b: "{{ ok ? 'bad {{ x }}' : 'fine' }}"
    a.b: "fine"
```

Because `serde_json::Map` is sorted in this build, the later `a.b` entry
overwrites the defective nested value in the `HashMap`. Both parsed surfaces
then retrieve `"fine"`, so prepare-time validation returns success. Reversing
which value is defective can instead produce a false positive. Keys such as
`files[0]` collide with array paths in the same way.

This is the failure mode the specification's source-attachment requirement was
designed to prevent. The fail-closed missing-record check does not help because
the colliding record exists. On a later lifecycle event, the missed defect can
survive until event-time handling after the provider has already run, recreating
the incident class and breaking editor/CLI agreement.

Required change: remove the parallel string-keyed association and attach the
authored scalar to the canonical typed surface record, as specified, or key
both walks by an unambiguous typed path whose key segments cannot collide with
indices or separators. Add Level 1 coverage for dotted keys, bracket-shaped
keys, and nested mappings/arrays, with the defect on each side of the
collision. The CLI process case must assert that the provider marker is never
written.

## Requirement Verification Levels

| User-facing requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Direct, inline, dry-run, proxy, retry/resume, loop, and sequence documents reject nested spans before provider launch | Level 1 library and fake-provider process tests | Correct level. Ordinary paths are well covered, but colliding `proxy.with` paths are absent and can bypass rejection; finding 2. |
| Shipped review/commit prompts compose and complete lifecycle handling without surviving braces | Level 1 passive corpus plus fake-provider process tests | Correct level and green in the implementation evidence. |
| Synthesized action bodies, mixed strings, ordinary frontmatter, and rescanning body expressions keep working | Level 1 unit and process tests | Correct level and appropriately exercises positive and negative behavior. |
| DMLS reports the new error with style-correct ranges, severity, and typed quick fixes | Level 1 in-process LSP/provider tests | Correct level. The escaped value whose parity changes during expression-literal decoding is missing; finding 1. |
| Backslash escapes preserve bytes, distinguish odd/even runs, and suppress expression diagnostics | Level 1 scanner, compose, and LSP-session tests | Correct level, but incomplete across the expression-literal decode boundary; finding 1. |
| List-item diagnostics, hover, completion, navigation, and element-level schema ranges work without acting on synthetic indices | Level 1 in-process provider/LSP tests | Correct level and includes positive and negative capability cases. |
| Malformed/unknown/nested-span severities and lifecycle-only roots follow the new policy | Level 1 diagnostic tests and mapping-only corpus | Correct level and includes body/frontmatter and inside/outside-lifecycle controls. |
| Runtime surviving-span errors name the lifecycle property and select the typed hint | Level 1 executor and fake-provider process tests | Correct level; terminal-emulator rendering is not part of this textual error contract. |

Level 2 is not required because no requirement depends on glyph width, SGR,
scrolling, or behavior of a real terminal emulator. Level 3 is not required
because no requirement depends on terminal input encoding or OS keyboard/mouse
events.

## Verification Performed

- Read the complete specification and its implementation record, then traced
  the relevant lint, scalar projection, lifecycle source association, DMLS
  diagnostic, and process-test paths.
- Refreshed GitNexus for this worktree with `just gitnexus`. The graph reports
  the shared DMLS rewrite encoder as **HIGH** upstream impact across the
  frontmatter diagnostic path. Lifecycle validator caller resolution remained
  `UNKNOWN`, so its production and test call sites were confirmed by text
  search as required.
- The first Darkmatter L1 attempt failed before compilation because the shared
  target directory is read-only. It was rerun through the canonical area
  recipe with a fresh temporary `CARGO_TARGET_DIR`: **7,922 passed, 7 skipped,
  0 failed**.

## Design Assessment

The shared lint API, typed DMLS code-action payload, sequence-aware frontmatter
arena, and per-pass schema-shape memo are sound choices. The test suite also
does a notably good job of proving semantic rewrite equivalence rather than
merely reparsing generated text.

The lifecycle source association is the exception. Reconstructing identity in
a second walker duplicates verb/path semantics and turns display notation into
an internal key. Keeping typed source identity on the canonical surface would
both satisfy the specification and eliminate the collision class rather than
patching individual separator characters.
