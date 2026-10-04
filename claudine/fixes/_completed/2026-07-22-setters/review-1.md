---
$schema: feature-review.yaml
ready: true
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-02T10:37:39-07:00
spec: 2026-07-22-setters/spec.md
implemented: false
description: A **fix** review of `2026-07-22-setters/spec.md`
fix: 2026-07-22-setters/review-1.md
findings:
  - title: Existing shell-approval terminal capture fails on its second invocation
    priority: medium
---

# Review 1: Setters

The fix is **production ready** against its specification. No setter-specific correctness, coverage, ergonomics, or performance finding remains. No human decision is required. This verdict covers the current implementation, not completion of the broader CLI-switch dependency.

Only this review and the requested spec frontmatter are changed by the review. Production code and tests are unchanged.

## Findings

No finding blocks the setter fix. There is no earlier implementation review of this fix, so no finding recurs.

### Medium — Existing shell-approval terminal capture fails on its second invocation

**Defect class:** a repeated terminal-capture check returns no complete approval block for its second invocation, preventing the area-wide L2 gate from completing.

This is **outside the setter specification**: its fixture contains one document-body shell command and no caller setter or forwarded provider switch. In claudine-cli, [level2_dry_run_approval_prompt_matches_normal_mode_in_tmux](../../cli/tests/level2/level2_dry_run_approval_capture.rs) failed in both sequential `just test-l2` runs at the comparison on line 236. The normal capture contained the complete approval menu; the dry-run capture extracted an empty list. The root cause is not established: this observation does not prove the shipped dry-run handler omitted the prompt.

The sibling sweep covers the approval-region reader and capture paths in that file and the raw-PTY equivalents in claudine-cli's [level1_dry_run_pty.rs](../../cli/tests/l1/level1_dry_run_pty.rs). Both use a staged Markdown fixture containing the same single unapproved shell-command shape, varying only dry-run mode. The raw-PTY checks passed in the full sequential L1 run.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Real tmux capture and `prompt_region`, normal invocation | Fixture without `--dry-run` | Complete approval header, command, and five choices | Complete approval block |
| Same tmux pane and reader, second invocation | Same fixture with `--dry-run` | Empty extracted block in two sequential suite runs | Same complete block as normal mode |
| Fresh raw-PTY captures and `approval_prompt_region` | Same shell-command shape, normal and dry-run modes | Parity test passes | Matching complete approval blocks |
| Raw-PTY approval-and-allow path | Dry run, choose Allow once | Test passes | Approval appears and permitted command executes |

Investigate the second invocation's full frame and synchronization before changing product behavior. The capture waits for a final-menu marker, then the reader independently locates the header and end; a marker alone does not prove the returned frame contains the complete current block. Add a complete-block assertion with the full captured frame to distinguish a capture problem from a handler regression. This unrelated failure is recorded but does not change setter readiness; the full L2 gate is not reported as passing.

## Implementation and sibling sweep

The claudine library's [own_arguments](../../lib/src/composition/ownership.rs) assigns setter and provider tokens once, preserving their original order and adjacency. In claudine-cli, [own_caller_arguments](../../cli/src/commands/compose/ownership.rs) reads the authored document before overrides, and [parse_composition_positionals](../../cli/src/commands/compose/setters.rs) feeds the existing caller override path. Both compose modes and sequence use that same path. This avoids a second merge mechanism and preserves caller provenance.

The sweeps cover the scalar, variadic, no-value, attached-value, unknown-switch, and explicit-separator decisions, including clean sibling sites. The controlled library table asserts both ordered setter tokens and exact forwarded tokens; CLI partition tests additionally use the generated catalog. Compiled-binary tests verify effective prompts and child arguments rather than only parser return values.

| Site and shape tested | Observed result | Expected result |
| --- | --- | --- |
| Scalar switch: `--codex -c x=y phase=2` | Only `-c x=y` forwarded; `phase=2` applied | Matches |
| No provider or authored agent hint: `-c x=y phase=2` | Candidate union forwards `x=y`; `phase` remains a setter; incompatible Claude launch rejected | Matches |
| Claudine-owned `--yolo`, followed by `phase=2` | Option stays with Claudine; setter applied | Matches |
| Generated provider-only no-value switch: Codex `--ephemeral phase=2` | Only `--ephemeral` forwarded | Matches |
| Variadic switch: `--claude --add-dir a b phase=2` | `a b` forwarded; setter ends the run | Matches |
| Variadic switch: `--claude --add-dir x=y phase=2`, neither key declared | First adjacent setter-shaped value forwarded; second applied | Matches |
| Unknown switch: `--frobnicate phase=2` | Unknown switch forwarded; setter applied | Matches |
| Attached value: `--config=x=y phase=2` | Attached token preserved intact; setter applied | Matches |
| Claudine option interruption: `-c x=y -m gpt5 phase=2`; interrupted variadic run | Claudine option ends the value run; later words cannot reconnect | Matches |
| Declared `phase` immediately after `-c`, with and without later `x=y` | Missing-value error identifies switch, parameter, provider, and remedy; no launch | Matches |
| Inline mapping, second root-union arm, source-relative external schema, raw JSON Schema | Declared top-level names protect the setter; source-relative decoy is not selected | Matches |
| Templated schema with contested value; unreadable schema | Error before launch; no expression evaluation to establish ownership | Matches |
| Caller overrides of `agent` or `$schema` | Authored snapshot controls ownership; later preparation uses overrides | Matches |
| Authored `--`, including declared keys and incomplete provider switches after it | Boundary consumed; suffix forwarded unchanged and exempt from local missing-value checks | Matches |
| Shared grammar: ASCII keys, hyphens, underscores, digits after first character; dotted and non-ASCII keys | Setter parser, ownership, and completion agree; dotted keys follow ordinary argument rules | Matches |
| Numeric, boolean, empty, and additional-`=` values; repeated keys around switches and `--set` | Existing JSON5/string meanings preserved; last shorthand wins; shorthand outranks `--set` | Matches |
| Reserved `argv` through shorthand and `--set` | Rejected with bare-word guidance | Matches |
| Compose, inline-compose, and two sequence steps | Effective launch input contains the setter; child arguments do not; inline overlay is not persisted | Matches |
| Retry, resume, and proxy adoption | Caller setter retained; provider switch forwarded once per launch; no reclassification | Matches |
| Caller file reference through direct and proxy launches; sequence reserved overlays | Caller-relative anchoring and established overlay precedence retained | Matches |
| Switch or separator before the file | Existing ordering errors retained | Matches |

