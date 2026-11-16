---
$schema: feature-review.yaml
ready: false
findings:
    - "R1-F1 (high): Remote shell discovery ignores the inclusion's interpolate option"
    - "R1-F2 (high): The editor rejects valid expressions in interpolate values"
    - "R1-F3 (high): Child preparation and remote execution discard set parameters"
    - "R1-F4 (high): A recorded L1 test exceeds the hard ten-second limit"
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-15T21:57:46-08:00
spec: 2026-11-14-file-interpolate-option/spec.md
implemented: true
description: A **feature** review of `2026-11-14-file-interpolate-option/spec.md`
feature: 2026-11-14-file-interpolate-option/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/fix-sequences/darkmatter/features/2026-11-14-file-interpolate-option/log.md
implemented_by: claude/opus
next: 2026-11-14-file-interpolate-option/review-2.md
---

# Review 1: File interpolation option

The feature is **not production ready**. Four findings need repair. None
requires an author decision, and there are no earlier implementation-review
findings to carry forward.

## Findings

### R1-F1 (high): Remote shell discovery ignores the inclusion's interpolate option

**Authority — spec:** “Establish the child's effective prose setting before
passive call checks, context-demand discovery, remote-expression discovery,
shell-approval discovery, or body evaluation inspect it.” The verification
section also requires permitted remote Markdown to inherit suppression and
preflight to agree with composition.

**Defect class:** One recursive child-request path omits the local option
before downstream readers decide which expressions are executable.

In the Darkmatter library, the remote branch of
[`collect_document`](../../lib/src/markdown/compose/preflight/collect.rs:1030)
clones the parent's request and changes its source URL. It never folds
`directive.options.interpolate`. Execution does fold it in
[`TransclusionEngine`](../../lib/src/markdown/compose/transclusion/engine.rs:1220).
An enabled parent including remote documentation with `interpolate=false`
therefore gets different behavior from `compose_preflight` and `compose_with`.

Reproduced in process through both public APIs, using the existing remote
test fixture pattern: a loopback Wiremock server, an explicitly allowed
`127.0.0.1` host, and a temporary root containing either:

```md
::file http://127.0.0.1:PORT/sample.md interpolate=false
::url http://127.0.0.1:PORT/sample.md interpolate=false
```

Three remote payloads expose the decisions downstream of the missing fold:

- `Sample {{ length() }}.` — execution preserves it, but preflight rejects
  the sample call for its missing argument, including under deny-all policy.
- `Sample {{ as_markdown("::shell echo sample") }}.` — preflight adds
  `echo sample` to the approval set although execution must leave the example
  literal.
- `Sample {{ current.os }} and {{ ping('10.1.2.3') }}.` — preflight reports
  the `os` deferred context key, the `ping` capability, and one planned ICMP
  probe. No probe was sent by these review checks.

The bounded sweep covers each recursive discovery path and the later
decisions that consume its setting:

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R1-F1.1 | Remote `::file`, passive call check | Direct false; `length()` sample | Preflight rejects; execution preserves | Both preserve without an expression error |
| R1-F1.2 | Remote `::url`, passive call check | Same payload | Same rejection | Same literal result |
| R1-F1.3 | Remote `::file`, nested-command discovery | Direct false; prose `as_markdown` sample | Approval set contains `echo sample` | Empty approval set |
| R1-F1.4 | Remote `::url`, nested-command discovery | Same payload | Same unwanted command | Empty approval set |
| R1-F1.5 | Remote `::file`, deferred capability and authored-probe discovery | Direct false; `current.os` and `ping` samples | `os`, `ping`, one probe | No key, capability, or probe from prose |
| R1-F1.6 | Remote `::url`, deferred capability and authored-probe discovery | Same payload | Same unwanted metadata | No key, capability, or probe from prose |
| R1-F1.7 | Local-file discovery | Same three payloads, local false | No sample rejection, command, capability, or probe; clean | Same |
| R1-F1.8 | Remote discovery beneath a disabled local ancestor | Remote `::file` and `::url`; `length()` sample | Both APIs preserve; clean | Same |
| R1-F1.9 | Remote execution | Direct false; same invalid-call payload | Literal output; clean | Same |
| R1-F1.10 | Prologue/epilogue discovery | Disabled parent; no local option | Clean by invariant: local derivation folds default options; remote derivation clones the already-disabled parent | Retain suppression |
| R1-F1.11 | Nested `as_markdown` discovery | Disabled parent | Clean: derives through `inherited_child_options` with default options; existing active-input inheritance test passes | Retain suppression |

