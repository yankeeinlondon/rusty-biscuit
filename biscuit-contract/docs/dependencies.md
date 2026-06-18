# biscuit-contract Dependencies

`biscuit-contract` is a contract crate; it stays small on purpose so it can be
imported by deterministic consumers (Reaper, Darkmatter, and any future
provider-neutral crate) without pulling in a runtime, network client, JSON
Schema engine, or another provider's transitive dependencies. The rules below
are enforced in code review; the spec in
`reaper/features/2026-06-03-inference-trait/spec.md` is the source of truth
if this file ever drifts.

## Allowed Production Dependencies

Three production dependencies are permitted in v1. Each one earns its place
by being the minimum tool that makes the contract expressible in stable
Rust 2024:

- `async-trait` — provides the `#[async_trait]` macro that keeps the public
  `InferenceAdapter` trait object-safe so consumers can store adapters as
  `Arc<dyn InferenceAdapter>`. Without it, async fn in traits is stable, but
  `dyn InferenceAdapter` is not — and `Send + Sync` dispatch across threads
  is the whole point of the trait.
- `serde_json` — used to carry JSON Schema (`InferenceOutput::Structured`)
  and structured response payloads (`InferenceData::Structured`) as
  `serde_json::Value`. The contract itself does not validate against the
  schema; validation is the adapter's responsibility. `serde_json` is the
  smallest stable choice that lets consumers pass arbitrary JSON Schema
  documents without re-serialising them.
- `thiserror` — the repository-standard `derive(Error)` helper, used to
  give `InferenceError` a real `std::error::Error` and `Display` impl
  without forcing consumers to special-case a diagnostic-only struct. A
  hand-rolled impl would not be smaller, and it would drift from the rest
  of the monorepo.

## Allowed Dev-Dependencies

The `dev-dependencies` table may include test-only crates that do not leak
into the production build graph. `tokio` (with the `macros` and `rt`
features) is the only one needed today — it powers `#[tokio::test]` in the
deterministic L1 suite. Tokio is forbidden in `[dependencies]`, but it is
acceptable in `[dev-dependencies]` because it is not re-exported and is
never linked into consumer binaries.

## Forbidden Dependency Classes

`biscuit-contract` must not depend on any of the following, in either
`[dependencies]` or `[dev-dependencies]`, except where a class is explicitly
noted above:

- **Async runtimes** such as `tokio` (production), `async-std`, or `smol`.
  Adapters own the runtime boundary; the contract must stay
  runtime-agnostic so it can be reused across consumers. Tokio is
  permitted only in `[dev-dependencies]` for tests.
- **HTTP clients** such as `reqwest`, `hyper`, or `ureq`. Provider-specific
  transport is the adapter's concern.
- **JSON Schema engines** such as `jsonschema`. Each adapter validates
  structured output against the supplied schema using its own engine so
  consumers that only need prose inference are not forced to compile one.
- **TUI, terminal, browser, audio, Markdown, hashing, or other
  consumer-oriented crates** — anything that ties the contract to a
  particular rendering or interaction target.
- **Any consumer or provider crate**, including `reaper`, `darkmatter`,
  `claudine`, `unchained-ai`, and their transitive dependencies. Sharing
  types across the contract is the entire point; inverting the direction
  would re-create the coupling this crate exists to remove.

## Adding a New Dependency

Any new entry in `[dependencies]` requires a corresponding update to this
file explaining why no existing allowed crate can fill the role. PRs that
add a dependency without updating this file will be rejected in review.
