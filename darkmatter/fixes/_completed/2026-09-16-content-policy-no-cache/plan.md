---
area: darkmatter
fix: 2026-09-16-content-policy-no-cache
spec: ./spec.md
created: 2026-09-17
phase: 8
total_phases: 8
agent: claude/default
yolo: "true"
source_files_during_phase_1: []
docs_updated_during_phase_1:
    - darkmatter/fixes/2026-09-16-content-policy-no-cache/plan.md
    - darkmatter/fixes/2026-09-16-content-policy-no-cache/spec.md
    - darkmatter/fixes/2026-09-16-content-policy-no-cache/implementation-log.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/compose/cache/store.rs
    - darkmatter/lib/src/markdown/compose/cache/runtime.rs
    - darkmatter/lib/src/markdown/compose/cache/remote_cache.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/remote_fetch.rs
    - darkmatter/lib/tests/persistent_cache_disabled.rs
    - darkmatter/lib/tests/reference_integration.rs
    - darkmatter/cli/tests/compose_remote_caching.rs
docs_updated_during_phase_2:
    - darkmatter/docs/topics/caching.md
    - darkmatter/fixes/2026-09-16-content-policy-no-cache/plan.md
    - darkmatter/fixes/2026-09-16-content-policy-no-cache/implementation-log.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/compose/cache/remote_cache.rs
    - darkmatter/lib/src/markdown/compose/cache/store.rs
    - darkmatter/lib/src/markdown/compose/remote_fetch.rs
    - darkmatter/lib/src/markdown/compose/remote.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/cli/src/args/enums.rs
    - darkmatter/cli/src/args/command.rs
    - darkmatter/cli/tests/compose_remote_caching.rs
docs_updated_during_phase_3:
    - darkmatter/docs/topics/caching.md
    - darkmatter/fixes/2026-09-16-content-policy-no-cache/plan.md
    - darkmatter/fixes/2026-09-16-content-policy-no-cache/implementation-log.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_4:
    - darkmatter/lib/src/markdown/compose/cache/manifest.rs
    - darkmatter/lib/src/markdown/compose/cache/remote_cache.rs
    - darkmatter/lib/src/markdown/compose/cache/store.rs
    - darkmatter/lib/src/markdown/compose/cache/runtime.rs
    - darkmatter/lib/src/markdown/compose/remote_fetch.rs
    - darkmatter/cli/tests/compose_remote_caching.rs
docs_updated_during_phase_4:
    - darkmatter/docs/topics/caching.md
    - darkmatter/fixes/2026-09-16-content-policy-no-cache/plan.md
    - darkmatter/fixes/2026-09-16-content-policy-no-cache/implementation-log.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_5:
    - darkmatter/lib/src/markdown/compose/cache/runtime.rs
    - darkmatter/lib/src/markdown/compose/cache/manifest.rs
    - darkmatter/lib/src/markdown/compose/cache/types.rs
    - darkmatter/lib/src/markdown/compose/cache/hashing.rs
    - darkmatter/lib/src/markdown/compose/cache/operation.rs
    - darkmatter/lib/src/markdown/compose/cache/store.rs
    - darkmatter/lib/src/markdown/compose/cache/remote_cache.rs
    - darkmatter/lib/src/markdown/compose/cache/mod.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/remote_fetch.rs
    - darkmatter/lib/src/markdown/compose/nested.rs
    - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/context/runtime.rs
    - darkmatter/lib/src/markdown/compose/context/capture/groups.rs
    - darkmatter/lib/src/markdown/compose/tests/schema.rs
    - darkmatter/lib/src/markdown/compose/tests/transclusion.rs
    - darkmatter/lib/src/markdown/reference/graph.rs
    - darkmatter/lib/src/markdown/reference/provenance.rs
    - darkmatter/lib/src/markdown/reference/validate.rs
    - darkmatter/lib/tests/persistent_cache_disabled.rs
    - darkmatter/lib/tests/request_context_epoch.rs
    - darkmatter/lib/tests/reference_integration.rs
    - darkmatter/lib/tests/semantic_results_never_persist.rs
docs_updated_during_phase_5:
    - darkmatter/fixes/2026-09-16-content-policy-no-cache/plan.md
    - darkmatter/fixes/2026-09-16-content-policy-no-cache/implementation-log.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/darkmatter/library-surfaces.md
source_files_during_phase_6:
    - darkmatter/lib/src/markdown/compose/remote_fetch.rs
    - darkmatter/cli/tests/compose_remote_caching.rs
docs_updated_during_phase_6:
    - darkmatter/fixes/2026-09-16-content-policy-no-cache/plan.md
    - darkmatter/fixes/2026-09-16-content-policy-no-cache/implementation-log.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_7:
    - darkmatter/cli/src/args/command.rs
    - darkmatter/cli/src/args/enums.rs
    - darkmatter/cli/tests/help.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/remote.rs
    - darkmatter/lib/src/markdown/compose/cache/hashing.rs
    - darkmatter/lib/src/markdown/compose/cache/remote_cache.rs
    - darkmatter/lib/src/markdown/reference/graph.rs
docs_updated_during_phase_7:
    - darkmatter/docs/topics/caching.md
    - darkmatter/docs/topics/remote-url-references.md
    - darkmatter/docs/cli/compose.md
    - darkmatter/docs/structs/Markdown.md
    - darkmatter/features/2026-09-09-more-context/spec.md
    - darkmatter/fixes/2026-09-16-content-policy-no-cache/spec.md
    - darkmatter/fixes/2026-09-16-content-policy-no-cache/plan.md
    - darkmatter/fixes/2026-09-16-content-policy-no-cache/implementation-log.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/darkmatter/library-surfaces.md
source_files_during_phase_8: []
docs_updated_during_phase_8:
    - darkmatter/docs/topics/schema-definition.md
    - darkmatter/fixes/2026-09-16-content-policy-no-cache/spec.md
    - darkmatter/fixes/2026-09-16-content-policy-no-cache/plan.md
    - darkmatter/fixes/2026-09-16-content-policy-no-cache/implementation-log.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8:
    - .claude/skills/darkmatter/SKILL.md
source_code:
    - darkmatter/lib/src/markdown/compose/cache/store.rs
    - darkmatter/lib/src/markdown/compose/cache/runtime.rs
    - darkmatter/lib/src/markdown/compose/cache/remote_cache.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/remote_fetch.rs
    - darkmatter/lib/tests/persistent_cache_disabled.rs
    - darkmatter/lib/tests/reference_integration.rs
    - darkmatter/cli/tests/compose_remote_caching.rs
    - darkmatter/lib/src/markdown/compose/remote.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/cli/src/args/enums.rs
    - darkmatter/cli/src/args/command.rs
    - darkmatter/lib/src/markdown/compose/cache/manifest.rs
    - darkmatter/lib/src/markdown/compose/cache/types.rs
    - darkmatter/lib/src/markdown/compose/cache/hashing.rs
    - darkmatter/lib/src/markdown/compose/cache/operation.rs
    - darkmatter/lib/src/markdown/compose/cache/mod.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/nested.rs
    - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
    - darkmatter/lib/src/markdown/compose/context/runtime.rs
    - darkmatter/lib/src/markdown/compose/context/capture/groups.rs
    - darkmatter/lib/src/markdown/compose/tests/schema.rs
    - darkmatter/lib/src/markdown/compose/tests/transclusion.rs
    - darkmatter/lib/src/markdown/reference/graph.rs
    - darkmatter/lib/src/markdown/reference/provenance.rs
    - darkmatter/lib/src/markdown/reference/validate.rs
    - darkmatter/lib/tests/request_context_epoch.rs
    - darkmatter/lib/tests/semantic_results_never_persist.rs
    - darkmatter/cli/tests/help.rs
documentation:
    - darkmatter/fixes/2026-09-16-content-policy-no-cache/plan.md
    - darkmatter/fixes/2026-09-16-content-policy-no-cache/spec.md
    - darkmatter/fixes/2026-09-16-content-policy-no-cache/implementation-log.md
    - darkmatter/docs/topics/caching.md
    - darkmatter/docs/topics/remote-url-references.md
    - darkmatter/docs/cli/compose.md
    - darkmatter/docs/structs/Markdown.md
    - darkmatter/features/_completed/2026-09-09-more-context/spec.md
    - darkmatter/docs/topics/schema-definition.md
completed_phase: "8"
implemented: true
packages:
    - darkmatter
human_review: true
human_review_items:
    - "Decide the biscuit-file split Cache-Control gap (open since Phase 1): widen this fix into biscuit-file, or explicitly defer it. fetch()/post() keep only the first Cache-Control field line (HeaderMap::get), so a no-store sent on a separate header line is lost and acceptance criterion 4 holds for single-line Cache-Control only. The fix would join every field line with get_all (RFC 9110 §5.3) and add a fetch_integration.rs test. It is documented as a Known Gap in docs/topics/caching.md and spec.md. This decides whether the review may accept AC4 as met."
    - "Confirm or overrule the adopted defaults for Q2 (delete the semantic-result persistence machinery; already done in Phase 5) and Q3 (a new dependency-light shared library owns the ContentPolicy vocabulary), plus derived rulings R-A … R-I. The spec closure condition is 'Q1–Q3 are ruled'. Only Q1 has an owner ruling (R36), so that Definition of Done box is left unticked. Phase 7 also raised two semantic changes for the research::metadata migration: research's empty policy means 'evergreen', which the contract forbids, and Months/Years need exact calendar arithmetic."
    - "No behavioral cross-OS evidence exists for any phase of this fix. build-linux is locked by a stale nightly-reward-spike run (reward-20260914-c3e60d0, since 2026-09-14T18:25Z), build-win-native's W: volume has 4 KB free, and build-win (WSL2) resets SSH at key exchange (all re-checked 2026-09-18). Phase 8 produced Windows compile evidence only (x86_64-pc-windows-gnu). Release the lock and free W: (see the storage-strategy skill), or accept CI as the first Linux/Windows/WSL2 evidence."
