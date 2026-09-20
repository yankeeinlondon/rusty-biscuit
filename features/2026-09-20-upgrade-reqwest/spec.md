---
status: draft
created: 2026-09-20
owner: Ken Snyder <ken@ken.net>
origin: reqwest `system-proxy` removal and the 2026-09-20 dependency audit
packages:
    - messenger
    - biscuit-location
    - biscuit-browser-harness
    - darkmatter
    - schematic-oauth
---

# Retire reqwest 0.12 from the workspace

## Outcome

`reqwest 0.12` is gone from `Cargo.lock`. Every first-party crate that ships
an HTTP client is on `reqwest 0.13`, with `default-features = false` and its
TLS backend named explicitly, and the one external crate that still pinned
0.12 for a feature we use is upgraded past it.

## Where things stand

The 2026-09-20 audit found 20 `reqwest` declarations across 19 packages
(`schematic-schema` is declared twice: source and generated copy). All 20
already carry `default-features = false` after the `system-proxy` removal.
Three first-party crates are still on **0.12**:

| crate | declaration | why it is on 0.12 |
|---|---|---|
| `messenger` | `0.12`, `default-tls, charset, http2, json, multipart`, optional | Written 2026-03-09 and never revisited. Nothing in its graph requires 0.12. |
| `biscuit-location` | `0.12`, `json, rustls-tls`, no defaults | Same: a free choice, never revisited. Nothing requires 0.12. |
| `schematic-oauth` | `0.12`, `rustls-tls, json`, no defaults | **Genuinely pinned** by `oauth2 5.0.0`, which depends on `reqwest 0.12` and implements its `AsyncHttpClient` trait for *that* `reqwest::Client` type. |

The lockfile carries `reqwest 0.12.28` for five dependents: the three above
plus two external crates, `oauth2` (via `schematic-oauth`) and
`chromiumoxide 0.7` (via `biscuit-browser-harness` and darkmatter's
`browser-tests` feature). Both external holders have a way out —
`chromiumoxide` by upgrade, `oauth2` by the newtype below — so **removing
0.12 from the lock entirely is the goal**, not merely first-party code.

## What changes with 0.13, per crate

These are the facts each migration must state in its commit, because they
are behavior changes rather than version bumps.

### `messenger` — TLS backend flips

`default-tls` resolves to **native-tls** under 0.12 and to **rustls +
`rustls-platform-verifier`** under 0.13. Bumping the version with no other
change therefore moves messenger from Security.framework / SChannel to
rustls, which is what every other 0.13 consumer here, darkmatter included,
already uses. Certificate validation still consults the OS trust store
through the platform verifier.

Verified 2026-09-20: with only the version changed, `cargo check -p
messenger --all-targets` passes on 0.13 with no source edits.

### `biscuit-location` — feature rename *and* root-store change

0.13 has no `rustls-tls` feature; the equivalent is `rustls`. That rename is
mechanical. What it carries is not: 0.12's `rustls-tls` means
`rustls-tls-webpki-roots` — a **bundled** Mozilla root set that ignores the
OS trust store entirely — whereas 0.13's `rustls` uses the platform verifier
and the **OS** trust store. 0.13 offers no bundled-roots option.

Consequences for a crate that downloads MaxMind databases: a corporate CA or
inspecting proxy installed in the OS store starts being trusted (today it
fails the handshake), and a root the OS has distrusted stops being trusted
(today the bundled copy still honors it). This aligns `biscuit-location` with
every other rustls consumer in the workspace, which is the right end state,
but it is a change a user behind a private CA will notice — in the direction
of "starts working".

Verified 2026-09-20: with `"0.12"` → `"0.13"` and `rustls-tls` → `rustls`,
`cargo check -p biscuit-location --all-targets` passes with no source edits.

### `chromiumoxide` — an upgrade, not a pin

`chromiumoxide 0.7.0` declares `reqwest = "0.12"` as a **non-optional**
dependency, so no feature we disable removes it. `0.9.0` (2026-02-20)
"Update `reqwest` to v0.13"; `0.9.1` is current. Across 0.8 and 0.9 the only
breaking change is the removal of async-std, which we never used. Our whole
API surface is two files (`biscuit-browser-harness/src/lib.rs`, darkmatter's
browser tests) and six paths.

Verified 2026-09-20, `0.7` → `0.9`, both manifests: the compile needs exactly
two changes. The `tokio-runtime` feature no longer exists (0.8 made tokio the
default; 0.9 made it the only runtime), so both declarations drop it. And
`BrowserConfigBuilder::arg` now takes a struct `Arg` with `From<&str>` /
`From<String>` but not `From<&String>`, so `builder.arg(arg)` at
`biscuit-browser-harness/src/lib.rs:551` becomes `builder.arg(arg.as_str())`.
With those, `cargo check -p biscuit-browser-harness -p darkmatter --features
darkmatter/browser-tests --all-targets` passes and `chromiumoxide` leaves the
0.12 holders list.

