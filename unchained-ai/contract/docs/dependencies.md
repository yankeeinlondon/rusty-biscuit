# unchained-ai-contract Dependencies

`unchained-ai-contract` is the single crate where the tiny `biscuit-contract`
inference contract meets the large `unchained-ai` library. It implements
`biscuit_contract::inference::InferenceAdapter` by running a single-turn
completion through the `unchained-ai` execution surface and model resolver.
Consumers (Reaper, Darkmatter) depend only on `biscuit-contract` and inject
this adapter at their composition root.

## Dependency Direction

```text
biscuit-contract
  ^                      unchained-ai (lib)
  |                        ^
  +---- unchained-ai-contract -+
```

`unchained-ai-contract` depends on **both** `biscuit-contract` and
`unchained-ai`. It must **not** depend on `unchained-ai-cli`, and must not
re-export provider-native types through the contract surface.

## Internal Dependencies

- `biscuit-contract` (`../../biscuit-contract/lib`) — the `InferenceAdapter`
  trait and the request/response/error/profile types it implements.
- `unchained-ai` (`../lib`) — the execution surface (`complete`,
  `complete_blocking`, `CompletionBackend`), the capability-based model
  resolver (`models::selection`), provider/model abstractions
  (`ProviderModel`, `Provider`, `ProviderModelMetadata`), and provider-error
  types.

## Production Dependencies

- `async-trait` — matches the contract's default `#[async_trait]` (`Send`)
  bound so the adapter is object-safe as `Arc<dyn InferenceAdapter>`.
- `tokio` (features `rt`, `macros`) — runs the adapter's async `infer`
  implementation and the execution surface's tests. Already an
  `unchained-ai` dependency.
- `serde_json` — schema and structured-payload `Value`s.
- `jsonschema` (`0.42`, `default-features = false`) — adapter-owned JSON Schema
  (Draft 2020-12) validation. The contract crate deliberately bundles no schema
  engine, so validation lives here. Pinned to the workspace-wide `0.42` used by
  `darkmatter` and `schematic-gen`.
- `thiserror` — repository-standard error derive (kept available for any
  adapter-internal error type; all public failures surface as
  `biscuit_contract::inference::InferenceError`).
- `tracing` — redacted diagnostics. Provider detail (raw error text, auth
  headers, API keys) is logged here only and never placed in
  `InferenceError::message`.

## Dev Dependencies

- `tokio` (test features incl. `rt-multi-thread`) — `#[tokio::test]` for the
  L1 fake-backend suite and the gated `real_` provider tests.
- `serial_test` — isolates tests that mutate the global fake-backend factory.
- `wiremock` — available for any HTTP-level seams that are added in future
  tests.

## Forbidden Boundary

The contract surface must not expose `rig-core` types, provider-native
request/response types, or `unchained-ai` internals beyond what is required to
implement `InferenceAdapter`. Consumers hold `Arc<dyn InferenceAdapter>` and
know nothing about the provider stack underneath.

## Update Procedure

When adding or removing a dependency here, update this file, the root
[`docs/dependencies.md`](../../../docs/dependencies.md), and confirm the crate
still does not depend on `unchained-ai-cli` or re-export provider-native types.
