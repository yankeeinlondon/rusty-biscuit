---
$schema: feature-review.yaml
ready: true
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-02T04:22:16-07:00
spec: 2026-07-13-cli-switches/spec.md
implemented: false
description: A **fix** review of `2026-07-13-cli-switches/spec.md`
fix: 2026-07-13-cli-switches/review-3.md
previous: 2026-07-13-cli-switches/review-2.md
findings: []
---

# Review 3: CLI Switches

The fix is **production ready** against the specification. All three unblocked findings from Review 2 are resolved. The review found no remaining functionality, test-level, or performance blocker, and no human decision is needed.

This verdict covers the implementation present in the working tree, including the uncommitted repairs. Production source and tests were not edited during this review. Cross-OS results remain CI's responsibility and are not a readiness condition here.

## Previous findings

Review 2 explicitly records no blocked findings. Nothing needed to be unblocked before the latest implementation. Review 1 also had no blocked findings; its five findings were checked for recurrence as part of this review.

| Review 2 unblocked finding | Verification in Review 3 | Result |
| --- | --- | --- |
| Option-value completion still bypasses ownership errors | The owned-option branch now checks the committed arguments before the option, leaving its unfinished value out of clap validation. The binary matrix checks every separate value option and alias, attached long forms, empty and partial cursors, all three commands, invalid and ambiguous prefixes, and valid controls. A drift guard ties its option inventory to clap's actual surface. | Resolved |
| Composition help still requires a file argument | Binary tests check omitted, missing, and unreadable files, both help spellings, root help, mixed provider arguments, and the authored separator. Required-argument and nested administrative siblings also pass. | Resolved |
| The override input matrix is incomplete | The permanent override matrix covers the entry envelope, catalog discriminators, record fields, scalar optionality, variadic minimum, attachments, scopes, and command paths through public generation results. Its grid guard requires a case for every applicable field/shape cell. The research matrix and documented outcomes were also extended. | Resolved |

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

No finding recurs. The earlier completion and metadata-matrix classes now include the siblings missed in the first repair. Review 1's credential redaction, unknown-type explanation, and chooser-verification findings remain resolved in the focused binary, library, generator, and real-terminal checks.

## Class sweeps

The sweeps checked shared decision points and their command entrypoints, rather than treating a passing reproduction at one command as proof for its siblings.

| Decision or reader sites | Shapes exercised | Observed result | Required result |
| --- | --- | --- | --- |
| Completion: flag, setter-name, setter-value, bare-word, and provider-value cursors; all three composition commands | Missing scalar value, ambiguous candidates, valid provider value, explicit separator | Invalid/ambiguous prefixes offer nothing; valid Claudine slots retain suggestions; provider and opaque slots offer nothing | Matches |
| Completion: every owned separate-value option and alias, global logging level, sequence-only options, and attached long forms | Empty/partial cursor × missing-value, ambiguous, valid, and later-provider-selection prefixes | Earlier ownership errors suppress suggestions, including clap fallback; clean controls retain the baseline suggestions | Matches |
| Help: composition commands, all ten wrappers, root administrative help, nested administrative help | No required argument; missing/unreadable file; `--help` and `-h`; help after authored `--` | Claudine help exits successfully without reading the composition file or launching an agent; opaque help remains provider input | Matches |
| Ownership and launch checks: scalar, optional, numeric, variadic, unknown and absent records | Schema setter precedence, empty/attached values, interrupted runs, candidate disagreements, aliases, exact scopes, reserved `argv`, explicit suffix | Exact ordered assignments; documented ambiguity/errors; opaque suffix bypasses local checks | Matches |
| Launch consumers: compose, inline-compose, sequence steps, retry, proxy, resume, direct wrappers | Headline tail, mixed boundaries, repeated resume switches, changed launch provider | Original arguments retained once; researched mismatch fails before the applicable spawn | Matches |
| Argument displays: dry-run, debug, environment metadata, forwarding notice, correlated excerpts | Embedded/attached credentials and diagnostic echoes | Claudine displays mask recognized secrets; child arguments retain original values | Matches |
| Switch explanations: OpenCode/Kilo applicable and inapplicable scopes, missing records, known Codex alias | Unknown-type and absent-record controls; explicit suffix | Unestablished types are described honestly; researched alias is explained; explicit suffix remains opaque | Matches |
| Interactive ownership chooser: all three composition commands | Both answers in detached tmux, plus L1 PTY controls | Question and choices are captured; answer changes ownership only; actual incompatible Codex launch is refused | Matches |

Raw provider output remains provider output: this verdict concerns the specified Claudine argument displays, metadata, and correlated diagnostic excerpts. It does not claim that arbitrary live provider stderr is redacted.

## Input robustness matrix

