---
title: "Spike 1 results — one workflow reader, heuristic or parsed?"
created: 2026-09-15
status: complete
source: reviews/2026-09-15-python-test-code/review.md §3.3, §5.1
decides: whether scripts/ci takes a PyYAML dependency
decision: "Keep the heuristic — no PyYAML dependency. Measured against a `yaml.safe_load` twin over a ten-mutant corpus, the consolidated reader refuses every layout it cannot read correctly and misreads none silently; `yaml` is absent from three of the five interpreters measured, including stock macOS `/usr/bin/python3` and `build-linux`."
---

# Spike 1 results — one workflow reader: heuristic or parsed?

## The question

`scripts/ci` had four independent, indentation-based readers of GitHub workflow
YAML (§3.3). They had to become one. Should the consolidated reader keep the
indentation heuristic, or should `scripts/ci` take a PyYAML dependency?

## Ruling

**Keep the heuristic. No PyYAML dependency.**

The decision rule's first branch: *keep the heuristic if it raises on every
mutant it cannot handle correctly.* As first consolidated it did **not** — four
probes misread a mutant silently. Step 2's literal contract ("zero jobs or zero
`run:` steps is an error") does not catch a *partial* misread, which is the
dangerous case. Three added guards close all four, and the corpus now shows
**zero silent misreads**:

| Guard | Catches |
|---|---|
| a job-level line that is not a readable job header refuses | `quoted-job-name` |
| a step declaring neither `run:` nor `uses:` refuses | `flow-step` |
| a `run:` value opening `> | * &` refuses | `alias-run`, `folded-scalar` |

With those in place the first branch is satisfied and it is, as the plan
predicted, the cheapest outcome. The second branch (take PyYAML) would in any
case have landed on its own fallback — "otherwise take it with an explicit
`just init` install step" — because `yaml` is missing from three of the five
interpreters measured, and no CI workflow has a `setup-python` or `pip install`
step to add one to. That price buys nothing the guards do not.

## Baseline (the plan's precondition was not met; this is the real one)

The plan assumed a clean tree and 607 green tests. The tree carried ~229
modified/untracked files from an in-flight fix. **The test count held anyway**:
all 13 suites, 607 tests, 0 failures, 0 skips on this host (macOS 27.0,
Homebrew `python3` 3.14.7), ~2m49s wall. A recent review's count of 590 passing
did not reproduce; nothing in this spike explains the difference, and the suites
are green either way, so it is recorded and dropped.

| Suite | Tests | Wall (before) | Wall (after) |
|---|---:|---:|---:|
| `test_affected_scope.py` | 174 | 3.9s | 3.2s |
| `test_schema.py` | 69 | 0.1s | 0.1s |
| `test_evidence_reuse.py` | 66 | 46.1s | 43.7s |
| `test_resolved_plan.py` | 64 | 16.7s | 15.7s |
| `test_ci_local.py` | 62 → **66** | 70.4s | 57.8s |
| `test_runner_loss.py` | 43 | 0.1s | 0.1s |
| `test_constraints.py` | 39 | 0.8s | 0.8s |
| `test_local_evidence.py` | 20 | 11.5s | 11.0s |
| `test_publish_gaps.py` | 19 | 0.5s | 0.5s |
| `test_reuse_validation.py` | 15 | 0.1s | 0.1s |
| `test_cross_check.py` | 13 | 12.1s | 11.2s |
| `test_build_key.py` | 12 | 0.2s | 0.1s |
| `test_build_baseline_revision.py` | 11 | 6.3s | 5.6s |
| **Total** | **607 → 611** | **168.8s** | **149.9s** |

The three suites this spike touched also pass under stock macOS
`/usr/bin/python3` 3.9.6, which is the interpreter the pre-push hook may get.

## Step 1 — consolidation, no behavior change

`scripts/ci/workflow_reading.py` (315 lines) now holds the layout assumption,
behind `job_names`, `job_blocks`, `job_block`, `job_run_steps`, `step_script`,
and `step_run_lines`. All four call sites are ported.

**Lines of duplicated reader removed:**

| File | Removed | Replaced by |
|---|---:|---|
| `test_ci_local.py` | 86 (`workflow_step_script`, `JobStep`, `workflow_job_run_steps`) | `job_run_steps`, `step_script` |
| `test_build_baseline_revision.py` | 18 (`_run_body`) | `step_run_lines` |
| `test_runner_loss.py` | 11 (`jobs_in` body; a 3-line delegation stays) | `job_blocks` |
| **Total** | **115** | one module |

`job_label` and `status_artifact` stay in `test_runner_loss.py`. They are
`re.search` over a block `job_block` now supplies, and they encode GitHub's
job-labelling policy and the runner-loss artifact naming convention, not
workflow layout. The plan's API does not ask for them.

**Equivalence was proved, not asserted.** A differential harness ran each of
the four originals and its replacement over every one of the 14 shipped
workflows, every job in each, and every named step in each — 584 probes:

