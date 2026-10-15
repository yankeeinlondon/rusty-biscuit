---
$schema: feature-review.yaml
ready: true
findings: []
observations:
  - The build-cache helper still claims to mirror wt validation
human_review: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-11T20:33:30-07:00
spec: 2026-10-10-valid-paths/spec.md
implemented: false
description: "A **fix** review of `2026-10-10-valid-paths/spec.md`"
fix: 2026-10-10-valid-paths/review-1.md
---

# Valid Paths — Review 1

The fix is production ready. No high, medium, or low findings were identified.
The original wrong-OS path incident is prevented before directory creation,
configuration writing, branch creation, or worktree registration. Invalid
configuration now produces a failure exit status and recovery instructions
that identify the setting the user must change.

## Scope and assessment

Reviewed the specification, implementation plan, recorded departures, changed
library and CLI code, new tests, and corresponding documentation. The review
followed the worktree and rust-testing skills. Unrelated changes in the same
branch were excluded.

In the worktree package,
[`validate_base_dir`](../../lib/src/config.rs:106), which checks base-directory
syntax, uses the host's absolute-path check before classifying foreign syntax.
It preserves accepted spelling and performs no filesystem access. Environment
resolution, JSON resolution, configuration writing, and both setup-choice
paths use that validation before their respective filesystem operations.
Non-Unicode values cannot become an unset override or lossy persisted path.

The worktree-cli package's
[`choose_base_dir`](../../cli/src/commands/create.rs:275), which manages setup
choices, retries invalid values, stops on filesystem errors, and saves only an
accepted choice. Its terminal-prompt adapter is thin. Error rendering escapes
values, quotes configuration-link attributes, and returns each error to the
single renderer in `main`; rejected creation emits no shell `cd:` instruction.
WSL discovery occurs only while rendering a foreign-path rejection.

Two recorded departures are reasonable: rejecting duplicate `base_dir` keys
instead of the plan's last-value behavior avoids an ambiguous configuration;
adding a contextual unreadable-directory error preserves the actual I/O cause.
Both behaviors are tested and documented. Neither warrants adding compatibility
code in this repository's current lifecycle.

## Findings

None.

## Requirement verification

Every requirement in this fix is appropriately verified at Level 1. The spec
explicitly calls for pure logic and isolated filesystem/subprocess tests;
it introduces no terminal encoding, keyboard-event, or visual-style requirement
that needs Level 2 or Level 3 verification.

| Requirement | Level and evidence reviewed |
| --- | --- |
| Host-absolute acceptance, foreign syntax, native Windows special spellings | L1: native validator table and foreign-syntax helper tests in the worktree configuration module; target-specific acceptance remains part of CI |
| Blank, relative, quoted, unexpanded, and space-preserving inputs | L1: validator tests across all three origins |
| Environment priority and empty-value fallback | L1: public resolver tests using guarded fixture homes, plus the CLI invalid-override test |
| Missing paths, files, repository roots, nested directories, symlinks, and I/O causes | L1: isolated configuration tests, including both `.git` file and folder forms and an injected permission-denied result |
| Lossless Unicode handling and write protection | L1: target-specific non-Unicode environment/save tests, unchanged-config assertions, absent-config assertions, and save/reload tests |
| Setup validation, retry, cancellation, creation, and failure | L1: scripted setup tests through the same choice loop used by the prompt adapter |
| Noninteractive refusal and failure exit status | L1: interactivity truth-table tests and closed-stdin CLI tests |
| Incident mutation boundary and source-aware recovery | L1: isolated CLI tests assert repository entries, branches, worktree registrations, config bytes, stderr, and stdout |
| Single rendering, literal markup, and quoted links | L1: CLI error tests and rendered-markup tests |
| Conditional WSL mount suggestion | L1: injected native/WSL runtime tests, including drive-relative, network-share, and namespace exclusions |

The JSON reader has one load-bearing field, `base_dir`. Its matrix starts from
configuration produced by the actual writer and observes resolution results:

| Input shape | Verified result |
| --- | --- |
| Unedited control | Resolves the fixture directory |
| Absent field | Invalid configuration format |
| Explicit null | Invalid configuration format |
| Wrong whole-field type: number, object, array | Invalid configuration format |
| Wrong individual/all element types | Not applicable to a scalar string; array input is rejected as a whole |
| Empty string or whitespace-only string | Empty-directory error with configuration origin |
| Duplicate `base_dir` | Invalid configuration format; documented departure from the plan |
| Trailing invalid content or non-UTF-8 bytes | Invalid configuration format |
| Relative, home-relative, or foreign string | Corresponding source-aware syntax rejection |
| Restored control | Resolves the fixture directory again |

No permissive defaults or silent filtering appear on this field's read path.
The new integration files use Cargo's automatic test discovery, and the unit
tests are compiled into their declared library/binary targets. Their names
keep them in Level 1, with no feature gate withholding the new coverage.

Verification performed on macOS during this review:

- `just test` from `worktree/`: **1,173 passed**, 14 skipped by the existing
  suite selection; two slow reports occurred in unchanged tests.
- `just lint` from `worktree/`: passed for both packages.
- `just check-tier-coverage worktree`: passed, no stranded tests.

Cross-OS runs reported in the implementation log were not repeated or treated
as independently verified evidence. Their absence from this review does not
block readiness, as instructed. No source changes or commits were made.

## Observations

### The build-cache helper still claims to mirror wt validation

Outside this fix's implementation boundary,
[`worktree_base` in the build-cache script](../../../scripts/kache-host.sh:188)
still says it reproduces every setting `wt` refuses, but it accepts an existing
relative directory that `wt` now rejects and retains stale comments about files
and validation order. The
[initialization documentation](../../../docs/initialization.md:76) also claims
matching refusal behavior. This is information for a separate maintenance
decision: narrow those claims or share the validation behavior in a follow-up.
It does not require repair in this review cycle.
