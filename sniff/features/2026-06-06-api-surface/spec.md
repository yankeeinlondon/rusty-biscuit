# Sniff API Surface — Current State & Limitations (DRAFT)

**Status:** Draft. This document describes the *current* `sniff` request/detection API
surface — its three-tier structure, concurrency model, and the limitations that surfaced
during an audit (June 2026). It is deliberately scoped to **what exists today and where it
falls short of the stated design intent**. It does *not* prescribe a redesign; concrete
remediation will be a separate proposal once this current-state picture is agreed.

All citations are `file:line` against `sniff/lib/` at the time of writing.

### Design intent

`sniff` was meant to give callers two complementary ways to request data:

1. **Group retrieval at maximum performance.** A caller who needs a whole "group" (a
   domain, or the full system) gets it as fast as the technique allows — the library is
   free to parallelize, share work, or cache internally to deliver the group quickly.
2. **Granular metric retrieval.** A caller who needs one or a few individual metrics can
   request exactly those, without paying for siblings they did not ask for.

The audit below measures the implementation against those two goals. **Goal 1 is strongly
realized. Goal 2 is only partially realized and is structurally inconsistent.**

---

### Current architecture

#### The three tiers

The public surface is layered as a funnel — all three tiers ultimately route through
`detect_with_plan` for the four plan-driven domains.

- **Tier 1 — Convenience.** `detect()` (`lib.rs:213`) → `detect_with_config(SniffConfig::default())`
  → `detect_with_plan(DetectionPlan::default())`. Returns all four domains at full detail.
- **Tier 2 — Plan-based.** `detect_with_plan(DetectionPlan)` (`lib.rs:266`) is the canonical
  entry point. `DetectionPlan` carries `Option<OsRequest>`, `Option<HardwareRequest>`,
  `Option<NetworkRequest>`, `Option<FilesystemRequest>`, a `base_dir`, and
  `include_performance` (`request.rs:33-46`). A `None` domain is skipped entirely.
- **Tier 3 — Module-level.** Free functions and handles a caller invokes directly
  (`detect_os_with_request`, `GitRepo::discover`, `detect_repo_structure`, …) to compose a
  bespoke pipeline without materializing a `SniffResult`. Used by downstream libraries
  (e.g. darkmatter, claudine compose-prep).
- **Legacy — `SniffConfig`.** Flat boolean `skip_*` flags + a single `deep` bool
  (`lib.rs:74-161`), lowered to a `DetectionPlan` via `From<SniffConfig>` (`lib.rs:163-196`).
  Still supported; new code should prefer `DetectionPlan`.

#### Domains

- **Plan-driven (in `DetectionPlan`):** `os`, `hardware`, `network`, `filesystem`.
- **Standalone (NOT in `DetectionPlan`):** `programs` (`ProgramsInfo::detect()`,
  `programs/mod.rs:201`), `services` (`ServiceManager::detect()` / `detect_services()`,
  `services/mod.rs:426`), and `package` (a package-manager abstraction registry, not a
  host-detection surface). These take no request argument and have no request type.

#### Concurrency & shared-work model (Goal 1)

This is the well-built half of the API.

- **Top-level domain fan-out** is automatic: `detect_with_plan` spawns every requested
  domain on its own `std::thread::scope` thread and joins after all are launched, so the
  four domains overlap fully (`lib.rs:279-354`). No opt-in; a skipped domain simply never
  spawns.
- **Filesystem** runs a concurrent prelude — git ∥ formatting ∥ a single shared
  parallel tree-walk (`build_filesystem_system_view`) — then sequential reuse stages
  (repo / inventory / docs) that project off the shared walk; the tree is walked at most
  once (`filesystem/mod.rs:94-278`). The shared walk is itself parallel
  (`ignore::WalkBuilder::build_parallel()`).
- **Programs** builds one `Arc<ExecutableIndex>` (one PATH/bundle scan) and detects 8
  categories via `rayon::join` pairs over O(1) hash lookups (`programs/mod.rs:201-246`).
