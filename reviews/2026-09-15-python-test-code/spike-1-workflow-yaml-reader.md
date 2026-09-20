---
title: "Spike 1 — one workflow reader, heuristic or parsed?"
created: 2026-09-15
status: complete
results: reviews/2026-09-15-python-test-code/spike-1-results.md
timebox: 1 day
source: reviews/2026-09-15-python-test-code/review.md §3.3, §5.1
decides: whether scripts/ci takes a PyYAML dependency
---

# Spike 1 — One workflow reader: heuristic or parsed?

## Question

`scripts/ci` has four independent, indentation-based readers of GitHub workflow
YAML. They must become one (§3.3). Should the consolidated reader keep the
indentation heuristic, or should `scripts/ci` take a PyYAML dependency?

## Why it is open

**For the heuristic.** The CI Python has *zero* third-party dependencies today.
That is why `ci-tooling` needs no install step beyond `just` and `nextest`, and why
`.githooks/pre-push` can run the suites on any developer machine without a
virtualenv. It is a real asset and the spike must not spend it carelessly.

**Measured against it:** stock macOS `/usr/bin/python3` has **no `yaml` module**
(verified 2026-09-15: `ModuleNotFoundError`). Homebrew's does. So a PyYAML
dependency is not free on the exact host class the pre-push hook targets.

**Against the heuristic.** Four hand-rolled parsers disagree about the layout they
assume, and some degrade *silently*:

| Reader | Assumes | On an unexpected layout |
|---|---|---|
| `test_ci_local.py:1020` `workflow_step_script` | 6/8/10-space indents | raises (walks past the step) |
| `test_ci_local.py:1057` `workflow_job_run_steps` | 2/6/8/10, strips 6 then 4 | **returns fewer steps, silently** |
| `test_runner_loss.py:414` `jobs_in` + `:440` `job_label` + `:453` `status_artifact` | `^  ([A-Za-z0-9_-]+):$` regex | returns `{}` / `None`, silently |
| `test_build_baseline_revision.py:238` `_run_body` | 8/10 | raises (`.index` / `next`) |

The silent cases are the hazard. If `workflow_job_run_steps` returned a truncated
step list, `WorkflowScopeStepTests` would execute a shorter job and **still pass** —
and that suite is the only thing that runs `ci.yml`'s scope step for real.

## Preconditions

- Working tree clean; all 607 tests green (`~2m45s` baseline, §0).
- No other in-flight change to `.github/workflows/` (the mutation corpus is cut
  from the shipped files).

## Steps

### 1. Consolidate behind one API, no behavior change (~3h)

Create `scripts/ci/workflow_reading.py` with the current heuristic behind:

```python
def job_names(workflow: Path) -> list[str]: ...
def job_block(workflow: Path, job: str) -> str: ...
def job_run_steps(workflow: Path, job: str) -> list[JobStep]: ...
def step_script(workflow: Path, step_name: str) -> str: ...
```

Port all four call sites. **Gate:** 607 still pass, and the diff touches no
assertion.

Record: lines removed from the three test files.

### 2. Make every degradation loud (~1h)

Independently of which implementation wins, `job_run_steps` and `jobs_in` must
raise rather than return a short list. Add a contract: a job block that yields zero
`run:` steps, or a workflow that yields zero jobs, is an error.

**Gate:** 607 still pass. If any test breaks, that test was relying on a silent
truncation — record it, it is a finding in its own right.

### 3. Build the mutation corpus (~2h)

`scripts/ci/fixtures/workflow_mutants/`, each cut from a real workflow:

| Mutant | Edit | Semantically |
|---|---|---|
| `reindent-job` | shift one job's steps by two spaces | unchanged |
| `folded-scalar` | one `run: \|` → `run: >-` | changed (line joining) |
| `anchor` | `&x` / `*x` for a repeated `with:` block | unchanged |
| `flow-mapping` | one `with: {a: 1, b: 2}` | unchanged |
| `key-reorder` | `run:` before `name:` in a step | unchanged |
| `comment-in-block` | `#` line inside a `run: \|` body | changed (a shell comment) |
| `quoted-job-name` | `"build":` instead of `build:` | unchanged |
| `trailing-blank` | blank line inside a `run: \|` body | unchanged |

### 4. Second implementation over PyYAML (~2h)

Same API, `yaml.safe_load`. Run both against the corpus.

### 5. Availability check (~30m)

| Host | `python3 -c "import yaml"` |
|---|---|
| macOS `/usr/bin/python3` | **fails** (measured) |
| macOS Homebrew `python3` | ok, 6.0.3 (measured) |
| `ubuntu-latest` runner | ? |
| `windows-latest` runner | ? |
| WSL2 guest | ? |

Fill the unknowns before deciding. The pre-push hook is the binding constraint: it
runs on whatever `python3` the developer has.

## Results to record

| Mutant | Heuristic | Parser | Same? |
|---|---|---|---|
| ... | reads same / reads differently / raises | ... | ... |

Plus: lines removed in step 1, tests broken in step 2, availability matrix.

## Decision rule

- **Keep the heuristic** if it *raises* on every mutant it cannot handle correctly.
  Loud degradation is acceptable; silent is not. This is the expected outcome and
  the cheapest one.
- **Take PyYAML** if the heuristic reads any mutant differently *without raising*
  **and** step 5 shows `yaml` present on every target host — otherwise take it with
  an explicit `just init` install step and a clear error when it is missing.
- **Split the difference** if only one call site needs real parsing: parse there,
  heuristic elsewhere. Record why.

## Ships regardless of the outcome

- One reader instead of four (§3.3 closed).
- Loud degradation on unexpected layout (step 2) — the actual hazard.
- The mutation corpus as a permanent regression fixture for whichever reader wins.

## Risks

- **The corpus is cut from today's workflows.** If a workflow is restructured
  mid-spike the mutants drift. Cut them once, at a recorded SHA, and commit them.
- **Step 2 may turn red.** If a suite depended on silent truncation, that is a
  latent defect surfacing, not a spike failure. Budget for it.
- **Scope creep into the workflows themselves.** The spike reads workflow YAML; it
  does not edit it.
