# Steering Fleet Research Run

Date: 2026-09-08
Status: Complete — passive research validated; live delivery unverified

The user authorized a full steering research refresh for all ten eligible
providers in `claudine/docs/providers.yaml`, using `gpt-5.6-sol` with low
reasoning, plus an update to Pi's non-interactive execution research.

## Contract and Scope

Steering schema revision 2 incorporates the four pilots: explicit launch profiles,
profile-specific capability cases, acknowledgment guarantees, target guards,
protocol maturity and framing, queue/tool-batch behavior, input interpretation,
and partial interruption outcomes. A synthetic document passed Darkmatter schema
validation before launch. Claude, OpenCode, and Codex then passed the revised
shape and relationship checks. The remaining six providers started while Pi's
refresh and substantive pilot corrections continued: the outstanding fixes were
report content, not contract changes. All ten reports still require the same
final gates before this research run can be declared complete.

This is passive research using documentation, source, and local inspection.
No live steering experiments are part of this run. Reports must retain
`verification: []`; researched capabilities do not establish activation eligibility.
Production provider facts, generated Rust, and wrapper behavior are not changed.

## Execution and Model Verification

The installed Claudine rejects the shared roster's older `list:` sequence key.
Temporary roster projections retain the canonical entries and template, changing
only the execution envelope to `sequence:` and dividing the roster into disjoint
batches. Biscuit-file performs YAML/JSON conversion. Temporary prompt copies use
the authored steering fleet body with absolute schema/roster references and a
bounded research instruction. No alternative permanent provider roster is created.

Batch membership:

- Pilot A: Claude Code, Codex.
- Pilot B: OpenCode, Pi.
- Remaining A: Gemini, Goose.
- Remaining B: Kimi, Qwen.
- Remaining C: Kilo, Antigravity.

The launch form is:

```sh
claudine sequence /tmp/claudine-steering-<batch>.md --yolo --codex -- -m gpt-5.6-sol -c model_reasoning_effort=low
```

`--yolo` permits the authorized unattended research and schema-validation shell
steps. Each prompt limits writes to its provider report and prohibits live
delivery, configuration changes, and production edits.

The first rehearsal showed the requested frontmatter model, but the first two
live sessions recorded `gpt-6-astra` with low reasoning. Both were interrupted
and excluded. Explicit native `-m gpt-5.6-sol` forwarding corrected selection;
the restarted sessions' own `turn_context` records confirm the requested model
and effort. Simultaneous tool launches also produced configuration-path errors
before starting some research sessions; individually launched retries started
successfully. Subsequent batch starts are serialized while research runs overlap.
The underlying startup issue was not diagnosed or repaired in this research task.
Model verification
must cover each subsequent provider launch, not just the initial batch member.

## Validation

In addition to `md schema validate`, the coordinator checks roster identity,
revision/date/model fields, unique IDs, evidence references, each profile's
declared Cartesian-product coverage, exactly 24 ordinary baseline combinations,
profile/OS/origin-matched discovery references, mechanism/profile/OS access and
compatibility coverage, and one receipt/lifecycle record per mechanism. Passive
reports may not claim disposable tests. Source review remains separate from these
mechanical checks.

All ten reports passed the final schema and relationship checks, including
non-inference evidence for supported cases. Totals: **24 launch profiles,
360 capability cases, 31 mechanisms, and 91 evidence records**. The 240 ordinary
baseline combinations are covered exactly once per provider; the remaining
120 cases cover special launch profiles. Every report has `verification: []`.
The Pi execution report also passes its existing topic schema, and scoped
`git diff --check` passes.

All five accepted sequence batches finished with two successful provider steps
and no failed steps. Earlier canceled model-mismatch launches and failed startup
attempts are excluded. Gemini's first step delegated one additional researcher;
both sessions used the requested model/effort. The fleet prompt now explicitly
identifies the worker as the already-running researcher and prohibits recursive
orchestration. Eleven accepted execution-context records were inspected in total,
including that nested Gemini researcher. Corrective review tasks used the same
requested model and low reasoning through explicit agent-launch settings.

## Provider Findings

These are researched candidates, not an enabled-support matrix. See each report
for exact OS, version, access, profile, and evidence limitations.

