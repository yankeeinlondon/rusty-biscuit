# Fleet Review — live roster telemetry and accepted checkpoint

Increment **B3** of spec `2026-07-11-provider-errors-as-data`. This records the
live ten-provider run, deterministic outcomes, cross-provider review,
source-liveness advisory, and Ken's accepted fleet checkpoint.

## Execution

The roster ran on 2026-07-14 through [`_fleet.md`](./_fleet.md), using Codex
with low reasoning effort. Codex's retained session metadata reports resolved
model `gpt-5.6-sol`; the authored `model: default` frontmatter records the
sequence's model selector rather than the resolved catalog model.

| Provider | Session start (PDT) | Document saved | Clean gate | Attempts | Resumes |
|---|---:|---:|---:|---:|---:|
| claude | 15:22:17 | 15:24:30 | 15:25:28 | 1 | 0 |
| codex | 15:25:29 | 15:28:05 | 15:28:50 | 1 | 0 |
| gemini | 15:28:52 | 15:32:20 | 15:32:29 | 1 | 0 |
| goose | 15:32:30 | 15:34:46 | 15:34:56 | 1 | 0 |
| kimi | 15:34:58 | 15:37:22 | 15:37:42 | 1 | 0 |
| opencode | 15:37:44 | 15:42:34 | 15:42:55 | 1 | 0 |
| qwen | 15:42:56 | 15:45:33 | 15:45:42 | 1 | 0 |
| pi | 15:45:44 | 15:47:58 | 15:48:15 | 1 | 0 |
| kilo | 15:48:16 | 15:50:31 | 15:50:43 | 1 | 0 |
| antigravity | 15:50:44 | 15:52:50 | 15:53:05 | 1 | 0 |

The sequence reached ten clean outcomes in 30 minutes 48 seconds. Each gate
followed the corresponding document write, all ten documents validate against
`_schema.yaml`, and all ten deterministic reports are explicitly
`status: clean`. No resume or provider retry fired.

## Roster and deterministic results

| Provider | Schema | Gate | Seeds | Accepted additions |
|---|---|---|---|---:|
| antigravity | valid | clean | all | 0 |
| claude | valid | clean | all | 12 |
| codex | valid | clean | all | 2 |
| gemini | valid | clean | all | 6 |
| goose | valid | clean | n/a; no parser | 0 |
| kilo | valid | clean | all | 5 |
| kimi | valid | clean | all | 4 |
| opencode | valid | clean | all | 6 |
| pi | valid | clean | all | 18 |
| qwen | valid | clean | all | 2 |

Every unresolved provider surface remains an explicit `gaps` entry. Goose
stays research-only with an empty runtime table. Antigravity retains its seed
vocabulary because no stronger published error contract was found.

## Cross-provider consistency

The flattened kind, message, and numeric-code rows were compared across all
provider pairs. No pair is identical after the live research. OpenCode and
Kilo retain shared seed lineage but now diverge through provider-specific
attested additions. Gemini and Qwen retain their expected fork-lineage
similarity without an identical vocabulary. No document was returned for
copy-paste remediation.

Within each provider, all additions are append-only. There are no removals,
re-kinds, reorderings, exact duplicates, or new cross-kind substring overlaps.
Six additions are benign same-kind refinements shadowed by an earlier broader
seed; the C1 report records them explicitly.

## Source-liveness advisory

On 2026-07-14, all HTTP(S) citations in the current ten-document corpus were
probed with redirects enabled, a 5-second connection timeout, and a 15-second
total timeout. The 96 cited URLs collapse to 64 distinct resources after
fragment identifiers are removed. All 64 returned an HTTP response below 400.

Liveness remains advisory. It does not strengthen provenance and cannot turn a
clean deterministic result into a failure. Tag-pinned source permalinks and
official documentation remain the preferred evidence in each provider file.

## Graduation

Ken accepted all 55 additions on 2026-07-14. The generated runtime vocabulary
now projects the accepted research frontmatter, and Level-1 coverage verifies:

- every accepted row is present in its adjudicated bucket and classifies to
  the expected `SemanticErrorKind`;
- exact numeric Kimi codes win before message prose;
- structured kinds win before message buckets;
- bucket ordering remains first-match-wins; and
- representative near-miss prose stays `AgentNative`.

## Human checkpoint (◆ B3) — accepted

Ken accepted the complete live roster, the retrospective B2 checkpoint, the
zero-resume convergence result, the advisory liveness report, and the full C1
delta on 2026-07-14. No provider requires rerunning before closeout.
