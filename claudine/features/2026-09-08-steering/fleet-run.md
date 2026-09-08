# Steering Fleet Research Run

Date: 2026-09-08
Status: In progress

The user authorized a full steering research refresh for all ten eligible
providers in `claudine/docs/providers.yaml`, using `gpt-5.6-sol` with low
reasoning, plus an update to Pi's non-interactive execution research.

## Contract and Scope

Steering schema revision 2 incorporates the four pilots: explicit launch profiles,
profile-specific capability cases, acknowledgment guarantees, target guards,
protocol maturity and framing, queue/tool-batch behavior, input interpretation,
and partial interruption outcomes. A synthetic document passed Darkmatter schema
validation before launch. The four existing reports are refreshed and checked
before the remaining six providers run.

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
and effort. One launch also failed before starting a research session with a
configuration-path error; its retry started successfully. Model verification
must cover each subsequent provider launch, not just the initial batch member.

## Validation

In addition to `md schema validate`, the coordinator checks roster identity,
revision/date/model fields, unique IDs, evidence references, each profile's
declared Cartesian-product coverage, exactly 24 ordinary baseline combinations,
profile/OS/origin-matched discovery references, mechanism/profile/OS access and
compatibility coverage, and one receipt/lifecycle record per mechanism. Passive
reports may not claim disposable tests. Source review remains separate from these
mechanical checks.

Final outcomes and provider findings will be recorded after the run completes.
