# More Context Variables — Remote (Network) Parts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **⚠️ HARD PREREQUISITE — DO NOT START THIS PLAN UNTIL `url-referencing` IS IMPLEMENTED.**
> This plan consumes APIs introduced by [`../url-referencing/spec.md`](../url-referencing/spec.md):
> the `Url` reference kind, `FileReference::resolve_target() -> Resolved::{Local, Remote}`,
> and the feature-gated `biscuit-file` fetch primitive (`fetch` / `fetch_blocking`) + `FetchPolicy`.
> It also assumes `plan-local.md` has shipped (the `EffectEngine` and the local expression functions exist).

**Goal:** Add the two network-touching pieces of the `more-context-variables` spec — the `http_post` side-effect verb, and URL-accepting behavior for the read-side expression functions — on top of the `url-referencing` substrate.

**Architecture:** `http_post` is a thin verb on the existing `EffectEngine` that delegates to the shared `biscuit-file` fetch primitive, gated by the engine's host allowlist (converted to a `FetchPolicy`). The read functions gain URL awareness by upgrading their shared resolution helper from "resolve to a local path" to "resolve to local content **or** fetched remote content," reusing url-referencing's fetch/cache layer.

**Tech Stack:** Rust, `biscuit-file` (`fetch` feature, `FetchPolicy`, `Resolved`), `serde_json::Value`, `tempfile` + `wiremock` (tests).

**Phases run in order (Phase 1 → 4).**

---

## Assumed Interfaces From `url-referencing`

These are the symbols this plan depends on. If their final names differ at implementation time, update the calls below to match — do not re-implement them here.

```rust
// biscuit-file (feature = "fetch")
pub struct FetchPolicy { pub allowed_hosts: Vec<HostPattern>, /* … */ }
pub struct FetchResponse { pub status: u16, pub body: bytes::Bytes,
                           pub etag: Option<String>, pub last_modified: Option<String>,
                           pub cache_control: Option<String> }
pub enum FetchError { HostNotAllowed(String), /* … */ }

pub fn fetch_blocking(url: &url::Url, policy: &FetchPolicy,
                      conditional: Option<Conditional>) -> Result<FetchResponse, FetchError>;
pub fn post_blocking(url: &url::Url, body: &[u8],
                     policy: &FetchPolicy) -> Result<FetchResponse, FetchError>;

// biscuit-file FileReference
pub enum Resolved { Local(std::path::PathBuf), Remote(url::Url) }
impl FileReference { pub fn resolve_target(&self) -> Result<Resolved, FileReferenceError>; }
```

For the read-function path, this plan uses `fetch_blocking` directly for correctness/simplicity. **Wiring the read functions into Darkmatter's concurrent prefetch cache (url-referencing Phase 2) is a follow-up optimization, tracked at the end of this plan — not a correctness requirement.**

---

## Test command convention

From `darkmatter/`: single test `cargo test -p darkmatter <name>`; the `fetch` feature must be enabled for these tests: `cargo test -p darkmatter --features fetch <name>` (confirm the feature is forwarded from `darkmatter/lib/Cargo.toml` — see Phase 1).

---

## Phase 1 — Enable the `fetch` feature path in darkmatter

### Task 1.1: forward a `fetch` feature to biscuit-file

**Files:**
- Modify: `darkmatter/lib/Cargo.toml` (forward a `fetch` feature to biscuit-file)
- Modify: `darkmatter/docs/dependencies.md`

- [ ] **Step 1: Write the failing test**

`darkmatter/lib/tests/remote_feature_gate.rs`:

```rust
// Compiles only when the fetch feature is on; asserts the engine exposes http_post.
#![cfg(feature = "fetch")]

#[test]
fn fetch_feature_compiles_and_engine_has_http_post() {
    let dir = tempfile::TempDir::new().unwrap();
    let _engine = darkmatter::effects::EffectEngine::builder()
        .mutation_root(dir.path())
        .allowed_hosts(["example.com"])
        .build();
    // Method existence is checked at compile time below.
    let _f: fn(&darkmatter::effects::EffectEngine, &str, serde_json::Value)
        -> Result<serde_json::Value, darkmatter::effects::EffectError> =
        darkmatter::effects::EffectEngine::http_post;
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter --features fetch fetch_feature_compiles_and_engine_has_http_post`
Expected: FAIL (no `fetch` feature / no `http_post`).

- [ ] **Step 3: Implement the feature wiring**