message_to_agent: >-
    Phase 8 (final) is complete; the fix is at "implementation complete,
    ready for review". The handoff note is in plan.md under Work-group 8B.
    For the reviewer or the author: (1) another session was concurrently
    editing interpolation error anchoring (SourceRef::OnDiskLine,
    directive_targets.rs, interpolation/rewrite.rs, missing_ctx_capture.rs)
    in this worktree. Those files are NOT part of this fix and are excluded
    from source_code. (2) When the fix moves to _completed,
    docs/topics/caching.md's link to this spec must follow. The
    more-context feature already moved to features/_completed/, and the
    spec's origin/activated_by frontmatter and compose/nested.rs:9 still name
    its old path. (3) HTTP-client tests time out as a cluster under heavy
    host load; re-run them at --test-threads 2 (now in the darkmatter skill).
---

# Implementation Plan — Disable persistent compose cache until ContentPolicy exists

## Summary and Definition of Done

### What this fix actually is

The safety-critical half of R18 already shipped: no production path attaches a
persistent store to `RunLocalCache`, so a warm cache can no longer replay
composed output, `::file` children, `::code` / `::toc-linking` results, or
document snapshots. What remains is **closing the boundary properly** rather
than re-implementing it.

Concretely, this fix has five bodies of work:

1. **Rulings (Q1–Q3).** The fix cannot be declared complete until the owner
   rules on whether raw HTTP bodies stay persistable, whether the dormant
   semantic-result machinery is deleted, and who owns the shared
   `ContentPolicy` vocabulary. Q1 and Q2 change the scope of phases 3–7
   materially; Q3 changes only documentation inside this fix.
2. **Cache-root side effects.** `ComposeOptions::build_remote_fetch_runtime`
   (`lib/src/markdown/compose/context/options.rs:1682`) calls
   `FileStore::new(root)`, and `FileStore::new` (`…/cache/store.rs:34`) does
   `create_dir_all` on `manifests/` and `blobs/` unconditionally. A local-only
   compose with `--cache-root` therefore materializes a cache tree with nothing
   in it. The contract must become: configuring a root mutates nothing; the
   directory structure is created lazily, immediately before a genuinely
   eligible artifact is written.
3. **Transport-cache standards compliance.** `parse_max_age`
   (`…/cache/remote_cache.rs:73`) collapses both `no-cache` and `no-store` to
   `Duration::ZERO`, and `write_cached` (`:156`) stores the body regardless.
   Worse, `decide_action` (`:117`) in `Optimistic` mode returns `ServeCache`
   for *any* present entry, so a `no-cache` body is reused with no
   revalidation, and `ttl_override` in `compute_expires_at` (`:92`) outranks
   every response directive including `no-store`. RFC 9111 §5.2.2.4/§5.2.2.5
   require: `no-cache` may be stored but must revalidate successfully before
   reuse; `no-store` must never be stored or reused, and an already-stored
   copy must get a best-effort removal.
4. **Manifest privacy.** `RemoteUrlManifest.url`
   (`…/cache/manifest.rs:94`) persists `url.to_string()` verbatim — userinfo
   and query string included — into a world-readable JSON file. The manifest
   only ever needs a stable identity hash plus a redacted diagnostic form; the
   live request already supplies the real URL.
5. **Surface reduction, coverage, and documentation.** Q2 decides how much of
   the dormant persistence machinery survives; whatever survives needs a
   structural guard so a future edit cannot silently reattach it. CLI help,
   library docs, `docs/topics/caching.md`, the `darkmatter` skill, and the
   originating more-context spec must all agree on the resolved boundary.

### What is already true (verified, do not redo)

- `register_and_fetch` (`lib/src/markdown/compose/remote_fetch.rs:533`) calls
  `check_allowed` **before** constructing a fetch task, so a denied host fails
  the slot without ever reaching `fetch_with_cache` and its `read_cached`.
  Acceptance criterion 5 needs a *test*, not a code change — but the test must
  also prove no cache directory appeared, which depends on Phase 2.
- Run-local vs. remote diagnostics are already distinguishable:
  `ComposeReport.cache_stats` (`CacheStats`) and
  `ComposeReport.remote_fetch_stats` (`RemoteFetchStats`) are separate fields
  rendered as separate `cache:` and `remote:` summary segments
  (`…/context/report.rs:227–250`).
- `lib/tests/persistent_cache_disabled.rs` and
  `cli/tests/compose_remote_caching.rs` already cover the implemented boundary.
- **No consumer outside `darkmatter/` uses `with_cache_root`,
  `with_cache_namespace`, `with_cache_access_mode`, `with_cache_freshness_mode`,
  `CacheFreshnessMode`, or `CacheStats`.** A repo-wide search found call sites
  only in `darkmatter/cli/src/commands/compose.rs:463` and darkmatter's own
  tests. Q2 option 1 is therefore cheap; no downstream migration is needed.
- A wiremock-backed async harness with conditional-GET responders already
  exists in `lib/src/markdown/compose/remote_fetch.rs` (`persistent_cache_tests`,
  `Conditional304`, `Conditional200`) — extend it rather than build a new one.

### Definition of Done

The fix is implementation-complete and ready for review when **all** hold:

