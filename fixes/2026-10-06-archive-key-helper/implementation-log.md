---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-wt-skill/fixes/2026-10-06-archive-key-helper/spec.md"
plan: "fixes/2026-10-06-archive-key-helper/plan.md"
implemented_by: "claude/sonnet"
started_phase: 1
source_files_during_phase_1:
  - just/devops.just
  - scripts/ci/build_key.py
  - scripts/ci/test_build_key.py
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
  - repo-deps
---

# Implementation Log for 2026-10-06-archive-key-helper (4 phases)

## Phase 1

- `just/devops.just` `_ci_build_verify`: emits `key_helper=${tool}` inside the
  `$GITHUB_OUTPUT` block (after the verifier ran); output doc comment lists it.
- `scripts/ci/build_key.py` `planned_keys`: helper child gets a copy of the
  environment with `BISCUIT_CI_BUILD_WRAP` removed and `RUSTC_WRAPPER=""`;
  parent env untouched; resolver unchanged. Docstring and WHY comment added.
- `scripts/ci/test_build_key.py`: added empty-override fallback, override
  precedence with a spaced path (single argv element), and subprocess-boundary
  env tests. Requirement-to-test: AC4 -> those three tests plus the existing
  invalid-override, helper-error, and pinned-digest tests.
- Gate: `python3 -m unittest scripts/ci/test_build_key.py` -> 15 tests OK.
- Rulings: all defaults taken (no human decision needed).
- Not run: `just lint` (no Rust changed; Python-only plus a justfile line).
