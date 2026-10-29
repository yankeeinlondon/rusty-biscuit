---
$schema: feature-review.yaml
ready: false
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-02T02:34:22-07:00
spec: 2026-07-13-cli-switches/spec.md
implemented: true
implemented_by: claude/opus
log: claudine/fixes/2026-07-13-cli-switches/log.md
description: A **fix** review of `2026-07-13-cli-switches/spec.md`
fix: 2026-07-13-cli-switches/review-2.md
previous: 2026-07-13-cli-switches/review-1.md
next: 2026-07-13-cli-switches/review-3.md
findings:
  - title: Option-value completion still bypasses ownership errors
    priority: medium
  - title: Composition help still requires a file argument
    priority: medium
  - title: The override input matrix is incomplete
    priority: medium
---

# Review 2: CLI Switches

The fix is **not production ready**. Three medium-priority findings remain: completion still suggests option values after invalid provider arguments, composition help fails without a file argument, and the newly validated metadata override path lacks the full permanent input matrix required by this review. None requires a human decision.

This review includes commit `3cf8fd68a` and the uncommitted repairs present when review began. Production source and tests were left unchanged. Only review metadata and this report were edited.

## Previous findings

The previous review has a single `## Findings` section, rather than separate blocked and unblocked sections. All five findings were unblocked; its frontmatter records no human-review requirement or blocked finding. There was nothing awaiting an unblocking decision.

| Previous finding | Review 2 result |
| --- | --- |
| Embedded credentials escape argument redaction | Resolved. Shared recognition masks embedded assignments and attached values; asserting binary tests cover dry-run, debug, metadata, notices, and correlated excerpts while preserving child arguments. |
| Flag completion bypasses ownership errors | Original flag-cursor reproduction is fixed. The same class remains in Claudine option-value slots; see the first finding. |
| Unknown switch types are described as recognized | Resolved. Unknown records receive the unestablished-type explanation. Tests cover OpenCode and Kilo, applicable and inapplicable command scopes, absent records, known aliases, explicit tails, and all four launch commands. |
| The ambiguity chooser lacks real-terminal verification | Resolved. A declared L2 test captures the chooser and checks both ownership answers through `compose`, `inline-compose`, and `sequence` in detached tmux panes. It passed during this review. |
| The permanent input matrix omits load-bearing fields and shapes | The research matrix now covers the previously missing columns. The repair also added an override reader with only partial matrix coverage; see the third finding. |

The implementation log explicitly leaves raw streamed provider stderr unchanged. I accept that scope distinction: the spec promises redaction of Claudine argument displays, metadata, and correlated diagnostic excerpts. This verdict does not claim that arbitrary provider output is redacted.

## Unblocked Findings

### Medium — Option-value completion still bypasses ownership errors

**Defect class:** completion suppresses suggestions after ownership errors for some cursor categories, but skips the same check when the cursor is a Claudine option's value.