- [ ] Q1, Q2, and Q3 are ruled, and the selected outcomes are folded back into
      `spec.md` (the spec's own closure condition).
- [x] Spec acceptance criteria 1–8 each map to a named, passing L1 test, and
      that mapping is recorded in this plan's Phase 8 table.
- [x] `just test` and `just lint` pass from `darkmatter/`.
- [x] A local-only compose with `--cache-root DIR` where `DIR` does not exist
      leaves `DIR` nonexistent; where `DIR` exists with sentinel content, the
      tree is byte-for-byte unchanged (compared via `Path` APIs, no `/`
      assumptions).
- [x] Under every `RemoteFreshnessMode` and with any `--remote-ttl`, a
      `no-store` response is never written to disk and a pre-existing
      `no-store` entry is neither reused nor left in place; a `no-cache`
      response is never served without a successful revalidation.
      *(Split `Cache-Control` field lines too, since review-1:
      `biscuit_file` now joins every field line.)*
- [x] No cache manifest on disk contains URL userinfo or query text.
- [x] A structural test fails if a production (non-`#[cfg(test)]`) call site
      attaches the semantic-result store to `RunLocalCache`.
- [x] CLI help, public builder docs, `docs/topics/caching.md`, the
      `.claude/skills/darkmatter/` files, and
      `darkmatter/features/2026-09-09-more-context/spec.md` describe the same
      boundary with the same semantic-result / transport-artifact vocabulary.

### Explicitly out of scope

- Implementing `ContentPolicy` itself (spec: "This fix does not implement
  `ContentPolicy`"). Phase 7 records the contract; it writes no evaluator.
- Creating the shared vocabulary package that Q3 option 1 recommends. Q3's
  answer is *recorded* here; building it is a separate feature.
- Deleting any user's existing on-disk cache tree, except the RFC-mandated
  best-effort removal of a `no-store` entry.
- Changing `biscuit-file`'s `FetchResponse` unless spike S1 proves
  `Cache-Control` alone cannot express the required behavior.

---

## Phase 1 — Rulings, spikes, and baseline capture

**Goal:** every decision that changes the shape of phases 2–8 is made and
written down, and the risky unknowns are measured before code is touched.

### Necessary Rulings

These are blocking for the phases named beside them. The spec's own
recommendation is carried forward as the **stated default**: if the owner does
not rule, the implementer proceeds on the default, records it in `spec.md`, and
flags it in the handoff.

#### Blocking rulings from the specification

- [x] **R-Q1 — Does `--cache-root` continue to persist raw HTTP response
      bodies?** *(blocks phases 3, 4, 6, 7)*
    - Spec options 1/2/3; **stated default: option 1** (keep as a transport-cache
      exception), which requires the ruling to read R18's "nothing is cached
      persistently" as "no semantic-result artifacts".
    - If option 2 is chosen instead, phases 3 and 4 collapse into a deletion of
      `remote_cache.rs`, `RemoteUrlManifest`, and `--cache-root`, and acceptance
      criteria 3, 4, and 6 become vacuous. Re-plan before proceeding.
    - **Decision (Phase 1):** owner-ruled option 1 (Ken, 2026-09-17, more-context R36). Raw HTTP response bodies stay persistable as a transport-cache exception; phases 3, 4, 6, 7 proceed as planned.

- [x] **R-Q2 — Retain or remove the dormant semantic-result persistence
      machinery?** *(blocks phases 5, 6, 7)*
    - Spec options 1/2/3; **stated default: option 1** (delete now).
    - Evidence supporting the default: no consumer outside `darkmatter/`
      touches these APIs (verified), so the removal is internal.
    - Spike S4 must produce the deletion inventory *before* this ruling is
      finalized so the cost is known, not guessed.
    - **Decision (Phase 1):** no owner ruling in this session; **stated default adopted (option 1, delete now)** and recorded in `spec.md`. S4 measured the cost (see "Phase 1 spike findings" below): larger than "internal only" suggested because the run-local API signatures and dependency plumbing carry the dormant types, but still crate-internal apart from `CacheFreshnessMode`, `with_cache_freshness_mode`, and four `CacheStats` fields.

- [x] **R-Q3 — Which package owns the shared `ContentPolicy` vocabulary?**
      *(blocks phase 7 documentation only)*
    - Spec options 1/2/3; **stated default: option 1** (dependency-light shared
      library consumed by Research, Darkmatter, and Claudine).
    - This fix records the decision and the migration note for
      `research::metadata`'s draft `ContentPolicy` / `ContentExpiry` and for
      Claudine's documented `ContentPolicy` concept. It creates no package.
    - **Decision (Phase 1):** no owner ruling; **stated default adopted (option 1, dependency-light shared library)** and recorded in `spec.md`. No package is created by this fix.

#### Derived rulings the specification does not settle

- [x] **R-A — Does `CacheAccessMode` stay public?** It is *not* purely a
      persistence control: `PipelineRuntime::new(depth, CacheAccessMode)` uses
      it to gate run-local caching (`…/context/authority.rs:271`). Under Q2
      option 1 it therefore has a real, non-false advertised effect.
      **Stated default: keep `CacheAccessMode` public, restate its docs as
      run-local only, and remove `CacheFreshnessMode` and
      `with_cache_freshness_mode`,** whose only consumer was the deleted
      persistent read path.
    - **Decision (Phase 1):** stated default adopted.

- [x] **R-B — Does `with_cache_namespace` survive?** Under Q1 option 1 it still
      has an effect: it participates in `FileStore::resolve_cache_root` for the
      remote store. **Stated default: keep it, redocument it as
      "namespaces the remote transport cache root".**
    - **Decision (Phase 1):** stated default adopted.

- [x] **R-C — Delete the platform-cache fallback in
      `FileStore::resolve_cache_root`?** The `workspace_root: None` branch
      (`store.rs:44–52`, falling back to `dirs::cache_dir()`) is unreachable
      from production: `build_remote_fetch_runtime` always passes `Some(root)`.
      The spec explicitly wants persistence to stay "explicit opt-in rather than
      a default platform-cache side effect". **Stated default: change the
      signature to take `&Path` (non-optional) and delete the fallback branch
      and its unit test.**
    - **Decision (Phase 1):** stated default adopted. S3 confirmed the only `resolve_cache_root` production caller passes `Some(root)`.

- [x] **R-D — What happens to `CacheStats`' persistent counters?**
      `persistent_hits`, `persistent_writes`, `revalidations`, and `stale_hits`
      live on the *run-local* stats type; the transport cache reports through
      `RemoteFetchStats` instead. Under Q2 option 1 these four fields can never
      be non-zero. **Stated default: delete the four fields from `CacheStats`,
      update `merge`/`has_activity`, and leave `RemoteFetchStats` as the sole
      transport reporter.** This directly satisfies "cache statistics must not
      report creation of empty directories as a write."
    - **Decision (Phase 1):** stated default adopted. The `remote:` summary segment already reads `RemoteFetchStats.revalidations`, not the `CacheStats` field, so the report does not lose a counter.

- [x] **R-E — Scope of `no-store` best-effort removal.** Blob paths are keyed by
      content hash, so in principle two manifests could reference one blob.
      **Stated default: remove the manifest unconditionally, then attempt to
      remove the blob unconditionally.** Rationale: a collision means identical
      bytes, and the worst case is one extra re-fetch by an unrelated entry —
      strictly safer than leaving `no-store` bytes on disk. Record this
      trade-off in the code as a WHY comment at the removal site.
    - **Decision (Phase 1):** stated default adopted. S1 adds a constraint: `read_cached` returns `None` when the blob is missing, so the purge must read the manifest independently of the blob.

- [x] **R-F — How does a cleanup failure surface as "a cache warning"?** The
      fetch runs in a task on a detached Tokio runtime
      (`remote_fetch.rs:558–592`), while `ComposeWarning` is assembled on the
      compose path and folded in at `pipeline/mod.rs:99`. **Stated default: add
      a `cache_warnings: Vec<String>` to `RemoteFetchStats`, appended under its
      existing stats mutex, and fold each entry into `report.warnings` as a
      `ComposeWarning` with `stage = "remote-cache"` and a stable `code`.**
      A cleanup failure must remain non-fatal.
    - **Decision (Phase 1):** stated default adopted.

- [x] **R-G — `no-cache` + failed revalidation under `Fallback` mode.** The spec
      says `no-cache` "must be successfully revalidated before reuse, including
      in optimistic mode," and separately that `Fallback` serves stale on
      failure. These collide. **Stated default: `no-cache` outranks
      `Fallback` — a failed revalidation of a `no-cache` entry is an error, not
      a stale serve.** RFC 9111 §5.2.2.4 permits stale reuse of a `no-cache`
      response only where the origin explicitly allows it, which we cannot
      determine here.
    - **Decision (Phase 1):** stated default adopted.

- [x] **R-H — Exact shape of the redacted diagnostic URL.** **Stated default:
      `scheme://host[:port]/path` with userinfo removed entirely and any query
      replaced by the literal marker `?<redacted>`; the fragment is dropped.**
      Note the residual risk: a path segment can itself carry a token. If the
      owner judges paths sensitive, the fallback is `scheme://host[:port]` plus
      the identity hash only — decide now, because it changes the manifest
      shape and the assertion in acceptance criterion 6.
    - **Decision (Phase 1):** stated default adopted (`scheme://host[:port]/path?<redacted>`, no userinfo, no fragment). Residual path-token risk is flagged for the owner in the Phase 1 handoff, not blocking.

- [x] **R-I — Legacy on-disk entries after the manifest shape changes.**
      **Stated default: bump `CACHE_VERSION` from `1` to `2`
      (`…/cache/manifest.rs:13`); a manifest whose `cache_version` does not
      match is a cache miss and is left on disk untouched** — except a
      `no-store` entry, which R-E removes. The fix must not delete a user's
      cache tree automatically (spec, "Semantic-result persistence").
    - **Decision (Phase 1):** stated default adopted. Note: the cache directory name embeds `CACHE_VERSION` (`FileStore::resolve_cache_root` → `.darkmatter/cache/v{N}`), so the bump moves new writes to `v2/` and a `v1/` tree is never read again. The R-E purge of a `v1` `no-store` entry therefore only happens if Phase 4 decouples the directory version from the manifest version, or if the purge deliberately looks in `v1/` too. Phase 4 must decide; see the Phase 1 handoff.

### Spikes

Run these before or alongside the rulings. Each is bounded and answers a
question that would otherwise be guessed during implementation.

#### Work-group 1A — investigation spikes (all four run concurrently)

- [x] **S1 — Header sufficiency spike**
    - Confirm `biscuit_file::file_reference::fetch::FetchResponse`
      (`biscuit-file/lib/src/file_reference/fetch.rs:83`) exposes everything the
      RFC behavior needs. It currently carries `status`, `body`, `etag`,
      `last_modified`, `cache_control` — and **nothing else**: no `Pragma`, no
      `Vary`, no `Age`.
    - Determine whether multiple `Cache-Control` response headers are collapsed
      or whether only the first survives; a split `no-store` on a second header
      line must not be missed.
    - **Deliverable:** a one-paragraph finding stating either "`Cache-Control`
      alone is sufficient" (expected) or "a `biscuit-file` change is required",
      which would widen scope to a second package area and must be escalated
      before Phase 3 starts.

- [x] **S2 — Test-harness capability spike**
    - Lib side: confirm the existing wiremock `persistent_cache_tests` module
      can express `Cache-Control: no-store` and `no-cache` responses plus
      conditional revalidation. Expected: yes, via `ResponseTemplate`.
    - CLI side: `MockHttpResponse` (`cli/tests/common/mod.rs:36`) exposes only
      `status`, `body`, and `cache_control` — **no `ETag`, no per-request
      scripting**. Determine the minimum extension needed for acceptance
      criterion 3 (remote bytes hit the transport cache while local content is
      recomposed).
    - **Deliverable:** the exact `MockHttpServer` diff required, or a statement
      that criterion 3 is satisfiable with the current fixture.

- [x] **S3 — Filesystem-observability spike**
    - Prove a test can assert "a nonexistent cache root stays nonexistent" and
      "an existing tree is unchanged" portably across macOS, Linux, native
      Windows, and WSL2, using `Path`/`fs` APIs with no `/` separator
      assumptions.
    - Verify nothing *else* in the compose path touches the configured root:
      audit `tracing` sinks, `tempfile` usage in `FileStore::atomic_write`
      (`store.rs:175`), and `dirs::cache_dir()` reads.
    - **Deliverable:** a reusable `assert_tree_unchanged(root)` helper sketch
      plus a list of any incidental writers found.

- [x] **S4 — Q2 deletion-inventory spike** *(input to R-Q2)*
    - Using GitNexus `impact` (upstream) on `RunLocalCache::with_persistent`,
      `try_persistent_read_compose`, `try_persistent_write_compose`,
      `try_persistent_read_operation`, `try_persistent_write_operation`,
      `create_document_snapshot`, `PersistentContext`,
      `OperationPersistentContext`, `ContextClosureIdentity`, and the
      `DocumentSnapshotManifest` / `ComposedDocumentManifest` /
      `OperationResultManifest` types, enumerate every symbol reachable **only**
      from the dormant path.
    - Treat `risk: UNKNOWN` as unresolved and confirm each candidate with a text
      search before listing it as deletable.
    - Note that `…/cache/hashing.rs` (810 lines) is shared with run-local key
      computation — separate the genuinely dead hashing surface from the live
      one rather than assuming the file is all one or all the other.
    - **Deliverable:** a deletable/keep table with line counts, attached to this
      plan, so R-Q2 is ruled against a measured cost.

### Baseline capture

- [x] **Record the pre-change baseline**
    - Run `just test` and `just lint` from `darkmatter/` and record the passing
      baseline, so any later red is attributable to this fix.
    - Capture the current `md compose --help` text for `--cache-root` and the
      current `docs/topics/caching.md` structure, for the Phase 7 diff.

### Phase 1 spike findings

#### S1 — `Cache-Control` alone is sufficient, but it is not read completely

- `Cache-Control` is the only header the RFC 9111 behavior in this fix needs:
  `Pragma` has no defined response semantics, `Age` affects only shared-cache
  freshness arithmetic we do not perform, and `Vary` is moot for a
  URL-keyed private cache that sends no varying request headers.
- **Escalation — a `biscuit-file` change is required.** `fetch()` and `post()`
  (`biscuit-file/lib/src/file_reference/fetch.rs:221`, `:292`) read the header
  with `HeaderMap::get`, which returns only the **first** field line. Verified
  empirically with a throwaway wiremock test (deleted afterwards): a response
  carrying `Cache-Control: max-age=3600` and a second `Cache-Control: no-store`
  line produced `cache_control == Some("max-age=3600")`, so the split
  `no-store` was lost. Minimum fix: combine every field line with `", "`
  (`get_all`, per RFC 9110 §5.3) and pin it with a `fetch_integration.rs`
  test. This widens the fix to the `biscuit-file` package area and must be
  approved before Phase 3. *(Implemented in review-1.)*
- **A darkmatter-side defect of the same kind:** `parse_max_age`
  (`…/cache/remote_cache.rs:73`) returns on the first recognized directive, so
  even a single `Cache-Control: max-age=3600, no-store` line yields 3600 s and
  ignores `no-store`. Phase 3 must parse the whole directive list into a
  structured value (`no_store`, `no_cache`, `max_age`) rather than
  first-match. Also handle case-insensitive names, `max-age="60"` (quoted), and
  the `no-cache="field"` form (treat as `no-cache`).

#### S2 — harness capability

- **Lib side:** sufficient. wiremock 0.6.5's `ResponseTemplate` has
  `insert_header` and `append_header`, so `no-store`, `no-cache`, split
  header lines, and conditional responders (`Conditional304` /
  `Conditional200`) are all expressible. No new harness is needed.
- **CLI side:** acceptance criterion 3 is **already satisfied** by
  `cli/tests/compose_remote_caching.rs::test_compose_cache_root_never_replays_composed_local_output`
  (remote body served from cache on the warm run with one request; local
  `::file`, `::code`, and `current_env` probes recomposed). No fixture change is
  needed for criterion 3.
- For an optional CLI-level `no-cache` e2e test, the minimum
  `MockHttpServer` diff is one field, `etag: Option<&'static str>` on
  `MockHttpResponse`, emitted as an `ETag:` header in `cli/tests/common/mod.rs`
  beside `Cache-Control`. The fixture already maps `304` to `Not Modified` and
  captures raw request text via `requests()`, so `If-None-Match` can be asserted
  without per-request scripting. Adding the field means updating every existing
  `MockHttpResponse { .. }` literal (about 13 sites in
  `compose_remote_caching.rs`).
- That test path hard-codes `.darkmatter/cache/v1/manifests` with `/`
  separators; Phase 4's `CACHE_VERSION` bump must update it, and it should
  build the path with chained `Path::join`.

#### S3 — filesystem observability

- The only production writer under the configured root is `FileStore`. The
  eager side effect is exactly the two `create_dir_all` calls in
  `FileStore::new` (`store.rs:34–36`). `atomic_write` (`store.rs:175`) already
  creates the target's parent lazily and places its `NamedTempFile` in that
  parent, so no temp file escapes the root and no other directory is touched.
- `build_remote_fetch_runtime` (`options.rs:1770`) swallows a `FileStore::new`
  error with `.ok()`, silently disabling the transport cache. Once creation is
  lazy, that failure moves to write time. Phase 2 should record the new
  failure point rather than keep the silent `.ok()`.
- No other writer was found: no `tracing` file appender exists in `lib/` or
  `cli/`; `dirs::cache_dir()` is read only in the unreachable
  `resolve_cache_root(None, …)` branch (R-C); remaining `tempfile` uses are in
  the editor, effects, and tests, none on the compose path.
- Doc drift found: `store.rs:11` claims "cross-process safety uses
  filesystem-level lock files". No lock files exist; safety comes from
  tempfile+rename. Fix it in Phase 2 (per the repository comment policy, the
  code is correct).
- Reusable helper sketch (portable: compares `Path` components, never
  strings with `/`):

  ```rust
  /// Relative path, file kind, and bytes for every entry under `root`, sorted.
  fn snapshot_tree(root: &Path) -> Vec<(PathBuf, bool, Vec<u8>)> {
      fn walk(root: &Path, dir: &Path, out: &mut Vec<(PathBuf, bool, Vec<u8>)>) {
          for entry in std::fs::read_dir(dir).unwrap() {
              let path = entry.unwrap().path();
              let rel = path.strip_prefix(root).unwrap().to_path_buf();
              if path.is_dir() {
                  out.push((rel, true, Vec::new()));
                  walk(root, &path, out);
              } else {
                  out.push((rel, false, std::fs::read(&path).unwrap()));
              }
          }
      }
      let mut out = Vec::new();
      walk(root, root, &mut out);
      out.sort();
      out
  }
  // nonexistent root: assert!(!root.exists()) after compose (use
  //   `symlink_metadata` to also catch a dangling link)
  // existing root: assert_eq!(snapshot_tree(root), before)
  ```

  mtime is deliberately excluded: sentinel files are never rewritten, and
  directory mtimes vary by filesystem, which would make the test flaky on
  Windows and WSL2 `drvfs`.

#### S4 — Q2 deletion inventory

GitNexus could not be used: every darkmatter symbol lookup (including
`FileStore`) returned "not found", and `.gitnexus/` held an `analyze.lock` plus a
`lbug.staging.*` directory from an earlier run. No re-index was started in
case another session owns the lock. The inventory below comes from `rg`,
checked call site by call site, which the plan already required for any
`UNKNOWN` result. **Phase 5 must re-run GitNexus `impact` once the index is
healthy (`just gitnexus`) before deleting anything.**

| Surface | Location | Lines (approx.) | Verdict |
|---|---|---|---|
| Persistent read/write/validate methods (`try_persistent_*`, `try_write_document_snapshot`, `cache_document_snapshot`, `get_document_snapshot`, `compose_entry_key`, `read_blob_string`, `validate_*_manifest`, `resolve_dependency_closure_hash`, `operation_closure_hash`, `compose_dependency_ref`, `operation_dependency_ref`, `record_revalidation`, `record_stale_hit`, `PersistentLookup`) | `cache/runtime.rs:101–104, 656–1179, 1201–1208` | ~540 | delete |
| `RunLocalCache` fields `persistent`, `document_snapshots`, `remote_fetch` + `with_persistent` + `with_remote_fetch` | `cache/runtime.rs:122–197` | ~35 | delete (`remote_fetch` exists only to revalidate `RemoteUrl` deps during compose-manifest validation; caller chain `shell_expansion/types.rs:1415, 1446`) |
| `PersistentContext`, `ContextClosureIdentity`, `OperationPersistentContext` | `cache/runtime.rs:64–99` | ~35 | delete the types; **keep** the hash inputs, which build the run-local key |
| `get_or_compute_compose` / `get_or_compute_operation` params `persistent_ctx`, `freshness_mode`, `persistent_eligible` | `cache/runtime.rs:277, 416` + callers `transclusion/engine.rs:1256, 1538, 1676` | signature change | keep functions, drop params |
| `ComposeResult.dependencies` | `cache/runtime.rs:53` | small | delete; **keep** `context_groups` (live: `runtime.record_context_groups`) |
| Persistent unit tests (`persistent_cache()`, `persistent_*`, `fallback_mode_serves_stale_*`, `optimistic_mode_accepts_stale_compose_*`, `compose_dependency_invalidation_*`) | `cache/runtime.rs:1218–1224, 1553–2086` | ~540 | delete; the run-local tests (1226–1552) stay |
| `DocumentSnapshotManifest`, `ComposedDocumentManifest`, `OperationResultManifest` + impls + tests | `cache/manifest.rs:20–89, 127–209, tests` | ~350 of 608 | delete; **keep** `CACHE_VERSION`, `RemoteUrlManifest` (its `content_hash` doc comment cites closure-hash invalidation — reword it in Phase 5) |
| `ArtifactClass::{DocumentSnapshot, ComposeDocumentCore, OperationResult}`, `SourceKind` (used only by `DocumentSnapshotManifest`; `RemoteUrlManifest` has no `source_kind`), `DependencyRef`, `CacheFreshnessMode` | `cache/types.rs:19–68` | ~50 | delete; **keep** `ArtifactClass::RemoteUrl` (store path class) and `CacheAccessMode` (R-A) |
| `CacheStats.{persistent_hits, persistent_writes, revalidations, stale_hits}` | `cache/types.rs:84–93, 105–108, 116–117` | ~15 | delete (R-D); public-API change |
| `FileOperation` + `CacheableOperation::FileOperation` impl | `cache/operation.rs:66–118` + tests | ~60 | delete (no production caller; the header comment already calls it a model only) |
| `CodeOperation`, `TocLinkingOperation`, `ParamBuckets` | `cache/operation.rs` | — | **keep** — `variant_cache_key` / `cache_key_string` build the run-local `code:` / TOC keys |
| hashing.rs dead: `compose_entry_key`, `operation_entry_key`, `closure_hash`, `context_groups_hash`, `body_semantic_hash`, `body_template_hash`, `effective_state_hash` (zero callers today), `raw_bytes_hash` (only for the dead `source_content_hash`) | `cache/hashing.rs` | ~250 of 810 incl. tests | delete |
| hashing.rs live: `compose_cache_key`, `source_id_hash`, `frontmatter_hash` (toc, reference provenance), `context_hash`, `options_hash`, `combine_options_overlay_hash`, `set_overlay_hash`, `canonical_json_sorted` | `cache/hashing.rs` | — | **keep** |
| Transclusion engine persistence plumbing: `RequestContextClosure` + its `ContextClosureIdentity` impl, `PersistentContext`/`OperationPersistentContext` construction, `*_dependency_ref` recording, the `DependencyRef` push for remote URLs, and the **extra `std::fs::read` of every `::code` / `::toc-linking` source** used only for the dead `source_content_hash` | `transclusion/engine.rs:71, 477–480, 1234–1250, 1283–1287, 1510–1516, 1579–1581, 1644–1662, 1706` | ~80 | delete (also removes one redundant file read per operation) |
| Dependency propagation: `PipelineRuntime.dependencies` / `record_dependency` / `dependencies()`, nested-compose `dependencies` merge | `shell_expansion/types.rs:1380–1505`, `nested.rs:21, 111, 161, 226` | ~25 | delete |
| `ComposeOptions.cache_freshness_mode`, `with_cache_freshness_mode`, `persistent_cache_eligible`, `Debug` field, identity encoding tag | `context/options.rs:256, 517, 788, 943, 2251, 2485–2490, 2812` + tests `3714–3722` | ~40 | delete (R-A). Removing the identity-encoding field changes `options_hash` values; harmless because they key only run-local entries after Q2 |
| Public re-export of `CacheFreshnessMode` | `compose/mod.rs:110`, `cache/mod.rs:24` | 2 | delete |
| Tests touching the removed public surface | `lib/tests/persistent_cache_disabled.rs`, `lib/tests/reference_integration.rs`, `lib/tests/request_context_epoch.rs`, `compose/tests/schema.rs:398–540`, `compose/tests/transclusion.rs:1797+` | — | rewrite to the surviving surface; do not merely delete — they carry acceptance criteria 1 and 2 |

Estimated net deletion: roughly **1,900–2,100 lines** (about 900 production, the
rest tests), all inside `darkmatter/lib`. No crate outside `darkmatter/` is
affected (re-confirmed by `rg` for every public symbol listed).

**Consequence for acceptance criterion 2:** its wording "under every legacy
`CacheFreshnessMode`" becomes vacuous once that type is deleted. Phase 5/8
should restate criterion 2 as "cold and warm runs recompose under every
remaining cache control (`CacheAccessMode` × configured cache root)" and fold
that wording into `spec.md`.

#### Baseline (2026-09-17, before any Phase 2 change)

- `just test` (darkmatter): **8,098 passed, 14 skipped, 0 failed** (144 s).
- `just lint` (darkmatter): **green**, including the `zed-dmls` wasm32 check.
- `md compose --help`, cache-related flags as shipped:
  - `--cache-root <DIR>` — "Cache root for fetched remote URL bodies (composed
    output is never persisted)"
  - `--remote-ttl <SECONDS>` — "Remote artifact TTL in seconds (default: use
    server cache headers)"
  - `--remote-refresh` — "Force revalidation of cached remote artifacts"
  - `--remote-freshness` — `optimistic`: "Serve any cached artifact without
    revalidation, even when stale"; `strict`: "Always revalidate with a
    conditional GET"; `fallback`: "Serve stale on network failure (the
    default)".
  - **Drift for Phase 7:** `strict` help says "Always revalidate", but
    `decide_action` (`remote_cache.rs:117`) serves the cache without
    revalidation while within TTL under both `Strict` and `Fallback`. The code
    is presumed correct; Phase 7 should correct the help text.
