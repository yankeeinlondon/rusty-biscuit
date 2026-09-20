# Workflow mutation corpus

Ten one-edit mutations of `base.yml`, used by `WorkflowReadingCorpusTests`
(`scripts/ci/test_ci_local.py`) to hold `scripts/ci/workflow_reading.py` to its
contract: **every layout the indentation heuristic cannot read correctly, it
refuses.** A reader that silently returns a shorter step list would let
`WorkflowScopeStepTests` execute a shorter `ci.yml` scope job and still pass.

Cut once, by hand, from the shipped workflows and committed. Do not regenerate
them against a later `ci.yml` — they are a fixed corpus, not a mirror of the
current file. Provenance:

| | |
|---|---|
| Source | `.github/workflows/ci.yml` (working tree, `git hash-object` `da4ac96be2b8415fd7cd5f12481748038d9bc3c5`) |
| Repository `HEAD` at cut | `12a632ea1b77a5f291bc16ec289f698e5ec04ca9` |
| Cut | 2026-09-15, Spike 1 (`reviews/2026-09-15-python-test-code/spike-1-results.md`) |

`base.yml` is verbatim `ci.yml` lines: `preflight` (447-481) and, under a
synthesized `ci-gate:` header, the `actions/checkout` step (456-459) followed by
`ci-gate`'s `steps:` body (878-908). It exercises a `uses:` step with a `with:`
mapping, a one-line `run:`, two `run: |` block scalars, and comment lines
between steps.

| Mutant | Edit | Semantically (measured with `yaml.safe_load`) |
|---|---|---|
| `reindent-job` | `ci-gate`'s body shifted right two spaces | unchanged |
| `folded-scalar` | one `run: \|` becomes `run: >-` | changed (line joining) |
| `anchor` | the two `with:` blocks become `&checkout` / `*checkout` | unchanged |
| `flow-mapping` | one `with:` becomes `{ref: ..., fetch-depth: 0}` | unchanged |
| `key-reorder` | a step's `run:` written before its `name:` | unchanged |
| `comment-in-block` | a `#` line inside a `run: \|` body | changed (a shell comment) |
| `quoted-job-name` | `"ci-gate":` instead of `ci-gate:` | unchanged |
| `trailing-blank` | a blank line inside a `run: \|` body | changed (the scalar keeps it) |
| `flow-step` | one step written as a flow mapping | unchanged |
| `alias-run` | a `run:` value becomes an alias (`*toolchain`) | changed |

The last two are not in the spike plan. Its `anchor` and `flow-mapping` mutants
put both features inside a `uses:` step's `with:`, which every reader skips, so
neither probes the reader at all; `flow-step` and `alias-run` put the same two
features where the reader does look. They were the only mutants that produced a
silent misread, and closing them is what earned the heuristic its keep.
