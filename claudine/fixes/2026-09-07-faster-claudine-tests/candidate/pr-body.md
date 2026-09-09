## Summary

Faster Claudine tests through complete evaluation and explicit fixtures —
`claudine/fixes/2026-09-07-faster-claudine-tests/`, Phases 4–8 implemented,
Phase 9 (CI evidence) opened by this PR.

- **One environment policy, two command surfaces** (Phase 4): the L1 process
  fixture and the raw-command builder share one child-environment contract.
- **L1 spawn burn-down** (Phase 5): the spawn allow-list is empty; the isolated
  population went from 37 to 74 files with zero escapes; eight contamination
  probes prove the isolation is not vacuous.
- **Non-spawn cost** (Phase 6): library L1 summed duration 187 s → 102 s locally;
  eight nextest override blocks removed, none added; four previously
  unreachable identities now run; CI tier metadata declared in every manifest.
- **Bounded time and resource ownership** (Phase 7): every sleep site
  dispositioned; a process leak that played audio on the host and a Windows
  endpoint-isolation defect keyed on the pid alone are closed.
- **Local measurement** (Phase 8): paired candidate ÷ baseline for `just test`
  0.64–0.77 in all ten alternating pairs, elapsed and summed alike; every
  eliminated-work claim has an lldb work counter or a shim sentinel behind it.

Local numbers are attribution only. This PR's CI runs are the candidate half of
the evidence; the baseline half is `main`'s runs after the predecessor merge
(`444213eb5`), one of three collected so far.

## Verification

- `just test`, `just lint`, `just test-cli`, `just test-l2`, `just test-rendezvous`
  in `claudine/` — recorded per phase in the fix's `log.md`.
- `just check-windows` (mingw, `--tests`) — exit 0.
- `just ci-local` at the repo root over the full workspace scope (the
  `.config/nextest.toml` change is global) — see
  `claudine/fixes/2026-09-07-faster-claudine-tests/candidate/local-gates/ci-local.log`.
- The JUnit gate (`junit-metrics.ts`) passes over both stored baseline runs and
  now reports `windows-latest`'s eleven `#![cfg(unix)]` exclusions apart from
  violations.

## Evidence still pending on CI

Three consecutive green candidate runs per leg (`ubuntu-latest`,
`macos-latest`, `windows-latest`, `wsl2-ubuntu`), compared within each
environment against its own baseline run; budgets remain underivable until the
baseline has three runs. PR readiness is not merge readiness for the
performance criteria — see the fix's `plan.md` Phase 9.