- `docs/topics/caching.md` structure (H2): Overview · Current Status · Runtime
  Architecture · Persistent Store · Artifact Classes (document_snapshot,
  compose_document_core, operation_result) · Hashing Strategy · Persistent
  Key Model · Parameter Buckets · Dependency-aware Invalidation · Freshness
  and Access Modes · Read and Write Flow · Snapshot Caching in Memory · Cache
  Statistics · Reference Analysis as a Cache Consumer · Current Limitations and
  Future Work · Source Files. **No section describes the remote transport
  cache** (`remote_url` artifacts, `no-cache`/`no-store`, TTL precedence), and
  most of the file documents machinery that Q2 deletes. Phase 7 is therefore
  closer to a rewrite than an edit.

### Phase 1 exit criteria

- R-Q1, R-Q2, R-Q3 and R-A … R-I each have a recorded decision (owner ruling or
  adopted stated default).
- S1–S4 deliverables exist; S1 has not escalated into a `biscuit-file` change,
  or the escalation has been raised.
- Baseline `just test` / `just lint` are green.

---

## Phase 2 — Cache-root side effects: lazy store creation

**Goal:** configuring a cache root mutates nothing. The store's directory
structure comes into existence only immediately before an eligible artifact is
actually written.

**Depends on:** Phase 1 (R-C, S3). **Independent of Phase 3** — these two
phases touch disjoint code and may be worked concurrently by two implementers.

