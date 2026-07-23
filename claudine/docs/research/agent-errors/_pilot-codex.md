# Codex Pilot — live telemetry and retrospective checkpoint

Increment **B2** of spec `2026-07-11-provider-errors-as-data`. This records the
real Codex research execution, deterministic-gate outcome, collision review,
and Ken's accepted retrospective checkpoint.

## Execution

The live roster sequence ran on 2026-07-14 with:

```sh
claudine sequence docs/research/agent-errors/_fleet.md -y --codex \
  -c 'model_reasoning_effort="low"'
```

The Codex step was a real `source=exec` session in the provider-errors
worktree. Codex retained rollout
`019f62bc-375a-7560-8c52-ae00f441edb9`, started at 15:25:29 PDT, using
resolved model `gpt-5.6-sol` with low reasoning effort. The prompt's `model`
selector was `default`, which is why the research frontmatter records
`model: default`; the rollout metadata records the resolved model.

[`codex.md`](./codex.md) was saved at 15:28:05. The success lifecycle then
validated its schema and ran the deterministic gate, which wrote an explicit
clean outcome at 15:28:50. The step therefore reached clean in 3 minutes 21
seconds from session start.

## Validate-and-resume telemetry

| Measure | Observed result |
|---|---|
| Provider sessions | 1 |
| Research attempts | 1 |
| Resumes fired | 0 |
| Deterministic findings | 0 |
| Final outcome | `status: clean` |
| Budget | 1 of the initial attempt plus 2 allowed remediation turns used |

The clean first attempt establishes that the live provider received the fleet
prompt, wrote the intended document, retained every seed, supplied coherent
provenance, and completed within the configured budget. No correction prompt
was needed. Synthetic lifecycle tests continue to cover the findings/resume
and exhausted-budget paths that a clean live document cannot exercise.

## Collision review

The accepted Codex additions remain narrow message needles in the first
`api_remote` bucket:

- `overloaded`
- `selected model is at capacity`

Both are appended after the immutable seeds. Structured configuration kinds
still win before message classification, and ordinary capacity-planning prose
without the exact phrase remains `AgentNative`. Bare HTTP numbers and the broad
words `model` and `capacity` were not added as message needles.

## Signals boundary

Usage-cap and rate-limit detection records remain in
[`signals/codex.md`](../signals/codex.md). This topic owns only the rendering
classification vocabulary. [`_signals-overlap.md`](./_signals-overlap.md)
records that boundary.

## Human checkpoint (◆ B2) — accepted retrospectively

The full roster was launched as one sequence, so the originally specified
human pause between the Codex pilot and the remaining providers did not occur.
After the run, the live Codex rollout, clean gate result, convergence, and
budget fit were reviewed. Ken accepted this retrospective B2 checkpoint on
2026-07-14 and authorized C2/C3. The sequencing variance did not conceal a
repair need: Codex and every later provider converged in one attempt.
