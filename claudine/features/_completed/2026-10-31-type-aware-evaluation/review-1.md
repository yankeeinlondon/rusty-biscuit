---
$schema: feature-review.yaml
ready: false
findings:
  - 'R1-F1 (high): Deliver guarded editor analysis for Claudine lifecycle expressions'
  - 'R1-F2 (high): Stop execution when a runtime warning becomes blocking'
  - 'R1-F3 (high): Preserve shared diagnostics on lifecycle call rejection'
  - 'R1-F4 (high): Preserve the ordered passive diagnostic history'
  - 'R1-F5 (high): Do not restrict nullable-parameter suppression by return type'
observations: []
human_in_the_loop: null
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: '2026-10-24T21:38:43-07:00'
spec: 2026-10-31-type-aware-evaluation/spec.md
implemented: true
description: 'A **feature** review of `2026-10-31-type-aware-evaluation/spec.md`'
feature: 2026-10-31-type-aware-evaluation/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/feat-pipeline/claudine/features/2026-10-31-type-aware-evaluation/log.md
implemented_by: claude/opus
next: 2026-10-31-type-aware-evaluation/review-2.md
---

# Review 1

The feature is **not production ready**. Five findings require repair. No human decision is needed, and this is the first review, so there are no previous findings to reconcile.

## Decisions and scope

Two agents independently read the specification and implementation log before the implementation comparison. Both found `decisions_for_author: []` and no decision meeting the human-review threshold. The log records choices about conservative schema projection, shared narrowing, registration-owned host facts, passive file validation, diagnostic identity, CLI policy, report output, and recursion limits.

The following choices remain settled:

- Claudine registers no expression functions, so an empty function catalog and facts artifact would serve no purpose. The test host proves the registration and editor-delivery mechanism.
- The shipped `md` command also registers no host functions. Public-library composition plus a separate editor session is an appropriate host-function test boundary.
- Where the file-predicate section contradicts itself about existing remote behavior, retain the recorded decision to preserve denied/failed-fetch errors. Both predicate names share this behavior. A local HTTP-server check confirmed denied host and HTTP 404 errors, HTTP 200 success, malformed-reference false, and unchanged local and `file://` results.
- Object refinement from a discriminant member is outside the promised stable-subject narrowing; its planned marker is not a finding.

Two departures are requested scope left undelivered: guarded editor analysis of Claudine lifecycle bodies, and suppression for direct nullable arguments when the function also returns null. They are findings R1-F1 and R1-F5, rather than questions for the author. The review's decisions, alternatives, reasons, and downsides are recorded under **Review 1 → Design Decisions** in the implementation log.

## Findings

### R1-F1 (high): Deliver guarded editor analysis for Claudine lifecycle expressions

**Authority — spec:** section 2 requires “Runtime and editor analysis agree on every narrowing”; criterion 34 requires runtime and DMLS coverage of lifecycle guards, writes, and event boundaries. The final implementation log explicitly leaves DMLS narrowing of Claudine lifecycle bodies planned. Requested scope cannot be deferred.

**Class:** editor analysis loses the host's lifecycle context and checks guarded expressions with an unguarded type scope.

In the `dmls` package, [GuardedStackCalls::for_document](../../../darkmatter/dmls/src/diagnostics/frontmatter.rs:931) hardcodes the Markdown host's parser and registry. Claudine-only events do not enter this guarded walk. Shared events using host-only actions also lose their expression surfaces because the Markdown parser ignores those verbs. Both routes fall back to ordinary expression checking.

Start from the existing lifecycle write-invalidation fixture, use the shipped Claudine global catalog, and enable `[expressions] strict = true`:

```yaml
---
$schema:
  count: number
count: 1
success:
  - when: count
    then:
      - stdout: "{{ min(count, 5) }}"
      - set:
          count: 7
      - stdout: "{{ min(count, 6) }}"
---
Body
```

The editor warns at **both** calls. It must be silent at the first, where the guard proves presence, and warn only at the second, after the write invalidates the fact. Runtime library coverage already establishes that distinction.