### Work-group 2A — implementation

- [x] **Make `FileStore` construction non-mutating**
    - `FileStore::new` (`lib/src/markdown/compose/cache/store.rs:34`) must stop
      calling `fs::create_dir_all` for `manifests/` and `blobs/`.
    - Prefer changing the constructor to an infallible `FileStore::at(root)`
      that only records the path; the `io::Result` return exists solely because
      of the eager `create_dir_all`, and callers currently swallow it with
      `.ok()` (`options.rs:1689`) or `tracing::warn!` (`runtime.rs:190`).
    - `atomic_write` (`store.rs:175`) already does `create_dir_all(parent)` —
      that is the lazy creation point and needs no change.
    - Complexity: the fanout directories (`manifests/{class}/{ab}/{cd}/`) are
      created by `atomic_write`'s parent call, so removing the eager roots does
      not break writes. Verify this holds for `write_blob` too.

- [x] **Drop the platform-cache fallback** *(per R-C, skip if ruled otherwise)*
    - Narrow `FileStore::resolve_cache_root` to take a non-optional `&Path`
      workspace root and delete the `dirs::cache_dir()` branch plus
      `resolve_cache_root_without_workspace` (`store.rs:387`).
    - Remove the now-unused `dirs` import if nothing else in the module uses it;
      check whether the crate-level `dirs` dependency is still needed elsewhere
      before touching `Cargo.toml`.

- [x] **Audit every remaining eager-mutation path**
    - Re-read `build_remote_fetch_runtime` (`options.rs:1682`) and confirm that
      after the change it performs zero filesystem I/O.
    - Confirm no other `FileStore::new` call site exists outside tests
      (currently: `options.rs:1689`, `runtime.rs:189`, and test helpers at
      `remote_fetch.rs:1048` and `store.rs:209`).

### Work-group 2B — tests (write concurrently with 2A; they gate it)

- [x] **Nonexistent-root test** *(acceptance criterion 1, first half)*
    - Library-level: compose a local-only document with
      `ComposeOptions::with_cache_root(<temp>/never-created)` and assert the
      path still does not exist afterwards.
    - Add to `lib/tests/persistent_cache_disabled.rs` — it already owns this
      contract's narrative.

- [x] **Unchanged-existing-root test** *(acceptance criterion 1, second half)*
    - Pre-populate a cache root with sentinel files and nested directories,
      compose local-only, then assert the tree is unchanged: same set of
      relative paths, same file contents, no new entries.
    - Use the S3 `assert_tree_unchanged` helper; walk with `walkdir` or
      `fs::read_dir` recursion and compare `Path` components, never string
      slashes.

- [x] **Stats do not report empty directories as writes**
    - Assert `report.cache_stats` shows no persistent write activity for the
      local-only run (shape depends on R-D; if the persistent counters are
      deleted, assert the `cache:` summary segment contains no write claim).

### Phase 2 exit criteria / validation checkpoint

- New tests fail against the pre-Phase-2 code and pass after it (verify by
  running them once before the `store.rs` edit).
- `just test` green; `just lint` green.
- Acceptance criterion 1 is fully satisfied.

---

## Phase 3 — Transport-cache RFC 9111 compliance

**Goal:** `no-store` is never written or reused; `no-cache` is never reused
without a successful revalidation; an explicit TTL never overrides either.

**Depends on:** Phase 1 (R-Q1 = keep, R-E, R-F, R-G; spikes S1, S2).
**Skip entirely if R-Q1 resolves to option 2** (remove `--cache-root`), in
which case re-plan phases 3–4 as a deletion.

### Work-group 3A — directive modeling (do first; 3B and 3C build on it)

- [x] **Introduce an explicit response-directive model**
    - Replace the overloaded `parse_max_age`
      (`lib/src/markdown/compose/cache/remote_cache.rs:73`) with a small
      `ResponseDirectives { no_store: bool, no_cache: bool, max_age:
      Option<Duration> }` parsed once from `Cache-Control`.
    - Keep parsing case-insensitive and comma-split as today; handle
      `max-age=0` as a genuine zero max-age *distinct from* `no-store`.
    - Delete the current behavior where `no-cache`/`no-store` masquerade as
      `Duration::ZERO` — the spec explicitly forbids describing a zero TTL as
      equivalent to `no-store`.
    - Update the three existing unit tests that assert the old collapsing
      behavior (`parse_max_age_no_cache_is_zero`, and the `max-age=0` case in
      `parse_max_age_extracts_seconds`).

- [x] **Fix TTL precedence in `compute_expires_at`**
    - `compute_expires_at` (`remote_cache.rs:92`) currently returns
      `now + ttl_override` before consulting the response at all.
    - New rule: `no_store` short-circuits storage entirely (the function is
      never reached for a `no-store` response); `no_cache` forces
      must-revalidate regardless of any TTL; otherwise `ttl_override` may
      outrank `max-age`.
    - Encode "TTL cannot override `no-store`" structurally — make it impossible
      to reach the write path with `no_store == true` — rather than as a runtime
      check that a later edit can bypass.

### Work-group 3B — storage and reuse decisions

- [x] **Never persist a `no-store` response**
    - Gate `write_cached` (`remote_cache.rs:156`) on the parsed directives:
      a `no_store` response returns its body to the caller and writes nothing.
    - Apply to all three write sites: the `304` metadata refresh (`:253`), the
      revalidated `200` (`:279`), and `FetchFresh` (`:321`). A response that
      becomes `no-store` on revalidation must also stop refreshing the old
      entry's metadata.

- [x] **Purge a pre-existing `no-store` entry** *(per R-E)*
    - In `read_cached` (`remote_cache.rs:141`), when the loaded manifest's
      `cache_control` records `no-store`, treat it as absent **and** attempt
      removal of both the manifest file and the body blob.
    - Add the removal primitives to `FileStore` (`remove_manifest`,
      `remove_blob`) returning `io::Result<()>`; a `NotFound` is success.
    - A removal failure is non-fatal and produces a cache warning (per R-F),
      never an error. Add a WHY comment at the blob removal recording the R-E
      trade-off.

- [x] **Force revalidation for `no-cache`, including optimistic mode**
    - `decide_action` (`remote_cache.rs:117`) must take the cached entry's
      directives, not just `within_ttl`. A `no_cache` entry yields
      `Revalidate` under **every** `RemoteFreshnessMode`, including
      `Optimistic`, and regardless of `ttl_override`.
    - Per R-G, a failed revalidation of a `no-cache` entry is an error even in
      `Fallback` mode — do not fall through to `StaleServed`.
    - Update the existing `decide_*` unit tests, which currently pass only
      `(cached_present, within_ttl, cfg)`.

### Work-group 3C — diagnostics wiring

