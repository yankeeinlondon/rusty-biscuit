---
kind: feature
name: next-gen-pipeline
date: 2026-10-26
status: draft-spec
next_spec: 2026-11-06-rust-evidence-client
related:
  - 2026-10-26-change-detection
reviewed: false
implemented: false
---

# The evidence store

This directory's [`d1-model.md`](./d1-model.md) is the detailed design and
[`todays-pipeline.md`](./todays-pipeline.md) describes the current process. This
spec sets the store's scope and takes over the "Better Evidence" intent first
drafted in `2026-10-26-change-detection`.

## Why

Evidence lives in Git notes and is queried awkwardly. Reports live in Actions
artifacts or on the host that produced them, and when an artifact expires its
cell runs again. Much of the Python exists to reconcile those places.

## Outcome

One durable, queryable record of every test attempt, with its failures, that
hosts and CI write to. At the end of this spec the store is live and receiving
evidence, but nothing yet relies on it.

## Scope

- A small TypeScript Worker owning the D1 binding. Clients call its API and never
  hold database credentials or submit SQL.
- The `d1-model.md` schema: append-only observations, report-location events, and
  evaluations.
- Object storage for JUnit reports, recommended to be R2, so that reports outlive
  Actions artifacts and are readable from any host.
- Authentication: OIDC for GitHub Actions, individually revocable tokens for
  hosts, with permissions derived from the caller rather than the payload.
- An event hook that fires when new evidence arrives. Consumers come later.
- Backup, restore, and a stated behavior for outages: a store that cannot be
  reached means tests run, not that coverage is assumed.

## Spikes needed

- **Host authentication.** How does a laptop or test rig get and rotate a token
  without weakening the model? `d1-model.md` leaves this open.
- **Real D1 behavior.** The DDL has only run on local SQLite. Test batches,
  generated columns, triggers, and concurrent writes on a deployed instance, and
  check limits against real observation and report sizes.
- **Retention and cost.** How long to keep reports, and what that costs.

## Not in scope

- Qualification rules, the verifier, and the client: `2026-11-06-rust-evidence-client`.
- Changing which results count: `2026-11-06-any-host-evidence`.

## Correction to the earlier draft

The first draft said Git notes last 90 days. Notes are refs and do not expire.
Artifacts expire, and with them the reports that back a hosted observation.
