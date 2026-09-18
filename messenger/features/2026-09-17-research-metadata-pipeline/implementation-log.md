---
spec: /Volumes/coding/wt/rusty-biscuit/feat-better-static-analysis/messenger/features/2026-09-17-research-metadata-pipeline/spec.md
plan: messenger/features/2026-09-17-research-metadata-pipeline/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1:
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/.gitignore
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/Cargo.toml
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/src/fault.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/src/fsutil.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/src/generations.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/src/journal.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/src/lib.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/src/model.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/tests/common/mod.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/tests/generations_interruption.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/tests/journal_interruption.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/tests/open_handles.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/orchestration/probe_budget.py
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/orchestration/probe_tree_windows.py
docs_updated_during_phase_1:
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/orchestration/findings.md
docs_created_during_phase_1:
    - messenger/features/2026-09-17-research-metadata-pipeline/architecture.md
    - messenger/features/2026-09-17-research-metadata-pipeline/fixture-matrix.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/surface-inventory/inventory.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/findings.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/findings.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/orchestration/probe-results.json
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/_schema.yaml
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/_types.yaml
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/_mappings.schema.yaml
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/_mappings.types.yaml
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/mappings.pilot.yaml
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/discord.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/telegram.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/slack.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/signal.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/negative/
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/probes/
skills_files_updated_during_phase_1:
    - .claude/skills/os/windows.md
packages:
    - messenger
---

# Implementation Log for 2026-09-17-research-metadata-pipeline (8 phases)

## Phase 1

- Phase 1 is discovery and architecture: its outputs are spike evidence and
  design records under `spikes/` and `architecture.md`, not product behavior.
- Wave 1 tasks run as four parallel agents with disjoint output directories:
  `spikes/surface-inventory/`, `spikes/schema-pilots/`, `spikes/publication/`,
  and `spikes/orchestration/` (extended).
- Drafted `fixture-matrix.md` while Wave 1 ran: all 37 acceptance criteria map
  to named fixtures, a proof boundary, and a landing phase; every row is L1.
- Finding: `just test`/`just lint` build the `messenger` library with only
  `desktop`, so research code needs explicit `--features research` and
  `--all-features` checks (recorded in the matrix; no new CI cell — CI already
  runs `all-features`).
- Finding: `cargo tree -p messenger --all-features` already reaches
  `biscuit-file` and YAML crates through `sniff` → `schematic-*`; the
  "ordinary send build has no maintenance deps" check must therefore use
  no-default, default, and `desktop` feature sets (all clean on 2026-09-17).
- Wave 1 Surface Inventory complete → `spikes/surface-inventory/inventory.md`
  (revision `d57faf7e8`). Key outcomes: adapter IDs reuse the persisted
  `ProviderKind::as_str()` strings; interface IDs are separate snake_case
  names; `Summarized` bodies, location-with-text, and plain text behave very
  differently per adapter; `supports_location` is `true` for all seven even
  where only a text fallback exists; error detail (codes, subcodes, field
  errors, correlation IDs, success warnings) is lost in most adapters. Three
  suspected runtime defects (Slack webhook response parsing, Signal group
  method name, Telegram 429 `retry_after` loss) are recorded as research
  priorities, not fixed here — this feature must not change delivery behavior.
- Wave 1 Publication Spike complete → `spikes/publication/findings.md` and a
  standalone prototype crate (`spikes/publication/prototype/`, own empty
  `[workspace]`, not a monorepo member). Chosen: committed manifest
  `messenger/docs/research/publication.json` as the single selection point,
  local journal + staged/backup copies under gitignored
  `messenger/.research-state/publication/`, and a hash-guarded, idempotent
  roll-back/roll-forward recovery. All 17 prototype tests pass on macOS
  (re-run by the orchestrator: 2 + 9 + 6 tests OK), build-linux, and
  build-win-native. **Gap:** WSL2 not run (SSH to build-win reset twice; W:
  nearly full). The subagent could not write its report file; the orchestrator
  saved its content verbatim.