**Recommended repair:** Fold the remote directive's setting before
`collect_recursive`, using the same existing fold as execution. Preserve the
remote source and permission policy. Add public preflight regressions for
both directive spellings and the three payloads above. No new protection
layer or syntax is needed.

### R1-F2 (high): The editor rejects valid expressions in interpolate values

**Authority — spec:** “Inputs to executable directives keep their normal
evaluation,” including “expressions in file targets and option values”; the
verification table requires “library and editor agree.”

**Defect class:** Passive validation treats an unevaluated expression as a
final scalar value.

In the Darkmatter language-server package,
[`invalid_interpolate_options`](../../dmls/src/overlay/directives.rs:228)
passes every authored value directly to the boolean-word validator.
Composition evaluates option expressions first. This ordinary document
composes successfully but receives an ERROR diagnostic in the editor:

```md
---
flag: false
---
::file ./child.md interpolate={{ flag }}
```

The message says the option must be true or false, “not {{ flag }}”.
The implementation's own repeated-inclusion test already uses a dynamic
`interpolate={{ item }}` value, so this is an intended authoring pattern.

The sweep used the existing DMLS `Editor` fixture and its public
`ProviderRegistry` diagnostics, plus public composition for local inputs:

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R1-F2.1 | DMLS `::file` option validation | `{{ flag }}`, `{{flag}}`, `"{{ flag }}"`, `"{{ false }}"` | Each produces `dm.directive.invalid_option_value`; all compose | No scalar-value error before expression resolution |
| R1-F2.2 | DMLS `::url` option validation | Same four expression spellings | Same erroneous diagnostic | Same passive treatment as `::file` |
| R1-F2.3 | DMLS literal validation | `false`, quoted false, null, aliases, repeated valid values | Clean; accepted by composition | Accept |
| R1-F2.4 | DMLS invalid literal validation | `banana`, quoted null, empty/missing, invalid-then-valid | Clean: rejects at the option; existing agreement test passes | Reject every invalid occurrence |
| R1-F2.5 | DMLS `::code` | Literal and all four expression spellings | Clean: no interpolate-value diagnostic; remains an unknown option | Preserve existing unknown-option behavior |
| R1-F2.6 | Runtime and preflight repeated-inclusion paths | `interpolate={{ item }}` with true/false items | Clean; existing public behavior test passes | Evaluate the option, then validate the result |

**Recommended repair:** Recognize authored expression-bearing values
passively and defer their boolean validation until composition has their
result. Keep static literal checking and the existing expression syntax
diagnostics. Do not execute expressions in DMLS. Extend the existing
agreement test with the expression spellings above.

### R1-F3 (high): Child preparation and remote execution discard set parameters

**Authority — defect:** A real `::file ./child.md interpolate=false
set.word="supplied"` invocation can fail reference validation against a
branch that execution removes. Remote `::file` and `::url` invocations also
silently ignore the supplied value. The relevant child-handling code was
read during this review; this finding includes behavior that predates the
feature. The spec additionally requires “one interpretation across every
path that examines a child” and normal supplied-value precedence under false.

**Defect class:** Recursive consumers load the same child with different
explicit parameter state, so their conditions and command bytes diverge.