The sweep used actual stdio LSP sessions with the current `dmls` executable. Each event named below was checked separately; `stdout` and `say` were separate variants.

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F1.1 | Claudine `start`, `success`, `blocked`, `failure` | Guard → call → set → call, using `stdout` and `say` | Both calls warn in every variant | Only the post-write call warns |
| R1-F1.2 | Shared `initialize`, `composed`, `failed`, `cancelled`, `finalize` | Same sequence, `say` action | Both calls warn in every event | Only the post-write call warns |
| R1-F1.3 | Claudine `loop.gate` | Valid mapping with `until: loop.count > 2` and guarded gate list | Both calls warn | Only the post-write call warns |
| R1-F1.4 | Claudine `success` false branch | `when: is_null(count)`, calls and write in `else` | Both calls warn | Only the post-write call warns |
| R1-F1.5 | Claudine `success` nested write | Guarded call, nested conditional set, later call | Both calls warn | Only the later call warns |
| R1-F1.6 | Markdown `initialize`, `composed`, `failed`, `cancelled`, `finalize` | Same guard/write sequence, `stdout` | Only the post-write call warns | Same; clean controls |
| R1-F1.7 | `err.msg` availability in Claudine `blocked`/`failure` and Markdown `failed` | Available error-global read | No expression diagnostic | Same; clean availability controls |

The other host-only verbs (`speak`, `effect`, `notify`, `retry`, `resume`, `proxy`, `prep`, `break`, `defer`) are explicitly ignored by the same Markdown parse options. Their expression operands cannot acquire host lifecycle scopes through this path; this is a shared parser/registry boundary, not a separate narrowing algorithm to implement per verb. Preserve the library's one scoped walk and provide it with the host's passive lifecycle declarations. Extend editor tests to the host-only events, host-only operands, and loop gates, including the write and event-boundary controls above.

### R1-F2 (high): Stop execution when a runtime warning becomes blocking

**Authority — spec:** section 7 says “When a blocking diagnostic becomes known, the command stops before further work or effects”; criterion 27 requires stopping before the next effectful step, without rolling back earlier effects.

**Class:** runtime warnings accumulate until composition finishes, allowing later effects after a denied diagnostic is known.

The `darkmatter-cli` package checks ordinary warnings only after [run_compose](../../../darkmatter/cli/src/commands/compose.rs:510) receives the completed document. The lifecycle composition route likewise checks after `compose_with_warnings`. Only the passive call-warning path checks its blocking policy inside the library pipeline.

In a temporary directory, create `.darkmatter-shell-whitelist` containing `prefix touch`, and compose this document with `--deny all --output report-json`:

```text
{{ missing }}

::shell touch marker
```

The command fails and reports `dm.expression.undeclared_property`, but **creates `marker`**. The warning arose in interpolation before the shell stage. This is later work that should never have started, not an earlier effect requiring rollback. The same reproduction without the denial is the positive control.

The sweep followed the warning from each runtime reader through the CLI's outcome decision:

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F2.1 | Body interpolation → shell | Fixture above | Exit 1; marker created | Exit nonzero; no marker |
| R1-F2.2 | Frontmatter interpolation → shell | `title: "{{ missing }}"`, then body shell | Exit 1; marker created | Stop before shell |
| R1-F2.3 | Page-block condition → shell | `when="is_empty(trim(missing))"`, shell in retained body | Exit 1; marker created | Stop before shell |
| R1-F2.4 | Transclusion condition → child execution | `::file ./child.md when="is_empty(trim(missing))"`; child touches marker | Exit 1; marker created | Stop before child effect |
| R1-F2.5 | Transclusion condition → next directive | Missing bare guard skips first child; second directive runs it | Exit 1; marker created | Stop before next directive's effect |
| R1-F2.6 | Frontmatter shell ternary → later shell | `gated: "$(missing ? echo a : echo b)"`, then body touch; `echo` also whitelisted | Exit 1; marker created | Stop once condition warning is known |
| R1-F2.7 | Transcluded child's interpolation → child's shell | Parent includes a child containing the first fixture | Exit 1; marker created | Child stops before its shell |
| R1-F2.8 | Passive category-3 call → shell | Typed string passed to `min`, then touch | Exit 1; no marker | Same; clean passive control |
| R1-F2.9 | Explicit absence handling | `is_null(missing)`, then touch | Success; marker created; no warning | Same; clean suppression control |
| R1-F2.10 | Unexecuted branch | Missing read inside false block, then touch outside | Success; marker created; no warning | Same; clean branch control |

