# Host Capability Broadcast

Claudine is going to support running _jobs_ both **later** in time (think "queuing" and "scheduling") as well as across a mesh of host platforms (rather than always being executed on the current host). In order to setup for that future we will need for all "compute nodes" in the mesh to broadcast their "capabilities" and "characteristics" so that when a compute job is triggered it can choose an appropriate host to run on.

## Capabilities and Characteristics

In this section we will detail out what is expressed a capability or characteristic of a host (referred to as just "capabilities" going forward):

### Hardware Detection

All of the below are detectable using the sniff library (our cross-platform host-detection library):

- `os`: enum(macOs,Linux,Windows)
- `os_version`
- `memory`: number
    - the amount of RAM the machine has
- `cpu_cores`: number
- `gpu`: enum(none,metal,nvidia,other)
- `gpu_features`: features[]
- `machine`: enum(bare-metal,virtual-machine,lxc-container)
- `arch`: enum(amd64,arm64,etc.)
- `avx`: boolean
- `avx2`: boolean
- `avx512bw`: boolean
- `avx512f`: boolean
- `avx512vl`: boolean
- `neon`: boolean
- `sse`: boolean
- `sse2`: boolean
- `sse3`: boolean
- `sse4_1`: boolean
- `sse4_2`: boolean
- `ssse3`: boolean
- `available_storage`: number
    - note: unlike the rest of the hardware fields this one changes constantly — see D2 (quantization)

### Other

- `id`: string
    - an immutable and unique identifier on the network
- `name`: string
    - a unique (for the mesh) name that is typically the hostname
    - unlike `id` a `name` is allowed to be changed so long as it maintains a name that is unique
    - note: global uniqueness cannot be *enforced* in an eventually-consistent mesh — see D4

- `repos`: "{ '<string>': string }"
    - a dictionary of repos which the machine has already checked out and the last commit of that repo (keys are repo, values are last commit hash)
    - this is important because of the latency advantage of working a repo that is already on local storage
    - repos should be represented in a canonical manner which uniquely identifies the remote host
    - note: this field updates on *every commit of every repo* — a much hotter cadence than the rest of the document — see D1

## State Storage

Storage follows the shared [rendezvous data-model doc](../../rendezvous/docs/crdt.md). Host capabilities are a **Kind-2 state register**: a Loro map document (Loro is our CRDT library — a document multiple machines can replicate and merge; a "register" is our term for a document holding a *current value* rather than an event history) with exactly **one writer**.

