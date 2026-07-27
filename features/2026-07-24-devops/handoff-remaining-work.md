# DevOps CI/CD — handoff for remaining work

Source of truth: `features/2026-07-24-devops/{plan.md, spec.md, execution.md}`.
Read all three first; `spec.md` → "Implementation Decisions (2026-07-25 session)"
and `execution.md` capture the live decisions and learnings.

## Already done and on `main`
Phases 1–3 (bootstrap/kache/preflight/release-gating; pinned toolchain 1.97.1 +
advisory; explicit `ci` nextest profile, retries=0, Claudine sharding,
`--no-fail-fast`, per-shard JUnit; staged area gates; L2 tmux-only provisioning;
native-prereq provisioning) and Phase 4 tasks 4.1–4.3, 4.5-scope, 4.6 + the
`hooks-tests`→`pre-push-hook-tests` rename. Playa was fixed on `main`
(feature-flag rationalization + `_ensure-native-libs` in `just init`).

## In progress (branch `devops-phase-4-orchestration`, PR #4 draft — needs rebase onto `main`)
Task 4.4 pattern proven: `claudine-windows-ctrl-c.yml` and `rendezvous-tests.yml`
are now reusable (`workflow_call`), no longer self-trigger, honor the pinned
toolchain (`rustup show`), and are orchestrated by `ci.yml` gated on scope (via a
new `area_names` scope output), canary-aware. Contract tests updated.
**First step: rebase this branch onto the updated `main`.**

## Remaining work (do on a branch; verify each on a branch PR run)

1. **Native libraries — single install implementation (spec: Native libraries).**
   CI must run the isolated `_ensure-native-libs` recipe **before build/test**
   (before check/test/lint/specialized build commands) so `-sys` crates never
   fail on a missing system lib. Consolidate the duplicate
   `.github/actions/install-native` composite into that one recipe (CI calls
   `just _ensure-native-libs`; ensure `just` is available in the jobs that need
   it). `areas.json` `native` stays the single source of truth.

2. **Finish Task 4.4 — specialized-workflow consolidation (D12).** Apply the
   proven pattern to the remaining workflows: make each reusable
   (`workflow_call` + `workflow_dispatch`, drop standalone `push`/`pull_request`
   path triggers, `dtolnay@stable`→`rustup show`, remove `concurrency`), and add
   an orchestration job in `ci.yml` gated on scope + canary:
   - `biscuit-tui-windows-captured-stdout.yml` → gate on `biscuit-tui` in
     `area_names`.
   - `playa-windows.yml` → gate on `playa` in `area_names`.
   - `messenger-desktop-tests.yml` (matrix + WSL job) → `messenger` is an
     EXEMPT package, so gate on `contains(fromJSON(needs.scope.outputs.packages),
     'messenger')`; update its exemption reason in `.github/ci/exemptions.json`
     to "covered by the messenger-desktop specialized job."
   - LEAVE `build-integrations.yml` (release-triggered artifact build, different
     lifecycle).
   - Add the **failure-class summary** (D15 pt2): a final `ci.yml` job (`needs:`
     all jobs, `if: always()`) that classifies the first actionable failure
     (bootstrap/build/lint/L1/L2/browser/release/...) into `$GITHUB_STEP_SUMMARY`.
   - Add contract tests asserting each is reusable + orchestrated + preserves its
     unique runtime evidence.

3. **Canary refinement (spec: Canaries must be green).** In `areas.json`, DROP
   `"canary": true` from `darkmatter` (keep `biscuit-hash` + `playa`) until
   darkmatter's L1 tests are green. Document "avoid homelab/research as canaries"
   in `.github/ci/README.md`'s canary note. A red canary currently blocks all
   global-change fan-out.

4. **Phase 5 — separate/harden scheduled automation.** Per plan Phase 5:
   `bench-nightly` push-trigger removal + measured timeout budgets +
   execution-vs-upload separation; coverage/fuzz distinct names/schedules/
   summaries; a recurring maintenance audit (advisory). Update docs + skills.

5. **Optional QoL — CI-aware `just commit`.** When `CI` is set, do a plain,
   deterministic, non-interactive `git commit` (no `claudine`/LLM, no `_speak`,
   no network), message supplied by caller. Local behavior unchanged.

## Constraints & learnings (important)
- **CI-only bugs are the norm here.** Local `actionlint` + text contract tests
  passed on 4 bugs that only a real runner caught (composite `if:`+local-`uses:`
  load error; `kache-action@v1` rejects `win32-x64`; `${VAR:-{}}` → jq `{}}`;
  canary choice). **Verify every workflow change on a branch PR run.**
- Local `actionlint` hangs via its shellcheck integration on `ci.yml` — use
  `actionlint -shellcheck=` for native checks; run `shellcheck` on `run:`
  scripts separately.
- Toolchain = `rustup show` (honors pinned `rust-toolchain.toml` = 1.97.1). kache
  is Linux/macOS-only. Area nextest uses `NEXTEST_PROFILE: ci`, retries=0, L1
  shards `--no-fail-fast`.
- Verify locally: `actionlint -shellcheck=`;
  `cargo nextest run -p test-toolkit --test ci_workflow_contracts`;
  `python3 scripts/ci/test_affected_scope.py`. Contract tests live in
  `tools/test-toolkit/tests/ci_workflow_contracts.rs`.
- Repo rules: never `cargo fmt`; commit/push only when asked; branch off `main`;
  after editing a `.claude/skills/**` file with a `hash:`, regenerate it with
  `md hash <file>`. Commits are GPG-signed (needs a host with `pinentry`).
- **Pre-existing product failures** (biscuit-speaks/homelab/research/tree-hugger
  lint; darkmatter tests) are the spec's explicit non-goal — do NOT fix them as
  part of this work; they belong to their areas.
- `spec.md` has an uncommitted "Implementation Decisions (2026-07-25 session)"
  addition (and this handoff file) — commit them with the rest.