```
identical=484  differing=0  new-only-raise=100  old-only-raise=0
```

Zero differing reads. The 100 new raises are step 2's loudness, on inputs no
caller asks for: 96 are `step_run_lines` against a `uses:` step (the old
`_run_body` scanned unbounded and would have returned **a later step's**
script — see findings) and 4 are `job_run_steps` against a pure `uses:`
delegation job.

## Step 2 — loud degradation

**No test broke.** The plan budgeted for a suite that depended on a silent
truncation; none did. The gate is met with the count *up*, not down.

## Steps 3+4 — the mutation corpus against both implementations

`scripts/ci/fixtures/workflow_mutants/` — `base.yml` cut verbatim from
`.github/workflows/ci.yml` plus ten one-edit mutants, all valid YAML.
Provenance is recorded in the fixture `README.md` (repository `HEAD`
`12a632ea1b77a5f291bc16ec289f698e5ec04ca9`, `ci.yml` blob
`da4ac96be2b8415fd7cd5f12481748038d9bc3c5`). A second implementation of the
same API over `yaml.safe_load` was written for this step only and is not
shipped.

Five probes per mutant, each mirroring a real call site. `reads` is same /
differs / raises against that reader's own answer on the base; `semantics` is
ground truth from `yaml.safe_load`. Rows where both readers read `same` are
omitted.

| Mutant | Semantics | Probe | Heuristic | Parser | Heuristic == parser? |
|---|---|---|---|---|---|
| `reindent-job` | unchanged | `job_run_steps(ci-gate)` | **raises** | same | raises |
| `reindent-job` | unchanged | `step_run_lines(gate)` | **raises** | same | n/a |
| `folded-scalar` | changed | `job_run_steps(preflight)` | **raises** | differs | raises |
| `folded-scalar` | changed | `step_script(toolchain)` | **raises** | differs | raises |
| `anchor` | unchanged | (all five) | same | same | yes |
| `flow-mapping` | unchanged | (all five) | same | same | yes |
| `key-reorder` | unchanged | `step_script(toolchain)` | **raises** | same | raises |
| `comment-in-block` | changed | `job_run_steps(preflight)` | differs | differs | yes |
| `comment-in-block` | changed | `step_script(toolchain)` | differs | differs | yes |
| `quoted-job-name` | unchanged | `job_names` | **raises** | same | raises |
| `quoted-job-name` | unchanged | `job_run_steps(preflight)` | **raises** | same | raises |
| `quoted-job-name` | unchanged | `job_run_steps(ci-gate)` | **raises** | same | raises |
| `trailing-blank` | changed | `job_run_steps(preflight)` | differs | differs | yes |
| `trailing-blank` | changed | `step_script(toolchain)` | differs | differs | yes |
| `flow-step` | unchanged | `job_run_steps(preflight)` | **raises** | same | raises |
| `alias-run` | changed | `job_run_steps(preflight)` | **raises** | differs | raises |

**Silent misreads: 0.** Every `differs` is on a mutant `yaml.safe_load` also
calls changed, and the parser reads it differently too. Every case the
heuristic cannot read is a refusal.

Before the three guards were added, the same corpus produced **four** silent
misreads: `quoted-job-name` / `job_names`, `quoted-job-name` /
`job_run_steps(preflight)`, `flow-step` / `job_run_steps(preflight)`, and
`alias-run` / `job_run_steps(preflight)`. That measurement is the whole
justification for the ruling and is why the corpus ships.

## Step 5 — availability

| Host | Interpreter | `import yaml` | How known |
|---|---|---|---|
| macOS (this host) `/usr/bin/python3` | 3.9.6 | **fails** | measured |
| macOS (this host) Homebrew `python3` | 3.14.7 | ok, 6.0.3 | measured |
| `BUILD_LINUX` `/usr/bin/python3` | 3.13.5 | **fails** | measured over SSH |
| `BUILD_WSL` `/usr/bin/python3` | 3.14.4 | ok, 6.0.3 | measured over SSH |
| `BUILD_WIN` `py -3.13` | 3.13.14 | **fails** | measured over SSH |
| `ubuntu-latest` runner | — | **not measured** | see below |
| `windows-latest` runner | — | **not measured** | see below |

`BUILD_MACOS` is undeclared from this host.

**The two hosted-runner rows could not be measured and are not guessed.**
Measuring them means running a step on a hosted runner, which means editing a
workflow and pushing — both outside this spike. The `actions/runner-images`
Ubuntu 24.04 software manifest does not list PyYAML in any form, so there is no
documented answer either. What *is* measurable locally and matters as much: no
workflow in `.github/workflows/` has a `setup-python` or `pip install` step, so
a PyYAML dependency would require editing the workflows — the one thing the
plan puts out of scope.

Three of the five interpreters actually measured lack `yaml`, including both
interpreters the pre-push hook is most likely to get on a fresh macOS or Linux
developer machine. The hook is the binding constraint and it says no.