Two 0.9 changes are behavioral and must be checked by running the browser
tier, not by compiling: "Browser process no longer inherits stdout" (the
harness may have relied on it for Chrome's own diagnostics), and the bumped
CDP / fetcher revisions (`r1566079` / `r1585606`). `just test-browser` in
`darkmatter/` is the gate; skips cleanly when Chrome is absent, so run it
where Chrome is present.

### `schematic-oauth` — pinned, with one escape route

`schematic-oauth` builds its own `reqwest::Client` (`manager/mod.rs:150`) and
hands `&self.http_client` to `oauth2`'s `request_async` in four places
(`authorization_code.rs:87`, `client_credentials.rs:25`, `refresh.rs:57`,
`revocation.rs:32`). `oauth2 5.0.0` implements `AsyncHttpClient<'c> for
reqwest::Client` in `src/reqwest_client.rs` — for **0.12**'s `Client`. A
0.13 `Client` is a different type and does not satisfy the trait, so a bare
version bump cannot compile. `oauth2 5.0.0` is the newest release; there is
no upstream version on 0.13 as of 2026-09-20.

The escape route exists because `oauth2`'s trait is not reqwest-shaped:

```rust
pub type HttpRequest  = http::Request<Vec<u8>>;    // oauth2/src/endpoint.rs:17
pub type HttpResponse = http::Response<Vec<u8>>;   // oauth2/src/endpoint.rs:20
```

Both `oauth2` (`http 1.0`) and `reqwest 0.13` (`http 1.1`) are on `http 1.x`,
so a local newtype around a 0.13 client can implement `AsyncHttpClient` by
converting `http::Request<Vec<u8>>` to a reqwest request and the reply back
to `http::Response<Vec<u8>>`. With that in place `schematic-oauth` can set
`oauth2 = { version = "5", default-features = false }` — dropping `oauth2`'s
own `reqwest` + `rustls-tls` defaults — and 0.12 leaves its graph entirely.
This is real work (a trait impl, redirect-policy parity with the current
`Policy::none()` client, and error mapping into `OAuthError::Http`), not a
manifest edit.

## Scope

Four independent changes, each its own commit with its own suite run:

1. **`messenger`** — version `"0.12"` → `"0.13"`. Feature list unchanged.
   Commit message states the native-tls → rustls change.
2. **`biscuit-location`** — version `"0.12"` → `"0.13"`, feature
   `rustls-tls` → `rustls`. Commit message states the bundled-roots → OS
   trust store change.
3. **`chromiumoxide`** — `"0.7"` → `"0.9"` in `biscuit-browser-harness/Cargo.toml`
   and `darkmatter/lib/Cargo.toml`, dropping the `tokio-runtime` feature from
   both; `builder.arg(arg.as_str())` in the harness. Gated by
   `just test-browser`, not `just test`.
4. **`schematic-oauth`** — decide between:
   - **(a) wait** for an `oauth2` release on `reqwest 0.13`, recording the
     upstream tracker here; or
   - **(b) the newtype** described above, plus `oauth2` with
     `default-features = false`.

   (b) is the only path that actually empties the lock of 0.12. Recommend
   (b) unless upstream is imminent; either way the decision and its date go
   in this spec.

Out of scope: any change to the `system-proxy` policy, which every
declaration already carries.

## Acceptance criteria

1. `messenger`, `biscuit-location`, and (under 4b) `schematic-oauth` declare
   `reqwest = { version = "0.13", default-features = false, … }`;
   `biscuit-browser-harness` and `darkmatter` declare `chromiumoxide = "0.9"`.
2. `cargo tree --workspace -e normal -i reqwest@0.12.28` reports **nothing**
   — under 4a, `oauth2` alone, recorded as the one deliberate residue.
3. `default-features = false` and an explicit TLS feature on every changed
   line; `cargo tree --workspace --all-features -e features` shows no
   `reqwest feature "system-proxy"` and no `hyper-util feature
   "client-proxy-system"`.
4. `just test` and `just lint` pass in each changed package area. For
   `messenger` that is its ~330 library tests plus the CLI; for
   `biscuit-location` the L1 suite; for `schematic-oauth` under 4b, the
   OAuth flow tests exercising all four `request_async` call sites through
   the newtype; for `chromiumoxide`, `just test-browser` in `darkmatter/` on
   a host with Chrome.
5. Each commit message names the behavior change from the section above.
   A reviewer reading only the message must learn that messenger changed TLS
   backends and that biscuit-location changed root-store source.
6. `docs/dependencies.md`'s reqwest note is updated to say all first-party
   code is on 0.13 and to name the remaining external 0.12 holders.

## Notes

- The `system-proxy` removal already applied to the 0.12 lines, so none of
  this work affects that fix; it is independent cleanup surfaced by the same
  audit.
- `schematic-schema` appears twice in the audit because
  `schematic/gen/schematic/schema/` is a generated copy of
  `schematic/schema/`. Both are already on 0.13; neither is in scope.
- Trial bumps for `messenger`, `biscuit-location`, and `chromiumoxide` were
  compiled and then reverted on 2026-09-20 so they would not ride into an
  unrelated merge commit. They are not in the tree.
- An earlier draft of this spec said 0.12 could not leave the lock because of
  `chromiumoxide`. That was wrong: 0.7's `reqwest` is non-optional, but 0.9
  is on 0.13. Corrected the same day.
