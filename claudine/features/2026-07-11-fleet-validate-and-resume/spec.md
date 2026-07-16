# Fleet Research: Deterministic Validate-and-Resume Lifecycles

Upgrade the fleet-research recipe from lifecycles that *report* progress to
lifecycles that *gate* on deterministic checks and **resume the research
session** to correct surprising or known-untrue outcomes while the session
still has its full context.

## The idea (Ken, 2026-07-11)

> currently our fleet research uses lifecycle hooks to communicate
> progress; that's good but we may be missing a trick because the more
> deterministic checks we can do during the lifecycle and then Resume the
> research session to validate surprising outcomes or things we know to be
> untrue.

Schema sidecars already validate *shape* at compose time; this validates
*content* at event time. A `success`-stack check script inspects the
produced research doc, and when it catches something mechanical —
dropped seed data, invented provenance, a required coverage class neither
researched nor gap-acknowledged — a `resume` control re-enters the same
session with the findings as the prompt. The model fixes its own output
with context intact instead of a human (or a fresh, context-free session)
re-deriving it.

## Mechanism already exists

Verified 2026-07-11 — no harness work needed:

- lifecycle `resume` seeds `next_resume_session_id` + `next_prompt_override`
  (`harness_orch/loop_control/control_dispatch.rs`, Resume arm), budgeted
  by `max_attempts` (`ControlBudgets`), gated on the **wrapper agent's**
  `supports_resume` capability (metadata) — note it is the CLI *running*
  the research that must support resume, not the provider being researched;
- the resume `message` is late-binding interpolated (DM2), so it can carry
  check findings;
- budget exhaustion falls through (`Exhausted → Fallthrough`): the run
  completes and the still-failing doc lands in human review — a bounded
  loop that fails open.

What's missing is purely authoring: fleet `_fleet.md` prompts that wire
check scripts + `when:`-guarded `resume` items into their lifecycle stacks,
and a small library of reusable check scripts.

## Proving ground

The `agent-errors` topic pilots the pattern
(`features/_completed/2026-07-11-provider-errors-as-data/spec.md`, D10 + P5): seed
preservation, needle hygiene, provenance coherence, and coverage checks,
with `max_attempts: 2` resumes. Its B2 (Codex pilot) checkpoint reviews the
telemetry — how many resumes fired, whether corrections converged, budget
fit — and decides whether this generalizes.

## Scope when scheduled

- Distill the pilot's check-script + stack wiring into a reusable pattern
  in the research recipe docs (`2026-07-02-provider-metadata` spec's topic
  recipe section / `docs/research/_TEMPLATE.md`).
- Retrofit high-value checks into existing topics on their next refresh
  (not a mass migration): seed/prior-frontmatter preservation and
  provenance coherence generalize to every topic; coverage checks are
  per-topic.
- Consider a shared checks location (e.g. `docs/research/_checks/`) so
  fleet docs reference rather than copy them.
- Decide how check findings are best threaded into the resume message
  (findings file + read-side function vs `merge_frontmatter`) based on
  what the pilot's B1 wiring settles on.

## Status

Unscheduled — gated on the agent-errors B2 pilot checkpoint.
