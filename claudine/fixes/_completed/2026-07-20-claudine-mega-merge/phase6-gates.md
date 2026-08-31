# Phase 6 verification evidence

Host: native macOS (`America/Los_Angeles`)  
Candidate: `df13f68dd7ad3ef22ef7e324dbdc213ed75afcd6`  
Audit checkout: detached, clean candidate checkout

## Results

| Gate | Result |
|---|---|
| `claudine-gen check` | passed; all ten providers, catalog, signals, vocabulary, families, and roster were clean |
| focused drift/source/seam guards | passed: 61 passed / 2 skipped |
| `just test` | passed: catalog types 21; library 3,954 / 7 skipped; contract 47 / 5 skipped; CLI 2,313 / 247 skipped; generator 152 / 4 skipped |
| `just test-l2 --no-fail-fast` | failed: 227 passed, 1 failed, 1 slow, 1 flaky pass, 2,332 non-L2 tests filtered |
| isolated failing L2 test | failed on 4/4 attempts |
| `just lint` | passed for all five Claudine crates, including 18 transport guards and the lifecycle-doc-facets guard; one nextest leaked-handle retry passed on attempt 2 |

The first `just test` attempt found the audit checkout's temporary target did
not contain `md`, which `inline_compose_hash` expects at the checkout target.
After building `darkmatter-cli` into the isolated target and setting
`CARGO_BIN_EXE_md`, the targeted test passed and the complete, uninterrupted
`just test` rerun passed. This was audit-environment setup, not a waived test.

## Blocking L2 regression

`level2_lifecycle_retry_to_an_unavailable_provider_matches_direct_selection`
fails at `claudine/cli/tests/level2_lifecycle_control.rs:1325`. The exact test
document remains `agent: goose` after the opening provider fails, although its
failure stack contains:

```yaml
- action:
    - {set_frontmatter: ["doc.md", "agent", "gemini"]}
- action: {retry: 1}
```

The full L2 run retried it four times; a one-test run then reproduced the same
failure four more times. No implementation was changed after candidate freeze.

`level2_lifecycle_sequence_step_proxy_rebuilds_target_launch_bundle` passed on
attempt 2 in the full run. This flaky pass is reported, not hidden.

## Repository and platform blockers

- Neither frozen feature tip is an ancestor of the candidate. The branch has a
  reconciled tree but not the required reviewed merge history.
- `git diff --check main...df13f68dd` reports inherited whitespace defects in
  the frozen candidate. Rewriting them after test evidence would change the
  candidate and shipped prompt hashes.
- GitNexus reports CRITICAL cumulative integration risk for the expected broad
  merge surface; see `impact/final-audit-detect.md`.
- The Phase 6 documentation-only worktree diff passes `git diff --check`, and
  both anchored worktree/index conflict-marker scans are empty after the
  recorded evidence-directory exclusions.
- Native Linux L1/lint and dedicated tmux L2 evidence, native Windows runtime
  evidence, and attended L3 evidence were not available in this
  non-interactive macOS session.
- No evidence/closeout commit, lifecycle-directory move, branch fast-forward,
  or freeze closure was performed. The request expressly prohibits staging,
  committing, and moving this fix record.

These findings prevent Phase 6 exit. Passing L1, generator, focused guards, and
lint do not substitute for the failed L2 assertion or missing native evidence.