In the Darkmatter library,
[`build_node`](../../lib/src/markdown/reference/graph.rs:626) prepares a local
child without applying its directive's `set` overlay. The remote
[`execution branch`](../../lib/src/markdown/compose/transclusion/engine.rs:1272)
and remote
[`preflight branch`](../../lib/src/markdown/compose/preflight/collect.rs:1034)
also omit that overlay. Local execution and preflight apply it correctly.

Reproduction child:

```md
---
word: default
---
::block when="word == 'supplied'"
[Selected](./supplied.md)
::end-block
::block when="word == 'default'"
[Default](./default.md)
::end-block
```

With only `supplied.md` present, the normal `md compose` invocation above
exits 1 and reports missing `./default.md` at the child's line 9. Public
composition itself correctly emits only the Selected link. With the same
child served over allowed loopback HTTP, public composition emits Default
despite the explicit override. A shell-child variant, with body
`::shell echo {{ word }}`, exposes the value in the public approval set.

Each row below was checked with an unedited control, property-form
`set.word="supplied"`, and object-form `set='{word: "supplied"}'`:

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R1-F3.1 | Local execution | No set / property / object | Default / Selected / Selected; clean | Same |
| R1-F3.2 | Local preflight | Same forms, shell-child variant | `echo default` / `echo supplied` / `echo supplied`; clean | Same |
| R1-F3.3 | Local reference preparation and branch selection | Same forms | Always records `./default.md` | Control records default; both overrides record supplied |
| R1-F3.4 | CLI reference-check decision | Property override; only supplied target exists | Rejects missing default target | Compose Selected successfully |
| R1-F3.5 | Remote `::file` execution | No set / property / object | Always emits Default | Both overrides emit Selected |
| R1-F3.6 | Remote `::url` execution | Same forms | Always emits Default | Both overrides emit Selected |
| R1-F3.7 | Remote `::file` preflight command resolution | Same forms, shell-child variant | Always approves `echo default` | Both overrides approve `echo supplied` |
| R1-F3.8 | Remote `::url` preflight command resolution | Same forms | Always approves `echo default` | Both overrides approve `echo supplied` |
| R1-F3.9 | Frontmatter includes and nested-expression children | No directive `set` parameter exists at these sites | Clean by contract: no overlay is supplied to discard | Keep their existing inheritance |

Reference graphs do not fetch remote children; the remote graph controls
contained no child hyperlinks, so they are not evidence of correct remote
parameter application. Parameter identity itself distinguishes differing
overlays; it cannot repair a consumer that never applies their values.

**Recommended repair:** Reuse the existing child-overlay application when
preparing local reference children and executing/walking remote children.
Keep applying the overlay before frontmatter interpolation and schema
preparation, and preserve the existing value-origin tracking. Prove the
dependent branch and command results, including a normal CLI run with no
default target. This needs no new syntax or author decision.

### R1-F4 (high): A recorded L1 test exceeds the hard ten-second limit

**Authority — review's test gate:** “a test at 10 s or longer fails,
whatever it takes alone,” and an uncleared slow test in an implementer's
recorded run is a finding, including outside the changed code.

**Defect class:** Recorded L1 verification exceeded the hard duration limit
and was left unresolved instead of repaired.

Phase 4 records the test-toolkit test
[`ensure_is_idempotent_on_a_second_run`](../../../tools/test-toolkit/tests/kache_init_contracts.rs:317)
at **12.4 seconds** in one individual rerun. The log explicitly leaves its
speed evidence uncleared. This test runs the complete `_ensure-kache` path
twice to check that the second invocation makes no changes.

Fresh measurements are better, but do not erase the recorded hard-limit
failure. The bounded sweep of the three deferred tests is:

