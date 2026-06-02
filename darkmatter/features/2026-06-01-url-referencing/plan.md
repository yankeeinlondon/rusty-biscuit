---
phases: 6
created: 2026-06-02
start_phase: 1
---

# Execution Plan: URL Referencing

## Phase 1: biscuit-file URL Classification
*Dependency: None*

**Goal:** Teach `biscuit-file` to classify HTTP(S) descriptors and expose typed local-vs-remote resolution without changing existing local `resolve()` behavior.

- [ ] Audit current `FileReference` parsing, normalization, error types, and tests in `biscuit-file` to identify the smallest URL-aware extension points.
- [ ] Add a `Url` reference kind for descriptors with `http://` or `https://` schemes.
- [ ] Ensure URL normalization preserves scheme separators and does not apply local path-only normalization to URL authority or scheme syntax.
- [ ] Add `url` as a `biscuit-file` dependency if it is not already present.
- [ ] Add `Resolved::Local(PathBuf)` and `Resolved::Remote(url::Url)` as the typed target model.
- [ ] Implement `FileReference::resolve_target() -> Result<Resolved, FileReferenceError>` for all existing local reference kinds and the new URL kind.
- [ ] Add `FileReferenceError::RemoteNotLocal` and make existing `resolve() -> PathBuf` return it for URL references.
- [ ] Preserve existing `resolve()` behavior and public tests for all local reference kinds.
- [ ] Add classification and resolution tests for valid `http` and `https` URLs, invalid URL shapes, unsupported schemes, and local paths containing URL-like text.
- [ ] **Validation Checkpoint:** `cargo test -p biscuit-file` passes and URL references fail through `resolve()` with `RemoteNotLocal` while succeeding through `resolve_target()`.

## Phase 2: biscuit-file Fetch Primitive
*Dependency: Phase 1*

**Goal:** Add a feature-gated, policy-enforcing HTTP fetch primitive that all remote read/write consumers can share.

- [ ] Add an off-by-default `fetch` feature to `biscuit-file` and gate all HTTP client dependencies behind it.
- [ ] Add `FetchPolicy` with deny-all default host policy and `HostPattern` matching for exact hosts and any wildcard forms accepted by the implementation.
- [ ] Add `Conditional` request metadata for `If-None-Match` and `If-Modified-Since`.
- [ ] Add `FetchResponse` with status, body, `etag`, `last_modified`, and `cache_control` fields.
- [ ] Add `FetchError` variants for policy denial, unsupported scheme, request failure, invalid response metadata, and response/body read failures.
- [ ] Implement async `fetch(client, url, policy, conditional)` using a caller-supplied shared `reqwest::Client`.
- [ ] Enforce allowed-host policy inside `fetch()` before any network request is attempted.
- [ ] Implement `fetch_blocking()` as a sync convenience wrapper without requiring callers to manage a runtime.
- [ ] Add unit tests for host allowlist matching, deny-all default, unsupported schemes, and conditional header construction.
- [ ] Add HTTP tests with a local mock server for `200`, `304`, error status handling, response headers, and body capture.
- [ ] **Validation Checkpoint:** `cargo test -p biscuit-file --features fetch` passes and no HTTP dependency is compiled when `fetch` is disabled.

## Phase 3: Darkmatter URL Discovery and Configuration
*Dependency: Phase 1*
*Parallelizable:* After Phase 1, this phase can proceed in parallel with Phase 2 except for any code that calls the final `fetch` primitive.

**Goal:** Make Darkmatter discover URL dependencies and configure remote access policy without performing remote fetches yet.

- [ ] Audit current compose reference traversal, `reference_graph()`, transclusion preparation, expression function argument handling, and link-resolve/finalization passes.
- [ ] Add remote-reference data structures for normalized URL keys, source location, consumer kind, and cache slot identity.
- [ ] Extend transclusion discovery to collect URL targets used by `::file` and `::code`.
- [ ] Extend read-side expression function discovery to collect URL-typed arguments used by functions such as `frontmatter(url)`, `file_exists(url)`, and `markdown_title(url)`.
- [ ] Deduplicate discovered URLs by normalized URL string before orchestration.
- [ ] Add remote read configuration to `ComposeOptions`: fetch policy, remote concurrency cap, remote TTL override, refresh behavior, and freshness mode mapping.
- [ ] Add CLI flags and environment variable parsing for the remote concurrency cap, remote TTL, refresh, and allowed hosts.
- [ ] Keep the default network policy deny-all, and surface a precise error when remote reads are requested without an allowed host.
- [ ] Update link resolution and finalization so absolute HTTP(S) URLs are validated for scheme/shape and left intact in rendered output.
- [ ] Add tests for URL discovery from transclusion, expression functions, deduplication, deny-all configuration, CLI/env precedence, and link finalization.
- [ ] **Validation Checkpoint:** `cargo test -p darkmatter url` or the closest targeted Darkmatter test filter passes with no network access required.

## Phase 4: Darkmatter Eager Fetch Orchestration
*Dependency: Phase 2, Phase 3*

