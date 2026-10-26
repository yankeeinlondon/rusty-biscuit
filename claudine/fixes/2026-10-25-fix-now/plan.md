---
kind: plan
created: 2026-10-26
total_phases: 3
phase: 1
agent: codex/gpt-6.1-sol
yolo: true
human_review: false
---

# Pipeline review: fix now — implementation plan

## Phase 1: Establish contracts and verification

Implement all seven fixes in `2026-10-25-fix-now`: reclaim runtime-loaded lifecycle catalogs and profiles, stop discovery at the first usable profile, move both ElevenLabs publications off executor workers, bound refined-type validators, remove the preliminary named-type YAML parse and replaced-body clone, and load only the blacklist during runtime checks.

Success means obsolete profile revisions release their allocations when their last owners drop; ongoing analyses retain valid old data; discovery avoids later content reads; combined diagnostics discover one host with preserved panic isolation; both publications leave the async worker free; and the five remaining work reductions preserve their specified semantics. Artifact bytes and versions, schema-root precedence, diagnostic ordering/codes/ranges, validator behavior, atomic durability, and approval behavior remain compatible. The two intentional diagnostic changes are strict-loader numeric-admission wording and removal of runtime-only whitelist warnings.

Every specification acceptance item must have either direct regression evidence or the explicit inspection evidence it requests. All new tests are hermetic L1 tests, each under five seconds, using temporary files, fake transports, and in-process synchronization. No network, speech playback, terminal windows, or browser windows are needed. The implementation ends ready for review, with scoped checks and actual OS evidence recorded.

### Decisions

1. **Runtime ownership**
   - **Decision:** Use `Cow<'static, str>` for requirement names, equality targets, and declared host globals. Keep the constant static shape constructor, add an owned-name constructor, and use borrowed-static-or-`Arc` wrappers for catalogs and static-slice-or-shared-owned wrappers for ignored verbs, with `From` conversions. Requirements and policies become `Clone`, not `Copy`.
   - **Alternatives considered:** Make compiled catalogs owned and deep-copy them; retain static-only interfaces and runtime leaks.
   - **Why:** This adopts the specification's design while preserving short compiled registrations and reclaimable disk data. Registry accessors borrow from the registry; separately retained registries and policies retain their own data.
   - **Downsides:** Public struct literals, enum operands, copy assumptions, and accessor lifetimes require repository-wide consumer updates. `CatalogErrorReason::Mismatch` must also carry an owned-or-borrowed target so error construction cannot reintroduce a static requirement.

2. **Revision retention**
   - **Decision:** Keep one synchronized cache entry per existing discovery path identity, containing the catalog content hash, exact artifact state, and a profile or failure reason. Serialize lookup, build, and replacement under the cache mutex initially; recover poisoned locks as today. A failure retains no catalog or registry. Evict observed unreadable, missing, invalid, or no-longer-host candidates before skipping them.
   - **Alternatives considered:** Revision-keyed retention; a capacity-tuned LRU; building concurrently outside the lock with duplicate builds.
   - **Why:** Latest-per-path is settled by the spec. Keeping construction under the lock matches the existing synchronization, avoids duplicate unchanged builds, and makes replacement unambiguous. Use `biscuit-hash` content identity, never file timestamps or lengths.
   - **Downsides:** Different paths can briefly serialize profile construction. Distinct unvisited paths can remain until process exit; undo can rebuild. Installation order defines latest, without a cross-file or cross-request transaction.

3. **Observed catalog bytes**
   - **Decision:** Extract a shared host-catalog text loader receiving path, supplied text, and the existing file-resolution context. Both `from_host_file` and lifecycle discovery use it; discovery hashes and parses the same text. Preserve import resolution and path identity without new canonicalization.
   - **Alternatives considered:** Reread after hashing; snapshot the filesystem or track imported-file revisions.
   - **Why:** This closes the specified authentication race with the smallest shared implementation.
   - **Downsides:** Catalog and artifact reads remain separate observations. Concurrent edits may yield a temporary stale-artifact warning; unchanged imports remain governed by existing behavior.