| Child ID | Site | Recorded shape and result | Review measurement | Expected result |
|---|---|---|---|---|
| R1-F4.1 | `kache_init_contracts::ensure_is_idempotent_on_a_second_run` | 5.3 s in package; individual reruns include 12.4 s | 2.501 s alone; 3.961 s in package | Preserve both-run assertions and remove the cause of the recorded hard-limit failure |
| R1-F4.2 | `kache_host_contracts::report_resolves_the_worktree_base_by_the_wt_contract` | 6.0 s in package; 7.9 s alone | 3.123 s alone; 4.686 s in package; current evidence clean | Under 5 s alone and within the applicable package allowance |
| R1-F4.3 | `archive_path_guard::no_archive_executed_target_bakes_in_a_producer_path` | 6.2 s in package; 7.2 s alone | 4.969 s alone; 5.543 s in package; cleared under loaded-host allowance | Same |

Review host load averages were about 31–35 on the implementation record's
16-core host; the package run finished with load about 35. All 555 selected
test-toolkit tests passed their assertions, with four skipped; the archive
guard was its only slow test. The unrelated shell-probe and glob-guard
durations in the implementation log already have qualifying individual
measurements under its loaded-host allowance.

**Recommended repair:** Profile the two `_ensure-kache` passes and remove
unnecessary fixture/process work while retaining the real second-run state
and command-log assertions. Re-measure alone and with the package. Do not
rename the test into another tier, relax the limit, or remove the second run.

## Implementation decisions (informational)

Two independent sub-agents read the complete spec and implementation log in
parallel before this comparison, starting with `decisions_for_review`.
The log has nine explicitly flagged entries and no separate “Decided during
implementation” heading. The spec has no unresolved questions or general
autonomy clause; it leaves internal implementation choices open. None of
the following decisions needs a human to choose a safe repair or outcome.

- **Invalid-value caret placement:** Keep pointing at the value. The spec
  asks for the authored option position without choosing key versus value.
- **Whole directive lines as active input:** Keep this internal choice to
  support spaced target expressions. Disclosure summaries remain prose;
  their option values remain active. Including file-link and TOC-linking
  inputs follows the same rule.
- **Earlier preflight scanner changes:** Keep excluding suppressed prose
  from authored-expression discovery. The phase in which this landed is
  immaterial; the behavior is required. The remaining remote omission is
  R1-F1, not an unresolved design choice.
- **Remote discovery filters active ranges rather than masking Markdown:**
  Keep the implementation approach, which retains code-region context.
- **Brace-bearing link destinations keep normal validation:** Keep this.
  A direct CLI check confirmed the log's `[x](./{{name}}.md)` reproduction
  exits 1 with `dm.reference.failure`, `failure: invalid-reference`, and
  unknown line/column. The spec explicitly preserves link handling. This
  is not an interpolation-option finding or a request to exempt links.
- **Reference-child inheritance:** Keep suppression for prologue, epilogue,
  and TOC children. Independently read fragment targets retain normal
  interpolation because they are not included through the disabled branch.
- **Hover on every option value:** Keep it. Explaining false, defaulting,
  and inheritance while the author edits another value is useful and cheap
  to change later.
- **Shared missing-value reason:** Keep routing trailing `interpolate=`
  through the same validator as the editor.
- **Implementation metadata:** Keep the actual implementer identity.
  Leaving workflow status management to the pipeline is not a product
  decision requiring reaffirmation. This review updates `review_iterations`
  to 1 and does not mark the spec completed.
- **Test placement and direct implementation:** Existing test modules and
  the decision to work without implementation sub-agents are internal
  organization choices. The declared targets compile the new tests, and
  tier coverage reports zero stranded tests.
- **Structured parse-error locations:** Retain unknown JSON coordinates
  where existing provenance cannot establish an authored location. The
  existing CLI tests separately verify the including file and formatted
  line/caret. This is the spec's existing directive error handling.
- **Pending-read resolver left unchanged:** Its accepted-span invariant
  keeps delayed reads confined to previously selected active expressions.
- **Expression skill left unchanged:** Its produced-value rules are
  unchanged; composition guidance documents the new inclusion behavior.
- **No caller-skill adoption edits:** The audit found only fenced directive
  examples. Keeping intentional examples and publishing the updated guidance
  is appropriate; no adoption work remains deferred.