- New measured Windows fact added to `.claude/skills/os/windows.md`:
  `std::fs::rename` succeeds over a std-opened reader while
  `tempfile::persist` fails with error 5.
- Wave 1 Budget Spike complete → new section in
  `spikes/orchestration/findings.md` plus `probe_budget.py`,
  `probe_tree_windows.py`, `probe-results.json`. Built committed HEAD
  `d57faf7e8` from a `git archive` snapshot because the working tree's
  Claudine crates currently fail to compile due to another effort's
  uncommitted edits (not touched). Key outcomes: worker cwd is the repo root
  (prepared scratch cwd needed for discovery); inline-compose exposes prior
  prose by design (never use it for discovery); Unix timeout kills only the
  child's process group (a `setsid` grandchild survives; wrapper death
  orphans the worker); Windows Job Object kills the tree; lifecycle retries
  launch inside one Claudine command, so admission must hook inside
  Claudine's launch path (`execute_attempt_phase` before
  `execute_harness_attempt`); Claudine persists no run state today.
  GitNexus impact was unreliable (index mid-update by another session) —
  Phase 5 must re-run impact on a fresh index. **Gap:** WSL2 unreachable;
  native Windows evidence is OS-level (Job Object) only.
- Wave 1 Schema Pilots complete → `spikes/schema-pilots/` (pilot schema,
  four pilot documents, mappings pilot, negative corpus, probes). The subagent
  could not write `findings.md`; the orchestrator saved it and added a note
  that production IDs/fingerprints follow `architecture.md`. Independently
  re-verified with the installed `md`: the pilots exit 0, all 13
  `schema-rejects/` samples exit 1, and all 9 `schema-accepts-rust-must-reject/`
  samples exit 0 (as designed). Thirteen SimplifiedSchema limitations were
  recorded (L1–L13); the most consequential are root closure only through a
  pattern key (L1), type leakage requiring a separate `_types.yaml` (L2), no
  arrays of unions (L3–L5), and multi-line types failing to load from
  standalone files (L6, likely a Darkmatter bug).
- Wave 2 complete: `architecture.md` (IDs, module/feature graph, schema/type
  boundaries, artifacts, local state, snapshot protocol, fingerprints,
  CLI/recipes, budget boundary, cleanup rules, resolution of every spec
  planning item) and `fixture-matrix.md`. Confirmed that Darkmatter already
  exposes `DarkmatterSchemas::validate` as a library API, and that its
  `effects-instrumentation` feature can prove loader passivity.
- Decision recorded (not escalated): downtime after a crash is not charged.
  The run enters `interrupted`, an operator-resumption state, which the spec's
  active-clock ruling already excludes; time through the last heartbeat plus
  one interval stays charged, so no consumed allowance is refunded.

### Verification

- Phase 1 changes no product behavior: no workspace crate, public API,
  provider, or `CapabilitySet` changed. The requirement-to-test mapping for
  the whole feature is `fixture-matrix.md`; Phase 1's own claims are backed by
  spike evidence rather than workspace tests:
  - snapshot protocol: 17 prototype tests (interruption at every injection
    point, abort-based crashes, Windows open-handle cases), passing on
    macOS (re-run by the orchestrator), build-linux, and build-win-native;
  - schema expressiveness: `md schema validate` over pilots and negative
    samples (re-run by the orchestrator);
  - budget semantics: `probe_budget.py` on macOS and build-linux,
    `probe_tree_windows.py` on build-win-native (`probe-results.json`).
- Gates: `just test` in `messenger/` — 532 run, 532 passed, 2 skipped
  (pre-existing skips); `just lint` in `messenger/` — clean.
- Skipped / gaps: WSL2 evidence for both spikes (host unreachable); the
  Claudine binary was not run on native Windows; `just cross-check` was not
  used because no workspace package changed.
- Plan checkpoint item "obtain maintainer review" is left unchecked: it needs
  a human. `human_review: true` is set on the spec with two items (extra
  lifecycle subcommands; artifact-layout additions).
- Pre-existing: the working tree's Claudine crates did not compile during
  this phase because of another effort's uncommitted edits (not touched).