- **Hardware** and **network** fan their slow probes (audio/storage/gpu; wan-ip) out on
  scoped threads (`hardware/mod.rs:128-241`, `network/mod.rs:160-209`).
- Process/TTL caches: eager-PATH/bundle index, package-manager registry, network
  interface (1s) and WAN-IP (5min) TTL caches, host-capability disk cache (90d).

**Conclusion on Goal 1:** if you want a predefined *group*, the library delivers it about
as fast as the technique allows.

#### Request-type catalog (Tier 2)

All request-shaping types live in `request.rs`. Presets and granular knobs as they exist
today:

| Type | Fields (granular knobs) | Presets | Builder knobs |
|------|-------------------------|---------|---------------|
| `DetectionPlan` (`:33`) | `os`/`hardware`/`network`/`filesystem` (each `Option`), `base_dir`, `include_performance` | `new()`/`Default` only (all 4 domains at full) | `os/hardware/network/filesystem`, `without_*`, `performance(bool)` |
| `OsRequest` (`:120`) | `include_package_managers`, `include_locale`, `include_timezone`, `include_ntp_status` | `summary()` (all false), `full()` (all true) | one `(bool)` setter per field |
| `HardwareRequest` (`:175`) | `include_cpu`, `include_memory`, `include_storage`, `include_gpu`, `include_audio` | `summary()` (cpu+mem), `full()` (all) | one `(bool)` setter per field |
| `NetworkRequest` (`:239`) | `include_wan_ip`, `force_refresh` | `interfaces_only()`, `full()` | `include_wan_ip`, `force_refresh` |
| `GitRequest` (`:279`) | `commit_count`, `include_file_changes`, `include_file_diffs`, `include_worktrees`, `refresh_remote_tracking`, `include_remote_branch_details`, `include_commit_remote_containment`, `max_remote_branches` | `minimal()`, `summary()`, `full()`, `deep()` | setters for 6 of 8 fields (see L6) |
| `RepoRequest` (`:418`) | `structure_only` (single bool) | `structure()`, `full()` | **none** |
| `FilesystemRequest` (`:444`) | `git`/`repo` (`Option`), `include_file_inventory`, `include_formatting`, `include_docs` | `new()`/`Default` only (full) | `git`, `repo`, `without_*`, `include_docs(bool)` only |
| `SniffConfig` (legacy, `lib.rs:74`) | `skip_*` per domain, `deep`, `commit_count` | `new()`/`Default` | `skip_*`, `deep(bool)`, `commit_count` |

#### Tier-3 granular surface (current)

Three *different* patterns coexist, one per "shape":

- **Handle with atomic getters** — git only. `GitRepo::discover()` (`git/types.rs:512`)
  opens the repo and exposes zero-cost getters: `repo_root()` (`:533`), `current_branch()`
  (`:538`), `in_worktree()` (`:552`), `remotes()`, `branches()`, `recent_commits()`, etc.
  The handle is explicitly designed for "request only the data you need without paying for
  a full sweep" (`git/types.rs:459-462`). `repo_status()`/`file_changes()` are the only
  getters that run a status walk.
- **Free-function-per-metric** — `os` (`detect_os_type`, `detect_locale`,
  `detect_timezone`, `detect_ntp_status`, package-manager fns — `os/mod.rs:33-46`),
  `hardware` partial (`detect_gpus`, `detect_storage`, `detect_audio_devices`,
  `detect_simd` — `hardware/mod.rs:17-21`), and `programs` (`find_program`,
  `find_programs_parallel` — `programs/find_program.rs:27,34`).
- **Request-struct-only** — `network` (sub-detectors are private), `repo`
  (`detect_repo_structure`/`detect_repo`/`detect_repo_identity` are the dial, not per-field
  fns), and `hardware` CPU/memory (no standalone fn; only reachable via
  `detect_hardware_with_request`).

---

### Limitations (Goal 2 and consistency)

The following are the problems the audit identified. They are interrelated; the root theme
is **granularity is modeled as vertical preset tiers, while the genuinely granular access
lives in a separate, ad-hoc Tier-3 that the request API cannot express or compose.**