| Provider | Profiles | Cases | Mechanisms | Main finding and limitation |
| --- | ---: | ---: | ---: | --- |

| [Claude Code](../../docs/research/steering/claude.md) | 3 | 36 | 4 | Native peer messaging is a candidate; raw framing is partly undocumented, Windows token access is unresolved, and ordinary one-shot lifetime must not be treated as retained idle availability. |
| [Codex](../../docs/research/steering/codex.md) | 2 | 36 | 3 | Managed app-server supports guarded active-turn steering and distinct idle turn starts; ordinary exec access and native exposed-server discovery remain gaps. |
| [Gemini](../../docs/research/steering/gemini.md) | 2 | 30 | 2 | Managed ACP supplies control, but active replacement requires interruption; idle prompting is a separate operation. |
| [Goose](../../docs/research/steering/goose.md) | 2 | 36 | 3 | Managed ACP provides guarded steering with queued/pickup signals; delivery waits for a model/tool boundary and cannot rescue uninterrupted generation. |
| [Kimi](../../docs/research/steering/kimi.md) | 3 | 48 | 4 | Web-server steering and ACP cancellation/replacement differ; interface selection must compare their actual delivery behavior. |
| [OpenCode](../../docs/research/steering/opencode.md) | 4 | 48 | 5 | Exposed HTTP profiles differ from ordinary CLI sessions; async acceptance does not establish persistence, scheduling, or timely incorporation during active work. |
| [Qwen](../../docs/research/steering/qwen.md) | 2 | 30 | 3 | The serve daemon queues correlated follow-ups for the next turn. It can accept during active work but cannot break a loop that never finishes its current turn. |
| [Pi](../../docs/research/steering/pi.md) | 2 | 36 | 3 | Retained RPC provides steering after the full tool batch. Abort retains queues; current-session targeting is mutable; full settlement uses agent_settled. |
| [Kilo](../../docs/research/steering/kilo.md) | 2 | 30 | 3 | A managed authenticated HTTP server offers queued input; an immediate response proves admission rather than model delivery. |
| [Antigravity](../../docs/research/steering/antigravity.md) | 2 | 30 | 1 | Retained structured input is an idle-session candidate; active delivery, exact request shape, and acknowledgment guarantees remain unresolved. |

## Review Corrections and Pi Execution Research

Review corrected profile conflation, active-versus-idle operation intent,
undocumented protocol maturity, and post-delivery permission holds incorrectly
listed as incoming-message holds. Successful terminal responses are distinguished
from early acceptance receipts; a prompt handled by an extension is not assumed
to schedule a model turn. Ordinary TUI lifetime is not classified as one-shot.

[Pi execution research](../../docs/research/non-interactive-sessions/pi.md) now
prefers managed RPC for one-shot and retained work, preserves extensions, skills,
templates, and context, and permits a verified compatible launch fallback with a
warning before submission. It rejects replay after ambiguous submission. Review
corrected the earlier claim that `agent_settled` was absent: v0.84.4 defines and
forwards it after post-run continuation and extension settlement. Shutdown and
unattended extension UI remain explicit adapter responsibilities.

The [existing execution fleet prompt](../../docs/research/non-interactive-sessions/_fleet.md)
now enumerates control interfaces before output formats, prefers usable RPC or
equivalent bidirectional control, requires evidenced alternatives, and prohibits
disabling enabled features merely to simplify parsing. Only Pi's provider report
was refreshed in that topic; typed execution-interface schema/generator changes
and the remaining execution-topic reports are separate follow-on work.

## Outstanding Implementation Gates

- Implement reviewed discovery, transport ownership, target guards, and delivery
  adapters through the existing generated metadata pipeline.
- Define unattended extension/reverse-request handling without fabricating human
  approval or silently disabling features.
- Perform disposable-session tests for every enabled OS/version/profile/mechanism,
  including timing, same-conversation delivery, stale targets, queue behavior,
  interruption cleanup, and ambiguity without unsafe retries.
- Resolve the spec's remaining engineering choices, including recovery-window
  sizing and warning-threshold rounding, then implement and test the feature.

No production facts, generated Rust, or wrapper behavior were changed in this run.