In package **claudine-cli**, [cursor_is_claudines](../../cli/src/completion/engine/ownership.rs#L39), which decides whether completion may offer suggestions, returns `true` immediately for a trailing `CallerArgument::ClaudineOption`. The partitioner leaves that marker after extracting an owned option and its value. Consequently, earlier arguments never reach the shared ownership check, and [clap_dynamic_fallback](../../cli/src/completion/engine/mod.rs#L356) offers option values on a line that execution rejects.

Use the existing plan fixture declaring `$schema: {phase: number, mode: enum(fast, slow)}`. For example:

```sh
claudine __complete --current 7 -- claudine compose plan.md --codex -c phase=2 --exclude co
```

This prints `codex`, although `phase=2` is a document setter and leaves Codex's `-c` without a value. Changing `--exclude co` to `--on-rate-limit a` prints `abort`. The previous repair's cursor matrix omitted owned-option values.

The sweep ran 186 combinations of command, preceding argument state, and option-value slot, followed by alias checks. Both an empty cursor and representative partial values were checked. Every row below applies to all three composition commands unless stated otherwise.

| Site / cursor slot | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| `--provider` | Missing-value prefix `--codex -c phase=2`; cursor empty or `co` | Provider names / `codex` | No suggestions |
| `--exclude` | Same prefix and cursor | Provider names / `codex` | No suggestions |
| `--on-rate-limit` | Same prefix; cursor empty or `a` | Policy names / `abort` | No suggestions |
| Global `--debug`, after the file | Same prefix; empty cursor | Logging levels | No suggestions |
| `sequence --budget-ledger` | Same prefix; empty cursor | Fixture files and directories | No suggestions |
| All five slots above | Ambiguous prefix `-c low`, no authored agent; empty cursor | Same suggestions | Decline where ownership remains unresolved; an actual provider selection may resolve ambiguity, but must still reject a missing-value prefix |
| All five slots above | Valid prefix `--codex -c low` | Expected option-value suggestions | Preserve valid completion |
| `--include`, `--model`/`-m`, `--output`/`-o`, `--append-system-prompt`/`--asp`, `--replace-system-prompt`/`--rsp`, `--timeout`/`-t`, `--step-timeout`, `--stall-timeout`, `--operation`/`--op`, `--set`, `--use`, `--max-iterations`; sequence `--fail-fast` | Invalid prefix; empty option-value cursor | No suggestions | Clean observable controls; these slots still use the bypass, but currently offer no candidates |
| Flag cursor `--mod`, setter `mode=`, bare word `ph`, provider value cursor | Missing-value or ambiguous prefix | No suggestions | Clean repaired controls |
| Attached `--provider=co`, `--exclude=co`, `--on-rate-limit=a`, `--debug=` | Missing-value prefix | No suggestions | Clean attached-form controls |
| Any cursor after authored `--` | Invalid prefix or valid prefix | No suggestions | Clean opaque-tail control |

Validate the preceding provider arguments before returning from the owned-option branch. Keep an unfinished owned value out of strict clap validation while checking the provider tail; otherwise a repair could suppress legitimate completion of the option itself. Extend the existing fixture-driven matrix with every value-bearing option and alias from the owned surface, including globals and sequence-only options. L1 binary verification is appropriate.

### Medium — Composition help still requires a file argument

**Defect class:** help is represented as an ordinary Boolean and evaluated after required-argument validation, so a help request fails before it can display help.

In package **claudine-cli**, [hoist_composition_help](../../cli/src/argv/rule4_help_hoist.rs#L34) moves `--help` or `-h` to the root. [parse_cli_from](../../cli/src/main.rs#L117) then validates the still-required composition positional before the custom root help handler can run. Thus `claudine compose --help` exits 2 with “required arguments were not provided.” The same happens with `inline-compose` and `sequence`.

The spec requires help to work without a readable composition file. The existing [help_opens_no_composition_file](../../cli/tests/l1/provider_tail_ownership.rs#L474) supplies a nonexistent filename, which avoids opening a file but does not exercise omission of the file argument. This is an existing parsing interaction still left open by the specification, rather than a regression introduced by the latest repairs.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| `compose` | `--help`, `-h`, or root `--help compose`, without a file | Exit 2; missing `<ARG>` | Exit 0 and help; no file read |
| `inline-compose` | Same three help forms | Same failure | Same help behavior |
| `sequence` | Same three help forms | Same failure | Same help behavior |
| All three commands | `missing.md --help`, or `--help missing.md` | Exit 0 without opening the file | Clean control |
| All ten direct wrappers | `<provider> --help`, without a prompt | Exit 0 and wrapper help | Clean siblings |
| Administrative `hooks`, `providers`, `actions` | `--help`, without other arguments | Exit 0 and help | Clean injected-help controls |
| All three commands | `missing.md -- --help` | Normal missing-document failure, rather than Claudine help | Clean boundary control: the provider owns this help token |

Handle a Claudine help request before enforcing the composition file requirement, retaining the authored `--` boundary. Extend the binary help test to cover the omitted-file forms across all three commands. No real provider or terminal is needed.

## Input robustness matrix

### Medium — The override input matrix is incomplete

**Defect class:** a file reader's acceptance rules are checked by selected malformed examples instead of a permanent matrix covering every load-bearing field and applicable shape on each input path.

In package **claudine-gen**, [check_cli_switch_catalog](../../gen/src/generate/coerce/cli_switches.rs#L104) now validates catalog-shaped overrides before emission. This closes the previously permissive sibling path. However, [an_override_is_held_to_the_research_rules](../../gen/tests/l1/cli_switches.rs#L495) has 27 selected cases rather than the full per-field matrix required for this changed reader. The research matrix cannot substitute for these cases: the override adapter reads different field names and shapes, transforms them into research records, and rejects noncanonical round trips.

For example, override `attachments` has an empty-array case but no absent, null, wrong whole type, bad-element, or duplicate-key cases. Override `scopes` similarly lacks most shape cases, and the nested `command`, scalar `optional`, and variadic `min` columns are incomplete or absent. Such gaps allow a future adapter edit to bypass the rules while every research-side test stays green.

I copied the actual Codex generator inputs and catalog into a temporary area, edited one field per cell, and invoked `claudine-gen --area <fixture> validate codex`. The original catalog and whole-provider gap passed; malformed shapes were rejected. Variadic and unknown-record controls were separately validated before their edits. This is a **permanent-test gap**, not a newly observed permissive-parser defect.

| Sibling reader / projection | Shape tested | Observed result | Expected result / permanent coverage |
| --- | --- | --- | --- |
| Research document revision and switch-record reader | Existing 117-cell fixture matrix, including the newly added fields | Passes | Research-side omissions from review 1 resolved |
| Frozen legacy contract | Revision-1 fixture and missing-contract control | Passes / rejects as expected | Clean |
| Research YAML loader | Duplicate keys and invalid YAML in the permanent matrix | Rejected | Clean |
| Override YAML loader and catalog adapter | Field/shape sweep with independently passing catalog, variadic, and unknown-record controls | Correct acceptance/rejection | Add the missing permanent columns and cells |
| Generated Rust and catalog projection | Unedited generator fixture and every compiled provider | Typed output preserved; researched records or stated gap present | Clean |
| Shipped `validate` command | Copied-input positive control and every manual edit | Uses the same generation decisions | Clean public verification route |

The following tables define the tested override outcomes. `R` means generation rejects the shape; `A` means it accepts and preserves the valid meaning; `—` means an element shape does not apply. The format is YAML, including its JSON-compatible flow mappings. All duplicate-key cases and trailing invalid YAML were rejected by the file loader.

| Shape | `researched` | `flag` | `aliases` | `value` | scalar `optional` | variadic `min` |
| --- | --- | --- | --- | --- | --- | --- |
| Absent | R | R | R | R | R | R |
| Null | R | R | R | R | R | R |
| Wrong whole type | R | R | R | R | R | R |
| One bad element | R | — | R | — | — | — |
| Every element bad | R | — | R | — | — | — |
| Empty | R | R | R with retained short attachment | R | R | R |
| Duplicate key | R | R | R | R | R | R |
| Invalid trailing content | R | R | R | R | R | R |

`min: 123` is a valid integer, not a wrong-type case; it was accepted. Text, array, object, and Boolean minimums were rejected. Empty aliases are legal when the dependent short-attachment requirement is removed; the research matrix already proves that relationship, and the override matrix should preserve it too.

| Shape | `attachments` | `scopes` | nested `command` | `description` | record `gap` when unknown | `unknown.gap` |
| --- | --- | --- | --- | --- | --- | --- |
| Absent | R | R | R | R | R | R |
| Null | R | R | R | R | R | R |
| Wrong whole type | R | R | R | R | R | R |
| One bad element | R | R | R | — | — | — |
| Every element bad | R | R | R | — | — | — |
| Empty | R for scalar switch | R | A for root path | R | R | R |
| Duplicate key | R | R | R | R | R | R |
| Invalid trailing content | R | R | R | R | R | R |

A known record's canonical `gap: null` is valid; null in an unknown record is rejected. An empty command path means the root entrypoint and is valid when it does not duplicate another scope. A no-value record accepts empty attachments. Include these positive controls alongside rejection cases. Also cover the outer `unknown` and `researched` discriminators, missing/invalid records, extra members, and canonical ordering; existing selected cases cover only part of that outer shape.

Extend the single override fixture matrix to every column above, with assertions through public generation results, and document the outcome table for both research and override inputs. Reuse the existing fixture and mutation helpers rather than adding isolated regression tests. Keep this in the declared L1 target.

## Blocked Findings

None. All three repairs can be implemented and verified by an agent.

## Recurrence

- **Review 1, “Flag completion bypasses ownership errors”:** the repair swept flag, setter, bare-word, provider-value, and opaque-tail cursors but omitted Claudine option-value cursors. It should also have checked the early `ClaudineOption` return and clap fallback across all owned values, including global logging levels and sequence's budget path. The first finding carries the complete observed sibling list.
- **Review 1, “The permanent input matrix omits load-bearing fields and shapes”:** the repair completed the research columns and discovered the override sibling, but gave that changed path selected examples rather than its own complete matrix. It should have swept the catalog discriminators, record fields, scalar/variadic payloads, and scope paths through the override adapter. The third finding lists these columns together, including clean reader and emission siblings.

## Requirement verification

Levels use this review request's definitions. Binary subprocess tests and manufactured PTY input are L1; detached tmux capture is L2.

| Acceptance criteria | User-facing behavior | Strongest applicable verification | Assessment |
| --- | --- | --- | --- |
| 1–7, 13, 25 | Exact tails, boundaries, interleaved setters/options, retry/proxy/resume and sequence carry-over | L1 `provider_tail_launch`, `provider_tail_ownership`, partition/ownership tests | Appropriate; focused tests pass |
| 8 | One notice per provider/tail pair; quiet/silent suppression | L1 `provider_tail_notice`, notice-state tests | Appropriate; passes |
| 9, 28 redaction | Displays, metadata and correlated excerpts mask secrets; child receives originals | L1 embedded-credential binary sweep and shared recognition tests | Appropriate; prior finding resolved |
| 10, 28 correlation | Native rejection correlation, injected-switch exclusion, exit-code preservation | L1 `provider_tail_launch` | Appropriate; passes |
| 11 | Shared direct-wrapper reporting with unchanged child argv | L1 notice tests and `wrap_direct_argv` | Appropriate; passes |
| 12, 27 completion | Shared ownership; invalid/ambiguous input declines; read-only operation | L1 `completion_ownership` plus shipped-binary sweep | Remaining option-value failure |
| 14, 22 reporting | Catalog aliases and unknown-type explanations | L1 generator, runtime reporting and four-command notice tests | Appropriate; prior reporting finding resolved |
| 15 | Non-UTF-8 arguments refused | L1 partition and binary tests | Appropriate; focused tests pass |
| 16–19, 21–24, 26, 29 | Schema precedence, authored candidates/snapshot, scalar/variadic/attached values, positionals and launch checks | L1 ownership and binary tests; generator matrices | Runtime tests pass; override matrix gap remains |
| 20 | Ambiguity chooser renders choices; answer changes ownership only | L1 PTY plus L2 tmux, both answers × all three commands | Appropriate; L2 ran and passed; no new terminal-encoder contract requiring L3 |
| 27 help | Help without opening or supplying a file | L1 existing test plus no-file binary sweep | No-file forms fail |

The L2 module is declared in `cli/tests/level2/main.rs`, its target enables `terminal-tests`, and CI metadata includes that feature. The tier-coverage check reports no stranded tests. No test was added or renamed during review.

## Checks performed

| Command / check | Result |
| --- | --- |
| `just test-cli provider_tail` | 71 passed |
| `just test-cli completion_ownership` | 10 passed |
| `just test-library ownership` | 35 passed |
| `just test-library secrets::` | 12 passed |
| `just test-gen cli_switches` | 8 passed; signal-fixture check passed |
| `just test-cli wrap_direct_argv` | 2 passed |
| `just test-cli level1_ownership_prompt_pty` | 2 passed |
| `just test-l2 ownership_prompt_capture` | 1 CLI L2 test passed, covering six chooser runs; generator filter selected 0 tests |
| `just check-tier-coverage claudine` | No stranded tests |
| `cargo run --color=never -p claudine-gen -- check` | Generated outputs clean; existing stale model-artifact warning |
| Isolated shipped-binary completion/help probes and generator-input edits | Confirmed the findings and clean controls described above |

Terminal tests used the canonical recipe and background harnesses; no OS keyboard injection was performed. No live provider, network research, production edit, formatting command, or commit was needed. Full-area lint and unrelated terminal suites were not rerun for this review-only change. Cross-OS evidence remains CI's responsibility and does not affect readiness.
