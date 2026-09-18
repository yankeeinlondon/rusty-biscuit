---
$schema: feature-review.yaml
ready: false
findings:
    - title: The accepted five-platform baseline and its generated publications do not exist
      priority: high
    - title: An empty approver name satisfies the human-approval gate
      priority: high
    - title: Impossible calendar dates are accepted as research evidence dates
      priority: high
    - title: Corrupt active-run records are ignored when selecting new work
      priority: high
    - title: Prepared runs embed absolute paths and interpolate the repository root into a shell command
      priority: high
human_review: true
human_review_items:
    - |-
        Complete and approve the first five-platform research publication after the implementation findings in this review are fixed.

        The maintainer must:

        1. Choose an AI research agent and, if needed, its model.
        2. Set a maximum elapsed time and maximum number of agent launches for each platform. Either use one pair of limits for all five platforms or set separate limits per platform; starting with a small Discord run is the recommended way to measure realistic usage.
        3. Run the reviewed workflow for Discord, Slack, Telegram, WhatsApp, and Signal.
        4. Inspect the evidence review and unresolved gaps, then provide the non-empty maintainer name that should be recorded as approving the initial publication.

        Approval should confirm that the resulting catalog, cross-provider summary, skill projection, change history, and detailed platform documents agree with the reviewed evidence. This human decision is required by the specification; automated tests cannot replace it.
reviewed_by: codex/default
created: "2026-09-18T01:00:12-07:00"
spec: 2026-09-17-research-metadata-pipeline/spec.md
implemented: false
description: "A **feature** review of `2026-09-17-research-metadata-pipeline/spec.md`"
feature: 2026-09-17-research-metadata-pipeline/review-1.md
---

# Review 1: Research Metadata Pipeline

## Verdict

The feature is **not production ready**. The typed contract, validator,
deterministic projection, publication recovery, refresh lifecycle, and CLI
surface are substantial and generally well structured. The Messenger Level 1
suite passes, and the maintenance dependencies remain outside the ordinary send
path. However, the feature's central deliverable—the reviewed five-platform
research baseline—has not been produced, so the catalog, summary, skill
projection, accepted change history, and real-fleet handoffs are absent.

Four independent correctness defects also need fixes before the live baseline
is run: human approval accepts an empty identity, calendar dates accept
impossible days, corrupt active-run records are silently ignored, and prepared
shell steps do not safely represent arbitrary repository paths.

## Findings

### High — The accepted five-platform baseline and its generated publications do not exist

The specification requires a reviewed initial baseline and generated
publications in deliverables 2–10 and acceptance criteria 2, 3, 9–12, and
15–22. The worktree has no
`messenger/docs/research/publication.json`, generated `catalog.json`, generated
cross-provider summary, accepted review records/CHANGELOG, implementation
mapping, or `.claude/skills/messenger/platform-metadata.md`. The five shipped
platform files remain legacy prose without the new schema binding. Running
`messenger research validate` against the worktree returns findings, as the
implementation log also records.