## What shipped

| Path | New? |
|---|---|
| `scripts/ci/workflow_reading.py` | new |
| `scripts/ci/fixtures/workflow_mutants/` (11 `.yml` + `README.md`) | new |
| `scripts/ci/test_ci_local.py` | modified (86 removed, `WorkflowReadingCorpusTests` added) |
| `scripts/ci/test_runner_loss.py` | modified (11 removed) |
| `scripts/ci/test_build_baseline_revision.py` | modified (18 removed) |

- §3.3 closed: one reader, four call sites.
- Loud degradation, verified by the corpus rather than by inspection.
- The corpus ships as a permanent regression fixture, pinned by
  `WorkflowReadingCorpusTests` in `test_ci_local.py` (4 tests). It went there
  rather than into a new `test_workflow_reading.py` on purpose: §1.1 of the
  review is that two suites are executed by nothing, and `test_ci_local.py` is
  already in both the `ci-tooling` CI job and `just ci-local`'s self-test loop.
  A new suite would have needed enrolling in four places (§3.6).

## What did not ship

- The `yaml.safe_load` implementation. It exists only as a spike artifact and
  was deleted with the scratchpad; re-deriving it from the API is an hour.
- No workflow YAML was edited (the plan forbids it).

## Found, and not anticipated by the plan

1. **`_run_body` could silently return a different step's script.**
   `test_build_baseline_revision.py:238` searched forward from its anchor for
   `^        run:` with no upper bound, so a `uses:`-only gate step would have
   made it return the *next* step's `run:` body. Its whole job is a
   byte-identity comparison of the measured commands between two revisions
   (`test_the_measured_commands_are_byte_identical_to_the_base`), so it would
   have compared the wrong pair of scripts and passed. 96 of the differential's
   100 new raises are this case. `step_run_lines` now bounds the search to the
   named step.

2. **`job_run_steps` read `needs:` list items as steps.**
   `ci.yml`'s `ci-gate` and `summary` declare `needs:` as a sequence whose
   items sit at the same six-space indent as a step, so the original
   `workflow_job_run_steps` treated `- validation`, `- scope`, … as nine
   phantom steps. They were silently dropped for having no `run:`, which is the
   only reason the old reader gave the right answer. Reading is now bounded to
   the job's `steps:` key.

3. **The plan's `anchor` and `flow-mapping` mutants are vacuous.** Both put
   their YAML feature inside a `uses:` step's `with:` block, which every reader
   skips, so neither probes the reader at all — both read identically under both
   implementations on all five probes. `flow-step` and `alias-run` were added to
   put the same two features where the readers do look, and they were two of the
   four mutants that produced a silent misread.

4. **`trailing-blank` is not semantically unchanged.** The plan lists it as
   unchanged. `yaml.safe_load` disagrees: a blank line inside a literal block
   scalar is content, and both readers correctly read the mutant differently.
   The fixture `README.md` records the measured semantics, not the declared one.

5. **`step_script` and `job_run_steps` disagree about key order.** On
   `key-reorder`, `job_run_steps` reads the step correctly (it looks `name:` up
   as a key) while `step_script` refuses (it identifies a step by the exact
   opener line `      - name: <name>`). Not a defect — both are loud or right —
   but two functions in one module with different tolerances is a trap for the
   next reader, and worth a follow-up to make `step_script` locate a step the
   way `job_run_steps` does.

6. **No workflow declares a PyYAML dependency route.** Beyond deciding this
   spike, it means any future `scripts/ci` third-party dependency needs a
   `setup-python`/install step added to `ci.yml`'s `preflight` and `ci-tooling`
   jobs *and* a documented `just init` path for the hook. Worth knowing before
   the question is asked again.

7. **The native Windows build host has no usable `python3`.** On `BUILD_WIN`,
   `python3` resolves to `C:\cygwin64\bin\python3`, a Cygwin shim pointing at a
   `Python313\python.exe` that no longer exists; PowerShell cannot exec it at
   all and Git Bash fails with `No such file or directory`. A working 3.13.14 is
   reachable only as `py`. Nothing in this spike depends on it, but any plan to
   run the Python suites there — or a pre-push hook invoked on that host — would
   fail on the interpreter, not on the tests. Low severity, easy to fix, worth
   recording in the `os` skill.

## Follow-ups this spike deliberately did not take

- Make `step_script` locate a step by its `name:` key rather than its opener
  line, so it tolerates key order as `job_run_steps` does (finding 5).
- Move `job_label` / `status_artifact` behind named helpers if a third caller
  appears; two is not yet duplication.
- `scripts/ci/fixtures/workflow_mutants/` gives the reader a regression corpus.
  Nothing yet stops a *shipped* workflow from adopting a layout the reader
  refuses: that failure would surface as a red `test_ci_local.py`, which is
  loud and correct but only after the fact. A lint that reads every shipped
  workflow through `job_run_steps` at CI time would catch it at the edit.
