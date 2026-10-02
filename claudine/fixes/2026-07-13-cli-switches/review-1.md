---
$schema: feature-review.yaml
ready: false
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-02T01:30:56-07:00
spec: 2026-07-13-cli-switches/spec.md
implemented: true
next: 2026-07-13-cli-switches/review-2.md
implemented_by: claude/opus
log: claudine/fixes/2026-07-13-cli-switches/log.md
description: A **fix** review of `2026-07-13-cli-switches/spec.md`
fix: 2026-07-13-cli-switches/review-1.md
findings:
  - title: Embedded credentials escape argument redaction
    priority: high
  - title: Flag completion bypasses ownership errors
    priority: medium
  - title: Unknown switch types are described as recognized
    priority: medium
  - title: The ambiguity chooser lacks real-terminal verification
    priority: high
  - title: The permanent input matrix omits load-bearing fields and shapes
    priority: medium
---

# Review 1: CLI Switches

The fix is **not production ready**. Exact forwarding, retry/proxy/resume carry-over, positional arguments, and most ownership rules work in the tested cases. However, argument displays expose embedded credentials, completion suggests flags after arguments it cannot interpret, and notices misrepresent explicitly unknown switch types. The new chooser and parts of the metadata reader also lack the required regression coverage. All findings can be resolved by agents; none needs a human design decision.

This review covers the current implementation, including the uncommitted completion changes present when review began. Existing diagnostic probes in the working tree were run but were not treated as regression tests: they print observations without asserting the promised behavior. No production source or test file was changed during review.

## Findings

### High — Embedded credentials escape argument redaction

**Defect class:** argument redaction recognizes secrets at selected token boundaries but misses the same credentials embedded in a configuration assignment or a long attached value.

