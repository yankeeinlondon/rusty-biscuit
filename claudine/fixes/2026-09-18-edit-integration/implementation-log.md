---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-schema-enhancement/claudine/fixes/2026-09-18-edit-integration/spec.md"
plan: "claudine/fixes/2026-09-18-edit-integration/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1:
    - claudine/fixes/2026-09-18-edit-integration/spike-interactive-startup.md
    - claudine/fixes/2026-09-18-edit-integration/implementation-log.md
skills_files_updated_during_phase_1: []
packages: []
---

# Implementation Log for 2026-09-18-edit-integration (5 phases)

## Phase 1

Started and finished 2026-09-28 on macOS (dev Mac). Phase 1 is rulings and
spikes only, and no source code, test, doc, or skill file changed. The spike
record is `spike-interactive-startup.md` in this directory.

### Rulings N1–N10: accepted

Each ruling's code claims were checked against the tree before acceptance:

- **N1** accepted. Repairing which existing `PromptDelivery` variant and which
  native flags a profile emits is in scope. No new variant.
- **N2** accepted, and **it fired for Kimi** (see below).
- **N3** accepted. Confirmed: `validate_timeout_constraints`
  (`wrapper_stages.rs:18`) owns the two `cannot be used with --interactive`
  messages and the `edit_requested` branch (`:32`), and has one caller
  (`mod.rs:394`, Stage 5). Stage 2 is at `mod.rs:305` and Stage 3 (the editor)
  at `mod.rs:323`. The later duplicate at `mod.rs:624–636` exists as described.
- **N4** accepted. Confirmed: `extract_wrapper_flags_from_passthrough_with_boundary`
  (`flags.rs:316`) lifts `-y -i --edit --repo -q --silent -v --perf
  --operation/--op` and none of `--timeout`, `--step-timeout`, or `--dry-run`.
- **N5** accepted as the planner's 2026-09-28 observation. It was not
  re-observed, because running binaries was refused in this session (see
  "Evidence limits").
- **N6** accepted (out of scope; filed in Phase 5). Same caveat as N5.
- **N7** accepted. Confirmed: `PROVIDERS_DISPLAY_ORDER: [Provider;
  PROVIDER_COUNT]` at `claudine/lib/src/provider_id.rs:56`.
- **N8**, **N9**, and **N10** accepted as written.

### Spike outcomes (details in the spike doc)

- **S1.** Claude, Codex, Gemini, Qwen, and OpenCode: keep their current forms.
  Goose (`run --text=<p> --interactive`), Kilo (`--prompt`, OpenCode-style
  attached value), Antigravity (`--prompt-interactive`), and Pi (`-- <p>`)
  need repair. Kimi has no native interactive startup-prompt surface, so
  **N2 fired**.
- **S2.** Pi accepts `--` from **0.84.3** (changelog, #7269). The dev Mac runs
  0.87.1 per the plan. A piped stdin always forces Pi into print mode
  (`main.ts` `resolveAppMode` plus the piped-stdin override), which confirms
  today's interactive Pi delivery is broken. Decided shape: interactive →
  `AppendArgs(["--", prompt])`, non-interactive stays `Stdin`.
- **S3.** Only Pi newly moves to argv. Antigravity and Goose already argv-deliver
  in both modes, so the plan's premise that they might newly move was
  corrected. The Pi guard is per-profile (one new call site, Rule 2). **New
  risk:** Pi upstream issue #9200 reports SIGKILL for positional messages of
  about 993 bytes or more (0.83.0, 0.85.1; closed as not planned). This is
  escalated to the author.

### Gate G1

- S1–S3 are recorded in the spike doc. N1–N10 are accepted above.
- N2 fired for Kimi, so the spec is set to `status: human-in-the-loop` with
  `human_review: true`. Phase 2 does not depend on the ruling and can proceed.
  Only the Kimi Phase 3 task and Kimi's row in the fleet test's expectation
  table wait on it.

### Evidence limits and departures from the plan

- **Departure (S1): no live terminal run.** This session was non-interactive,
  and its permission policy refused `tmux -V`, `pi --help`, and every other
  provider binary. It also refused `ls` and `Read` outside the worktree, so the
  installed npm packages could not be inspected either. S1 and S2 were done
  from upstream source, changelog, and docs (fetched 2026-09-28) plus the
  in-repo research corpus, and each row is labeled with its tier. The spike
  doc lists the exact live checks still owed. Phase 4's real Pi test covers
  Pi. The other repaired providers (Goose, Kilo, Antigravity) have no planned
  live test, so a human-permitted pass is recommended before Phase 5 states
  per-provider behavior as verified.
- Goose is not installed on the dev Mac. Its evidence is upstream source
  (`block/goose` `crates/goose-cli/src/cli.rs`), as the plan allowed.
- **Finding outside Phase 1 scope:** Goose's `-t/--text` has no
  `allow_hyphen_values`, so a `-`-prefixed prompt also breaks the
  **non-interactive** `run -t <p>` path. Not fixed here; see the spec's
  `message_to_agent`.
- **Finding for Phase 3:** after `--`, Pi still reads an `@`-prefixed token as
  a file argument, so a prompt that begins with `@` would be misread.
- No `just test` or `just lint` run was needed, since no code changed.