In `darkmatter/lib/Cargo.toml`:

```toml
[features]
# Enables remote URL fetching via biscuit-file's fetch primitive.
fetch = ["biscuit-file/fetch"]
```

Ensure the `biscuit-file` dependency can receive the feature (it already declares `fetch` per the url-referencing work). Add a one-line note to `darkmatter/docs/dependencies.md` recording the optional `fetch` feature and that it pulls `reqwest` transitively through `biscuit-file`.

> The test still fails until Phase 2 adds `http_post`. That is expected — Phase 1 and Phase 2 form one logical unit; commit them together at the end of Phase 2. Do not land Phase 1 alone with a dangling reference.

- [ ] **Step 4: Run test to verify it passes** (after Phase 2)

Run: `cargo test -p darkmatter --features fetch fetch_feature_compiles_and_engine_has_http_post`
Expected: PASS

- [ ] **Step 5: Commit** (with Phase 2)

```bash
git add darkmatter/lib/Cargo.toml darkmatter/docs/dependencies.md
git commit -m "build(darkmatter): add optional fetch feature forwarding to biscuit-file"
```

---

## Phase 2 — `http_post` Side-Effect Verb

### Task 2.1: `http_post` on `EffectEngine`

**Files:**
- Create: `darkmatter/lib/src/effects/network.rs` (feature-gated)
- Modify: `darkmatter/lib/src/effects/mod.rs` (declare module under `#[cfg(feature = "fetch")]`)
- Modify: `darkmatter/lib/src/effects/error.rs` (map `FetchError` → `EffectError`)
- Test: `darkmatter/lib/tests/effects_network.rs` (wiremock)

- [ ] **Step 1: Write the failing test**

`darkmatter/lib/tests/effects_network.rs`:

```rust
#![cfg(feature = "fetch")]

use darkmatter::effects::EffectEngine;
use serde_json::json;

#[tokio::test]
async fn http_post_sends_body_and_returns_status() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/hook"))
        .respond_with(ResponseTemplate::new(202).set_body_string("queued"))
        .mount(&server)
        .await;

    let uri = server.uri(); // e.g. http://127.0.0.1:PORT
    let host = uri
        .trim_start_matches("http://")
        .split(':')
        .next()
        .unwrap()
        .to_string();

    let dir = tempfile::TempDir::new().unwrap();
    let engine = EffectEngine::builder()
        .mutation_root(dir.path())
        .allowed_hosts([host])
        .build();

    // EffectEngine is sync; run the blocking call off the async runtime.
    let url = format!("{uri}/hook");
    let result = tokio::task::spawn_blocking(move || {
        engine.http_post(&url, json!({"event": "done"}))
    })
    .await
    .unwrap()
    .unwrap();

    assert_eq!(result["status"], json!(202));
    assert_eq!(result["body"], json!("queued"));
}

#[tokio::test]
async fn http_post_refuses_disallowed_host() {
    let dir = tempfile::TempDir::new().unwrap();
    let engine = EffectEngine::builder()
        .mutation_root(dir.path())
        // deny-all: no hosts allowed
        .build();
    let err = tokio::task::spawn_blocking(move || {
        engine.http_post("http://example.com/x", serde_json::json!({}))
    })
    .await
    .unwrap()
    .unwrap_err();
    assert!(matches!(err, darkmatter::effects::EffectError::HostNotAllowed(_)));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p darkmatter --features fetch --test effects_network`
Expected: FAIL (no `http_post`).

- [ ] **Step 3: Implement**

`effects/network.rs`:

```rust
//! Network side-effect verbs (feature `fetch`).

use crate::effects::error::EffectError;
use crate::effects::EffectEngine;
use serde_json::{json, Value};

impl EffectEngine {
    /// `http_post(url, body) -> { status, body }`.
    ///
    /// Gated by the engine's host allowlist (deny-all by default), enforced
    /// inside the shared `biscuit-file` fetch primitive via `FetchPolicy`.
    pub fn http_post(&self, url: &str, body: Value) -> Result<Value, EffectError> {
        let parsed = url::Url::parse(url)
            .map_err(|e| EffectError::InvalidFilePath(format!("{url:?}: {e}")))?;
        let policy = self.fetch_policy();
        let payload = serde_json::to_vec(&body)
            .map_err(|e| EffectError::Markdown(e.to_string()))?;
        let response = biscuit_file::post_blocking(&parsed, &payload, &policy)
            .map_err(EffectError::from)?;
        // Body is best-effort decoded as UTF-8 (callers usually inspect status).
        let body_str = String::from_utf8_lossy(&response.body).to_string();
        Ok(json!({ "status": response.status, "body": body_str }))
    }

    /// Builds a `FetchPolicy` from the engine's configured host allowlist.
    fn fetch_policy(&self) -> biscuit_file::FetchPolicy {
        biscuit_file::FetchPolicy {
            allowed_hosts: self
                .allowed_hosts()
                .iter()
                .map(|h| biscuit_file::HostPattern::exact(h))
                .collect(),
        }
    }
}
```