- [x] **Add the cache-warning channel** *(per R-F)*
    - Add `cache_warnings: Vec<String>` to `RemoteFetchStats`
      (`lib/src/markdown/compose/remote_fetch.rs:59` region), appended under the
      existing stats mutex so the detached fetch task can report safely.
    - Fold entries into `report.warnings` where `remote_fetch_stats` is already
      attached (`lib/src/markdown/compose/pipeline/mod.rs:99`), as
      `ComposeWarning` with `stage = "remote-cache"` and a stable `code`.
    - Keep the `cache:` (run-local) and `remote:` (transport) summary segments
      in `…/context/report.rs:227–250` distinct, and make sure a cache warning
      does not silently inflate the `remote:` counters.

### Work-group 3D — tests *(acceptance criterion 4)*

- [x] **`no-store` matrix test**
    - Extend `persistent_cache_tests` in `remote_fetch.rs`: for each of
      `Strict`, `Fallback`, `Optimistic`, and with `ttl_override` both `None`
      and `Some(1h)`, a `Cache-Control: no-store` response writes **no**
      manifest and **no** blob, and a second fetch hits the network again.
    - Assert on the filesystem, not only on `RemoteOutcomeEvent`.

- [x] **Pre-existing `no-store` entry test**
    - Seed a store with a manifest whose `cache_control` is `no-store` plus its
      blob, fetch, and assert: the cached body is not served, and both files are
      gone afterwards.
    - Add a companion test where removal fails (e.g. a read-only parent where
      the platform allows it; otherwise inject the failure) asserting the run
      still succeeds and a cache warning is recorded. If the failure cannot be
      induced portably, test the warning path at the function level instead and
      say so in a comment.

- [x] **`no-cache` revalidation test**
    - For each freshness mode including `Optimistic`, and with an explicit TTL
      that would otherwise mark the entry fresh, assert a conditional GET is
      issued. Reuse `Conditional304` / `Conditional200`.
    - Assert that under `Fallback`, a `no-cache` entry whose revalidation fails
      produces an error rather than a stale serve (R-G).

- [x] **TTL-does-not-override test**
    - Explicitly assert that `--remote-ttl 1h` does not cause a `no-store`
      response to be stored, nor a `no-cache` entry to be served unrevalidated.

### Phase 3 exit criteria / validation checkpoint

- Every new test fails against pre-Phase-3 code and passes after.
- Acceptance criterion 4 fully satisfied.
- `just test` and `just lint` green.

---

## Phase 4 — Manifest privacy and identity redaction

**Goal:** no cache manifest on disk contains URL credentials or query text.

**Depends on:** Phase 3 (both phases modify `RemoteUrlManifest`; sequencing them
avoids a merge conflict and lets a single `CACHE_VERSION` bump cover both
shape changes). Skip if R-Q1 resolves to option 2.

### Work-group 4A — implementation

- [x] **Replace the cleartext URL field**
    - `RemoteUrlManifest.url` (`lib/src/markdown/compose/cache/manifest.rs:94`)
      currently stores `url.to_string()` including userinfo and query.
    - Replace it with a redacted diagnostic string per R-H plus the existing
      `source_id_hash` as the stable identity. The identity hash must continue
      to be computed over the **full** URL (`remote_cache.rs:226`,
      `xx_hash(url.as_str())`) so two URLs differing only in query do not
      collide — redaction is for the manifest, never for the key.
    - Implement redaction as a small, directly unit-tested helper next to the
      manifest type, not inline at the write site.

- [x] **Bump `CACHE_VERSION` and define legacy handling** *(per R-I)*
    - `CACHE_VERSION: u16 = 1` → `2` (`manifest.rs:13`).
    - A manifest whose `cache_version` differs is a miss; it is left on disk
      untouched (the exception remains the R-E `no-store` purge, which must
      still work on a v1 manifest — so version checking must not prevent
      reading enough of an old manifest to spot `no-store`).
    - Complexity to watch: a v1 manifest may not deserialize into the v2 struct
      at all once the `url` field changes shape. Decide and document how the
      `no-store` purge reads a v1 entry — either keep `url` as an optional
      legacy field during deserialization, or check the raw JSON. Prefer the
      former; it is simpler and testable.

- [x] **Sweep for other cleartext leaks**
    - Check whether any remaining manifest or log line writes a full URL:
      `tracing` calls in `remote_fetch.rs`, error strings from
      `RemoteReadError::InvalidUrl` (`remote_fetch.rs:511`), and the
      `DeniedByPolicy` host-only message.
    - Errors surfaced to the user may keep the URL (the user supplied it); only
      **persisted** artifacts must be redacted. State this distinction in the
      doc comment so a later reader does not over-redact diagnostics.

### Work-group 4B — tests *(acceptance criterion 6)*

- [x] **Manifest-redaction test**
    - Fetch `http://user:secret@127.0.0.1:PORT/doc.md?token=abc` through the
      caching path, read every file under the cache root, and assert none
      contains `secret`, `token`, or `abc`.
    - Assert the redacted form is still useful: it contains the host and
      (per R-H) the path.

- [x] **Identity-collision test**
    - Two URLs differing only in query string produce two distinct manifests
      (different `source_id_hash`) and do not serve each other's bodies.

- [x] **Legacy-version test**
    - A seeded v1 manifest is treated as a miss and left in place; a seeded v1
      `no-store` manifest is purged.

### Phase 4 exit criteria / validation checkpoint

- Acceptance criterion 6 satisfied; criterion 4's legacy-purge case still
  passes against a v1 entry.
- `just test` and `just lint` green.

---

## Phase 5 — Semantic-result surface reduction and structural guard

**Goal:** the forbidden state becomes structurally unreachable, and no public
control advertises an effect it does not have.

**Depends on:** Phase 1 (R-Q2, R-A, R-B, R-D, spike S4) and Phase 2 (both touch
`FileStore`). Scope varies sharply by ruling:

- **R-Q2 option 1 (default):** work-groups 5A, 5B, 5C all apply.
- **R-Q2 option 2:** 5A becomes "move behind a Cargo feature", 5B still applies.
- **R-Q2 option 3:** 5A is skipped; 5C (the guard) becomes the phase's whole
  point and its most important deliverable.

### Work-group 5A — remove dormant machinery *(R-Q2 option 1)*

- [x] **Delete the persistent semantic-result read/write paths**
    - From `lib/src/markdown/compose/cache/runtime.rs` (2086 lines): remove
      `with_persistent` (`:188`), the `persistent: Option<Arc<FileStore>>` field
      (`:131`), `try_persistent_read_compose` (`:656`),
      `try_persistent_write_compose` (`:782`), `try_persistent_read_operation`
      (`:732`), `try_persistent_write_operation` (`:873`),
      `create_document_snapshot` (`:925`), and the snapshot-read helpers
      (`:1007`, `:1030`).
    - Simplify the `persistent_eligible` / `persistent_ctx` parameters out of
      `compose_or_wait` (`:280`) and the operation equivalent (`:419`) once
      nothing reads them. Watch for callers in the transclusion pipeline that
      pass these through; use GitNexus `impact` before each removal.

- [x] **Delete the orphaned manifest and context types**
    - `DocumentSnapshotManifest`, `ComposedDocumentManifest`,
      `OperationResultManifest`, `PersistentContext`,
      `OperationPersistentContext`, `ContextClosureIdentity`, `DependencyRef`,
      and the `ArtifactClass` variants `DocumentSnapshot`,
      `ComposeDocumentCore`, `OperationResult` — retaining only `RemoteUrl`
      (which may then collapse the class-directory match in
      `FileStore::manifest_path`, `store.rs:146`).
    - Use the S4 inventory as the authoritative list; do not delete anything S4
      did not confirm unreachable.

- [x] **Separate live from dead hashing**
    - `…/cache/hashing.rs` is 810 lines and serves **both** run-local key
      computation (`compose_cache_key_for_path`, still live) and the dormant
      closure/dependency hashing. Remove only the dependency-closure and
      context-closure surface that S4 marked dead; keep run-local keying intact.
    - This is the highest-regression-risk task in the phase — run the full
      `just test` immediately after it, in isolation from other edits.

### Work-group 5B — public API truth-up *(runs after 5A; independent of 5C)*

- [x] **Remove or redocument the inactive controls**
    - Per R-A: delete `CacheFreshnessMode` and
      `ComposeOptions::with_cache_freshness_mode` (`options.rs:853`) and the
      `cache_freshness_mode` field, including its debug-format entry (`:517`)
      and its stable-encoding entries (`:2397–2402`) — note that changing the
      stable encoding may change a compose-identity hash; check whether any test
      pins that encoding and update deliberately.
    - Per R-A: keep `CacheAccessMode`, rewriting its docs to say run-local only.
    - Per R-B: keep `with_cache_namespace`, redocumented as the remote
      transport-cache root namespace.
    - Per R-D: delete `persistent_hits`, `persistent_writes`, `revalidations`,
      and `stale_hits` from `CacheStats` and fix `merge` / `has_activity`.
    - Update `lib/src/markdown/compose/mod.rs:110` re-exports accordingly.

- [x] **Fix every affected test and doc example**
    - Known call sites to update: `lib/tests/persistent_cache_disabled.rs`
      (uses all four `CacheFreshnessMode` variants at `:80–83`),
      `lib/tests/request_context_epoch.rs:19,373`,
      `lib/src/markdown/compose/tests/schema.rs:372,417,477,498`,
      `lib/src/markdown/compose/tests/transclusion.rs:1797–1814`,
      `lib/tests/reference_integration.rs:840–1579`.
    - The `persistent_cache_disabled.rs` freshness-mode matrix is acceptance
      criterion 2 — it must keep proving recomposition, so restructure it around
      the surviving controls rather than deleting the coverage.

### Work-group 5C — structural guard *(acceptance criterion 8; independent of 5B)*

- [x] **Add a source-level guard test**
    - Model it on the existing precedent: `cli/tests/spawn_site_guard.rs` and
      `cli/tests/common/source_scan.rs`.
    - The guard must fail if any non-`#[cfg(test)]` source line attaches a
      persistent semantic-result store to `RunLocalCache` — e.g. a
      `with_persistent(` call site, or (under option 1, where the method is
      gone) any reintroduction of a `FileStore` field on `RunLocalCache`.
    - Include a stale-exemption check so the guard cannot rot into a no-op, and
      a self-test proving the guard actually catches a planted violation.
    - Place it in `lib/tests/` so it protects the library where the invariant
      lives, and name it for the invariant rather than the mechanism.

