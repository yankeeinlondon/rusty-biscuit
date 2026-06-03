# URL Referencing

Darkmatter's referencing model is filesystem-only today: `FileReference`
(in `biscuit-file`) classifies a compact descriptor and `resolve()` returns a
local `PathBuf`. Local file referencing was built first as the simpler base.
This feature extends referencing to **remote HTTP(S) URLs** so the same
descriptors that drive transclusion, expression functions, links, and side
effects can also point at remote content.

## Scope (v1)

- **Schemes:** `http` and `https` only. No `gh:`, `s3:`, `gs:`, `git:`, or
  custom schemes in v1.
- **Use cases in scope:**
    - **Remote reads** into composition — transclusion (`::file`, `::code`) and
      read-side expression functions (`frontmatter(url)`, `file_exists(url)`,
      `markdown_title(url)`, …).
    - **Remote writes** — the side-effect engine's `http_post` (see
      [`more-context-variables`](../more-context-variables/spec.md#side-effects)).
    - **Links in rendered output** — URLs as first-class link targets handled by
      the link-resolve / finalization passes.

## Ownership Split

| Concern | Owner |
|---|---|
| URL **classification** (new `Url` reference kind, `Resolved` target enum) | **biscuit-file** (no network I/O) |
| Low-level **fetch primitive** (request, `FetchPolicy` enforcement, error mapping) | **biscuit-file** (`fetch` feature, off by default) |
| Concurrent **prefetch orchestration** + cache integration | **Darkmatter** |
| Remote writes (`http_post`) | Darkmatter side-effect engine, via the biscuit-file primitive |

The design principle mirrors the existing read/write and engine/driver splits:
`biscuit-file` owns *classification and the fetch primitive*; the *pipeline
owner* (Darkmatter) owns *orchestration, concurrency, and caching*.

## Phase 1 — biscuit-file Prerequisites

These land first; Darkmatter's orchestration depends on them.

### `Url` reference kind

`FileReference` gains a `Url` variant alongside the existing kinds (Relative,
Implicit-Relative, Absolute, Magic `@`, Package `!`, Vault `vault:`). A
descriptor is classified as `Url` when it carries an `http://` or `https://`
scheme. Existing normalization still applies where meaningful (e.g. collapsing
`//` in the path component is **not** applied to the scheme's `//`).

### Resolution returns a typed target (non-breaking)

`resolve()` stays exactly as-is — `-> PathBuf` — and returns a new typed error
`FileReferenceError::RemoteNotLocal` when called on a `Url` reference. A new
method classifies without forcing a path:

```rust
pub enum Resolved {
    Local(PathBuf),
    Remote(url::Url),
}

impl FileReference {
    pub fn resolve_target(&self) -> Result<Resolved, FileReferenceError>;
}
```

All existing local-only consumers of `resolve()` are untouched. URL-aware
consumers migrate to `resolve_target()`.

### `fetch` feature (off by default)

A feature-gated `fetch` module adds the only HTTP dependency in `biscuit-file`
and exposes a single policy-enforcing primitive plus a blocking convenience
wrapper:

```rust
pub struct FetchPolicy {
    /// Allowed hosts. Empty == deny-all (the default).
    pub allowed_hosts: Vec<HostPattern>,
    // future: per-host caps, scheme restrictions, max body size, timeouts
}

pub struct FetchResponse {
    pub status: u16,
    pub body: bytes::Bytes,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub cache_control: Option<String>,
}

/// Async primitive — takes a shared client so callers pool connections.
pub async fn fetch(
    client: &reqwest::Client,
    url: &url::Url,
    policy: &FetchPolicy,
    conditional: Option<Conditional>, // If-None-Match / If-Modified-Since
) -> Result<FetchResponse, FetchError>;

/// Blocking wrapper for sync / one-shot consumers.
pub fn fetch_blocking(/* … */) -> Result<FetchResponse, FetchError>;
```

- **Safety lives here.** `FetchPolicy` enforcement (host allowlist, deny-all
  default) happens inside `fetch`, so *every* consumer — Darkmatter compose
  reads, side-effect writes, and any other crate — inherits SSRF protection.
  Consumer configures the policy; `biscuit-file` enforces it. This realizes the
  "Darkmatter-wide network policy" anticipated in the side-effects spec, one
  layer down in the shared primitive.
- `fetch` takes a **shared `reqwest::Client`** so orchestrators reuse
  connections (keep-alive, HTTP/2 multiplexing) across many requests.
- `HostNotAllowed` is a distinct `FetchError` variant so callers can surface it
  precisely.

## Phase 2 — Darkmatter Orchestration

### Eager prefetch, block at point-of-use

Network latency dominates remote cost, so it is overlapped with all local
composition work:

1. **Discovery pass.** Reuse the existing document-graph traversal (the same
   one `reference_graph()` / transclusion use) to collect every **normalized,
   deduplicated** remote URL — from transclusion directives and URL-typed
   expression-function arguments. Each unique URL is registered as an **eager
   in-flight slot** in the existing `RunLocalCache` single-flight map, and its
   fetch is spawned immediately.
2. **Shared async executor.** Fetches run as tasks on Darkmatter's existing
   `tokio` runtime through a **single shared `reqwest::Client`**. In-flight work
   is bounded by a **concurrency cap (default ~16, overridable** via
   `ComposeOptions`, a CLI flag, and an env var).
3. **Block at point-of-use.** When compose needs a URL's content, it blocks on
   that URL's single-flight slot. Because the fetch began during discovery, the
   wait is usually near-zero. Tokio runtime threads are distinct from the rayon
   pool, so a waiting rayon worker merely parks.
4. **Register-on-discovery for nested URLs.** Remote content can itself contain
   remote references invisible to the initial pass. The moment fetched content
   is parsed, its further remote refs are registered and spawned. Waves overlap;
   nothing waits for a discovery-phase boundary.

> **Single-flight caveat for remote.** The existing single-flight "timeout →
> duplicate compute" fallback (a rayon-deadlock guard) must be **disabled** for
> remote slots — re-fetching defeats the cache and doubles network load. Past a
> timeout, waiters keep waiting or fail per the freshness mode.

### Caching (extends the existing two-layer cache)

Both layers already exist for local transclusion; remote extends them, and the
snapshot manifest already reserves a `SourceKind::remote_url` variant for this.

- **Run-local** — the single-flight `DashMap`, populated eagerly during
  discovery. One fetch per unique URL per run.
- **Persistent** — `.darkmatter/cache/v1/` gains `remote_url` artifacts: the
  response body as a blob plus a manifest carrying `etag`, `last_modified`,
  `fetched_at`, `expires_at`, and a content hash.

**Default freshness — TTL → conditional → stale-on-failure:**

- **Within TTL** (`Cache-Control: max-age`, an explicit `--remote-ttl`, or
  `expires_at`): serve the cached blob with **no network**.
- **Past TTL:** issue a conditional GET (`If-None-Match` / `If-Modified-Since`);
  `304` reuses the blob cheaply, `200` refreshes.
- **On network failure:** fall back to the stale cached copy (`Fallback` mode) —
  resilient for CI/offline builds. `--refresh` forces revalidation.

The existing freshness modes map directly: `Optimistic` serves cache without
revalidation, `Strict` always revalidates, `Fallback` serves stale on failure.

**Invalidation.** A remote artifact's content hash flows into the existing
Merkle-style `closure_hash` exactly like a local source, so a changed remote
dependency correctly invalidates the parents that transclude it.

### Links in rendered output

The finalization link pass leaves absolute URLs **intact** — they are never
rewritten into portable relative paths the way local links are. Validation is
limited to scheme/shape (`http`/`https`, well-formed). **Reachability checking
is out of scope for v1**: it would couple rendering to the network and is better
served by an explicit, opt-in validation command later.

## Safety Summary

- All remote access — reads and writes — flows through the single `biscuit-file`
  fetch primitive, where `FetchPolicy` (host allowlist, **deny-all default**) is
  enforced. There is no unguarded fetch path.
- `md compose` performing remote **reads** is network egress (not mutation), but
  carries the same SSRF/privacy surface as writes; the shared policy gates both.
- Side-effect **writes** additionally honor the side-effect engine's mutation
  rules; see the side-effects spec.

## Dependencies & Sequencing

1. **Phase 1 (biscuit-file):** `Url` kind, `resolve_target()` + `Resolved`,
   `RemoteNotLocal` error, the `fetch` feature (`reqwest` primitive +
   `FetchPolicy`). Prerequisite for everything else.
2. **Phase 2 (Darkmatter):** discovery pass, shared executor + concurrency cap,
   eager single-flight population, persistent `remote_url` cache + freshness,
   link-pass handling.
3. **Consumes:** the side-effect engine's `http_post`
   ([`more-context-variables`](../more-context-variables/spec.md#side-effects))
   uses the same biscuit-file fetch primitive and `FetchPolicy`.

## Out of Scope (v1)

- Non-HTTP schemes (`gh:`, `s3:`, `gs:`, `git:`, custom).
- Authentication flows beyond what a caller can put on the shared client /
  policy (no built-in OAuth, signing, etc.).
- Remote-link reachability validation during rendering.
- Per-host concurrency caps (global cap only in v1; per-host is a noted future
  improvement).
</content>