Recovery denials were also checked with `--allow all` plus a specific denial: scalar `ctx`, invalid transclusion assignment, and duplicate assignment fail before the later touch. They already refuse the denied recovery and are clean controls for this defect class. Final document publication is correctly withheld in the failing cases; the defect is the effects before that final check.

The smallest repair is to enforce the existing blocking policy where runtime diagnostics are reported and before subsequent effectful work, including within a stage that evaluates multiple expressions. Do not run additional expressions to discover warnings or move legitimate preparation ahead of its dependencies. Add in-process effect counters for these boundaries and a small CLI wiring check.

### R1-F3 (high): Preserve shared diagnostics on lifecycle call rejection

**Authority — spec:** section 6 requires composition, CLI, and DMLS to report identical diagnostic fields for the same problem; criterion 19 names code, severity, message, source position, and expected/actual types. Section 1 includes lifecycle event expressions in the shared analysis.

**Class:** a lifecycle rejection discards its shared diagnostic and is reclassified as an unlocated evaluation failure.

In the `darkmatter` package, [reject_incompatible_calls](../../../darkmatter/lib/src/lifecycle/validate.rs:219) creates `LifecycleCallRejected` with the expression error but no located diagnostic. [error_diagnostics](../../../darkmatter/lib/src/markdown/compose/report_diagnostics.rs:93) follows that error chain to `ExpressionError`; its fallback labels the rejected call `dm.expression.evaluation_failure`. The Claudine error conversion also retains no shared record.

```yaml
---
composed:
  - stdout: "{{ min(true, 5) }}"
---
Body
```

Compare `md compose --output report-json` with a DMLS session on the same bytes:

| Field | CLI report | Editor / expected shared record |
|---|---|---|
| code | `dm.expression.evaluation_failure` | `dm.expression.incompatible_call` |
| severity | `error` | `error` |
| message | Runtime binder wording | Shared category-5 wording describing `literal true` against `number` |
| source | `null` | File, line 3, column 21 |
| expected_type | `null` | `number` |
| actual_type | `null` | `literal true` |

The sweep compared all six fields in real CLI/LSP runs. Every named event was tested separately.

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F3.1 | `initialize`, `composed`, `failed`, `cancelled`, `finalize` action text | `stdout: "{{ min(true, 5) }}"` | Wrong code/message; source and types null | Exact shared record |
| R1-F3.2 | Same five events' conditions | `when: min(true, 5)` with a valid `then` list | Same loss; editor locates line 3, column 15 | Exact shared record |
| R1-F3.3 | Lifecycle shell command | `shell: "{{ min(true,5) }}"` | Same loss | Exact shared record |
| R1-F3.4 | Lifecycle side-effect argument | `ensure_file: ["marker", "{{ min(true,5) }}"]` | Same loss; no effect launched | Exact shared record |
| R1-F3.5 | Lifecycle runtime-set value | `set: {count: "{{ min(true,5) }}"}` | Same loss | Exact shared record |
| R1-F3.6 | Lifecycle control reason | `error: "{{ min(true,5) }}"` | Same loss | Exact shared record |
| R1-F3.7 | Body and frontmatter interpolation | Typed required boolean passed to `min` | Incompatible-call code, located source, `number`/`boolean` | Same; clean controls |
| R1-F3.8 | Page-block and transclusion conditions | `min(true,5)` | Incompatible-call code, located source, `number`/`literal true` | Same; clean controls |

