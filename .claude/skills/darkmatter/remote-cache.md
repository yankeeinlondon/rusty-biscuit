# Remote Reads and the Transport Cache

Detail behind the "Remote and cache safety" section of [SKILL.md](SKILL.md).

- CLI callers opt in per run with `md compose --allow host=<host>`, or for
  every run with `permissions: network: allow` (user file, and inside a
  repository its `.darkmatter.yaml` at both `HEAD` and the working copy).
- One projection decides consent for composition and schema slots alike:
  `config::network::NetworkPermission::from_snapshot(config_snapshot)` (no
  snapshot = `Deny`; an invalid `permissions` block = `Invalid`, which denies
  and puts the config error in `RemoteReadError::InvalidConfiguration`), then
  `.fetch_policy(exact_hosts)` (`FetchPolicy::allow_all()` under `Allow`).
  `ComposeOptions::network_permission()` feeds `RemoteFetchRuntime::with_store_and_network`
  and `SlotSession`/`GrantBook::with_network` (slot gate and slot fetch policy);
  the decision is `ConfigSnapshot::policy()?.network()`: the AND of user,
  `HEAD`, and worktree, with each `permissions` reference followed. The decision is in both
  option identities (`network_permission` field). Attach the config snapshot
  before `with_shared_remote_fetch()`; `with_config_snapshot` and
  `ComposeRequest` assembly rebuild a shared runtime whose decision differs
  (`align_remote_network`). Pinned by `lib/tests/l1/network_permission.rs`.
- Use the two-category vocabulary everywhere (help, docs, comments):
  **semantic-result artifacts** (composed documents, `::file` children,
  `::code`/`::toc-linking` results, snapshots) are memory-only until a
  `ContentPolicy` exists (R18); **transport artifacts** (raw HTTP bodies) are
  the one class `--cache-root` / `ComposeOptions::with_cache_root(...)` may
  persist (R36, the fix's Q1). Local transclusion stays run-local even after
  `ContentPolicy`. `docs/topics/caching.md` is the user-facing contract, and
  `cli/tests/l1/help.rs::test_compose_help_states_the_transport_cache_boundary`
  pins the `md compose --help` wording. The semantic-result persistence path was deleted, not kept
  dormant: `RunLocalCache` is memory-only, `CacheAccessMode` governs run-local
  reuse only, and `CacheStats` has no persistent counters (transport activity
  is `RemoteFetchStats`). `lib/tests/l1/semantic_results_never_persist.rs` pins
  this: `RunLocalCache` may not name `FileStore`/`RemoteFetchRuntime`,
  `FileStore` is allowlisted (exact counts) to the remote transport cache, and
  the deleted symbols may not reappear. Update its allowlist deliberately when
  a transport-cache file legitimately changes its `FileStore` uses.
- Configuring a cache root mutates nothing. `FileStore::at` only records the
  path and creates directories inside the write that needs them. A missing root
  stays missing and an existing one stays byte-identical unless an artifact is
  written. Never add eager `create_dir_all` or a platform-cache fallback.
  These no-fallback rules govern the remote transport cache only. The DMLS
  index store (`dmls/src/index_store/`) deliberately differs: it creates its
  directory under `biscuit_file::user_cache_dir()` (or `DMLS_CACHE_DIR`) and
  falls back to memory whenever the store is unusable (see `dmls.md`).
- Freshness is controlled by `RemoteReadConfig` and the CLI remote freshness,
  refresh, and TTL flags. Response `Cache-Control` outranks all of them:
  `no-store` is never written (an on-disk `no-store` entry is purged), and
  `no-cache` is revalidated before every reuse, even under `Optimistic`, with
  no stale serve under `Fallback`. `Fallback` serves stale only when the
  revalidation's `RemoteFetchFailure::kind` is `Transport`; a policy refusal
  (denied redirect), invalid reference, or infrastructure failure is returned
  so its kind reaches the readers' typed adapter. The write path accepts only
  `StorableDirectives`, so a TTL override structurally cannot store a
  `no-store` response; keep it that way.
- Transport-cache I/O failures (write or purge) are non-fatal. They flow
  through `RemoteFetchStats::cache_warnings` into `ComposeReport.warnings`
  (stage `remote_cache`, code `dm.remote_cache.io_failure`); never swallow
  them with `let _ =`.
- Remote manifests persist `redacted_url` (`redact_url`: no userinfo or
  fragment, query shown as `?<redacted>`), never the raw URL; the entry key
  stays `xx_hash` of the full URL. Warnings may name the redacted form only.
  Manifest `CACHE_VERSION` (now `2`) is separate from the `v{N}` directory's
  `STORE_LAYOUT_VERSION` (`1`), so old entries stay at the same path: another
  version is a miss left on disk, but a `no-store` one is still purged via the
  version-stable `RemoteUrlManifestHeader`. Bump the layout version only when
  the path scheme changes.
- Host policy must be checked before the transport cache is read.
  `fetch_with_cache` reads the store before its policy-enforcing client runs,
  so the early `check_allowed` in `RemoteFetchRuntime::register_and_fetch` is
  the only thing keeping a denied host's cached bytes out of output. A cache
  root never authorizes a host. `denied_host_never_reads_a_fresh_cached_entry`
  and the CLI `test_compose_denied_host_never_reads_a_seeded_cache_entry` pin
  this ordering.
- The cache-only path (`RemoteFetchRuntime::cache_only`, `cached_text(url,
  now)`, over `cache::remote_cache::lookup_fresh`) is for editor reads: it
  checks scheme, the `network` configuration error, and host policy before the
  store, serves only a current-version, non-`no-store`, non-`no-cache` entry
  whose `expires_at` is after the injected `now` (body plus expiry), and never
  fetches, registers a slot, revalidates, purges, or writes. Both paths share
  `decode_cached`; only the fetching path purges `no-store`. Every miss is a
  typed `CacheOnlyPending`, never a read failure.
- A failed fetch is typed where it happens: `compose/remote_fetch/error.rs::RemoteFetchFailure`
  (`kind: expression::RemoteReadKind`, `message`). `Transport` covers the
  request, HTTP status, body read, header, and response UTF-8 failures;
  `Policy` a policy denial or blocked redirect; `InvalidReference` an
  unsupported scheme or unparsable URL; `Infrastructure` the runtime or client
  failing to start, or a slot missing after registration. `FetchSlot::Failed`
  and `fetch_with_cache` keep the typed failure; `get_content` stays the
  string adapter (the message, byte for byte) for transclusion and preflight,
  and `get_content_typed` feeds `ResolutionContext::fetch_remote_text`, whose
  policy-derived `RemoteTextError::Denied` is checked first so a denied host is
  never typed `Transport`. `ExpressionError::RemoteRead.kind` carries the kind;
  only `Transport` is recoverable (see expressions.md and
  [Recovering from read failures](../../../darkmatter/docs/topics/darkmatter-expressions.md#recovering-from-read-failures)).
  Never classify a fetch by its message; a new `FetchError` or
  `RemoteReadError` variant gets its kind in the exhaustive `From` impls in
  `remote_fetch/error.rs`.
- `absolute` and `relative` are local path transforms, never remote fetches.
- `EffectEngine::http_post` uses the same host policy as remote reads.
