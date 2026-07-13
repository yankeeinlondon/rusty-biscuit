# Spike: Register History Compaction

> **Status: COMPLETE (2026-07-12).** This spike answers risk item **S1** of the
> [host-capability spec](./spec.md) and the "register history compaction" open question in
> the [rendezvous data-model doc](../../rendezvous/docs/crdt.md). It gates any Kind-2
> state register with a hot write cadence (notably the D1 `repos` register).

## Questions

Loro (our CRDT library) keeps every document's full edit history, so a **state
register** — a map document whose values are overwritten in place — grows forever even
though its *state* stays tiny. The spike set out to answer:

1. **How fast do registers actually grow** with write count, and does the shape of the
   churn (one hot key vs many rotating keys) matter?
2. **Does shallow-snapshot re-basing work** as the compaction mechanism? (A *shallow
   snapshot* is Loro's export mode that keeps the current state but drops history before
   a chosen point — re-basing means the owner adopts that trimmed document as its new
   working copy.)
3. **Is re-basing safe for peers** — what happens to readers that are at, behind, or
   ahead of the re-base point when deltas start flowing from a trimmed document?

## Method

A standalone harness at
`claudine/rendezvous/daemon/examples/register_compaction_spike.rs`, rerunnable with:

```sh
cargo run -p rendezvous-daemon --example register_compaction_spike
```

It simulates the two register shapes we care about, one Loro commit per write
(matching the daemon's write-on-change behavior), with a single writer (`peer_id = 1`):

- **`repos` shape** — 20 keys, each write overwrites one key's 40-char commit hash
  (a commit landing in one of 20 checked-out repos)
- **`capability` hot-key shape** — ~30 cold fields written once, then one key
  (`available_storage`) overwritten every write (the unquantized-volatile worst case)

Loro version: 1.12.0 (per `Cargo.lock`). Byte sizes are profile-independent; the few
timings quoted were taken in a debug build and are indicative only.

## Results

### 1. Growth is linear and unbounded — but modest

| writes | `repos` full snapshot | `capability` full snapshot | shallow snapshot (either) |
|-------:|--------------------:|--------------------------:|--------------------------:|
| 100 | 5.5 KiB | 1.2 KiB | ~1 KiB |
| 1,000 | 42.6 KiB | 4.7 KiB | ~1 KiB |
| 10,000 | 414.7 KiB | 31.0 KiB | ~1 KiB |
| 50,000 | 2,068 KiB | 150.6 KiB | ~1 KiB |

- Marginal growth settles at **~42 bytes/write** for the `repos` shape and **~3
  bytes/write** for the single-hot-key shape (Loro compresses same-key overwrites of
  small scalars extremely well; distinct keys with 40-char values cost more).
- The **shallow snapshot stays flat** (~1.4 KiB for `repos`, ~0.6 KiB for capability)
  regardless of write count — it is effectively "state size", confirming compaction
  reclaims ~everything (**306× smaller** at the 10k-write mark).
- Calendar translation: a commit-heavy `repos` register (100 commits/day) grows
  **~1.5 MB/year**; an *unquantized* `available_storage` written every 30s grows
  **~3 MB/year**. Neither is an emergency on one host — but every byte replicates to
  every mesh peer and lives in every full snapshot redb persists, so compaction is
  still warranted; there is just no cliff forcing an aggressive cadence.

### 2. Re-basing works, including the parts we worried about

At 10k writes: shallow export took ~1ms (vs ~14ms for the full snapshot), and:

- state is **byte-identical** across the re-base (`get_deep_value` equality)
- the re-based document reports `is_shallow() == true` with `shallow_since_vv`
  at the trim point
- **the owner can keep writing as the same peer** — `set_peer_id(owner)` on the
  re-based doc succeeds and op counters continue from where they left off, so
  single-writer enforcement can rely on a stable peer id across re-bases
- **the redb persistence path is unaffected** — a shallow doc's `Snapshot` export
  round-trips through `LoroDoc::from_snapshot` and comes back still shallow

### 3. Sync safety: one silent trap, one simple rule

| Scenario | Result |
|----------|--------|
| C1 — reader exactly at the re-base point, requests updates-since | ✅ converges normally |
| C2a — reader **behind** the re-base point, requests updates-since | ⚠️ **silent failure**: export succeeds, import returns `Ok` with the ops parked as `pending`, reader **does not converge**, no error anywhere |
| C2b — stale reader imports the current shallow snapshot in place | ⚠️ still does not converge (partial pending remains) |
| C2c — stale reader **discards its replica** and re-adopts the shallow snapshot | ✅ converges |
| C3 — brand-new reader bootstraps from shallow snapshot, then follows deltas | ✅ converges |
| C4 — an old pre-re-base delta arrives at a shallow reader out of order | ✅ harmless no-op (ops already covered), stays converged |

C2a is the finding that matters. The gap (ops between the stale reader's version and
the re-base point) no longer exists anywhere — the owner trimmed it — so delta sync can
never repair it, and **neither side surfaces an error**: the reader just silently stops
advancing. In-place snapshot import (C2b) does not reliably repair it either. The only
reliable recovery is wholesale replacement (C2c) — which for a register costs ~1.4 KiB,
*less* than the accumulated deltas it replaces.

## Design Rules (PROPOSED)

1. **The sync engine must gate on the shallow root.** When a peer requests
   updates-since `vv` and `vv < shallow_since_vv` for that document, the owner must
   respond with a **snapshot-replace** (current shallow snapshot + an instruction to
   discard the local replica), never a delta. Deltas exported past a stale vv *look*
   valid and import "successfully" — the failure is invisible at the wire layer.
2. **Readers must treat `pending ≠ None` on a register import as a failed sync** and
   request snapshot-replace, as defense-in-depth for rule 1.
3. **Replace, don't merge, on recovery.** Register replicas are read-only (single
   writer), so discarding a stale replica loses nothing. This makes recovery trivially
   correct — no in-place repair path needed.