4. **Lazy diagnostics discovery**
   - **Decision:** Change Darkmatter discovery to a lazy iterator and adapt DMLS in the same implementation task. Preserve root order, sorted filenames, and shadowing before candidate usability is known. The diagnostics dispatcher discovers once inside a dedicated `catch_unwind`, then supplies the same borrowed host to frontmatter and DSL checks while retaining each provider's existing panic boundary.
   - **Alternatives considered:** An eager vector; a first-success-only library API; a `DocumentContext` or session cache.
   - **Why:** The spec requires lazy discovery and a common host per pass, without broader caching. Standalone helper entrypoints may discover independently, but combined-pass helpers must use the supplied host throughout.
   - **Downsides:** Provider inputs change. Discovery panic must explicitly skip both affected providers, log with existing provider-panic conventions, and allow all other providers to continue. Directory enumeration/sorting in the current root can remain eager.

5. **Whole blocking publications**
   - **Decision:** Await one `spawn_blocking` closure per ElevenLabs audio or default-voice-record publication. Audio moves its buffer and destination path, retaining length separately for logging. Record publication moves the `Snapshot`, including path selection, serialization, and atomic write. Add Tokio's production `rt` feature.
   - **Alternatives considered:** Multiple async filesystem calls; detached writes; inline synchronous writes.
   - **Why:** The existing atomic writer already owns staging cleanup and durability. Audio write/join failures remain `TempFileError`; record write/join failures only warn. Existing record skip conditions remain intact.
   - **Downsides:** Pool scheduling has overhead, and started or queued jobs can finish after their awaiting caller is dropped. Cancellation is not rollback; cleanup after process death is not promised.

6. **Shared validator policy**
   - **Decision:** Replace the private fragment `HashMap` with a module-local `LazyLock<ValidatorCache>` using `structural_validator_for` and its existing defaults. Retain the validator `Arc` inside each assertion.
   - **Alternatives considered:** Another eviction policy; no cache; unlimited memoization.
   - **Why:** Existing modules already use the same bounded cache and validator builder, including custom formats and syntax-only file checks.
   - **Downsides:** Evicted fragments can be compiled again; live assertions can retain validators outside the cache bound.

7. **Strict import authority**
   - **Decision:** Remove only `named_types_from_text`'s preliminary deserialization and stale explanation. The existing standalone classifier and admitted-YAML loader are authoritative; numeric-admission errors intentionally use their wording.
   - **Alternatives considered:** A parse cache; a second parser; preserving the redundant preliminary parse for its errors.
   - **Why:** The classifier already reports malformed YAML as a located `SchemaDocument`; removing the extra parse preserves the remaining import categories.
   - **Downsides:** Numeric error text changes. This does not promise that all internal numeric-admission passes become one parser pass.

8. **Explicit definition projection**
   - **Decision:** Construct projected definitions with the projected body and clones of `origin` and `json_key`, with no struct-update clone.
   - **Alternatives considered:** Clone the entire definition; change definition storage or identity.
   - **Why:** This removes the specified redundant allocation and makes future fields produce a compiler reminder.
   - **Downsides:** A new definition field requires an explicit edit here. No new behavioral test is justified; existing identity/lowering tests plus inspection prove this change.

9. **Fresh blacklist loading**
   - **Decision:** Extract the existing per-file ruleset load closure into a shared private helper. `user_blacklist` uses `policy_paths_at` with the captured root and loads only its blacklist; `SessionPolicy::load` continues loading both files and denying an underivable root.
   - **Alternatives considered:** Duplicate path derivation; rediscover home/repository roots; cache policy rules; make approval failures permissive.
   - **Why:** The existing roots and per-file fallback semantics already implement the contract. Every dispatch must observe current blacklist contents and retain built-in checks.
   - **Downsides:** Blacklist I/O remains per dispatch. Missing/unreadable runtime user rules still fall back to an empty user set as specified.