> This assumes url-referencing exposes a `post_blocking(url, body, policy)` companion to `fetch_blocking`. If only `fetch_blocking` (GET) exists, that is a coordination item to raise on the url-referencing plan — the side-effects spec lists `http_post` as a consumer, so the POST entry point belongs in the shared fetch primitive. Do not hand-roll a second reqwest client here; that would duplicate the guarded fetch path the whole design centralizes.

In `effects/error.rs`, add the `From` mapping (feature-gated):

```rust
#[cfg(feature = "fetch")]
impl From<biscuit_file::FetchError> for EffectError {
    fn from(e: biscuit_file::FetchError) -> Self {
        match e {
            biscuit_file::FetchError::HostNotAllowed(host) => EffectError::HostNotAllowed(host),
            other => EffectError::Markdown(other.to_string()),
        }
    }
}
```

In `effects/mod.rs`:

```rust
#[cfg(feature = "fetch")]
mod network;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p darkmatter --features fetch --test effects_network`
Expected: PASS

- [ ] **Step 5: Commit** (includes Phase 1's Cargo/doc changes)

```bash
git add darkmatter/lib/src/effects/network.rs darkmatter/lib/src/effects/mod.rs darkmatter/lib/src/effects/error.rs darkmatter/lib/tests/effects_network.rs darkmatter/lib/tests/remote_feature_gate.rs darkmatter/lib/Cargo.toml darkmatter/docs/dependencies.md
git commit -m "feat(darkmatter): add http_post side-effect verb behind fetch feature"
```

---

## Phase 3 — URL-Aware Read Functions

Upgrade the shared resolution helper used by `absolute`, `file_exists`, `frontmatter`, `markdown_body_empty`, `markdown_title`, and `validate_schema` so a URL reference fetches remote content instead of erroring. Behavior per function:

- `absolute(url)` → the normalized URL string (a URL is already absolute).
- `relative(url)` → the URL string unchanged (relativization is undefined for URLs; do not error).
- `file_exists(url)` → `true` if a fetch returns a success status, else `false` (never errors).
- `frontmatter(url[, prop])`, `markdown_body_empty(url)`, `markdown_title(url)`, `validate_schema(url)` → fetch the body, parse as `Markdown`, then behave exactly as the local path forms.

### Task 3.1: resolution upgrade to `Resolved::Local | Remote`

**Files:**
- Modify: `darkmatter/lib/src/markdown/compose/expression/functions.rs` (the `resolve_arg` / `load_markdown` helpers from plan-local Phase 5)
- Modify: `darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs` (add fetch policy to `ResolutionContext`, defined in plan-local Task 4.1)
- Test: `functions.rs` tests with `wiremock`

- [ ] **Step 1: Write the failing test**

In `functions.rs` tests (feature-gated):

```rust
#[cfg(feature = "fetch")]
#[tokio::test]
async fn frontmatter_reads_remote_markdown() {
    use crate::markdown::compose::expression::ResolutionContext;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/doc.md"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("---\ntitle: Remote\n---\nBody\n"),
        )
        .mount(&server)
        .await;
    let host = server.address().ip().to_string();
    let url = format!("{}/doc.md", server.uri());

    let dir = tempfile::TempDir::new().unwrap();
    let ctx = ResolutionContext::new(dir.path().to_path_buf()).with_allowed_hosts([host]);

    let title = tokio::task::spawn_blocking(move || {
        frontmatter_fn(&[serde_json::json!(url), serde_json::json!("title")], &ctx)
    })
    .await
    .unwrap()
    .unwrap();
    assert_eq!(title, serde_json::json!("Remote"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter --features fetch frontmatter_reads_remote_markdown`
Expected: FAIL (resolution helper does not handle URLs / `with_allowed_hosts` missing).

- [ ] **Step 3: Implement**

Extend `ResolutionContext` (`resolve_ctx.rs`, defined in plan-local Task 4.1) with an allowlist used for read-side fetches. Keep the single struct shape — always include the field; gate only the fetch-specific method:

```rust
#[derive(Clone, Debug, Default)]
pub struct ResolutionContext {
    pub base_dir: std::path::PathBuf,
    pub magic_paths: Vec<(std::path::PathBuf, biscuit_file::PathPosition)>,
    pub allowed_hosts: Vec<String>,
}

#[cfg(feature = "fetch")]
impl ResolutionContext {
    pub fn with_allowed_hosts<I, S>(mut self, hosts: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.allowed_hosts = hosts.into_iter().map(Into::into).collect();
        self
    }
}
```

In `functions.rs`, route `load_markdown` / `absolute_fn` through `resolve_target` so a `Resolved::Remote(url)` fetches instead of erroring:

```rust
/// Loads a Markdown document from a local path or a remote URL.
fn load_markdown(raw: &str, ctx: &ResolutionContext, fname: &str) -> Result<Markdown, String> {
    let normalized = normalize_path_arg(raw);
    let file_ref = biscuit_file::FileReference::new(&normalized)
        .map_err(|e| format!("{fname}() invalid file path {raw:?}: {e}"))?;
    match file_ref
        .resolve_target()
        .map_err(|e| format!("{fname}() invalid file path {raw:?}: {e}"))?
    {
        biscuit_file::Resolved::Local(path) => {
            // Re-resolve via resolve_from for `@`/relative document-relative refs.
            let path = resolve_arg(raw, ctx)?.unwrap_or(path);
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("{fname}() invalid file path {raw:?}: {e}"))?;
            Markdown::try_from_content(content)
                .map_err(|e| format!("{fname}() failed to parse {raw:?}: {e}"))
        }
        #[cfg(feature = "fetch")]
        biscuit_file::Resolved::Remote(url) => {
            let policy = fetch_policy_from(&ctx.allowed_hosts);
            let resp = biscuit_file::fetch_blocking(&url, &policy, None)
                .map_err(|e| format!("{fname}() fetch failed for {raw:?}: {e}"))?;
            let content = String::from_utf8_lossy(&resp.body).to_string();
            Markdown::try_from_content(content)
                .map_err(|e| format!("{fname}() failed to parse remote {raw:?}: {e}"))
        }
        #[cfg(not(feature = "fetch"))]
        biscuit_file::Resolved::Remote(_) => {
            Err(format!("{fname}() remote URLs require the 'fetch' feature: {raw:?}"))
        }
    }
}

#[cfg(feature = "fetch")]
fn fetch_policy_from(hosts: &[String]) -> biscuit_file::FetchPolicy {
    biscuit_file::FetchPolicy {
        allowed_hosts: hosts
            .iter()
            .map(|h| biscuit_file::HostPattern::exact(h))
            .collect(),
    }
}
```

Update `absolute_fn` to short-circuit on `Resolved::Remote` (complete replacement of the plan-local version):

```rust
pub fn absolute_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    require_args("absolute", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let raw = require_string("absolute", &args[0])?;
    let normalized = normalize_path_arg(raw);
    let file_ref = biscuit_file::FileReference::new(&normalized)
        .map_err(|e| format!("absolute() invalid file path {raw:?}: {e}"))?;
    match file_ref.resolve_target() {
        Ok(biscuit_file::Resolved::Remote(url)) => Ok(Value::String(url.to_string())),
        _ => match resolve_arg(raw, ctx)? {
            Some(p) => Ok(Value::String(p.to_string_lossy().to_string())),
            None => Err(format!("absolute() invalid file path: {raw:?}")),
        },
    }
}
```

Update `file_exists_fn`'s remote branch: if `resolve_target()` yields `Resolved::Remote(url)` and the `fetch` feature is on, attempt a fetch and return `Value::Bool(success)`; on any error return `Value::Bool(false)` (never error). When the feature is off, a remote ref returns `false`. `relative_fn` returns the URL string unchanged for remote refs.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p darkmatter --features fetch frontmatter_reads_remote_markdown`
Expected: PASS

Also confirm the non-feature build still compiles and remote refs error/`false` cleanly:
Run: `cargo test -p darkmatter` (no `--features fetch`)
Expected: PASS (remote branch returns the "requires 'fetch' feature" error for content reads).

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/expression/functions.rs darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
git commit -m "feat(darkmatter): make read-side expression functions URL-aware behind fetch feature"
```

---

## Phase 4 — Thread `allowed_hosts` From the Compose Pipeline

The `ResolutionContext` returned by the compose evaluator's `EvaluationLookup::resolution_context()` must populate `allowed_hosts` from `ComposeOptions`, so remote reads during composition honor the same deny-all-by-default policy.

### Task 4.1: `ComposeOptions.allowed_hosts` → `ResolutionContext`

**Files:**
- Modify: wherever `ResolutionContext` is constructed for the compose evaluator (the `EvaluationLookup::resolution_context()` impl added in plan-local Task 4.1)
- Modify: `ComposeOptions` to carry `allowed_hosts: Vec<String>` (default empty)
- Test: end-to-end compose test with `wiremock`

- [ ] **Step 1: Write the failing test**

`darkmatter/lib/tests/remote_compose.rs`:

```rust
#![cfg(feature = "fetch")]

#[tokio::test]
async fn compose_blocks_remote_read_when_host_not_allowed() {
    // A document that interpolates frontmatter from a remote URL must fail
    // composition when the host is not in ComposeOptions.allowed_hosts, and
    // succeed against a wiremock server when the host IS allowed.
    // Mirror the ComposeOptions + compose_with harness in
    // darkmatter/lib/tests/shell_block_integration.rs.
}
```

> Flesh this out against the existing compose integration-test harness (`shell_block_integration.rs` shows the `ComposeOptions` + `compose_with` pattern). Assertion: with no `allowed_hosts`, a `{{ frontmatter("https://host/x.md", "title") }}` interpolation fails composition; with the host allowed, it succeeds against a `wiremock` server.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter --features fetch --test remote_compose`
Expected: FAIL.

- [ ] **Step 3: Implement**

Add `allowed_hosts: Vec<String>` to `ComposeOptions` with a builder `with_allowed_hosts(...)`, default empty (deny-all). In the `EvaluationLookup::resolution_context()` impl, copy `options.allowed_hosts` into the returned `ResolutionContext`. No other code changes — the read functions (Phase 3) already enforce via the policy.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p darkmatter --features fetch --test remote_compose`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/ darkmatter/lib/tests/remote_compose.rs
git commit -m "feat(darkmatter): thread allowed_hosts from ComposeOptions into remote reads"
```

---

## Final Verification

- [ ] **Run with and without the feature**

Run: `cargo test -p darkmatter` then `cargo test -p darkmatter --features fetch`
Expected: both pass.

- [ ] **Lint both configurations**

Run: `cargo clippy -p darkmatter` and `cargo clippy -p darkmatter --features fetch`
Expected: clean.

- [ ] **Docs drift pass**

Update `darkmatter/docs/topics/side-effects.md` (mark `http_post` implemented), `darkmatter-expressions.md` (note URL-accepting function variants), and `darkmatter/docs/dependencies.md` (the optional `fetch` feature). Regenerate skill `hash:` frontmatter with `md hash <file>` if a skill changed.

---

## Follow-Up (optimization, not required for correctness)

- [ ] **Route read-side fetches through url-referencing's concurrent prefetch cache.**
  Phase 3 uses `fetch_blocking` per call (correct but serial). Once url-referencing Phase 2's eager discovery + single-flight remote cache is in place, change `load_markdown`'s remote branch to consult that cache (block on the in-flight slot) instead of issuing a standalone blocking GET. This is the performance payoff described in the url-referencing spec; it does not change observable behavior, only latency. Track as a separate task after both features land.

---

## Self-Review Notes (spec coverage)

- `http_post` side-effect verb, host-allowlist gated (Phase 2). ✅
- URL-accepting variants of read functions: `absolute`, `relative`, `file_exists`, `frontmatter`, `markdown_body_empty`, `markdown_title`, `validate_schema` (Phase 3). ✅
- Network safety enforced via the shared `biscuit-file` `FetchPolicy` (Phases 2–3), configured from `ComposeOptions` for compose-time reads (Phase 4). ✅
- Feature-gating so the non-network build is unaffected (Phase 1, gated modules/tests). ✅
- **Dependency on `url-referencing` stated as a hard prerequisite** (header). ✅
- **Flagged interface assumption:** url-referencing must expose a POST entry point (`post_blocking`) alongside `fetch_blocking` (Phase 2). If it does not, that is a coordination item to raise on the url-referencing plan, not something to work around here.
</content>