4. **Compaction policy can be lazy.** Given ~42 B/write worst-case growth, re-base on
   coarse thresholds — e.g. when a register's persisted snapshot exceeds **256 KiB** or
   **10k ops** — checked opportunistically (at write time or daemon startup). Every
   re-base knocks any behind-the-point reader into the (cheap) recovery path, so there
   is no reason to re-base aggressively.
5. **Owner keeps its peer id across re-bases** (verified safe), so foreign-writer
   enforcement (spec S2) composes cleanly with compaction.

## What This Unblocks

- **Host-capability D1** — the dedicated `repos/{node_id}` register is viable as
  designed; even a commit-heavy year stays ~1.5 MB before compaction and ~1.4 KiB after.
- **Quantization (D2) is confirmed as nice-to-have, not survival** — unquantized
  volatile writes cost ~3 B/write; still worth doing to keep sync traffic and history
  noise down.
- **Dashboard S3** — `sessions-active` register churn is bounded by the same policy.
- **Data-model doc** — the "register history compaction" open question is answered.
  **The shallow-root gate and snapshot-replace frame are IMPLEMENTED (2026-07-12,
  sync protocol v2):** `export_updates_since` answers a peer whose version predates
  the document's shallow root with `PayloadKind::SnapshotReplace`; the receiver
  swaps its replica wholesale (`stage_remote_replace` / `commit_staged_replace`,
  still enforcing metadata identity, schema, and the append-only entry prefix); and
  a defense-in-depth pending guard fails any import whose `ImportStatus` parks ops
  as pending instead of silently not converging.

## Caveats

- Single-writer scenarios only — exactly our register model; no claim is made about
  re-basing documents with concurrent writers (Kind 3), which we avoid anyway.
- Loro 1.12.0 behavior; re-verify the C-matrix on major Loro upgrades (the harness is
  kept for this purpose).
- Debug-profile timings; sizes (the load-bearing numbers) are profile-independent.