10. **Direct resource evidence**
    - **Decision:** Use production-path private observers/injection for cache builds, reads, discovery calls, bounded assertion construction, and publication writers; weak references prove successful-object reclamation. Keep injection scoped to each test or instance rather than introducing a public testing API or process-global races. No spike is scheduled.
    - **Alternatives considered:** Timing benchmarks; warning absence; permission tricks; a cross-host performance study.
    - **Why:** The spec already identifies the wasted work and prescribes deterministic evidence. Invalid profiles have no successful object to weak-reference. A synchronous publication watchdog must expire independently of Tokio's current-thread worker.
    - **Downsides:** Small internal seams must be maintained and shown to exercise production behavior. Behavior-preserving parse/clone changes need inspection rather than a forced pre-fix test failure.

11. **Prerequisite and scope**
    - **Decision:** Treat `2026-10-26-dmls-frontmatter-typing-allocation` as a technical predecessor: verify its shared frontmatter typing interfaces before changing DMLS, and preserve those interfaces and allocation reductions. If its required changes are absent, implement its existing settled plan as prerequisite work before this plan's DMLS integration; do not skip discovery sharing or defer it. Keep host-function catalog ownership and other excluded findings outside this fix.
    - **Alternatives considered:** Ignore the dependency; add another frontmatter typing cache; defer affected acceptance items.
    - **Why:** The spec explicitly declares this dependency, and its active plan establishes the shared typing surface that these provider edits must compose with. Its empty implementation log is not evidence of completion.
    - **Downsides:** Missing prerequisite implementation increases the work before DMLS integration. It does not require a new author decision or authorize changing the predecessor's design.

### Input robustness contract

This work touches file readers: host-catalog YAML, lifecycle-artifact JSON, named-import YAML, and shell-policy rules. Preserve their existing grammars and fail/fallback semantics. For catalog/schema/artifact inputs, every invalid shape produces a located public error or unusable-profile reason, runs nothing, and publishes nothing. Discovery skips invalid catalogs while ordinary catalog diagnostics remain responsible for their errors; an invalid artifact yields incomplete-analysis diagnostics only when no usable profile wins. Policy loading has the explicit per-line/per-file exceptions below.

Test each field validator once for absent, explicit null, wrong whole-field type, and empty value; collections additionally receive mixed valid/invalid elements and all-invalid elements. Keep absent and null distinct. Reuse existing coverage where it already proves the public outcome, and add one valid control plus one invalid wiring case per other field sharing that validator. Build mutations one at a time from existing valid host fixtures and committed artifacts; assert through public catalog/profile/resolver/approval results, not private parser values.

| Reader / validator group | Load-bearing fields and exceptions to default rejection |
| --- | --- |
| Host-catalog YAML | `kind`, `$schema`, and each declared global definition determine admission and shape. `description` absent means no description; null/blank are errors. `kind` and `$schema` cannot be empty. Global definitions preserve existing nonempty-object/open-object requirements, equality, reserved-name, import, and unknown-key checks. Dynamic discovery must not silently discard nonstring declaration keys. |
| Artifact required scalars and objects | `format_version`, `catalog_hash`, `host`, `global_catalog`; nested event, view, declaration, outcome, and unavailable-reason fields. Version/hash/registration checks remain authoritative. `global_catalog: false` is valid and does not register a catalog. No absent-field defaults. String validators retain their current empty-string behavior; semantic name/scope/code checks still run where applicable. |
| Artifact required collections | `events`, event `actions`, view `declarations`, `host_globals`, `effects`, `outcome_pair.downgrades`, and `unknown_verbs.ignore_unprovided`. Empty host globals/effects/ignored verbs/actions/declarations are valid where registry invariants allow them; empty events fail the required-core-event check. Empty downgrades are valid for a valid pair; each downgrade element must contain exactly two strings. Mixed and wholly invalid elements never disappear silently. |
| Artifact nullable fields | `outcome_pair` and event `core`, `position`, `role` require explicit null for none; absence remains an error. Core events require null position/role, while host events require valid position/role and null core. |
| Artifact tagged values | `unknown_verbs` is `reject`, `ignore`, or an exact object containing the string list `ignore_unprovided`. Declaration `availability` is `available`, `execution-dependent`, or the existing unavailable object. Reason `parameters` is an object whose values preserve arbitrary JSON, including null/empty values. Empty parameters is valid. Unknown tags/fields remain rejected. |
| Named-import YAML | Use the existing admitted-YAML/classifier validators for schema envelopes, types, and definitions; do not add a parallel validation layer. Valid ordinary YAML without a schema remains `AmbiguousReferenced`; malformed YAML is a located `SchemaDocument` starting `not valid YAML`; unsupported envelopes, root unions, whole-file references, and numeric rejection retain their specified outcomes. |
| Shell-policy line reader | Existing rules and line grammar are unchanged. Blank/comment lines and empty files retain their existing meaning; malformed lines warn and are skipped while valid lines still apply. No/underivable root or missing/unreadable blacklist yields empty user rules at runtime, with built-ins still enforced. Full approval still denies underivable roots. No JSON-style field matrix applies to this text grammar. |