- **Initially deferred cache proof:** It was completed after suppression
  existed, with both inclusion orders, enabled/disabled caches, warning
  independence, and reuse counters.
- **Unchanged link/JSON behavior:** These were deliberately retained, not
  remaining feature implementation work.
- **Unrelated Claudine failure:** The log records its independent failure
  with this feature reversed. This review did not inspect or rerun that
  unrelated code and makes no claim that its suite passes.
- **Deferred speed and cross-OS evidence:** The test-toolkit speed evidence
  is addressed explicitly in R1-F4 and the table above. Leaving cross-OS
  execution to CI follows this review's instructions and needs no human
  review item.

## Verification and coverage

All feature requirements concern text composition or passive editor
analysis, so **L1 is the appropriate level**. No requirement needs a real
terminal, browser, window focus, or OS keyboard injection.

| User-facing requirement | Strongest existing verification inspected | Assessment |
|---|---|---|
| Defaults, aliases, quoted words, null, repeated/invalid options | L1 parser cases, composition cases, DMLS agreement and CLI invalid-value checks | Literal cases pass; expression-valued editor gap is R1-F2 |
| Known/unknown names, invalid/malformed samples, triple braces, deny-all | L1 public composition and frozen publication CLI test | Local behavior passes; remote preflight gap is R1-F1 |
| Code-block opt-in cannot re-enable interpolation | L1 public composition, fenced and indented controls | Appropriate level; passes |
| Frontmatter resolution and supplied-value precedence | L1 public composition and review's parameter sweep | Cross-path failures are R1-F3 |
| Conditional and collection inputs stay active; repeated prose stays literal | L1 public composition and repeated-site preflight test | Appropriate level; passes covered cases |
| Nested target, option, and shell inputs evaluate normally | L1 public composition and approval-byte comparison | Appropriate level; remote omissions identified above |
| Descendant inheritance and parent/sibling isolation | L1 public composition, graph and cache identity/reuse tests | Local paths pass; direct remote preflight gap remains |
| Prologue/epilogue, nested composition, permitted remote Markdown | L1 public composition with temporary files and loopback HTTP | Runtime checks pass; remote discovery was missing from the test combination |
| No reads/fetches/context/approvals from prose samples | L1 public read-set checks and zero-request mock, plus scanner checks | Existing checks pass; review exposed remote preflight decisions |
| Replacement, exclusion, links, wrappers, fresh-root recomposition | L1 public composition and reference validation | Appropriate level; normal link-validation behavior confirmed |
| Completion, hover, invalid values, independently opened child | L1 provider registry and semantic-token checks | Appropriate level; literal checks pass, dynamic values need R1-F2 |
| Publication with a colliding `name`, braces, and deny-all guard | L1 spawned normal `md compose`; frozen caller-skill fixture | Positive result and negative control pass |
| Passive shipped caller-skill source parsing | L1 shipped-source corpus test | Passes; no active adoption sites were found |

Reviewer runs:

- `just test file_interpolate`: **39 passed**, none at or above five seconds.
- `just test interpolate`: **135 passed**, none at or above five seconds.
- `just test active_inputs`: **10 passed**, none at or above five seconds.
- `just check-tier-coverage darkmatter`: **zero stranded tests**.
- The three individual test-toolkit measurements and full package run are
  reported under R1-F4, including their limits.
- Temporary public-API/provider probes collected the complete instance
  tables above. They deliberately failed to print their observed results;
  they were removed afterward. No production code or permanent test change
  is part of this review.

The configuration robustness matrix does not apply: this feature changes a
Markdown directive grammar, not a configuration/document loader. Its value
shapes were checked in the parser and editor tests instead. No new
dependency, persisted format, or terminal behavior was introduced.

## Implementation-record correction

Corrected the Phase 3 implementation-log statement that preflight child
derivation covered all inclusion paths. A dated note records the verified
remote exception and the public-API reproductions. The spec's decisions and
body were left intact. Correcting the record is not an additional finding;
the underlying implementation defect is R1-F1.
