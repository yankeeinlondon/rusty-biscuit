---
area: darkmatter
status: active
created: 2026-09-11
activated: 2026-09-16
activated_by: darkmatter/features/2026-09-09-more-context (plan Phase 1, Q3)
owner: Ken Snyder <ken@ken.net>
origin: darkmatter/features/2026-09-09-more-context/spec.md
reviewed: true
reviewed_by: codex/gpt-5.6-sol
reviewed_on: 2026-09-17
review_iterations: 3
packages:
    - darkmatter
    - darkmatter-cli
implemented: true
implemented_by: "claude/opus"
---

# Disable persistent compose cache until ContentPolicy exists

## Status and reader's note

The safety-critical part of this fix is implemented: production composition no
longer reads or writes persistent document snapshots, composed `::file`
children, or `::code` / `::toc-linking` operation results.

This review distinguishes two caches that the draft treated as one:

1. **semantic-result persistence**, which can replay output produced under an
   earlier composition context; and
2. **raw HTTP response persistence**, which stores downloaded bytes and is
   governed by HTTP validators and cache directives before Darkmatter composes
   those bytes again.

R18 unambiguously prohibits the first category until a content-freshness
policy exists. On 2026-09-17 Ken ruled (more-context spec **R36**, answering
Q1 below with option 1) that “nothing is cached persistently” means no
semantic-result artifact, and that raw HTTP response caching is retained as a
transport-cache exception. That exception is not permission to restore any
semantic-result persistence.

The review also found two gaps in the implemented boundary:

- a local-only invocation with a configured cache root currently creates the
  store's directory structure even though it has no persistable artifact; and