## Verified Execution Contexts

Every row below recorded `model: gpt-5.6-sol` and `effort: low` in the research
session's own Codex `turn_context`. Batch membership is listed above; the extra
Gemini worker is included under Remaining A.

| Batch | Session ID |
| --- | --- |
| Pilot A | `01a0816f-c9d8-72b1-bd6b-f9b1327b2630` |
| Pilot A | `01a08176-cda5-7db3-8c9a-b2df91021e2c` |
| Pilot B | `01a08170-f20b-76f3-b6d7-3b8d296c6ffe` |
| Pilot B | `01a08177-cf7a-7e02-b781-d341cdd4ea98` |
| Remaining A | `01a0817f-3f8a-7d73-a553-0ceba681c0c1` |
| Remaining A | `01a0818a-4e43-7c90-9507-39215694ac47` |
| Remaining A | `01a08181-03bb-73f1-9f57-70909825f32b` |
| Remaining B | `01a0817f-e35a-7c32-a9e5-2693e2b84dbc` |
| Remaining B | `01a0818c-8d1b-7752-85b7-1226f1175744` |
| Remaining C | `01a08180-0270-7e52-8c7b-38470dddd4bf` |
| Remaining C | `01a0818a-d283-72e0-8aa4-4af1dde303ef` |

## Revision 3 Follow-Up

The user authorized the post-fleet refinements. The ten steering reports were
migrated to revision 3 from the existing evidence, retaining their original
research provenance and empty live-verification records. The backfill adds
31 receipt-timing observations and an interface inventory with explicit unknown
coverage where the original profiles omit modes or origins. Discovery gaps now
have exact case keys; none of the currently supported cases lacks a discovery
reference. No additional provider experiment was performed.

OpenCode review removed interruption-consent prerequisites from non-interrupting
Linux paths and corrected asynchronous receipt text that described a synchronous
response. Request acceptance, useful current-turn delivery, and full settlement
remain separate concepts. In particular, next-turn queueing stays valid manual
messaging and does not qualify as rescue of an endless current turn.

The non-interactive execution contract now inventories interfaces independently
of output formats, with feature-preservation evidence, selection/fallback rules,
unattended requests, per-interface settlement, and references to steering
mechanisms. New evidence states describe source-backed findings, not live
activation. The backfill uses existing research and marks remaining uncertainty;
it does not reset the dates of the original provider observations.

The fleet success hook now requires `claudine providers steering check <slug>`
in addition to Darkmatter shape validation. This command belongs to the existing
generator-binary boundary, with `claudine-gen steering check` as its direct form.
Future fleet runs must use matching updated builds; a missing command fails the
gate rather than silently skipping validation.

Final follow-up validation:

- All ten steering reports pass revision-3 shape, relationship, coverage, and
  execution-topic reference checks through the maintained Rust command.
- All ten execution reports contain the five new typed sections. Original
  observation provenance is preserved; backfill notes identify the separate
  existing-evidence update. Provider backfill/review workers used `gpt-5.6-sol`
  with low reasoning.
- The generator and provider CLI compile, and generator lint passes.
- `just test --no-fail-fast` in `claudine/gen` passes **161 tests, one skipped**,
  including malformed-reference cases and rejection before generated output writes.
- Catalog regeneration adds only the five execution-research sections per provider
  relative to the pre-regeneration working catalog. No generated provider behavior
  changes result. The catalog byte baseline was updated using Biscuit-hash XXH64.
- Scoped `git diff --check` passes. No commits or live provider delivery tests
  were performed.

Generation validates steering before catalog/data/artifact application when the
topic exists. The existing `--scaffold` onboarding step can still create missing
hand-owned inputs before that gate; it does not establish research validity.
Legacy areas without the steering topic retain their prior generation path.
Research validation does not infer activation from the presence of a test record:
reports expose verification-record counts and passed-record counts, not a blanket
provider-level verified flag.

Remaining gaps are provider evidence and implementation work: unknown profile
combinations, feature parity and unattended request handling, exact delivery
boundaries and acknowledgment contracts, and disposable delivery tests. A new full
fleet run is unnecessary for this contract migration. Future research should
target those gaps, preserving the same model/effort requirements.