The library's `walk_action` routes shell error operands, expression-function arguments, and control operands/`with` values into the same surface-check collection and single rejection constructor. Claudine's conversion of that rejection is another preservation site, rather than another analyzer. Operand-free `stop`/`skip` have no call to classify. Preserve the located shared diagnostic at the producer and through error conversion, then consume it before the generic error fallback. Extend the parity tests to rejection, rather than only warnings, on lifecycle surfaces.

### R1-F4 (high): Preserve the ordered passive diagnostic history

**Authority — spec:** section 8 requires earlier diagnostics to remain in a failure report and passive diagnostics to follow source order; criterion 39 requires earlier warnings to survive later failure.

**Class:** passive diagnostic collection does not preserve the ordered prefix of findings when scanning multiple expression surfaces.

In the `darkmatter` package, [check_document](../../../darkmatter/lib/src/markdown/compose/call_precheck.rs:152) discards accumulated advisories when a later call is rejected. `first_rejected` also skips recording other findings in an expression that contains a rejection. Separately, the body walk appends block and transclusion-condition warnings before interpolation warnings, even when those conditions occur later in the file. The lifecycle checker likewise returns only its rejection if any surface fails.

```text
{{ file_exists("") }}

{{ min(true, 5) }}
```

`md compose --output report-json` returns only `dm.expression.incompatible_call`. The earlier always-false warning is absent. Removing the second expression reports that warning, proving the first expression is load-bearing.

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F4.1 | Separate body expressions | Fixture above | Only rejection | Earlier always-false warning, then rejection |
| R1-F4.2 | Separate frontmatter leaves | `a` contains malformed-reference call; `b` contains rejected `min` | Only rejection | Warning followed by rejection |
| R1-F4.3 | One expression | `file_exists("") + min(true,5)` | Only rejection | Earlier call warning followed by rejection |
| R1-F4.4 | Block condition then body rejection | Malformed-reference `when`, later rejected `min` | Only rejection | Earlier warning retained |
| R1-F4.5 | Lifecycle action list | First stdout contains malformed-reference call; next stdout contains rejected `min` | Only rejection, also misclassified as R1-F3 describes | Earlier warning retained before the corrected rejection |
| R1-F4.6 | Body interpolation before block condition | Malformed-reference interpolation on line 1, malformed-reference `when` on line 3 | Successful report lists line 3 before line 1 | Line 1 before line 3 |
| R1-F4.7 | Body interpolation before transclusion condition | Same line-1 warning, malformed-reference `::file … when` on line 3 | Successful report lists line 3 before line 1 | Line 1 before line 3 |
| R1-F4.8 | Runtime failure after passive warning | Malformed-reference call, then `{{ 1/0 }}` | Warning followed by evaluation failure | Same; clean failure-history control |

Block/transclusion conditions and frontmatter/body leaves all feed the one advisory collector; lifecycle surfaces feed their own collection before the common report adapter. Preserve the collected prefix on both success and failure, and order passive findings by authored position before exposing them. A focused extension of the existing report API is sufficient; a second evaluation or document render is unnecessary.

### R1-F5 (high): Do not restrict nullable-parameter suppression by return type

**Authority — spec:** section 6 explicitly says suppression applies to “any function, including a host function, whose parameter declares `| null`”; criterion 36 repeats the parameter-based contract. Neither limits the function's return type. The implementation log deliberately adds that limitation, narrowing requested scope.

**Class:** absence handling uses an unrelated return-type condition, so a direct argument that meets the promised parameter rule still warns and can fail under denial.

In the `darkmatter` package, [FunctionRegistration::handles_absence_at](../../../darkmatter/lib/src/markdown/compose/expression/call_binding/registration.rs:197), which decides whether an argument handles absence, requires `!signature.returns().admits_null_explicitly()`. Consequently `{{ trim(x) }}` and `{{ lower(x) }}` warn for undeclared `x`, although both parameters explicitly admit null. With `--deny all`, both documents fail. The recorded reason is preserving the separate required example `is_empty(trim(x))`; that example does not authorize excluding every direct null-returning call.