**Goal:** Start remote fetches eagerly, reuse a single client, and block consumers on one single-flight slot per URL.

- [ ] Enable `biscuit-file/fetch` for Darkmatter through the appropriate Cargo feature wiring.
- [ ] Add a shared `reqwest::Client` to the compose runtime used for all remote fetches in a compose run.
- [ ] Register each discovered URL as an eager in-flight remote slot in the run-local cache before local composition work begins.
- [ ] Spawn remote fetch tasks on Darkmatter's Tokio runtime with the configured global concurrency cap.
- [ ] Implement point-of-use waiting so transclusion and expression functions block on the URL's existing single-flight slot.
- [ ] Disable duplicate-compute timeout fallback for remote slots; timeout handling must wait or fail according to remote freshness policy rather than issuing another request.
- [ ] Register and spawn newly discovered nested remote URLs immediately after fetched Markdown content is parsed.
- [ ] Wire `::file` URL transclusion to insert fetched body text.
- [ ] Wire `::code` URL transclusion to insert fetched body text with the same language, fence, and rendering behavior as local code transclusion.
- [ ] Wire URL-aware read-side expression functions to consume fetched content consistently with local file inputs.
- [ ] Add cache/report stats for remote fetches, waits, policy denials, network failures, and stale/cache use where the existing report model supports them.
- [ ] Add tests proving duplicate URL consumers perform one fetch per run, eager fetch overlaps with local work, nested remote references are discovered, and policy-denied URLs never attempt a request.
- [ ] **Validation Checkpoint:** Mock-server compose tests pass for `::file`, `::code`, and read-side expression functions with multiple consumers of the same URL.

## Phase 5: Persistent Remote Cache and Freshness
*Dependency: Phase 4*

**Goal:** Extend the existing persistent cache to store remote URL artifacts and apply TTL, conditional revalidation, and stale-on-failure behavior.

- [ ] Audit the existing `.darkmatter/cache/v1/` artifact layout, manifest schema, `SourceKind`, and closure-hash integration.
- [ ] Add `remote_url` artifact support with a response-body blob and manifest fields for URL, status, `etag`, `last_modified`, `cache_control`, `fetched_at`, `expires_at`, and content hash.
- [ ] Reuse the existing content hashing strategy for remote bodies so remote dependency changes flow into parent `closure_hash` values.
- [ ] Implement TTL calculation from `Cache-Control: max-age`, explicit remote TTL override, and existing manifest `expires_at`.
- [ ] Implement within-TTL behavior that serves the cached blob without network access.
- [ ] Implement past-TTL conditional GET behavior using `If-None-Match` and `If-Modified-Since` when available.
- [ ] Implement `304` handling that preserves the cached body and updates freshness metadata.
- [ ] Implement `200` refresh handling that replaces the cached body and manifest atomically.
- [ ] Implement stale-on-failure behavior for `Fallback` freshness mode.
- [ ] Map existing freshness modes so `Optimistic` serves cache without revalidation, `Strict` revalidates, and `Fallback` serves stale on network failure.
- [ ] Implement `--refresh` behavior so remote artifacts are revalidated even when cached content is otherwise fresh.
- [ ] Add tests for cache hits within TTL, conditional `304`, conditional `200`, network failure with stale fallback, strict failure without fallback, optimistic no-network behavior, and closure-hash invalidation.
- [ ] **Validation Checkpoint:** Persistent cache tests pass using a temporary cache directory and local mock server, including a no-network cached run.

## Phase 6: Integration, Documentation, and Release Checks
*Dependency: Phase 5*

**Goal:** Finish the feature as a coherent public capability with tests, docs, and release-facing maintenance updates.

- [ ] Wire the side-effect engine's `http_post` path to use the same `biscuit-file` fetch primitive and `FetchPolicy` enforcement.
- [ ] Verify remote writes still honor the side-effect engine's mutation rules in addition to the shared network policy.
- [ ] Add or update Darkmatter topic docs for remote URL references, allowed hosts, freshness modes, cache behavior, and unsupported v1 schemes.
- [ ] Update Darkmatter CLI docs for new remote flags and environment variables.
- [ ] Update `docs/dependencies.md` and per-area dependency docs for newly added `url`, `reqwest`, `bytes`, or related dependencies.
- [ ] Update `.claude/skills/darkmatter/SKILL.md` if compose pipeline architecture, cache behavior, or URL referencing workflow guidance changed.
- [ ] Add end-to-end CLI tests for allowed-host remote compose, deny-all failure, refresh, stale fallback, and rendered link preservation.
- [ ] Run targeted package checks for `biscuit-file`, `darkmatter`, and `darkmatter-cli`.
- [ ] Run the repo's canonical Darkmatter test recipe from the local `justfile` or the narrowest accepted equivalent if a full run is too costly.
- [ ] Review all touched rustdoc and inline comments for drift, deleting or updating comments that no longer match behavior.
- [ ] **Validation Checkpoint:** Build, lint, targeted tests, docs updates, and feature behavior are complete with no unguarded network fetch path.