Document-shape coverage is once per actual loader: catalog YAML's `serde_yaml_ng` branch (both `.yaml`/`.yml`), named-import admitted YAML, and artifact `parse_strict` JSON. Exercise duplicate keys, trailing/invalid content, YAML implicit null and a second document, and strict JSON rejection of comments/trailing commas. Reuse classifier-owned format coverage without multiplying it by each importing field. The extracted host text helper must run the same strict catalog validation as the file loader even if declaration enumeration uses a preliminary YAML value.

Retain or extend existing CLI wiring controls and one or two invalid-input cases per exposed loader, using the area's fixture command builder and asserting rendered file/field location and no launched work. Do not spawn a CLI per matrix cell or invent a new CLI for editor-only artifact reads; use DMLS's normal combined public diagnostic result for that wiring. Before closing the matrix, search affected reader paths for load-bearing `#[serde(default)]`, conflating `Option<T>`, `filter_map(..as_str())`, `unwrap_or_default()`, and `.ok()`. Classify permissive discovery probes and intentional policy fallbacks separately from authoritative validation. Repair silent authoritative acceptance without changing the declared format. A finding requires reviewing every shape of its validator and every field wired to it in the same pass.

### Wave 1: Contract preparation

- [ ] **P1.1 Contract inventory**
  - Verify the predecessor's implementation status from code and evidence, then satisfy Decision 11 before dependent DMLS edits. Preserve unrelated existing worktree changes.
  - Search all consumers of `GlobalRequirement`, `CatalogErrorReason::Mismatch`, registry catalog/global accessors, `UnknownVerbPolicy`, `HostLifecycleProfile::from_artifact`, and `host_lifecycle_profiles`, including tests, compiled hosts, examples, and `darkmatter/lib/benches/pipeline_work_support/mod.rs`.
  - Map existing fixtures and tests to spec acceptance items 1–10 and the reader groups above; identify only missing coverage. Record the implementation task/file owners before parallel work starts.
  - **Checkpoint:** All decisions above are settled, technical prerequisites are accounted for, and every acceptance item has a named implementation task below. No benchmark or exploratory spike is required.

## Phase 2: Implement the seven fixes

Prerequisite: Phase 1 is complete. Wave numbers are global and unique. Within a wave, tasks may run concurrently in separate subagents with disjoint file ownership. Cap each wave at three workers; every delegated brief must repeat that the entire session tree is non-interactive. Each worker owns its slice's regression tests and relevant docs/comments, uses scoped tests while iterating, and reports evidence to the integrating agent. Run the full area suites once after integration in Phase 3.

### Wave 2: Ownership and runtime boundaries

