---
kind: artifact-inventory
feature: 2026-09-19-direct-cell-execution
created: 2026-09-20
plan_phase: 1
status: complete
---

# Plan corpus — production recipes

Eight resolved plans, all captured offline on 2026-09-20 from the
`feat-single-os` worktree at tree head `1266e5fc98fef03535773ec7fe8a1519024c45e2`
(`1266e5fc9`). No hosted run produced any of these. Each file (except
`gap-only.json`, below) is the direct output of one `affected_scope.py`
invocation; each validates against `schema.validate_resolved_plan` at schema
version 4. Phase 3's equality-and-uniqueness proof runs over exactly these
files.

The zero base is the planner's NULL OID: corpus plans record a well-formed
base rather than a null, matching how `ci.yml`'s scope step invokes the
planner for events with no comparison base.

```sh
BASE=0000000000000000000000000000000000000000
HEAD=1266e5fc98fef03535773ec7fe8a1519024c45e2
P=features/2026-09-19-direct-cell-execution/spikes/plans

# full workspace, every environment
python3 scripts/ci/affected_scope.py --all --base "$BASE" --head "$HEAD" \
  --plan-out "$P/all.json" > scope-all.json

# manual full run (workflow_dispatch)
python3 scripts/ci/affected_scope.py --all --event workflow_dispatch \
  --base "$BASE" --head "$HEAD" --plan-out "$P/dispatch.json"

# nightly (schedule): wsl2-ubuntu cells, Linux producer cell-less
python3 scripts/ci/affected_scope.py --all --event schedule \
  --base "$BASE" --head "$HEAD" --plan-out "$P/nightly.json"

# PR-shaped: one real source file selects one area
python3 scripts/ci/affected_scope.py --event pull_request \
  --base "$BASE" --head "$HEAD" --plan-out "$P/pr.json" \
  biscuit-hash/lib/src/lib.rs
```

## Evidence overlays

`accepted-all.json` / `accepted-wsl.json` are `{package, environment, gate,
origin: "local", outcome: "pass"}` documents derived from `all.json`'s
executing cells (all of them, and the 66 wsl2-ubuntu L1 cells, respectively):

```sh
# all-reused: every executing cell accepted
python3 scripts/ci/affected_scope.py --all \
  --accepted-cells accepted-all.json \
  --base "$BASE" --head "$HEAD" --plan-out "$P/all-reused.json"

# mixed: only the wsl2-ubuntu L1 cells accepted
python3 scripts/ci/affected_scope.py --all \
  --accepted-cells accepted-wsl.json \
  --base "$BASE" --head "$HEAD" --plan-out "$P/mixed.json"
```

## Prohibition

`/tmp/ci-constraints/wsl.json` (ephemeral, contents below) forbids
`wsl2-ubuntu`:

```json
{"environment": "wsl2-ubuntu",
 "reason": "spike: prohibit the WSL2 leg to shape the corpus",
 "owner": "@ken", "expiry": "2027-12-31"}
```

```sh
python3 scripts/ci/affected_scope.py --all \
  --constraints /tmp/ci-constraints \
  --base "$BASE" --head "$HEAD" --plan-out "$P/prohibited.json"
```

## gap-only.json (synthesized)

No area is gap-only in a real plan today (every area carries a lint cell and
lint is never reusable), so this file is derived from `all.json`: keep only
the 38 `accepted-gap` cells, keep area records and packages only for the 11
gap-owning areas, empty `builds` (no executing test cells means no consumers),
and set `job_estimate` to audits + gap publishers. The transformation was
validated with `schema.validate_resolved_plan` (zero problems) before writing.
Phase 3's fixture builder should grow a first-class producer for this shape
rather than extending the hand-edit.