The green corpus test does not close this gap. Its shipped-document check
explicitly accepts either a schema-valid document **or** legacy prose that
delegates to the fleet prompt
([research_corpus.rs:224](../../lib/tests/research_corpus.rs#L224)). The
accepted-scope semantic checks exercise manufactured lifecycle fixtures, not
the shipped platform documents
([research_corpus.rs:645](../../lib/tests/research_corpus.rs#L645)). Therefore
659 passing Level 1 tests prove the machinery and fixtures, but not the actual
fleet required for production.

Run the three-pass research and independent evidence review for all five
platforms under explicit budgets, obtain the required human approval, promote
the initial fleet together, and commit the complete generated snapshot. Add a
passive Level 1 corpus assertion that, once publication exists, every active
shipped document passes accepted-scope schema and semantic validation and the
generated snapshot is drift-free. The live research and human approval are
acceptance evidence rather than L2/L3 terminal tests.

### High — An empty approver name satisfies the human-approval gate

`--approved-by` is required syntactically but accepts any `String`, including an
empty or whitespace-only value
([research_lifecycle.rs:87](../../cli/src/research_lifecycle.rs#L87)). The CLI
passes that value directly into `Request::Human`
([research_lifecycle.rs:406](../../cli/src/research_lifecycle.rs#L406)), and the
library treats every `Request::Human` as sufficient for a substantive change or
initial baseline
([promote.rs:124](../../lib/src/research/refresh/promote.rs#L124)). The empty
value is then persisted in the durable review record and run decision.

This bypasses the specification's named-maintainer approval boundary. A command
such as `--approved-by ""` can publish the initial baseline while leaving no
accountable approver. Validate and normalize the identity in the library
boundary—not only in clap—reject empty/whitespace-only names, and add both
library and real-CLI Level 1 regressions. Apply the same validation to the
`reject --by` identity and require a non-empty rejection reason.

### High — Impossible calendar dates are accepted as research evidence dates

`Date::parse` checks only `month <= 12` and `day <= 31`, despite documenting
calendar-range validation
([common.rs:53](../../lib/src/research/model/common.rs#L53)). It accepts dates
such as `2026-02-31` and `2026-04-31`; the CLI likewise accepts
`--today 2026-02-31`. Date values are used for evidence retrieval, source-check
matching, freshness, override expiry, approval history, and automatic renewal.
An impossible `retrieved` date can therefore match an equally impossible
source-check date and participate in the automatic-acceptance policy.

Validate real proleptic-Gregorian dates, including month lengths and leap-year
rules. Add table-driven parsing/serde/CLI tests for February 29 in leap and
non-leap years, February 30/31, and 30-day-month day 31. Also assert that an
invalid evidence date cannot qualify an unchanged renewal.

### High — Corrupt active-run records are ignored when selecting new work

`StateArea::list` deliberately returns unreadable records as errors, but
refresh selection drops every such error with `record.ok()`
([select.rs:178](../../lib/src/research/refresh/select.rs#L178)). If an active
run's `run.json` is truncated or otherwise corrupt, selection treats the
platform as having no open run and `prepare` may create another run with a new
budget. This defeats the retained-budget and one-open-run invariants precisely
after the crash or storage fault those invariants are intended to survive.

Selection must fail closed on an unreadable run record and report its
repository-relative path, or return an explicit protected/repair-required
selection state. Add a Level 1 regression that corrupts an active `run.json`
and proves that neither dry-run selection nor preparation can declare the
platform available or create another ledger. The same invariant should be
tested with two concurrent `prepare` attempts so the read-then-create window
cannot produce duplicate open runs.

### High — Prepared runs embed absolute paths and interpolate the repository root into a shell command

Every pass prompt renders paths from the absolute state directory with
`Path::display`
([prepare.rs:235](../../lib/src/research/refresh/prepare.rs#L235)), so prepared
agent inputs contain host-specific paths even though the Phase 7 log says they
do not. More seriously, `write_sequence` converts backslashes to slashes and
places the absolute root inside double quotes in a `shell:` string
([prepare.rs:365](../../lib/src/research/refresh/prepare.rs#L365)). It does not
escape double quotes, dollar expansion, backticks, newlines, or other shell
syntax. Because the sequence runs these checks with `--yolo`, a valid checkout
path containing shell metacharacters can change the command rather than remain
one `--root` argument.

Use run-relative portable paths in the pass inputs. For validation steps,
prefer an argument-vector execution form that never enters a shell. If
Claudine currently supports only `shell:`, add a single well-tested
cross-platform encoding boundary rather than hand-building a quoted command.
Add Level 1 end-to-end fixtures whose repository roots contain spaces, a single
quote, a double quote, a dollar sign, backticks, and non-ASCII characters; the
captured invocation must contain the original root as exactly one argument and
the prepared inputs must not contain the host prefix.

## Verification levels

| Requirement group | Strongest evidence present | Required level | Assessment |
| --- | --- | --- | --- |
| Schema, semantic rules, deterministic generation, publication recovery, deltas, mappings, reports, refresh state, budgets, and CLI exit behavior | Level 1 unit, fixture, and real-subprocess tests | Level 1 | Strong for fixtures and machinery, subject to the defects above |
| Shipped five-platform accepted metadata and generated publications | No accepted real-fleet artifact; Level 1 permits legacy delegation | Level 1 plus reviewed research evidence | **Gap** |
| Human approval of the initial substantive baseline | Not performed | Human decision | **Gap** |
| Terminal colors, glyph widths, scrolling, keyboard input, paste, IME, mouse, or hotkeys | Not asserted by this specification | L2/L3 not applicable | No tier mismatch |

The CLI uses `TerminalRenderable` components, but the specification does not
promise exact terminal styling or input behavior. L2 and L3 tests are therefore
not required for this feature's stated user-observable contract.

## Verification performed

- `messenger/ just test`: **659 passed**, 2 pre-existing tests skipped.
- `messenger/ just lint`: passed for `messenger` and `messenger-cli`.
- Direct CLI probe: `messenger research --today 2026-02-31 ...` accepted the
  impossible date and proceeded to validation, confirming the date defect.
- GitNexus repository `rusty` was refreshed to commit
  `4fa7bb555a61e362615c2c20807d86b9e5b02e9e`; graph exploration confirmed the
  generation path (`generate_with` → `load_fleet`/publication), promotion path,
  and all six callers of `apply_ledger`. The analyzer reported no incomplete
  index reasons, although its whole-repository process extraction warned that
  some unrelated flows exceeded global traversal caps.

Cross-OS execution evidence is intentionally not used as a readiness finding;
CI/CD owns that proof. The design itself must still fix the path handling above
so the same code is valid on macOS, Linux, native Windows, and WSL2.

## Ergonomics and performance

The opt-in `research` feature keeps Darkmatter, file-resolution, hashing, and
schema dependencies out of normal Messenger sends, and deterministic
`BTreeMap`/`BTreeSet` projections are appropriate for this maintenance path.
No performance issue warrants blocking production. The most valuable ergonomic
improvement is to make `prepare` persist an explicit agent/model choice so its
printed command is directly runnable in a noninteractive session; that design
decision is already recorded in the specification's human-review items.