- [ ] **P2.1 Profile discovery**
  - **Ownership:** Darkmatter `markdown/schemas/catalog.rs`, lifecycle `registry.rs`, `parse.rs`, `host_profile.rs`, their tests; affected compiled registrations/copy consumers in Darkmatter and Claudine; DMLS `overlay/event_views.rs`, `providers/mod.rs`, `providers/dsl.rs`, `diagnostics/frontmatter.rs`, and diagnostics helper plumbing. This task owns both spec items 1 and 2, including their shared public API migration.
  - Implement the ownership wrappers/Cow fields and error payload migration from Decision 1. Preserve constant registrations, verb matching, registration validation, deterministic serialization, and caller-owned registry/policy lifetimes. Update every inventory consumer, including benchmark support; remove runtime leaks and stale lifetime comments on these paths.
  - Implement observed-text loading, synchronized latest-per-path caching, unchanged-revision failure reuse, and lazy candidate iteration. Record shadowing before attempting a candidate. Prune stale success only on observed invalidation, including disappeared cached candidates in visited roots; use visited-directory listings to recognize disappearance without opening content beyond the winner. Never sweep unvisited roots or later content solely for pruning.
  - Reuse one discovered host in the normal combined diagnostics pass. Add a borrowed-host diagnostics entrypoint/default provider hook as needed while retaining existing standalone entrypoints. Route all downstream frontmatter/DSL lifecycle checks through that supplied host. Discovery panic logs and suppresses both affected providers, with other provider results and per-provider panic boundaries intact.
  - **Ownership checks:** Replace artifact and catalog independently with valid revisions; prove old profile/registry/catalog weak references expire after all holders drop. Separately retain a profile, a registry, and a cloned owned ignore policy across replacement; verify old semantics remain usable until their final owners drop. Exercise owned globals/verbs in parsing and artifact serialization.
  - **Cache checks:** Absent, unreadable, stale, and invalid artifacts keep their reasons, do not rebuild unchanged revisions, and retain no failed-build catalog/registry. Repair is observed next request. Observed removed/unreadable/invalid/non-host catalogs release cached success. Use build counters/cache inspection for failures and an injected read error for unreadability.
  - **Discovery checks:** Production-path read counters prove zero later catalog/artifact reads after a winner. Cover filename sorting, root order, earlier unusable filename shadowing, all failure reasons in order, one discovery for combined diagnostics, unchanged frontmatter/DSL results, repair clearing warnings, shared-discovery panic, and unaffected ordinary catalog diagnostics/other providers.
  - **Checkpoint:** Artifact comparisons remain byte-for-byte equal; a supplied-text test proves no second catalog read; focused ownership, registry, artifact, and combined DMLS tests pass. Runtime lifecycle load paths contain no `Box::leak`; separately scoped host-function leaks are not rewritten.

- [ ] **P2.2 Blocking publication**
  - **Ownership:** `biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs`, `elevenlabs/snapshot.rs` only if needed for a private seam, `lib/Cargo.toml`, and existing ElevenLabs/atomic-cache tests. Implements spec item 3 independently of P2.1/P2.3.
  - Move both whole publications into awaited blocking closures with owned inputs and unchanged skip conditions. Keep network calls, playback, and `cache_exists` outside. Map audio I/O and join errors to `TtsError::TempFileError`; warn for record I/O/join errors without returning them. Preserve portable `tempfile`, sync, rename, names, expiry, and last-writer semantics.
  - On a current-thread runtime, inject a synchronous writer into each real publication path. It signals entry and waits for another task on the same runtime to release it, with a short independent `recv_timeout` watchdog so an inline-write regression fails and tears down within five seconds. Include default-record publication on an audio-cache hit.
  - Exercise successful awaited publication, ordinary write errors, closure panic/join errors, record skip cases, existing staging cleanup, and dropping a caller after write entry followed by a complete owned publication. Join or release every worker on failure; no fixed sleeps or Tokio-only deadlock timeouts. Preserve isolation of `TMPDIR`, `TMP`, and `TEMP` in ElevenLabs fixtures.
  - **Checkpoint:** Both async-progress tests and fatal/nonfatal error tests pass using fake transport and no playback. A production-feature compile confirms `rt` is enabled independently of dev-dependency feature unification.