#### L1 — The plan/request world and the Tier-3 world do not compose

There is no way to request an arbitrary, caller-chosen subset of metrics *and* get the
library's parallelism. A caller must either (a) accept the top-level plan's fixed fan-out
and over-detect, or (b) hand-roll their own `thread::scope` over Tier-3 free functions /
handles. The library only parallelizes its *own predetermined groupings*; the sole place a
caller-chosen subset is parallelized for them is `programs::find_programs_parallel`
(`find_program.rs:34`). Nothing analogous exists across domains.

#### L2 — Granularity is preset-tiers, not composable atoms; you cannot drop below a preset

The request types expose vertical bundles (`minimal → summary → full → deep`) and you
cannot reach below the cheapest bundle. The canonical example: **no `GitRequest` level
returns the repo root / branch without a working-tree status walk.** Every level routes
through `GitRepo::detect_with_request`, whose three status paths all call `repo.statuses()`
(`git/types.rs:668-695`, `status.rs`). The cheap "root only" capability *exists* — but only
on the Tier-3 `GitRepo::discover().repo_root()` handle (`git/types.rs:512,533`), which is
undiscoverable from `request.rs` and gets none of the plan's scheduling. A caller reading
only the request API correctly concludes "every git request scans the tree."

This is the limitation that motivated the audit: claudine's compose-prep needs only the
repo root + package structure before its execution header, but the shared scan it uses
(`GitRequest::summary()`) forces a ~40 ms+ status walk it never reads.

#### L3 — Redundant presets and resulting doc drift

`GitRequest::minimal()` and `GitRequest::summary()` are **byte-identical** (`request.rs:304`
vs `:318`; asserted equal at `request.rs` tests). Because `summary()` sets
`include_file_changes: false` and satisfies `is_minimal()`, detection takes the
dirty-flag-only branch and leaves `staged_count`/`unstaged_count`/`untracked_count` at `0`
(`git/types.rs:670-681`) — i.e. `summary()` does **not** produce the per-category "counts"
its name and (pre-fix) docs implied. (Architecture doc and the in-code doc comment have
been corrected; this entry records the redundancy itself.)

#### L4 — Coarse flags bundle multiple metrics with no separation

- `RepoRequest::full()` (`structure_only: false`) bundles per-package language scan +
  framework detection + file-association inventory into one flag (`repo/detection.rs`); a
  caller wanting languages-but-not-frameworks has no knob. `RepoRequest` has only the
  single `structure_only` bool.
- `GitRequest::include_file_changes` bundles changed-paths + per-file status + line counts
  into one flag (`request.rs:283`).
- `refresh_remote_tracking` is a prerequisite gate for remote-branch-details and
  commit-remote-containment, which (see L6) are only reachable via the `deep()` bundle.

#### L5 — Metrics with no granular access at all

Reachable only as a field of a larger struct after a broader detect:
- **Hardware:** CPU, memory (no standalone fn; only via `detect_hardware_with_request`
  with siblings off — `hardware/mod.rs:82-124`).
- **OS:** kernel, arch, hostname, OS version (inline `System::*` in
  `detect_os_with_request`, `os/mod.rs:265-271`; arch actually lives on the hardware side).
- **Network:** primary interface and WAN-IP detectors are private (`network/mod.rs`); the
  only public dial is the 2-level `interfaces_only()` vs `full()`.

#### L6 — Fields settable only through a preset bundle

`GitRequest` has no builder setters for `include_remote_branch_details` or
`include_commit_remote_containment` (`request.rs:361-413`); they are reachable only via the
`deep()` preset or a struct literal. `RepoRequest` has **zero** builder methods at all.

#### L7 — Inconsistent request-type shape across domains

- **Preset naming** is not uniform: `summary()` (os/hardware/git) vs `interfaces_only()`
  (network) vs `structure()` (repo); `FilesystemRequest` and `DetectionPlan` have no
  summary-style preset (only `Default`). `deep()` exists only on `GitRequest`.