### Phase 5 exit criteria / validation checkpoint

- Acceptance criteria 2 and 8 satisfied.
- `just test` and `just lint` green; no `dead_code` or `unused` allows added to
  paper over a half-removal.
- GitNexus `detect_changes --scope all` run and reviewed after the deletions.

---

## Phase 6 — Acceptance coverage completion

**Goal:** every remaining spec acceptance criterion has a named test.

**Depends on:** phases 2–5. Criteria 1, 2, 4, 6, and 8 are delivered by earlier
phases; this phase closes 3 and 5 and audits the whole set.

### Work-group 6A — remaining end-to-end tests (concurrent)

- [x] **CLI end-to-end mixed-content test** *(acceptance criterion 3)*
    - Extend `cli/tests/compose_remote_caching.rs`: a document transcluding both
      a local `::file` with a changing runtime value **and** a remote URL,
      composed twice with `md compose FILE --cache-root DIR --allow-host
      127.0.0.1`.
    - Assert the remote body is served from the transport cache on the second
      run (one HTTP request total) while the local child is recomposed (value
      changes between runs).
    - Must use `CliProcessFixture` (`cli/tests/common/fixture.rs`) — never a raw
      spawn; `cli/tests/spawn_site_guard.rs` rejects those. Apply the
      `MockHttpServer` extension identified by spike S2 if one is needed.

- [x] **Denied-host precedence test** *(acceptance criterion 5)*
    - Prove a denied host fails **before** any cache read or network request:
      seed the cache root with a manifest+blob for the target URL, compose
      without `--allow-host`, and assert the seeded body never appears in
      output and no HTTP request was recorded.
    - Complements the existing `mock_server_policy_denial_no_network_request`
      (`remote_fetch.rs:942`) by adding the cache dimension.
    - Add the companion case: `--cache-root` alone, with no `--allow-host`,
      grants no network access and creates no directories (this depends on
      Phase 2).

### Work-group 6B — audit

- [x] **Build the criterion → test mapping table**
    - One row per spec acceptance criterion 1–8, naming the test function and
      file. Any row without a test is a gap to close, not a note to file.
    - Record the table in this plan under Phase 8 so a reviewer can check it
      without re-deriving coverage.

- [x] **Portability audit of the new assertions**
    - Grep the new tests for `/`-separator assumptions, hardcoded `/tmp`, and
      `to_string_lossy().contains("/")` patterns. Replace with `Path`
      components.
    - Confirm no new test can gain terminal or browser focus (none should need
      a real terminal; all new work here is L1).
    - Confirm temp-dir symlink resolution on macOS does not break the
      "tree unchanged" comparison (see the `os` skill's macOS notes).

### Phase 6 exit criteria / validation checkpoint

- All eight criteria have a named passing test.
- `just test` green; `just test-l2` untouched (no L2 work is expected — confirm
  none was accidentally introduced).

---

## Phase 7 — Documentation, CLI help, and specification reconciliation

**Goal:** every surface that describes caching describes the *same* boundary,
in the same vocabulary, and the `ContentPolicy` contract is recorded for the
future feature.

**Depends on:** phases 2–6 (documentation follows settled behavior) and Phase 1
rulings Q1/Q2/Q3.

### Work-group 7A — user-facing surfaces (concurrent)

- [x] **CLI help**
    - `--cache-root` (`cli/src/args/command.rs:229–232`) currently reads "Cache
      root for fetched remote URL bodies (composed output is never persisted)".
      Restate in the resolved semantic-result / transport-artifact vocabulary
      and name exactly which artifact classes it can persist.
    - Review the neighboring `--remote-ttl`, `--remote-refresh`, and
      `--remote-freshness` help strings for the same terminology, and make sure
      none implies a zero TTL is equivalent to `no-store`.

- [x] **Library docs**
    - `ComposeOptions::with_cache_root` (`options.rs:860`),
      `with_cache_namespace` (`:867`), `with_cache_access_mode` (`:846`),
      `with_shared_remote_fetch` (`:~1665`), and
      `build_remote_fetch_runtime` (`:1680`) — all currently say "persistent
      store" without qualifying the artifact class.
    - Also update `lib/src/markdown/reference/graph.rs:58`, which already
      documents the no-persistent-backing behavior, so its wording matches.
    - Follow the repo's Rust doc conventions: no H1 inside `///`; `## Examples`,
      `## Errors`, `## Notes` as H2.

- [x] **`docs/topics/caching.md`** *(578 lines — the largest doc task)*
    - Restructure around the two-category distinction: **semantic-result
      artifacts** (prohibited until `ContentPolicy`) versus **transport
      artifacts** (raw HTTP bodies, governed by HTTP validators and directives).
    - Delete or clearly re-frame every section describing machinery Phase 5
      removed. The current "Implemented and unit-tested, but disabled" list is
      false the moment R-Q2 option 1 lands.
    - Document the new RFC behavior: `no-cache` always revalidates, `no-store`
      is never stored or reused and old entries are purged best-effort, TTL
      never overrides either, manifests carry a redacted URL plus an identity
      hash, and the `CACHE_VERSION` bump.
    - Document the lazy-creation contract explicitly: configuring a root
      mutates nothing.

### Work-group 7B — internal knowledge surfaces (concurrent with 7A)

- [x] **`.claude/skills/darkmatter/` updates**
    - `SKILL.md` — the "Remote and cache safety" bullet list already states the
      R18 boundary; update it for the resolved Q1/Q2 outcome, the lazy-creation
      contract, and the `no-store`/`no-cache` rules.
    - `compose.md:351` — "Child compose-cache identity (run-local in production;
      the persistent …)" must be corrected once the persistent path is gone.
    - Do **not** confuse this with `::shell --no-cache` (`compose.md:708–719`),
      which is an unrelated run-local shell-execution control; make sure the
      edits keep those two `no-cache` concepts visibly distinct.

- [x] **Originating specification fold-back**
    - `darkmatter/features/2026-09-09-more-context/spec.md` — reconcile R18's
      wording with the Q1 outcome (if option 1, R18's "nothing is cached
      persistently" must be read as "no semantic-result artifacts", stated
      explicitly rather than left implicit).

- [x] **Fold rulings into this fix's `spec.md`**
    - Record the Q1, Q2, Q3 outcomes and each derived ruling R-A … R-I in
      `spec.md`, replacing the "Open Questions" section's pending state and
      updating the "Implementation status" section.
    - This is the spec's own stated closure condition — the fix cannot be
      declared ready for review without it.

- [x] **Record the `ContentPolicy` contract and Q3 migration guidance**
    - The spec's "ContentPolicy design constraints" section is already the
      contract; Phase 7 adds the Q3 ownership decision and the migration or
      re-export note for `research::metadata`'s draft `ContentPolicy` /
      `ContentExpiry` and Claudine's documented `ContentPolicy` concept.
    - Keep the "calendar months/years must define exact arithmetic and timezone
      or be replaced by unambiguous durations" requirement visible — it is the
      easiest constraint for a future implementer to miss.

- [x] **Dependency-drift check**
    - If Phase 2's R-C change or Phase 5's deletions removed a crate
      (e.g. `dirs`), update `darkmatter/docs/dependencies.md` and the root
      `docs/dependencies.md` per the repo's drift rules.

### Phase 7 exit criteria

- Acceptance criterion 7 satisfied: CLI help, public builders, the caching
  topic, the darkmatter skill, and the more-context spec agree.
- `spec.md` records all rulings and an updated implementation status.
- No comment or doc anywhere still describes deleted machinery as present
  (comment-drift rule: the code is right, the comment is wrong).

---

## Phase 8 — Full verification and review handoff

**Goal:** prove the whole fix, across the OS matrix the repo requires, and hand
off in the correct terminal state.

**Depends on:** phases 1–7.

### Work-group 8A — verification (mostly sequential; 8A-1 and 8A-2 concurrent)

- [x] **Package-area gates**
    - `cd darkmatter && just test` (L1) and `just lint`, both green.
    - Do **not** run workspace-wide Cargo gates for a darkmatter-only change.

- [x] **Downstream-consumer check**
    - Public API changed (`CacheFreshnessMode`, `CacheStats` fields). Use Sniff
      and GitNexus to confirm the earlier finding still holds — no consumer
      outside `darkmatter/` references the removed symbols — and, if Claudine
      or another area does, build it.

- [x] **Graph-change analysis before any commit**
    - `detect_changes --scope all`, then `--scope compare --base-ref main`.
      A `partial: true` or `truncated: true` result is not a clean check; re-run.

- [x] **Cross-OS consideration pass**
    - This host is macOS. Review every new filesystem assertion against the
      Windows and WSL2 traps recorded in the `os` skill: path spelling and case,
      `remove_file` on an open handle, and macOS `/var` → `/private/var` symlink
      resolution in temp-dir comparisons.
    - Load the `os` skill before asserting any OS cannot be exercised from here;
      if a leg genuinely needs remote evidence, say so explicitly rather than
      claiming coverage.

### Work-group 8B — handoff

- [x] **Publish the criterion → test mapping table**
    - Insert the Phase 6 table here, one row per acceptance criterion 1–8 with
      test file and function name.

#### Criterion → test mapping (built in Phase 6)

Paths are relative to `darkmatter/`. `remote_fetch.rs` means
`lib/src/markdown/compose/remote_fetch.rs`, module `persistent_cache_tests`.