- [ ] **P2.3 Blacklist loading**
  - **Ownership:** `claudine/lib/src/harness/shell.rs` and shell-policy/runtime dispatch tests; coordinate any nearby registration fixtures with P2.1. Implements spec item 7.
  - Extract the per-file helper and switch runtime loading to captured-root `policy_paths_at` plus blacklist only. Keep full approval's two-file loading and underivable-root denial; retain all built-in blacklist enforcement.
  - Through an actual runtime dispatch check and private file-read observer, prove blacklist reads and zero whitelist opens, including a malformed whitelist. Edit the blacklist between dispatches and prove immediate enforcement.
  - Cover no root, underivable root, missing/unreadable blacklist, malformed lines with valid rules, retained warnings, built-ins, and full approval reading both files. Inject read failures rather than depending on Unix permissions; do not rediscover home, working directory, or repository.
  - **Checkpoint:** Scoped shell/runtime tests pass and read observations demonstrate the avoided work, rather than merely missing whitelist warnings.

### Wave 3: Independent schema reductions

Prerequisite: Wave 2 is integrated. These tasks have no mutual dependency; their files are disjoint. P2.1 also establishes the final public ownership APIs before these tests compile against them.

- [ ] **P2.4 Bounded assertions**
  - **Ownership:** `darkmatter/lib/src/markdown/compose/expression/call_binding/translate.rs` and its assertion tests. Implements spec item 4.
  - Replace only the unbounded validator memo with default `LazyLock<ValidatorCache>` structural lookup. Preserve the assertion's retained `Arc`, custom formats, syntax-only file behavior, and `TranslateError` category/context. Correct the process-lifetime doc comment.
  - Allow a capacity-one cache through the same private assertion-building path production uses. Build A then B, prove eviction of A, then prove the live A still accepts/rejects correctly. A generic-cache-only test is insufficient.
  - **Checkpoint:** Eviction, custom format, syntax-only file, and build-error-context checks pass; the old fragment memo is absent.

- [ ] **P2.5 Named import parsing**
  - **Ownership:** `darkmatter/lib/src/markdown/schemas/resolve.rs` and named-import tests. Implements spec item 5.
  - Remove the preliminary YAML deserialization and its stale comment; retain strict classifier/numeric admission. Through the public resolver, import a file containing `kind: schema` and `types: [` and assert the error category, imported path, and `not valid YAML` prefix.
  - Cover numeric-admission wording, ordinary non-schema YAML, unsupported envelopes, root unions, and whole-file references. Apply the loader robustness contract using existing fixtures and shared validators.
  - **Checkpoint:** Focused resolver tests, DMLS `named_type_references.rs`, and CLI `schema_triggers.rs` regressions pass. Inspection confirms removal of the preliminary parse; matching error strings alone is not parse-count evidence.

- [ ] **P2.6 Definition projection**
  - **Ownership:** `darkmatter/lib/src/markdown/schemas/simplified/definitions.rs`. Implements spec item 6.
  - Construct each definition explicitly from its projected body and metadata clones, preserving `DefKey`, insertion/allocation order, origins, JSON names, and `by_json_key`.
  - Inspect the projection and run the existing recursive-definition/schema-lowering tests; do not add a behavior test that cannot distinguish this optimization.
  - **Checkpoint:** The replaced body is not cloned and existing identity/lowering assertions pass.

## Phase 3: Document, validate, and hand off

Prerequisite: All Phase 2 tasks are integrated, with focused checkpoints passing. Do not widen CI scope, add matrix cells, or introduce performance studies for this fix.

### Wave 4: Documentation and contract audit

These two tasks may run concurrently: one writes documentation, the other reads production/test code and produces the acceptance evidence ledger. Any repair found by the audit returns to its owning Phase 2 task before final gates.