Both changed metadata paths have permanent fixture-driven matrices in package **claudine-gen**, [cli_switches.rs](../../gen/tests/l1/cli_switches.rs). They make controlled edits to the frozen Codex research fixture and its generated override catalog, then assert acceptance, rejection, and preserved values through `generate_for_area`, the public generation route used by validation. The unchanged fixture is a positive control. The field/shape outcomes are documented in [Provider metadata](../../docs/topics/provider-metadata.md#switch-metadata-cli_switches).

| Reader or projection | Load-bearing columns checked | Shape coverage and result |
| --- | --- | --- |
| Research frontmatter and revision reader | `schema_revision`, `cli_switches`, `cli_switches_gap` | Absent, null, wrong whole type, bad list elements where applicable, empty, duplicate keys, and invalid content; supported legacy contract and whole-provider-gap controls pass |
| Research switch records | `flag`, `aliases`, `value_type`, `value_optional`, `variadic_min`, `attachment`, `invocation_scope`, `applies_to`, `command`, `description`, `gap` | All applicable matrix shapes pass their defined outcomes; relationship controls distinguish legal empty aliases/root paths/no-value attachments from invalid combinations |
| Override envelope and catalog reader | Entry `value`/`reason`, `researched`, `unknown`, `unknown.gap`, `flag`, `aliases`, `value`, scalar `optional`, variadic `min`, `attachments`, `scopes`, nested `command`, `description`, record `gap` | All applicable matrix shapes pass; additional controls cover extra members, nonmapping entries, canonical order, duplicate spellings, unknown minimum with a gap, and valid whole-provider gap |
| YAML loaders | Duplicate keys and malformed content in research and override files | Rejected before projection; invalid elements are not silently filtered |
| Emitted Rust/catalog and compiled provider inventories | Typed and unknown records, disjoint scopes, deterministic ordering, explicit gaps | Generation preserves the controls; every compiled provider has records or a stated gap; generated-artifact check passes |

The override grid checks completeness as well as behavior: each required cell must appear exactly once and agree with the outcome table. The strict envelope now rejects unrecognized entry keys and identifies a wrong-type reason accurately. No further permissive-reader defect was observed in these paths.

## Requirement verification

Levels use this review's definitions: in-process, subprocess, and manufactured PTY tests are **L1**; capture from detached tmux is **L2**. Each acceptance criterion is accounted for below. The chooser has no new physical-key or terminal-input-encoder requirement needing L3.

| Acceptance criteria | User-facing requirement | Strongest relevant verification | Assessment |
| --- | --- | --- | --- |
| 1–2 | Exact headline arguments and explicit separator | L1 binary `provider_tail_launch`, all composition entrypoints | Pass |
| 3, 5–6, 13, 15 | File ordering, owned switches, opaque operands, no synthetic boundary, non-UTF-8 refusal | L1 partition tests and binary ownership/notice tests | Pass; appropriate level |
| 4, 16–19 | Interleaved setters, schema precedence, contiguous variadic values, authored provider candidates | L1 ownership and binary `provider_tail_ownership` | Pass; appropriate level |
| 7, 25 | Sequence/retry/proxy/resume carry-over, mixed boundaries, repeated switches | L1 binary `provider_tail_launch` | Pass; appropriate level |
| 8 | Notice once per provider/tail pair, parallel tasks, quiet/silent suppression | L1 binary `provider_tail_notice` and notice-state tests | Pass; content/count contract verified |
| 9, 28 redaction | Secret-free Claudine argument displays and diagnostic excerpts; unchanged child values | L1 embedded-credential binary sweep and shared secret tests | Pass; appropriate level |
| 10, 28 correlation | Native rejection correlation, stdout/stderr, injected-switch exclusion, operand-only tail, unrelated failures, exit status | L1 binary `provider_tail_launch` | Pass; appropriate level |
| 11 | Shared direct-wrapper reports and unchanged child argv | L1 notice tests and `wrap_direct_argv` | Pass; appropriate level |
| 12, 27 completion | Shared ownership, no suggestions on invalid/ambiguous/provider/opaque slots, no writes or prompts | L1 binary `completion_ownership`, including owned-option matrix | Pass; appropriate level |
| 14, 22 | Generated aliases/types and honest unknown-switch handling | L1 generator matrices, ownership and four-command notice tests | Pass; appropriate level |
| 20 | Ambiguity chooser displays choices and changes ownership only; noninteractive guidance | L2 captured tmux chooser, both answers × three commands; L1 PTY/error/completion controls | Pass; appropriate level |
| 21, 24, 26 | Resolved-provider checks, exact/attached forms, optional/minimum/unknown semantics | L1 ownership and binary preflight/launch tests | Pass; appropriate level |
| 23 | Ordered `argv`, reserved setter rejection, caller override, propagation, inline nonpersistence | L1 binary `provider_tail_ownership` | Pass; appropriate level |
| 27 help and resolution, 29 | Help without a file; authored snapshot; source-relative/union schemas; final validation after overrides | L1 binary ownership/help tests and completion controls | Pass; appropriate level |

The L1 modules are declared in their consolidated binaries. The chooser is declared in the `level2` binary, its `terminal-tests` feature is in CI metadata, and `just check-tier-coverage claudine` reports no stranded tests. No test was added or renamed during review.

## Checks performed

| Command | Result |
| --- | --- |
| `just test-cli completion_ownership` | 16 passed |
| `just test-cli provider_tail` | 72 passed |
| `just test-cli help_needs_no_required_argument` | 1 passed; includes wrapper and administrative siblings |
| `just test-cli argv::` | 84 passed; includes partition/normalizer controls and direct-wrapper argv tests |
| `just test-cli wrap_direct_argv` | 2 passed |
| `just test-cli level1_ownership_prompt_pty` | 2 passed |
| `just test-library ownership` | 35 passed; filter also selects related existing tests |
| `just test-library secrets::` | 12 passed |
| `just test-gen cli_switches` | 8 passed; signal-fixture check passed |
| `just test-gen` | 204 passed; signal-fixture check passed |
| `just test-l2 ownership_prompt_capture` | 1 CLI L2 test passed, exercising six chooser runs; generator filter selected zero tests |
| `just check-tier-coverage claudine` | No stranded tests |
| `cargo run --color=never -p claudine-gen -- check` | Generated artifacts clean; existing stale model-catalog warning |

The successful builds also emitted a macOS linker warning about compact unwind-table size. Neither that warning nor the pre-existing model-catalog age warning indicates a CLI-switch defect. Full-area lint and unrelated terminal suites were not rerun for this review-only edit. The chooser ran in detached panes; no OS keyboard injection or live provider was used. No formatting command or commit was run. The fix directory remains in place for the author's review closure.