The minimal fixture is `{{ trim(x) }}`. For editor coverage, add a schema declaring another property so undeclared-property diagnostics are enabled; compare only `dm.expression.undeclared_property`, not the independent strict category-4 warning.

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F5.1 | Runtime body reader → CLI policy | Direct `trim(x)` and `lower(x)` | Undeclared-property warning; exit 1 under `--deny all` | Suppress that warning; success |
| R1-F5.2 | Runtime frontmatter reader → CLI policy | Same calls in `title` | Undeclared-property warning | Suppress that warning |
| R1-F5.3 | Static read inventory → DMLS body diagnostic | Same calls with a document schema | Undeclared-property warning, plus a separate strict call warning | Suppress only undeclared-property warning |
| R1-F5.4 | Shared registration validator, host functions | Explicit-null parameter and explicit-null return | Return-type condition excludes the registration by construction | Return type does not defeat direct-argument handling |
| R1-F5.5 | Direct non-null-returning predicate | `is_null(x)` in body and frontmatter | No undeclared-property warning; denial succeeds | Same; clean control |
| R1-F5.6 | Nested argument boundary | `is_empty(trim(x))` | Undeclared-property warning in runtime and DMLS body | Same; preserve criterion 21 |
| R1-F5.7 | Parameter/overload controls in existing public-result tests | Plain unknown, optional non-null, mixed-null overloads, all-null overloads | Warn/warn/warn/suppress respectively | Same; clean parameter-shape controls |

All built-ins and installed host functions use this registration method; aliases resolve the same registration. The string mutations, date-formatting calls, and nullable-returning file helpers do not have separate absence validators. The existing host and overload tests use non-null returns, so they do not exercise the excluded shape. DMLS frontmatter does not emit the body-only undeclared-property advisory in these fixtures; its independent strict call warning is not a defect reported here.

Preserve the spec's distinction between an expression's direct argument and a variable nested inside another call using call-site context, rather than using the callee's return type as a proxy. Keep the required nested example and the existing parameter/overload controls. This repairs promised scope; it does not require an author decision or a change to runtime values.

## Observations

None.

## Verification and limits

This review used the existing test fixtures as starting points, then changed one expression surface or declaration at a time in temporary directories. Manual CLI checks used the current workspace `target/debug/md`; editor checks used actual stdio protocol sessions with `target/debug/dmls`. HTTP checks used a local test server. No terminal or browser was focused, no live provider ran, and no implementation source was changed.

Two sequential focused `just test` runs in `darkmatter/` passed **104** and **121** tests, respectively. They covered policy/report behavior, host functions and artifact handling, file predicates, shared diagnostics, flow-sensitive scopes, projection, missing-property warnings, and editor call diagnostics. The focused `biscuit-file/` recipe passed its **4** structural-validation nextest cases and its **9** additional no-default-feature checks. These are L1 in-process, subprocess, and LSP checks, which are the levels required by this spec; no terminal encoder, keyboard, or visual-rendering behavior is promised here. The new manual reproductions expose cases absent from those passing tests.

The input-robustness review applies to the new host catalog/artifact reader. Its existing 50-edit public-API matrix exercises missing, null, wrong-type, empty, duplicate-key, trailing-content, missing-artifact, and stale-artifact behavior through shared validators; incomplete metadata prevents unsupported category/narrowing claims and never invokes a handler. The reader rejects duplicate keys and trailing content. The smell search found no permissive default or silently filtered field in that new reader. The raw-schema projection consumes already-loaded schema values; report JSON is a writer, not another configuration loader. No separate robustness finding was established.

The before/after performance evidence is present in the log, including the explained editor-analysis increase. Test target declarations include the relevant L1 modules; the implementation log also records zero stranded tests in the area tier audits. This review did not rerun full-area lint, full suites, or cross-OS checks because it changes only review metadata and the review record. Cross-OS execution evidence is not a readiness finding.