- **Removers** (`without_*`) exist only on `DetectionPlan` and `FilesystemRequest`; other
  types use `include_*(false)` (and `RepoRequest` has neither).
- **Builder coverage** is uneven: os/hardware/network have one setter per field;
  `GitRequest` is missing 2 of 8; `RepoRequest` has none; `FilesystemRequest` has a
  positive setter only for `include_docs`, relying on `without_*` for inventory/formatting.

#### L8 — Whole domains live outside the plan/request system

`programs`, `services`, and `package` have **no request type** and are **not in
`DetectionPlan`**. `ProgramsInfo::detect()` and `detect_services()` take no arguments and
always detect everything (all 8 program categories / all services). The architecture
rationale (`sniff-library-architecture.md`) is that they are system-global and share no
work with the project-scoped domains — defensible, but it means the "request what you need"
model simply does not apply to them, and there is no granular sub-request (e.g. "just
editors", though `programs::find_program` offers single-program lookup at Tier 3 only).

#### L9 — Tier-3 surface is ad hoc, not a uniform contract

Only git offers a stateful handle (`GitRepo`). os/programs offer a near-complete
free-function set; hardware offers free functions for gpu/storage/audio but not cpu/memory;
network/repo offer none below their request struct. The cheap repo-root derivation is even
duplicated: `GitRepo::discover()` and `detect_repo_identity` (`repo/identity.rs:72`) each
re-open the repo via raw `git2` independently rather than sharing one path.

#### L10 — Parallelism is fixed per detector, not caller-directed

Internal parallelism (top-level domains, hardware probes, network, programs categories) is
baked into each detector as a fixed fan-out. There is no mechanism for a caller to say "run
these N atoms concurrently"; that scheduling is unavailable outside the predefined groups
(restatement of L1 from the concurrency angle).

---

### Evidence index

| Claim | Location |
|-------|----------|
| Funnel: detect → config → plan | `lib.rs:213,231,266` |
| Top-level scoped-thread fan-out | `lib.rs:279-354` |
| Filesystem concurrent prelude + reuse | `filesystem/mod.rs:94-278` |
| Programs rayon + shared `ExecutableIndex` | `programs/mod.rs:201-246` |
| `minimal()` ≡ `summary()` | `request.rs:304-329` |
| `summary()` → dirty flag only, counts = 0 | `git/types.rs:670-681` |
| `is_minimal()` definition | `request.rs:388-393` |
| Every git request runs a status walk | `git/types.rs:668-695`, `status.rs` |
| Repo-root without status (Tier 3 only) | `git/types.rs:512,533` |
| `RepoRequest` single bool, no builders | `request.rs:418-438` |
| Git fields with no setter | `request.rs:361-413` |
| Programs/services have no request type | `programs/mod.rs:201`, `services/mod.rs:426` |
| Duplicate repo-root derivation | `git/types.rs:512`, `repo/identity.rs:72` |

---

### Out of scope (for this draft)

- The remediation design (composable capability atoms, generalizing the `GitRepo` handle
  pattern, a caller-directed parallel runner, folding programs/services into a uniform
  vocabulary). That belongs in a follow-up proposal once this current-state picture and the
  L1–L10 framing are agreed.
- Any code changes. The only changes made alongside this draft are documentation drift
  fixes (`sniff-library-architecture.md`, the `GitRequest::summary()` doc comment, and the
  sniff skill cheat sheet) to make the docs accurate to the current implementation.

### Open questions

1. Should `programs`/`services` be folded into the request/plan vocabulary, or explicitly
   documented as a deliberately separate axis?
2. Is the `GitRepo` handle pattern the model to generalize (atomic getters per domain), or
   should granularity be expressed purely through richer request types?
3. What is the compatibility contract — can `DetectionPlan` / `SniffConfig` be preserved as
   sugar over a new atom layer, or is a breaking change acceptable?
4. Where should "give me this arbitrary subset, scheduled in parallel" live — a new
   plan-level API, or a documented Tier-3 composition helper?
