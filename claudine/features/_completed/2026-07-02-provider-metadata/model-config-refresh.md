# model-config Refresh — 2026-07-02

Record of the quality revision executed per `model-config-plan.md`. The prompt is now
**frozen** at the version described below.

## The defect

The original 2026-07-02 fleet framed local-runner support as a property of the provider
having "native support" — the Claude Code doc claimed **no native support for local
model runners** and marked all five runners `unsupported`, despite every runner serving
an Anthropic-compatible endpoint reachable via `ANTHROPIC_BASE_URL`. The prompt never
asked about cross-cloud bridging at all.

## Schema iteration (one revision)

- `local_runners[].supported: enum(native,openai_compatible,unsupported)` →
  `integration: enum(first_class,base_url_override,proxy_required,unsupported)` plus an
  optional `standard: enum(openai_compatible,anthropic_compatible,bespoke)` naming the
  API standard the path rides on.
- Added `cloud_bridge: { supported: boolean(required), mechanism, example }`.
- Validated positive and negative (old key, old enum value, string-typed boolean all
  rejected) with `md schema validate` on temp docs.

## Prompt iterations (two revisions)

1. **Bridging reframe** — the local-models section now opens with the framing ("the
   question is never 'does X support Ollama'"), instructs reading
   `docs/research/local_runners/*.md` frontmatter as ground truth instead of
   re-researching runners, adds the cross-cloud bridging questions, and carries an
   explicit anti-pattern rule: never describe absent first-class integration as "no
   support" when a base-URL override path exists.
2. **Cloud-bridge consistency guard** (post-pilot) — the bridge example must be
   consistent with the standard(s) the client actually speaks; if the target vendor's
   native API doesn't serve that standard, show the translating proxy (LiteLLM), not a
   direct base URL. Added after the Codex pilot pointed a Responses-only client
   directly at Mistral's Chat-Completions API.

## Execution notes

- All 9 docs had been produced earlier the same day, so the initialize same-day skip
  would have blocked every re-run. Forcing mechanism (Ken-approved): backdate
  `last_updated` to 2026-07-01 on docs being re-run — keeps update mode (changelogs)
  intact, and the success stack still verifies today's stamp.
- Pilot: Claude Code + Codex (temp roster). Fleet: full roster, pilots correctly
  same-day-skipped. All steps ran on `opencode` / `kimi-for-coding/k2p7` (verified via
  first `llm_call_start`); no caps hit, no model substitution observed.

## Evaluation verdicts (9 docs, parallel subagents vs ground truth)

- **Acceptable as produced:** claude, gemini, goose, qwen-cli, kilo.
- **Acceptable via targeted edit** (applied by the orchestrator):
  - `codex.md` — cloud-bridge example targeted Mistral's API directly; rewritten to
    route through a Responses-translating LiteLLM proxy (frontmatter + body +
    changelog).
  - `opencode.md` — oMLX misclassified `base_url_override`; ground truth ships
    `omlx launch opencode` → reclassified `first_class` (frontmatter, body table,
    changes, reason).
  - `pi.md` — same oMLX miss (`omlx launch pi` exists) plus a fabricated
    "`ollama launch pi`" claim (no such hook in ground truth) — reclassified and
    deleted respectively.
  - `kimi.md` — cited `ANTHROPIC_BASE_URL` as an override the doc's own env inventory
    and precedence section say doesn't exist; dropped from mechanism and body.
- **Re-runs required:** none.

## Exit criteria (all met)

9/9 schema-valid; no doc claims "no local model support" where a base-URL path exists;
`local_runners[].integration` (6 records each) and `cloud_bridge` populated in all 9;
ground-truth spot-checks passed (ports, auth shapes, hooks, since-versions); dated
changelog entries present in every re-run doc.

## Outputs

- `.claude/skills/model-config/` — SKILL.md (cross-provider comparison, runner
  integration matrix, bridging + merge-semantics guidance) + `providers.md`
  (per-provider reference), distilled from the validated frontmatter.
- `claudine` skill research section updated: model-config summary + pointer to the
  **model-config** skill (hash regenerated).

## Remaining gaps

- The Kilo doc covers the Kilo CLI only; the VS Code extension's Roo/Cline-inherited
  provider dropdowns are out of scope and unmentioned (flagged by the evaluator as a
  candidate one-line scoping sentence for a future revision).
- Minor cosmetic drift in `goose.md` (an Ollama tag example sentence under a tagless
  example) — non-blocking, left as-is.