Primary executable evidence is in claudine-cli's [setter_after_switch.rs](../../cli/tests/l1/setter_after_switch.rs), [provider_tail_ownership.rs](../../cli/tests/l1/provider_tail_ownership.rs), [provider_tail_launch.rs](../../cli/tests/l1/provider_tail_launch.rs), and [compose_caller_file_provenance.rs](../../cli/tests/l1/compose_caller_file_provenance.rs). The controlled decision table is in the claudine library's [ownership setter tests](../../lib/src/composition/ownership/tests/setters.rs); generated-catalog partition checks are in claudine-cli's [partition tests](../../cli/src/argv/partition/tests.rs).

## Verification level by requirement

| User-facing requirement | Strongest relevant verification | Appropriate level |
| --- | --- | --- |
| Setter after a provider switch changes compose output while preserving switch values | L1 compiled CLI, recorded child arguments and stdin; dry-run control and reproduction | L1 |
| Inline setter changes launch input without persisting the overlay | L1 compiled CLI, recorded stdin and source-file assertions after agent edits | L1 |
| Every applicable sequence step receives the setter | L1 compiled CLI, two recorded launches and composed prompts | L1 |
| Retry, resume, and proxy retain caller inputs and original ownership | L1 compiled CLI, scripted failures, recorded launches, and success-state probes | L1 |
| File-reference anchoring and reserved sequence precedence survive routing | L1 hermetic filesystem and compiled CLI fixtures | L1 |
| Schema precedence rejects missing/contested values before launch | L1 shared classifier plus compiled CLI, no-launch and no-lifecycle assertions | L1 |
| Types, duplicate precedence, `--set`, grammar, and reserved `argv` | L1 public ownership results, parser/merge tests, and compiled CLI prompt assertions | L1 |
| Authored separator and ordering errors remain intact | L1 compiled CLI and exact token assertions | L1 |

The fix adds no keyboard, paste, mouse, scrolling, or terminal-style promise. L1 is therefore sufficient for its behavior. The dependency's interactive chooser is separate from this fix; the area's existing L2 suite is also run as a regression gate.

New integration tests are declared by [the L1 target](../../cli/tests/l1/main.rs), which is registered in [claudine-cli's manifest](../../cli/Cargo.toml). Their names carry no higher-tier marker. The recorder's Unix-only launch tests and the two portable binary tests are correctly selected at L1. Cross-OS evidence is not a readiness condition.

## Input robustness and documentation

This fix changes token ownership, error context, and delegation to the existing setter grammar. It adds or changes no configuration/file-format reader or deserializer, so the file-format robustness matrix does not apply to this change. Existing schema resolution and switch-metadata readers belong to the dependency. Setter-token shapes are explicitly exercised above, including empty values, additional `=`, duplicate keys, invalid key shapes, unavailable schemas, and the opaque suffix.

The current [composition documentation](../../docs/topics/composition.md#setters-after-a-provider-switch), CLI README, argument-normalization topic, and claudine skill explain the changed routing, schema priority, unchanged provider values, override precedence, propagation, and `--` escape hatch. The spec's historical statements about the old implementation remain a snapshot; they were not treated as current behavior or rewritten during this review.

The shared grammar and catalog lookup keep the implementation small. No concrete performance problem or lower-risk optimization was found.

## Checks run

- `just test`: **8,280 passed, 9 skipped**, sequential final run.
- Focused L1 runs: **12** setter end-to-end tests, **37** ownership tests, **80** provider-tail tests, and **18** setter-named tests passed. These groups overlap and are not an additional unique-test count.
- `just lint`: passed.
- `just check-tier-coverage claudine`: **0 stranded tests**.
- `just test-l2`: **not green**. Two sequential runs stopped on the out-of-scope shell-approval capture finding above: first **69 passed, 1 failed**; confirmation **67 passed, 1 failed**. Fail-fast left the remaining CLI tests and generator L2 tests unexecuted.

Early concurrent recipe runs replaced the shared CLI executable with builds lacking another tier's test features. That caused missing diagnostic snapshots and a missing teardown test hook. Those runs are not evidence of a product defect; verification was repeated with recipe execution serialized.