In package **claudine-cli**, [redact_sensitive_args](../../cli/src/commands/wrap/env/sanitize.rs#L129) protects argument displays and the `AGENT_PARAMS` environment metadata. Its final fallback retains the original token. Thus both `-c api_key=sk-proj-reviewsecret0123456789` and `--config=sk-proj-reviewsecret0123456789` survive redaction. These are forwarded provider arguments, not caller frontmatter setters. Dry-run output and `AGENT_PARAMS` expose the key; direct-wrapper debug output exposes it too. This violates the contract that the child receives the original value while every diagnostic or metadata surface masks it.

Reproduction used a copied plan fixture and a fake provider recording its argv and `AGENT_PARAMS`. The inline fixture had a `prompt` and the successful fake provider changed its body; the sequence fixture had a named step. Successful controls launched before inspecting metadata, so stale records or preparation failures could not be mistaken for clean results.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| `compose` dry-run and child metadata | Both embedded forms above | Dry-run and `AGENT_PARAMS` contain the key; child argv preserves it | Mask both display/metadata surfaces; preserve child argv |
| `inline-compose` dry-run and child metadata | Same forms | Same leaks; successful body-writing control exits 0 | Same masking contract |
| `sequence` dry-run and child metadata | Same forms, named step | Same leaks; successful control exits 0 | Same masking contract |
| Direct Codex wrapper, dry-run/debug/metadata | Same forms | Key appears in dry-run, debug stderr, and `AGENT_PARAMS` | Mask all three |
| Other direct wrappers, dry-run | `--config=<key>` through Claude, Gemini, Goose, Kimi, OpenCode, Qwen, Kilo, Pi, Antigravity | All ten wrappers, including Codex, expose the key; OpenCode requires a model for the positive control | Mask regardless of provider |
| All three composition commands and direct Codex wrapper | `-c<key>`, `--token=<key>` | Dry-run and metadata mask the key; child argv retains it | Clean controls |
| INFO notice, all three composition commands and direct wrapper | All four forms | Switch names only; no key | Clean control |
| Captured Goose diagnostic through all three composition commands | Provider echoes the key in an argument rejection | Excerpt/headline mask it, including a correlated rejection of `--config` | Clean control |
| Live structured Codex stderr | Provider echoes the key | Raw relayed diagnostic exposes it; later Claudine headline is masked | Account explicitly for this route when testing the promise about correlated-error output |

The sibling consumers are [composition dry-run](../../cli/src/commands/wrap/composition/dry_run.rs#L138), [direct-wrapper output](../../cli/src/output/mod.rs#L345), [environment metadata](../../cli/src/commands/wrap/env/mod.rs#L258), [direct debug tracing](../../cli/src/commands/wrap/mod.rs#L436), [composition debug tracing](../../cli/src/commands/wrap/composition/pipeline.rs#L836), and [tail_redactor](../../cli/src/commands/wrap/provider_tail_report.rs#L214), which supplies the native report. The common argument policy is the appropriate correction point, rather than separate patches to each display.

Use the existing shared secret recognizer on otherwise retained token contents, while retaining the sensitive-flag and short-attachment handling. Keep an unchanged argv vector for execution. Add asserting tests for embedded assignments, long attachment, short attachment, sensitive flags, and diagnostic echoes through every listed surface. The current passing secret tests use forms that the existing policy already handles and therefore do not detect this class.

### Medium — Flag completion bypasses ownership errors

**Defect class:** completion checks whether ordinary cursor words are safe to suggest, but a flag-shaped cursor bypasses the ownership-error check for the preceding arguments.

Package **claudine-cli** [run_with_context](../../cli/src/completion/engine/mod.rs#L178), which dispatches completion suggestions, gates setter names, setter values, and fallback completion. It excludes `CompositionProviderFlag`. Consequently, a malformed or ambiguous line still offers `--model` when the cursor is `--mod`. The requirement says missing-value errors and ambiguity produce no suggestions. A new switch cannot repair a missing value earlier on the line.

Use a plan declaring `$schema: {phase: number, mode: enum(fast, slow)}`. Run the shipped hidden completion command with its cursor on the final word; for example:

```sh
claudine __complete --current 6 -- claudine compose prompts/plan.md --codex -c phase=2 --mod
```

| Site | Shape tested after the file | Observed result | Expected result |
| --- | --- | --- | --- |
| `compose` completion | `--codex -c phase=2 --mod` | `--model` | No suggestions: declared `phase` leaves `-c` without a value |
| `inline-compose` completion | Same | `--model` | No suggestions |
| `sequence` completion | Same | `--model` | No suggestions |
| All three completion commands | `-c low --mod`, no authored agent | `--model` | No suggestions: candidate agents disagree about `low` |
| All three completion commands | `--codex -c phase=2 ph` | No suggestions | Clean error control |
| All three completion commands | `--codex -c low --mod` | `--model` | Clean valid-line control |
| All three completion commands | Provider value cursor, or cursor after authored `--` | No suggestions | Clean provider/opaque-tail controls |

Check ownership of the committed arguments before offering flag suggestions. Preserve completion of a new Claudine flag on a valid line: the partial flag itself should not become an unrecognized provider argument during this check. Extend the binary test to vary both the preceding error and the cursor shape, rather than testing errors only with setter cursors.

### Medium — Unknown switch types are described as recognized

**Defect class:** reporting treats the presence of a catalog record as proof of an established type, even when that record explicitly declares its type unknown.

In package **claudine-cli**, [switch_explanations](../../cli/src/commands/wrap/provider_tail_report.rs#L280) supplies the explanation below the forwarding notice. Its `SwitchLookup::Known` arm does not inspect `switch.value`. Ownership correctly interprets `SwitchValue::Unknown` as unrecognized, but the explanation calls it a known switch without mentioning the evidence gap. This breaks the requirement that ownership and reporting use the same interpretation.

With a fake OpenCode provider, `compose plan.md --opencode --get-yargs-completions foo` prints “--get-yargs-completions is a OpenCode switch …”. Its compiled record explicitly has `SwitchValue::Unknown`. The same explanation occurs through `sequence`, `inline-compose`, and the direct OpenCode wrapper; the inline notice is emitted before its body-change check.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| OpenCode, all four launch commands | `--get-yargs-completions foo` at the `run` entrypoint | Described as recognized | Explain that its type is unestablished at this entrypoint |
| OpenCode, all four commands | `--models foo` at `run` | Unrecognized wording | Clean: its unknown record applies only at `stats` |
| Kilo, all four commands | `--models foo` at `run` | Unrecognized wording | Clean: its unknown record also applies only at `stats` |
| Kilo, all four commands | `--get-yargs-completions foo` | Unrecognized wording | Clean: absent from the applicable catalog |
| OpenCode and Kilo, all four commands | `--new-unresearched-switch foo` | Unrecognized wording | Clean absent-record control |
| Codex notice | `-c x=y` | Explains the researched `--config` alias | Clean established-type control |
| Explicit tails | Switches after `--` | Opaque summary; no switch explanation | Clean opacity control |

The inventory sweep found exactly three explicitly unknown switch records: OpenCode's global completion-protocol switch, OpenCode's `--models` at `stats`, and Kilo's `--models` at `stats`. The latter two are not applicable at current wrapper `run` launches; their records must still receive the unknown-type branch if a reporting context names `stats`. No other provider record has this type.

Branch on the established value type inside the known-record case, reuse unrecognized wording for unknown types, and retain researched descriptions for established types. Add a reporting test using the real unknown record, plus the absent-record and known-alias controls.

### High — The ambiguity chooser lacks real-terminal verification

**Defect class:** a new interactive selection flow is verified only with manufactured PTY input, leaving its displayed choices and real-terminal interaction unverified.

Package **claudine-cli** [ask_which_agent](../../cli/src/commands/compose/ownership.rs#L130) displays the provider chooser when candidates interpret a bare word differently. The two tests in [level1_ownership_prompt_pty](../../cli/tests/l1/level1_ownership_prompt_pty.rs) pass, but explicitly use no terminal emulator. They verify the intended ownership and provider-check outcomes, not whether the actual chooser renders legibly in a terminal. Neither the declared L2 target nor the L3 target contains a test of this flow.

| Site | Shape tested or coverage inspected | Observed result | Expected result |
| --- | --- | --- | --- |
| `compose`, choose Codex | Authored `agent: [claude, codex]`, `-c foo`, manufactured `j`/Enter bytes | L1 PTY passes; forwards `foo` | Retain L1; add L2 capture of choices, selection, and resulting argv |
| `compose`, choose Claude | Same fixture, manufactured Enter | L1 PTY passes; actual Codex launch is refused for missing value | Retain L1; add L2 showing ownership choice does not change execution provider |
| `inline-compose` shared chooser call | Same ownership decision, appropriate inline fixture | No command-specific chooser test | Exercise through a background real terminal |
| `sequence` shared chooser call | Same decision, named-step fixture | No command-specific chooser test | Exercise through a background real terminal |
| Noninteractive ambiguity and completion | Same ambiguous switch/value shape | L1 verifies error/no suggestions | Correct level for these paths |

Add an asserting `level2_` test to the declared L2 binary and run it through `just test-l2`. Capture the pane, answer through the harness, and check both possible ownership decisions. Include the three composition entrypoints in the sweep. Keep all sessions in the background. This fix promises a choice flow, without introducing a particular bare-modifier encoder contract; an L3 modifier test is not required for this finding.

### Medium — The permanent input matrix omits load-bearing fields and shapes

**Defect class:** the metadata reader has a matrix test, but the table and test do not cover every field that changes its generated result or acceptance decision.

Package **claudine-gen** [cli_switch_catalog](../../gen/src/generate/coerce/cli_switches.rs#L37) reads authored switch metadata before emission. The permanent [matrix test](../../gen/tests/l1/cli_switches.rs#L121) covers many malformed typed-value, alias, attachment, and scope cases. It does not systematically cover `flag`, `description`, `gap`, `cli_switches_gap`, the containing `cli_switches` list, or all revision/scope-discriminator shapes. For example, duplicate `attachment` keys are marked not applicable in the implementation log although attachment is an authored YAML field.

The review copied the real-tool-derived revision-2 Codex fixture into a temporary provider area, changed one field per case, and called the shipped `claudine-gen --area <fixture> validate codex` command. Both the unchanged fixture and an empty inventory with a valid stated gap passed. The malformed cases below were rejected: no additional permissive-parser defect was found. These manual checks do not replace permanent regression tests.

| Reader/projection site | Shape tested | Observed result | Expected result / permanent-test state |
| --- | --- | --- | --- |
| Document revision reader | Absent, null, object, empty string, duplicate | Rejected in this fixture area | Reject unsupported input; absent revision additionally needs the legacy-contract fixture. Existing tests cover only part of the column |
| Switch-list reader | Absent, null, number, one invalid record, all invalid records, empty without gap, duplicate key | Rejected | Correct; full column absent from permanent matrix |
| Whole-provider gap reader | Absent with empty list, null, number, empty text, duplicate; valid text | Invalid cases rejected; valid gap accepted | Correct; partial permanent coverage |
| Canonical spelling reader | Absent, null, number, empty text, duplicate | Rejected | Correct; no field matrix |
| Alias reader | Null, whole-field number, one/every invalid element, duplicate | Rejected | Correct; existing matrix covers this class |
| Value-type and scalar-optionality readers | Absent, null, wrong type, empty string, duplicate | Rejected | Correct; existing matrix covers most of these shapes |
| Variadic-minimum reader | Absent, null, wrong type, empty string, duplicate | Rejected | Correct; existing matrix plus positive known/unknown controls |
| Attachment reader | Absent, null, wrong type, one/every invalid element, empty for a scalar, duplicate key | Rejected | Correct; duplicate-key cell missing |
| Scope-list/discriminator/path readers | Absent, null, wrong type, invalid elements where applicable, empty scope, duplicate | Rejected | Correct; discriminator and path columns are incomplete |
| Description reader | Absent, null, number, empty text, duplicate | Rejected | Correct; only absent/empty/blank have persistent cells |
| Per-record gap reader | Absent when required, null, number, empty text, duplicate | Rejected | Correct; most shapes missing from permanent matrix |
| Authored YAML loader, shared by all fields | Invalid YAML inside frontmatter; duplicate keys | Rejected | Correct; document-level rejection must remain in the matrix |
| Emitted catalog and runtime switch lookup | Valid typed and unknown records | Generator controls preserve values; ownership controls distinguish unknown from none | Clean downstream controls |

For completeness, these are the observed shape columns for the newly read switch fields. `R` means rejected, `A` accepted, and `—` the element shape does not apply to a scalar. All columns share rejection of invalid YAML content.

| Shape | `flag` | `value_type` | `value_optional` | `variadic_min` | `aliases` | `attachment` |
| --- | --- | --- | --- | --- | --- | --- |
| Absent | R | R | R on scalar | R on variadic | Allowed without a dependent short-attachment requirement | R |
| Null | R | R | R | R | R | R |
| Wrong whole type | R | R | R | R | R | R |
| One bad element | — | — | — | — | R | R |
| Every element bad | — | — | — | — | R | R |
| Empty | R | R | R for empty text | R | A when no short alias is required | R on scalar; A on no-value switch |
| Duplicate key | R | R | R | R | R | R |

| Shape | `invocation_scope` | `applies_to` | `command` | `description` | `gap` |
| --- | --- | --- | --- | --- | --- |
| Absent | R | R | R on command scope | R | R when unknown; allowed otherwise |
| Null | R | R | R | R | R |
| Wrong whole type | R | R | R | R | R |
| One bad element | R | — | R | — | — |
| Every element bad | R | — | R | — | — |
| Empty | R | R | A for root path, provided it does not duplicate another scope | R | R |
| Duplicate key | R | R | R | R | R |

| Shape | `schema_revision` | `cli_switches` | `cli_switches_gap` |
| --- | --- | --- | --- |
| Absent | Legacy revision, requiring its supported contract | R for revision 2 | R with empty inventory; allowed with records |
| Null | R | R | R |
| Wrong whole type | R | R | R |
| One bad element | — | R | — |
| Every element bad | — | R | — |
| Empty | R | A only with valid gap | R |
| Duplicate key | R | R | R |

Some empty/absent shapes interact with other fields: removing Codex's only short alias while retaining `short_attached` correctly fails, and replacing its `exec` scope with a second root scope correctly fails for duplication. Use separate valid controls for these relationships; do not turn those failures into a blanket prohibition on empty aliases or root paths.

Extend the single fixture-driven permanent matrix, retaining assertions through public generation results. Give each newly read field its own column and cover all applicable shapes; do not create isolated one-off tests for each review example. Keep supported legacy revisions and valid whole-provider gaps as positive controls. The existing tests are compiled by the declared `l1` target and selected by L1; placement is clean.

## Requirement verification

Levels below use the review request's definitions: subprocess and manufactured-PTY tests are **L1**; an actual terminal emulator or multiplexer is **L2**. Each acceptance requirement is accounted for, even where several share a test family.

| Acceptance requirements | Behavior | Strongest relevant evidence | Assessment |
| --- | --- | --- | --- |
| 1–2 | Exact headline tail and authored separator | L1 binary `provider_tail_launch`, across composition commands | Pass in tested cases |
| 3, 6 | File ordering and opaque provider operands | L1 partition/ownership tests | Appropriate level |
| 4–5, 13 | Setter/Claudine flag ownership; no synthetic boundary | L1 partition and `provider_tail_ownership` | Appropriate level |
| 7, 25 | Tail once through retry, proxy, resume, mixed boundary, multi-provider steps | L1 binary `provider_tail_launch`, including repeated resume switches | Appropriate level |
| 8 | Notice deduplication, parallel tasks, quiet/silent suppression | L1 binary `provider_tail_notice` and library notice-state tests | Appropriate for the notice content/count contract |
| 9, 28 (redaction) | No secret in display, metadata, echoed diagnostics | L1 existing secret tests plus broader binary probes | Fails: embedded credentials; see first finding |
| 10, 28 (correlation) | Rejection classification, stdout/stderr, injected switch, opaque operands, other causes | L1 binary `provider_tail_launch` | Covered for tested signatures; capture excerpt masks keys |
| 11 | Shared direct-wrapper reporting; unchanged child argv | L1 `wrap_direct_argv` and direct notice tests | Exact argv controls pass; shared redaction finding applies |
| 12, 27 (completion) | Shared ownership, no prompt/side effects, invalid lines decline | L1 binary `completion_ownership` | Fails for flag-shaped cursor; word controls pass |
| 14 | Generated aliases/types and consistent explanations | L1 generator matrix, lookup/ownership controls, notice tests | Unknown-type reporting finding applies |
| 15 | Non-UTF-8 refusal | L1 binary and partition tests | Appropriate level |
| 16–18 | Schema precedence, first setter-shaped value, contiguous variadic values | L1 ownership and binary tests | Appropriate level |
| 19 | CLI/authored-agent candidate narrowing | L1 ownership and binary tests | Appropriate level |
| 20 | Ambiguity error and interactive ownership-only decision | L1 binary error tests and two L1 PTY chooser tests | Missing L2 chooser verification |
| 21 | Value-count checks at ownership, static step preflight, proxy/resume/spawn | L1 binary `provider_tail_ownership` and `provider_tail_launch` | Appropriate level |
| 22, 26 | Unknown types, optional scalars, variadic minimum, disagreement | L1 library fixed/compiled catalogs and generator controls | Ownership controls pass; notice and matrix findings apply |
| 23 | Ordered `argv`, reserved name, overrides, propagation, inline nonpersistence | L1 binary `provider_tail_ownership` | Appropriate level |
| 24 | Exact aliases and attached forms; no cluster expansion | L1 ownership and lookup tests | Appropriate level |
| 27 (execution), 29 | Help without file; authored snapshot and source-relative schemas | L1 binary ownership/help/schema controls | Appropriate level for tested cases |

## Checks performed

| Command | Result |
| --- | --- |
| `just test-cli provider_tail` | 65 passed, including an existing nonasserting diagnostic probe |
| `just test-cli completion_ownership --success-output final` | 9 passed, including an existing nonasserting diagnostic probe |
| `just test-library ownership` | 34 passed; filter also selected related existing tests |
| `just test-gen cli_switches` | 8 passed; recipe's signal-fixture check passed |
| `just test-cli level1_ownership_prompt_pty` | 2 passed |
| `just test-cli wrap_direct_argv` | 2 passed |
| `just check-tier-coverage claudine` from repository root | No stranded tests |
| `cargo run --color=never -p claudine-gen -- check` | Generated provider files and catalogs clean; pre-existing stale model-artifact warning |
| Additional isolated shipped-binary probes | Confirmed the reported secret, completion, and unknown-notice behaviors; malformed metadata rejected |

No full-area lint or unrelated L2 suite was run for this documentation-only review. The missing chooser test was established by inspecting the declared targets and their sources; running other terminal tests would not prove this flow. No terminal window was opened or focused, no live provider or network was used, and no commit was created. Cross-OS evidence is left to CI and does not affect this verdict.