- `Cache-Control: no-store` is currently lowered to a zero-second TTL and the
  response is still written to disk. [RFC 9111 §5.2.2.5](https://www.rfc-editor.org/rfc/rfc9111.html#section-5.2.2.5)
  requires a cache not to store such a response and to make a best-effort
  removal of already stored data.

Both gaps are in scope for this fix now that Q1 retains raw HTTP response
caching. Both are now closed; see "Implementation status" below.

## Governing ruling

**Ruling (Ken, 2026-09-11, R18 of the more-context feature):** local file
transclusion does not need caching. Persistent caching is for content that
arrives through an expensive or agentic operation, such as summarizing a
website. Invalidating such content requires a `ContentPolicy` that defines
when the cached content becomes stale. Until that policy exists, composed or
derived content is not persistable.

This ruling does not affect request-scoped memoization. The run-local cache may
continue to deduplicate file loads, composition, heading extraction, shell
commands that permit caching, and other work within one invocation. It must
not acquire a file-backed store through any production path.

## Existing behavior and terminology

Before this fix, `ComposeOptions::with_cache_root` and
`md compose --cache-root <dir>` enabled one file store containing four artifact
classes:

- raw document snapshots;
- composed `::file` children;
- derived operation results (`::code` and `::toc-linking`); and
- raw HTTP(S) response bodies.

The first three classes are semantic-result artifacts. Their identities
included source, options, dependency-closure, and selected runtime-context
hashes, but identity is not freshness policy: a complete key can prove which
inputs produced an entry without deciding whether an expensive or agentic
result remains acceptable to reuse.

The fourth class is a transport artifact. It is keyed by URL and currently
uses `RemoteReadConfig` plus response validators, response cache directives,
and the `--remote-ttl`, `--remote-refresh`, and `--remote-freshness` controls.
A hit returns raw bytes to the normal composition pipeline; it never returns a
previously composed document.

## Required behavior

### Semantic-result persistence

- No production entry point may attach a persistent store to `RunLocalCache`.
- Document snapshots, composed children, and operation results must never be
  read from or written to disk, regardless of `CacheAccessMode`,
  `cache_namespace`, a configured cache root, or whether the caller enters
  through composition, preflight, or reference-graph APIs. (The former
  `CacheFreshnessMode` control was removed under Q2.)
- A warm cache must not replay composed output, warnings, runtime context,
  shell/probe output, or any parent result derived from those values.
- Local file transclusion remains run-local only. Adding `ContentPolicy` later
  does not reverse this part of R18 without a new ruling.
- Existing on-disk snapshot, composed, and operation entries are ignored.
  This fix must not delete a user's cache tree automatically.

### Cache-root side effects

Merely configuring a cache root must not mutate the filesystem. Store
directories are created lazily only immediately before an eligible persistent
artifact is written. Consequently, a local-only composition with
`--cache-root` leaves a nonexistent root nonexistent and an existing root
unchanged.

This is stronger and more observable than “no local manifest was written.” It
is the contract meant by “a local-only compose performs no persistent write.”

### Raw HTTP response persistence

Under Q1 (option 1, R36), raw response bodies are the sole production artifact
eligible for persistent storage. The transport cache must satisfy all of the
following:

- Network authorization remains independent of caching. A cached body cannot
  bypass the exact-host `FetchPolicy`, and `--cache-root` never grants network
  access.
- `Cache-Control: no-cache` may be stored but must be successfully revalidated
  before reuse, including in optimistic mode.
- `Cache-Control: no-store` must never be persisted or reused. On encountering
  an old entry whose manifest records `no-store`, Darkmatter ignores it and
  makes a best-effort removal of both its manifest and body. A cleanup failure
  is non-fatal but is reported as a cache warning.
- An explicit TTL may control freshness where HTTP permits storage, but it
  must not override `no-store`. The specification must not describe a zero TTL
  as equivalent to `no-store`.
- A manifest must not persist URL user information or query text in cleartext.
  The current request supplies the fetch URL; the manifest needs only a stable
  identity hash and a redacted diagnostic form. Cached bodies remain
  potentially sensitive, so persistence stays explicit opt-in rather than a
  default platform-cache side effect.
- Strict, fallback, and optimistic behavior applies only after the entry is
  found eligible for storage and reuse. Serving stale data on failure is never
  an implicit consequence of enabling a cache root.

### Public API and documentation

- CLI help, library docs, and `docs/topics/caching.md` must use the same
  semantic-result versus transport-artifact terminology.
- `--cache-root` and `ComposeOptions::with_cache_root` must state exactly which
  artifact classes they can persist (under Q1: raw remote response bodies
  only).
- Public local-persistence controls that have no production effect must not be
  presented as active behavior. Under Q2 they were removed.
- Cache statistics must not report creation of empty directories as a write.
  Remote transport-cache events and run-local semantic-cache events must
  remain distinguishable in diagnostics.

## ContentPolicy design constraints

This fix does not implement `ContentPolicy`; it records the minimum contract a
later feature must satisfy before expensive or agentic semantic results become
persistable.

`ContentPolicy` governs **content freshness**, not network authorization,
artifact identity, filesystem placement, or permission to persist sensitive
data. Presence of a policy makes an otherwise eligible producer *capable* of
cross-run reuse; it does not itself enable a cache root or override a
producer's non-persistable classification.

The eventual policy must provide:

- a versioned, serializable policy identity included in the artifact manifest;
- one or more explicit expiry predicates, with “any predicate expires the
  content” semantics and no empty-policy meaning;
- an evaluator for every predicate and the captured evidence needed to
  evaluate it without rediscovering ambient request state;
- an explicit stale action: recompute, serve with a surfaced warning, or fail;
- a generation timestamp and any source/model/tool versions required by the
  predicates, stored as artifact evidence rather than mutable policy fields;
- deterministic evaluation under an injected clock for tests; and
- fail-closed behavior: an absent, unknown, malformed, or newer policy version
  makes the entry a cache miss, never an optimistic hit.

At minimum the shared vocabulary needs a duration/max-age predicate, explicit
invalidation, source-content change, software/library version change, and
model retirement. Calendar “months” and “years” must either define exact
calendar arithmetic and timezone or be replaced by unambiguous durations.

A future persistent producer must key an artifact on the normalized producer
kind and version, inputs, dependency closure, and policy identity. Volatile
values must not be made safe by maintaining a blacklist of fields: an effect
or observation that is not represented in the producer's captured identity
and freshness evidence makes that result ineligible for persistence.

Shell execution, ICMP probes, current-value lookups, and other effects are not
persistable merely because their containing document has a policy. A future
producer must opt in at its own boundary and prove that its complete inputs and
freshness evidence are represented.

## Existing ContentPolicy contract elsewhere in the monorepo

`research::metadata` already exports draft `ContentPolicy` and
`ContentExpiry` types. They contain research-domain expiry reasons but do not
yet define evaluation, stale behavior, policy versioning, an injected clock,
or the evidence stored with an artifact. Darkmatter must not introduce a
second unqualified `ContentPolicy` with silently different semantics.

Darkmatter also cannot solve this by depending on `research`, because
`research` already depends on `darkmatter`. Q3 records the ownership decision.
Whichever option is selected must include migration or re-export guidance for
the existing research type and for Claudine's documented `ContentPolicy`
concept.

### Ownership and migration guidance (Q3 outcome)

A new dependency-light shared library owns the `ContentPolicy` vocabulary. The
feature that builds `ContentPolicy` creates it and chooses its name; this fix
creates nothing.

- **Shared library scope.** Only the serializable vocabulary: the versioned
  policy identity, the expiry predicates, the stale action, and the evidence
  record shape. No evaluators and no domain dependencies (Serde-level only).
  Domain-typed payloads, such as research's `LibraryVariant`, which uses
  `sniff::package::LanguagePackageManager`, need a neutral representation or
  stay in the consumer. Each consumer supplies its own evaluators.
- **`research::metadata`.** `ContentPolicy(Vec<ContentExpiry>)` becomes a
  re-export or a thin wrapper of the shared type. Its variants map onto the
  minimum vocabulary: `Days` → a duration predicate; `ContentHashConflict` →
  source-content change; `Flagged` → explicit invalidation; `SoftwareUpdate`,
  `MajorLibraryUpdate`, and `MinorLibraryUpdate` → software/library version
  change; `ModelArchived` → model retirement. Two semantic changes are
  required, not optional:
  - `Months` and `Years` must define exact calendar arithmetic and timezone, or
    be replaced by unambiguous durations.
  - Research documents treat an empty list as "evergreen". The shared contract
    gives an empty policy no meaning, so evergreen content needs an explicit
    representation (or no policy at all, which makes it non-persistable).
  - Existing research frontmatter `policy` values need a versioned
    deserializer or a one-time migration.
- **Claudine.** The model-catalog concept `{ generated_at, max_age,
  on_stale: warn }` (`claudine/gen/src/artifact.rs` `MAX_AGE_DAYS`,
  `claudine/lib/src/model_catalog/families.rs`) maps to a duration predicate,
  the "serve with a surfaced warning" stale action, and `generated_at` stored
  as artifact evidence. Claudine adopts the shared type when it next changes
  that artifact; nothing changes now.
- **Darkmatter.** Darkmatter consumes the shared type, supplies compose-side
  evaluators, and must never define its own unqualified `ContentPolicy`.

## Verification and acceptance criteria

All tests are L1 unless a real terminal or browser is genuinely required;
none is expected for this fix. Run them through the Darkmatter `just test` and
`just lint` recipes. The behavior must be portable across macOS, Linux, native
Windows, and WSL2; assertions inspect paths through `Path` APIs and must not
assume `/` separators.

1. A local-only library composition with a nonexistent cache root neither
   creates that root nor reads or writes a persistent artifact. Repeat for an
   existing sentinel-filled directory and prove its tree is unchanged.
2. Cold and warm runs prove that a `::file` child containing a changing runtime
   value is recomposed under every remaining cache control (`CacheAccessMode`
   × configured cache root; the legacy `CacheFreshnessMode` was deleted under
   Q2); `::code`,
   `::toc-linking`, snapshots, and reference-graph composition likewise
   produce no persistent read or write.
3. An end-to-end `md compose FILE --cache-root DIR` test proves raw remote bytes
   may hit the transport cache while local content is recomposed (Q1 keeps
   that behavior).
4. Remote-cache tests prove `no-cache` always revalidates and `no-store` never
   writes or reuses bytes under every remote freshness mode and under an
   explicit TTL override. A pre-existing `no-store` entry is ignored and
   removal is attempted.
5. A denied host fails before either a cache read or a network request. A cache
   root alone never authorizes the host.
6. Cache manifests contain no cleartext URL credentials or query values.
7. CLI help, public builders, the Darkmatter caching topic, the Darkmatter
   skill, and the originating more-context specification agree with the
   resolved Q1/Q2 boundary.
8. No production call site can attach the retained semantic-result store to
   `RunLocalCache`; a source-level guard or equivalent structural test protects
   this invariant if the machinery remains after Q2.

The fix is implementation-complete and ready for review only after Q1-Q3 are
ruled, their selected outcomes are folded into this document, and all
applicable criteria above pass. The author, not the implementing agent, moves
the fix to `_completed` after review closes.

## Implementation status (2026-09-18)

All implementation phases through documentation are complete; the fix awaits
Phase 8 verification and review. Implemented:

- **Semantic-result persistence is gone, not disabled (Q2).** `RunLocalCache`
  is memory-only and cannot name `FileStore` or `RemoteFetchRuntime`. The
  persistent read/write paths, snapshot/composed/operation manifests,
  dependency-closure hashing, `CacheFreshnessMode`,
  `ComposeOptions::with_cache_freshness_mode`, and the four persistent
  `CacheStats` counters were deleted. `CacheAccessMode` remains and governs
  run-local reuse only. `lib/tests/semantic_results_never_persist.rs` is the
  structural guard (acceptance criterion 8).
- **Lazy cache root.** `FileStore::at` records the path only and directories
  are created inside the write that needs them; the platform-cache fallback was
  deleted. A local-only compose leaves a missing root missing and an existing
  root byte-for-byte unchanged.
- **RFC 9111 transport cache.** `Cache-Control` is parsed as a whole directive
  list (case-insensitive, quoted `max-age`, `no-cache="field"`). `no-store` is
  never written and an existing `no-store` entry is purged best-effort;
  `no-cache` revalidates before every reuse in every mode and is never served
  stale; a TTL override never outranks either. Cleanup and write failures are
  non-fatal `remote_cache` compose warnings (`dm.remote_cache.io_failure`).
- **Manifest privacy.** Manifests persist `redacted_url` (no userinfo or
  fragment, query shown as `?<redacted>`) plus the full-URL identity hash.
  Manifest `CACHE_VERSION` is `2`; the directory's `STORE_LAYOUT_VERSION`
  stays `1`, so legacy entries are misses left on disk, except legacy
  `no-store` entries, which are purged.
- **Host policy precedes any cache read**, and a cache root never authorizes a
  host.
- **Documentation (acceptance criterion 7).** `md compose --help`,
  `ComposeOptions` builder docs, `docs/topics/caching.md`,
  `docs/topics/remote-url-references.md`, `docs/cli/compose.md`, the
  `darkmatter` skill, and the more-context specification use the same
  semantic-result / transport-artifact vocabulary.
  `cli/tests/help.rs::test_compose_help_states_the_transport_cache_boundary`
  pins the help wording.

Every acceptance criterion maps to a named passing test; the table is in
`plan.md` (Phase 8, "Criterion → test mapping").

Known gaps:

- *Closed (review-1):* `biscuit_file`'s `fetch()` / `post()` now join every
  `Cache-Control` field line in wire order per RFC 9110 §5.3, so a `no-store`
  on a separate field line is honored and acceptance criterion 4 holds for
  split `Cache-Control` too
  (`split_line_no_store_is_never_stored_under_any_mode_or_ttl`;
  `biscuit-file` `fetch_integration`
  `{fetch,post}_combines_split_cache_control_lines_in_wire_order`).
- `redacted_url` keeps the URL path, which can itself carry a token (R-H).

### Derived rulings

The plan settled these questions, which the specification left open. Each is
the plan's stated default, adopted without a separate owner ruling.

- **R-A:** `CacheAccessMode` stays public, documented as run-local only;
  `CacheFreshnessMode` and `with_cache_freshness_mode` are removed.
- **R-B:** `with_cache_namespace` stays, documented as namespacing the remote
  transport-cache root.
- **R-C:** `FileStore::resolve_cache_root` takes a non-optional `&Path`; the
  `dirs::cache_dir()` platform-cache fallback is deleted.
- **R-D:** `CacheStats` loses `persistent_hits`, `persistent_writes`,
  `revalidations`, and `stale_hits`; `RemoteFetchStats` is the sole transport
  reporter.
- **R-E:** a `no-store` purge removes the manifest, then the blob,
  unconditionally, even though a content-addressed blob could be shared.
- **R-F:** cleanup and write failures accumulate in
  `RemoteFetchStats::cache_warnings` and surface as `ComposeWarning`s with
  stage `remote_cache` and code `dm.remote_cache.io_failure`; they are never
  fatal.
- **R-G:** `no-cache` outranks `Fallback`: a failed revalidation of a
  `no-cache` entry is an error, not a stale serve.
- **R-H:** the redacted diagnostic URL is `scheme://host[:port]/path`, with
  userinfo and fragment removed and any query replaced by `?<redacted>`.
- **R-I:** manifest `CACHE_VERSION` is bumped to `2`; a non-matching entry is a
  miss left on disk, except a `no-store` entry. The store directory version
  was decoupled (`STORE_LAYOUT_VERSION`, still `1`) so legacy entries stay
  findable for that purge.

## Questions (resolved)

All three questions are owner-ruled. Q2 and Q3 were implemented as adopted
defaults and confirmed by the owner afterward (see each outcome).

### Q1 — Does `--cache-root` continue to persist raw HTTP response bodies?

The original ruling says both that persistent caching is for expensive or
agentic content and that nothing is cached persistently until `ContentPolicy`
exists. Raw response bodies already have a distinct HTTP freshness mechanism,
but retaining them requires an explicit exception to the broad wording.

1. **Keep raw HTTP response persistence as a transport-cache exception
   (recommended).**
   - Pros: preserves offline/fallback behavior; retains conditional requests
     and bandwidth savings; never replays composed context; matches the
     already-implemented boundary.
   - Cons: requires the ruling to define “nothing” as “no semantic-result
     artifacts”; requires the standards and privacy fixes in this spec.
2. **Remove `--cache-root` and all persistent storage until the shared
   `ContentPolicy` exists.**
   - Pros: follows the literal broad reading of R18; gives the smallest
     possible persistence/security surface.
   - Cons: removes useful remote revalidation and offline fallback; later work
     would need to rebuild or restore the transport cache even though HTTP
     already supplies a freshness protocol.
3. **Implement the shared `ContentPolicy` now and place remote bodies under
   it.**
   - Pros: reaches the eventual architecture immediately; avoids a temporary
     exception.
   - Cons: substantially expands this safety fix and is blocked on Q3; risks
     conflating HTTP response freshness with semantic content validity.

**Recommendation:** option 1. HTTP response caching is a lower-level transport
optimization with validators and standardized cache directives. Keeping it
separate preserves useful behavior without permitting an earlier composed or
agentic result to bypass current composition. This recommendation is
conditional on completing every raw-cache requirement above.

**Ruling (Ken, 2026-09-17):** option 1, recorded as **R36** in the
more-context spec. The raw-cache requirements above remain in scope.

### Q2 — Retain or remove the dormant semantic-result persistence machinery?

The manifest, hashing, and runtime read/write paths remain compiled and
unit-tested, while only store attachment is test-only. This preserves prior
work but also leaves inactive public controls and a large correctness surface
that can drift before a policy is designed.

1. **Delete dormant semantic-result persistence and its local-only public
   controls now; retain only run-local caching and the remote store selected by
   Q1 (recommended).**
   - Pros: makes the forbidden state structurally unavailable; removes inert
     API and maintenance burden; lets the eventual policy drive a clean design.
   - Cons: discards implementation that may inform the later feature; the
     future implementation may repeat some manifest and closure-hash work.
2. **Keep the machinery crate-private and test-only behind a dedicated Cargo
   feature.**
   - Pros: preserves a prototype without presenting it as production behavior;
     excludes it from ordinary builds.
   - Cons: still carries code and tests for an unapproved design; feature-gated
     code is more likely to drift.
3. **Keep the current always-compiled, test-only attachment path.**
   - Pros: smallest immediate diff; maximizes reuse if the future policy
     matches the prototype.
   - Cons: largest long-term maintenance and accidental-reenablement risk;
     leaves public options whose advertised effect is false.

**Recommendation:** option 1. The codebase has no established users, and the
future `ContentPolicy` contract is not yet settled. Structural removal is safer
and cheaper than maintaining a speculative persistent-cache implementation.
Keep design history in the specification and Git rather than in inactive
production modules.

**Outcome: option 1 (adopted default, implemented).** The owner had not ruled
when implementation Phase 1 ran (2026-09-17), so the plan's stated default
applies; implementation Phase 5 deleted the machinery. The Phase 1 deletion inventory
(`plan.md`, "S4 — Q2 deletion inventory") measured roughly 1,900–2,100 lines,
all inside `darkmatter/lib`, with no consumer outside `darkmatter/`. Once
`CacheFreshnessMode` is removed, acceptance criterion 2's "every legacy
`CacheFreshnessMode`" dimension no longer exists; criterion 2 above is
restated against the surviving cache controls.

**Ruling (Ken, 2026-09-18):** option 1 confirmed; the deletion stands.

### Q3 — Which package owns the shared ContentPolicy vocabulary?

The existing research type proves the concept is cross-cutting, but dependency
direction prevents Darkmatter from importing it.

1. **Move the small serializable vocabulary into a dependency-light shared
   library and have Research, Darkmatter, and Claudine consume or re-export it
   (recommended).**
   - Pros: one semantic contract with neutral ownership; no dependency cycle;
     each consumer can supply domain-specific evaluators.
   - Cons: adds a package and migration work; the shared boundary must remain
     small to avoid becoming a catch-all.
2. **Make Darkmatter the owner and have Research re-export or wrap its type.**
   - Pros: no new package; dependency direction already permits Research to
     consume Darkmatter.
   - Cons: gives a Markdown composition library ownership of policy used by
     non-Markdown artifacts; risks coupling Research policy evolution to
     Darkmatter releases.
3. **Keep separate domain types with explicit names such as
   `ComposeContentPolicy` and `ResearchContentPolicy`.**
   - Pros: no shared-package work; each domain evolves independently.
   - Cons: duplicates core semantics and invites incompatible stale behavior;
     cross-domain agentic artifacts need adapters and cannot claim one
     monorepo-wide contract.

**Recommendation:** option 1. Freshness semantics are already shared across at
least three package areas, while evaluation remains domain-specific. A narrow
lower-level vocabulary avoids both a dependency cycle and duplicate meanings.

**Outcome: option 1 (adopted default).** The owner had not ruled when
implementation Phase 1 ran (2026-09-17), so the plan's stated default applies.
This fix only records the decision and the migration guidance in "Existing
ContentPolicy contract elsewhere in the monorepo"; it creates no package.

**Ruling (Ken, 2026-09-18):** option 1 confirmed. `research` is likely to be
abandoned or substantially reworked, so it must not own the vocabulary, and
its migration guidance above applies only if the package survives.

Origin: [more-context specification](../../features/_completed/2026-09-09-more-context/spec.md).