- [ ] **P3.1 Current documentation**
  - Update `darkmatter/docs/topics/lifecycle/host-artifact.md` and `darkmatter/dmls/docs/diagnostics.md` with one retained revision per visited path, caller-held lifetimes, observed invalidation, first-usable lazy discovery, one host per diagnostics pass, panic isolation, and fresh discovery across requests. Include a compact lifetime/discovery Mermaid flow where it clarifies the behavior.
  - Update `biscuit-speaks/docs/tts-caching.md` with both awaited blocking publications, the fatal-audio/nonfatal-record split, and caller-drop behavior; update the existing Tokio entry in `biscuit-speaks/docs/dependencies.md` for production `rt`. No crate is added or removed.
  - Update `claudine/docs/topics/pre-flight-checks.md` with fresh blacklist-only dispatch reads versus full approval loading both policies. Correct any affected Darkmatter/Claudine skill lifetime statements and relevant README overview drift only where present.
  - Remove stale comments about runtime leaks, once-per-editor catalog loading, process-wide fragment retention, and classifier syntax handling. Topic docs describe current behavior without linking to a dated spec.
  - **Checkpoint:** Docs and comments match final behavior, preserve predecessor documentation, and make no process-lifetime ownership or cancellation-rollback claims.

- [ ] **P3.2 Acceptance audit**
  - Trace spec acceptance items 1–9 to named tests and inspection evidence from Phase 2. Audit the reader-validator groups and document loaders once each, preserving positive controls, absence/null distinctions, and the listed exceptions. Run the code-smell search and record intentional fallbacks.
  - Inspect all changed public-type consumers, including static registrations, `CatalogErrorReason::Mismatch`, artifact writers, test fixtures, examples, and benchmark support. Confirm exact artifact bytes, provider order/codes/ranges, panic boundaries, owned-policy lifetime, and no runtime lifecycle/catalog `Box::leak`.
  - Verify observers instrument actual production reads/builds/writes, capacity-one assertions use the production builder, tests are selected by L1 recipes, and async watchdogs release resources on regression. No success inference from warnings, permissions, cache-only tests, or behavioral parse/clone tests.
  - **Checkpoint:** Every acceptance item has evidence and no requested scope has been deferred; defects found here are fixed before proceeding.

### Wave 5: Integrated gates and closure

- [ ] **P3.3 Scoped validation**
  - Run `just test` and `just lint` from each of `darkmatter/`, `claudine/`, and `biscuit-speaks/` after integration. Darkmatter's recipes include DMLS and its CLI consumers. Use nextest through the recipes; never run `cargo test` suites or `cargo fmt`.
  - Compile affected declared examples and benchmarks with `cargo check --examples --benches` scoped to their actual packages/features, including Darkmatter's pipeline benchmark consumer. Verify the biscuit-speaks production-only feature graph includes `rt`, rather than relying on test runtime features.
  - Keep every new L1 case below five seconds. Use Test Toolkit fixtures/guards and portable failure injection. Read the `os` skill before any OS-specific/path-comparison edits or planning remote verification; use existing passing evidence when applicable and record macOS, Linux, native Windows, and WSL2 results separately. Never equate host-only evidence with all-platform verification.
  - **Checkpoint:** All required scoped gates pass, affected target consumers compile, acceptance item 10 is satisfied, and actual tested environments plus any unavailable evidence are explicit in the implementation log. Repair failures and rerun only checks invalidated by those repairs.

- [ ] **P3.4 Review handoff**
  - Record decisions, completed task evidence, test commands/results, inspected optimizations, intentional diagnostic changes, and any deviations in `implementation-log.md`. Check off completed plan tasks only after their checkpoints pass.
  - Verify the final diff contains only implementation, tests, and documentation needed for this scope and preserves preexisting changes. Report implementation complete, ready for review, with exact validation limits.
  - Leave the active fix directory in place. Do not run `just complete`, move it to `_completed`, commit, or push.