- document identity: `capability/{node_id}` (this supersedes the earlier `capability-${id}` naming — all documents now share the `{domain}/{owner_node_id}` path grammar defined in the data-model doc)
- all **Rendezvous** daemons have visibility into _all_ hosts in the mesh (every daemon holds a synced replica of every host's register)
- only the **Rendezvous** daemon on a given host _writes_ to that host's document
    - enforced by the daemon, and eventually on the sync import path too (see S2)
- **write-on-change only**: a capability refresh that detects no diff must not touch the document. This is what keeps the register within its cadence budget — Loro documents retain their full edit history, so a document rewritten every few seconds grows without bound

All of the capabilities have a relatively low genuine update cadence, so while any given daemon is only "eventually consistent" with its peers' capabilities, in practice their view will be in sync nearly all of the time.

### Resolved (RATIFIED 2026-07-12 via the data-model doc)

The three questions from the original draft are answered by the data-model taxonomy:

1. **`online` does not live in this document.** Liveness is high-cadence and is an *observation* rather than self-declared state (a dead host can't write `online: false`). It lives in the ephemeral layer: derived presence (`now - last_seen_sync < threshold`) by default, with Loro's ephemeral store available if we want richer presence. When history matters, presence *transitions* (`host_online` / `host_offline`) are recorded as Kind-1 facts in `presence-log/{node_id}/...`.
2. **Capabilities are projected to DuckDB, not moved.** redb keeps the living register (authoritative); DuckDB (our embedded analytics database) mirrors it as the `hosts` dimension table, rebuilt from redb at any time. If capabilities-over-time reporting is wanted, each register write also emits a `capability_changes` fact.
3. **24-hour uptime is easy — from facts, not from this document.** A register only knows its latest value, so the metric comes from a windowed SQL aggregation over the `presence_events` fact table (sum of online intervals intersected with the window). General rule: metrics are *event-shaped facts in, window queries out* — never stored in CRDT state.

## Decisions (RATIFIED 2026-07-12)

All decisions below were ratified as recommended (D6's trust posture is a noted
assumption to revisit if the mesh ever spans trust boundaries); the option discussion
is kept for context.

- **D1 — Where `repos` lives.** Its cadence (every commit, every repo) violates the register budget if kept alongside the cold hardware fields — every commit would grow the shared document's history. Options: (a) split into its own `repos/{node_id}` register so the churn is isolated; (b) drop the commit hash and sync only the repo list (colder, but the scheduler loses freshness information); (c) model repo-head-moved as facts. *Recommendation:* (a) — the scheduler genuinely wants the commit hash (how far behind is this checkout?), and isolating the churn keeps the capability document permanently cold. Pair with the compaction spike (S1). ✅ **IMPLEMENTED (2026-07-12):** dedicated `repos/{node_id}` register mapping canonical repo id → HEAD sha; bounded-depth scan of configured roots (`--repo-root` / `RENDEZVOUS_REPO_ROOTS`, opt-in until configured); replace-semantics writes so a deleted checkout leaves the register; `ListHostRepos` gRPC read surface; interval refresh shared with capabilities (event-driven tightening left for a post-commit source).
- **D2 — Volatile numbers need quantization.** `available_storage` changes constantly; write-on-change alone won't protect the register. *Recommendation:* quantize to coarse buckets (e.g. whole GB, or 5% steps) with hysteresis, so only *meaningful* movement writes. The scheduler needs "is there room", not byte precision. Same treatment for any future volatile field (load average, battery).
- **D3 — Refresh cadence & triggers.** How often does the daemon re-detect? *Recommendation:* detect at daemon startup + a slow interval (hourly) for hardware (which effectively never changes), a faster interval for `available_storage`, and event-driven for `repos` (post-commit, if the watcher exists) falling back to interval.
- **D4 — `name` uniqueness has no enforcer.** In an eventually-consistent mesh, two hosts can adopt the same `name` concurrently and both writes are "valid" — there is no coordinator to reject one. *Recommendation:* treat uniqueness as advisory: collisions are *detected* after convergence (every daemon can see all registers), surfaced to the user on both hosts, and disambiguated deterministically in UIs in the meantime (e.g. `name (id-prefix)`). Scheduling must key on `id`, never `name`.
- **D5 — Schema evolution.** Capability fields will grow. An older daemon will receive registers containing fields it doesn't know. *Recommendation:* include a `schema_version` field; readers ignore unknown fields (forward-compatible reads) and treat missing fields as "capability unknown" — a scheduler must distinguish *unknown* from *absent* (no GPU vs don't-know-about-GPUs).
- **D6 — Trust model (flag early, decide with the scheduler).** Capabilities are self-declared, and the future scheduler will make placement decisions from them. For a personal mesh of paired machines this is fine; note the assumption explicitly so it's revisited if the mesh ever spans trust boundaries.

## Risk Areas & Spikes

- **S1 — Register history growth / compaction (spike, shared with the data-model doc).** ✅ **SPIKE COMPLETE (2026-07-12)** — see [spike-register-compaction.md](./spike-register-compaction.md). Headlines: growth is linear but modest (~42 B/write for the `repos` shape, ~3 B/write for a hot scalar; shallow snapshot stays ~1.4 KiB regardless); re-basing preserves state, the owner's peer id, and the redb persistence path; the one trap is that a reader *behind* the re-base point **silently** stops converging on delta sync (import reports `Ok` with ops parked as pending) — so the sync engine must gate on `shallow_since_vv` and respond with snapshot-replace instead of a delta. **Implemented 2026-07-12** (sync protocol v2): the gate in `export_updates_since`, the `SnapshotReplace` payload kind + `SyncDelta.replace` wire flag, replica-swap receive semantics, and a pending-ops import guard.
- **S2 — Single-writer enforcement on import.** ✅ **IMPLEMENTED for registers (2026-07-12).** A register's Loro peer id is derived deterministically from its owner's node id, and `RegisterStore::stage_remote` rejects any update carrying ops from a different peer id (on top of the sync layer's namespace check, which already prevented a peer from writing outside its own namespace). Session-log chunks remain namespace-level only for now — existing chunk documents carry random peer ids, so op-level binding there needs a migration story.
- **S3 — Detection accuracy in virtualized environments.** `machine` (bare-metal / VM / LXC), GPU visibility, and CPU-feature flags are notoriously unreliable inside VMs and containers, and this is exactly where scheduling decisions bite (a "16-core" LXC guest may be sharing those cores). Spike: run sniff's detection across our real fleet (macOS bare-metal, Linux VMs, LXC containers, Windows) and record what lies each environment tells; add a `detection_confidence` note to fields that proved unreliable.
- **S4 — Canonical repo identity.** ✅ **IMPLEMENTED (2026-07-12):** `rendezvous_core::canonical_repo_id` — one canonicalization (`host[:port]/path`; scheme/credentials/`.git` stripped, host lowercased, path case preserved) for both SSH and HTTPS forms; remote-less checkouts return `None` (no mesh identity). The logging pipeline must use this same function when it lands repo attribution — do not grow a second canonicalization.
