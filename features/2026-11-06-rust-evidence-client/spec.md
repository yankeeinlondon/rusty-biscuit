---
kind: feature
name: rust-evidence-client
date: 2026-11-06
status: draft-spec
depends-on:
  - 2026-10-26-next-gen-pipeline
  - 2026-11-01-rust-pre-push
  - 2026-10-28-publish
next_spec: 2026-11-06-any-host-evidence
reviewed: false
implemented: false
---

# Move evidence to the store, in Rust

## Why

Hosts and CI both run the same large evidence scripts, reconciling notes,
artifacts, and receipts. With one store, every producer can do the same three
things: ask what is missing, run it, and write the result. CI becomes just
another host.

## Outcome

- A Rust client and verifier library, plus a CLI, is the only way to read or
  write evidence, on hosts and in CI.
- The store is authoritative for observations. The per-OS
  `refs/notes/ci-local/<environment>` notes are no longer written or read, and
  the replaced Python is deleted. The plan's own note,
  `refs/notes/ci-local/scope`, stays until `2026-11-06-rust-ci-planner` moves it.
- Behavior is unchanged: the same cells qualify as before. Changing the rules is
  the next spec's job, so that any difference found here is a bug.

## Scope

- Port `local_evidence.py`, `observation_notes.py`, `hosted_observations.py`,
  `reuse_validation.py`, and `evidence_index.py`. This includes Phase 2 of
  `2026-11-01-rust-pre-push`.
- Import existing observations without changing their IDs, using `biscuit-hash`
  with the current identity contract.
- Compare old and new decisions on pinned plans, then switch to one authoritative
  reader. There is no lasting dual-write.
- Install the tool on CI runners as a pinned prebuilt release, which is why this
  depends on `2026-10-28-publish`.

## Spikes needed

- **Is the reconciliation logic still needed?** Sort each Python rule into "the
  store makes this unnecessary" or "real policy". This decides how much is
  actually ported, and it is the main chance to shrink the system.
- **Verifier and planner seam.** The verifier needs gate-input identity, which is
  computed inside `affected_scope.py` today. Decide how it gets that value before
  the planner is ported.
- **Tool bootstrap.** A release workflow that needs a tool it also releases needs
  a pinned earlier version and a recovery path if that version is bad.

## Not in scope

Any-host qualification, or new triggers.
