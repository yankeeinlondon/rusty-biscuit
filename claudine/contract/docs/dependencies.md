# claudine-contract Dependencies

`claudine-contract` is the single crate where the tiny `biscuit-contract`
inference contract meets the large `claudine` library. It implements
`biscuit_contract::inference::InferenceAdapter` by running a Claudine
non-interactive, tool-free agentic-CLI session. Consumers (Reaper, Darkmatter)
depend only on `biscuit-contract` and inject this adapter at their composition
root.

## Dependency Direction

```text
biscuit-contract
  ^                      claudine (lib)
  |                        ^
  +---- claudine-contract -+
```

`claudine-contract` depends on **both** `biscuit-contract` and `claudine`. It
must **not** depend on `claudine-cli`, and must not re-export provider-native
types through the contract surface.

## Internal Dependencies

- `biscuit-contract` (`../../biscuit-contract/lib`) — the `InferenceAdapter`
  trait and the request/response/error/profile types it implements.
- `claudine` (`../lib`) — the provider registry (`provider_info`, typed
  entrypoints, output formats, prompt conventions, system-prompt and reasoning
  descriptors), the semantic stream parser (`create_semantic_parser`,
  `StreamExecutionSummary`), and the model-catalog source metadata.

## Production Dependencies

- `async-trait` — matches the contract's default `#[async_trait]` (`Send`)
  bound so the adapter is object-safe as `Arc<dyn InferenceAdapter>`.
- `tokio` (features `process`, `io-util`, `rt`, `macros`, `time`, `sync`) —
  spawns the provider process and streams its stdout. Already a `claudine`
  dependency.
- `serde_json` — schema and structured-payload `Value`s.
- `jsonschema` (`0.42`, `default-features = false`) — adapter-owned JSON Schema
  (Draft 2020-12) validation. The contract crate deliberately bundles no schema
  engine, so validation lives here. Pinned to the workspace-wide `0.42` used by
  `darkmatter` and `schematic-gen`.
- `tempfile` — the ephemeral, isolated working directory each session runs in.
- `thiserror` — repository-standard error derive (kept available for any
  adapter-internal error type; all public failures surface as
  `biscuit_contract::inference::InferenceError`).
- `tracing` — redacted diagnostics. Provider/session detail (stderr, raw error
  text, exit codes) is logged here only and never placed in
  `InferenceError::message`.

## Dev Dependencies

- `tokio` (test features incl. `rt-multi-thread`) — `#[tokio::test]` for the L1
  fake-runner suite and the gated `real_` provider tests.
- `tracing-subscriber` (registry layer only) — captures `tracing::warn!` events
  in tests that verify secret-free error messages after shadow-home failures.

## Update Procedure

When adding or removing a dependency here, update this file, the root
[`docs/dependencies.md`](../../../docs/dependencies.md), and confirm the crate
still does not depend on `claudine-cli` or re-export provider-native types.