| AC | Criterion | Test file | Test function(s) |
|---|---|---|---|
| 1 | Local-only compose never creates a missing cache root; an existing sentinel-filled root stays byte-identical | `lib/tests/persistent_cache_disabled.rs` | `a_nonexistent_cache_root_is_never_created_by_local_only_work`, `an_existing_cache_root_is_left_byte_identical_by_local_only_work`, `a_cache_root_that_is_a_file_does_not_fail_local_only_work` |
| 1 | (CLI path) | `cli/tests/compose_remote_caching.rs` | `test_compose_local_only_cache_root_is_never_created_or_modified` |
| 2 | Changing `::file` child recomposed under every remaining cache control; `::code`, `::toc-linking`, snapshots, and reference graph persist nothing | `lib/tests/persistent_cache_disabled.rs` | `a_warm_cache_root_never_replays_composed_local_output` (`CacheAccessMode` × no root / root / namespaced root), `a_reference_graph_with_a_cache_root_persists_no_local_artifact` |
| 2 | (reference graph, namespaced; frozen request) | `lib/tests/reference_integration.rs`, `lib/tests/request_context_epoch.rs` | `reference_graph_with_namespaced_cache_root_writes_nothing`, `a_persistent_entry_cannot_bypass_a_frozen_missing_capture` |
| 2 | (CLI path) | `cli/tests/compose_remote_caching.rs` | `test_compose_cache_root_never_replays_composed_local_output` |
| 3 | `md compose FILE --cache-root DIR` reuses raw remote bytes while recomposing local content | `cli/tests/compose_remote_caching.rs` | `test_compose_mixed_document_reuses_remote_bytes_and_recomposes_local_child` (Phase 6) |
| 4 | `no-cache` always revalidates; `no-store` never written or reused under every mode and TTL override; pre-existing `no-store` entry ignored and purged | `remote_fetch.rs` | `no_store_response_is_never_stored_under_any_mode_or_ttl`, `split_line_no_store_is_never_stored_under_any_mode_or_ttl` (review-1; with `biscuit-file` `fetch_integration`'s `{fetch,post}_combines_split_cache_control_lines_in_wire_order`), `pre_existing_no_store_entry_is_purged_not_served`, `no_cache_entry_is_revalidated_under_every_mode_despite_ttl`, `fallback_does_not_serve_a_no_cache_entry_after_failed_revalidation` |
| 4 | (CLI path) | `cli/tests/compose_remote_caching.rs` | `test_compose_remote_ttl_does_not_override_no_store_or_no_cache` |
| 5 | Denied host fails before a cache read or a network request | `remote_fetch.rs` | `denied_host_never_reads_a_fresh_cached_entry` (Phase 6) |
| 5 | (CLI path, seeded cache, every freshness mode) | `cli/tests/compose_remote_caching.rs` | `test_compose_denied_host_never_reads_a_seeded_cache_entry` (Phase 6) |
| 5 | A cache root alone never authorizes the host | `cli/tests/compose_remote_caching.rs` | `test_compose_cache_root_alone_never_authorizes_a_host` (Phase 6) |
| 6 | Manifests hold no cleartext URL credentials or query values | `remote_fetch.rs` | `manifest_never_persists_url_userinfo_or_query`, `urls_differing_only_in_query_or_userinfo_never_share_an_entry` |
| 6 | (CLI path) | `cli/tests/compose_remote_caching.rs` | `test_compose_cache_manifest_never_persists_url_userinfo_or_query` |
| 7 | CLI help, builders, caching topic, skill, and more-context spec agree | `cli/tests/help.rs::test_compose_help_states_the_transport_cache_boundary` | **Covered in Phase 7.** The test pins `md compose --help` through `CliProcessFixture`: the `--cache-root` semantic-result / transport-artifact wording, laziness, `no-store`, "never authorizes a host", `--remote-ttl` never making `no-store` storable, and per-mode `--remote-freshness` text (including the corrected `strict`). It also rejects the drifted strings. The prose surfaces (`ComposeOptions` builder docs, `docs/topics/caching.md`, `remote-url-references.md`, `docs/cli/compose.md`, the darkmatter skill, and the more-context spec) are aligned by review; Phase 8 re-reads them. |
| 8 | No production call site can attach a semantic-result store to `RunLocalCache` | `lib/tests/semantic_results_never_persist.rs` | `semantic_results_have_no_path_to_disk`, `the_guard_catches_planted_violations_and_honors_the_scope_rule` |

- [x] **Write the review handoff note**
    - State: rulings taken (owner ruling vs. adopted stated default, per
      ruling), anything deliberately left out and why, and any residual risk
      (notably R-H's path-token risk and R-E's shared-blob trade-off).

#### Review handoff note (2026-09-18)

**State:** implementation complete, ready for review. The fix directory stays
under `fixes/`; the author moves it after the review cycle closes.

**Ruling provenance**

| Ruling | Outcome | Provenance |
|---|---|---|
| Q1 | Option 1: raw HTTP bodies stay persistable as transport artifacts; no semantic result persists | **Owner ruling** (Ken, 2026-09-17, more-context spec R36) |
| Q2 | Option 1: delete the dormant semantic-result machinery (done in Phase 5) | **Adopted plan default**, not separately confirmed by the owner |
| Q3 | Option 1: a new dependency-light shared library owns the `ContentPolicy` vocabulary; only recorded here, not built | **Adopted plan default**, not separately confirmed by the owner |
| R-A … R-I | As listed in `spec.md` `### Derived rulings` | **Adopted plan defaults**, none separately confirmed by the owner |

**Deliberately left out**

- `ContentPolicy` itself and the shared vocabulary package (out of scope per
  spec; the contract and Q3 migration guidance are recorded in `spec.md`).
- ~~The `biscuit-file` multi-line `Cache-Control` fix.~~ Closed by review-1:
  `fetch()`/`post()` now join every field line in wire order (RFC 9110 §5.3),
  so acceptance criterion 4 holds for split `Cache-Control` as well.
- Deleting users' existing cache trees, other than the RFC-mandated
  best-effort `no-store` purge.

**Residual risk**

- **R-H path tokens:** `redacted_url` drops userinfo, fragment, and query, but
  keeps the path, which can itself carry a token (for example
  `/share/<secret>/doc.md`). Recorded as a Known Gap.
- **R-E shared blobs:** a `no-store` purge removes its body blob even if a
  second manifest references the same content-addressed blob. That entry then
  re-fetches: a performance cost, never a correctness or privacy one.
- **Cross-OS evidence:** no behavioral run on Linux, native Windows, or WSL2
  was possible in any phase (build-linux holds a stale `nightly-reward-spike`
  lock from 2026-09-14; build-win-native's `W:` has 4 KB free; build-win
  resets SSH at key exchange). This phase adds compile evidence only
  (`cargo check --tests --target x86_64-pc-windows-gnu` for `darkmatter` and
  `darkmatter-cli`) plus a manual review of every new filesystem assertion.
  CI is the first behavioral evidence for those three environments.
- **Links that follow the fix directory:** `docs/topics/caching.md` links to
  `../../fixes/2026-09-16-content-policy-no-cache/spec.md#contentpolicy-design-constraints`.
  When this fix moves to `_completed`, that link must move with it. The
  more-context feature was already moved to `features/_completed/` in the
  working tree, and the fix spec's `origin` / `activated_by` frontmatter and
  `lib/src/markdown/compose/nested.rs`'s module doc still name its old path.

- [x] **Stop at "implementation complete, ready for review"**
    - Do not move the fix to `_completed`, do not run `just complete`, and do
      not commit unless the prompt explicitly asks. The author moves the fix
      after the review cycle closes.

### Phase 8 exit criteria

- Every Definition of Done checkbox in this plan's summary is ticked.
- The handoff note names the ruling provenance for Q1, Q2, Q3, and R-A … R-I.

---

## Dependency and concurrency overview

```
Phase 1 (rulings + spikes S1-S4)
   ├──► Phase 2 (lazy store creation)  ─┐   [2 and 3 are independent:
   ├──► Phase 3 (RFC 9111 compliance)  ─┤    disjoint files, may run
   │        └──► Phase 4 (redaction)    │    concurrently]
   └──► Phase 5 (surface reduction) ◄───┘   [needs Phase 2's FileStore shape]
                    │
                    ▼
            Phase 6 (acceptance coverage)
                    │
                    ▼
            Phase 7 (docs + spec fold-back)
                    │
                    ▼
            Phase 8 (verification + handoff)
```

Concurrency summary:

| Work-group | Phase | Concurrent with | Notes |
|---|---|---|---|
| 1A (S1–S4) | 1 | each other | four independent investigations |
| 2A / 2B | 2 | each other; all of Phase 3 | write tests first, they gate 2A |
| 3A | 3 | Phase 2 | must precede 3B and 3C |
| 3B / 3C | 3 | each other; Phase 2 | both build on 3A's directive model |
| 3D | 3 | — | validates 3A–3C |
| 4A / 4B | 4 | — | Phase 4 follows Phase 3 (same struct) |
| 5A → 5B | 5 | 5C | 5B depends on 5A's deletions landing |
| 5C | 5 | 5A, 5B | the guard is independent of the removal |
| 6A / 6B | 6 | each other | 6B audits what 6A and earlier phases produced |
| 7A / 7B | 7 | each other | all documentation, no code |
| 8A-1 / 8A-2 | 8 | each other | gates and downstream check |

## Risk register

| Risk | Phase | Mitigation |
|---|---|---|
| R-Q1 resolves to option 2 (remove `--cache-root`) | 1 | Phases 3, 4, and criteria 3/4/6 become a deletion; re-plan before starting Phase 3 rather than discarding finished work. |
| `hashing.rs` deletion breaks live run-local keying | 5 | S4 must separate live from dead surface; run `just test` in isolation right after that single task. |
| Changing the stable compose-identity encoding (`options.rs:2397–2402`) shifts a pinned hash | 5 | Search for tests pinning the encoding before editing; change deliberately and update the pin with a note. |
| S1 finds `Cache-Control` insufficient, forcing a `biscuit-file` change | 1 | Escalate immediately — it widens the fix to a second package area and must be an explicit scope decision, not an inline one. |
| A v1 manifest cannot deserialize into v2, blocking the `no-store` purge | 4 | Keep `url` as an optional legacy field during deserialization; test the v1 purge explicitly. |
| Windows/WSL2 filesystem behavior differs on removal or tree comparison | 2, 3, 8 | Load the `os` skill; use `Path` APIs only; treat `NotFound` as removal success; do not claim OS coverage that was not produced. |
| A later edit silently reattaches the persistent store | 5 | The 5C structural guard, with a self-test proving it catches a planted violation. |
