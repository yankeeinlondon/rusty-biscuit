# Supplemental Design: Provider Module Split & Codegen Landing Zone

> **Status:** draft for Ken's review. Refines spec.md "DRY / Module Cogency
> Workstreams" items 1–2 and the `provider/<slug>/{data.rs, behavior.rs}` sketch.
> Lib-crate scope only; the CLI crate is design/pipeline-dry.md.

## Current reality the split must handle

Per-provider modules are flat files (`lib/src/provider/claude.rs`, ~718 lines) each
carrying THREE things: the `ProviderInfo` const, the four behavior-trait impls
(`ProviderBehavior`/`McpBehavior`/`AdapterBehavior`/`ConfiguratorBehavior`), and
~300 lines of legacy `AgentCapabilities` LazyLock builders. Stream parsers do NOT live
here — they live in `stream/providers/` behind the allow-listed factory (a consequence
of the `provider_id` leaf split that broke the provider⇄stream import cycle).

## File layout ruling

```
lib/src/provider/<slug>/
    mod.rs        # re-exports only; module wiring
    data.rs       # GENERATED — ProviderInfo const + typed catalog sections
    behavior.rs   # hand-written — the four trait impls
    legacy.rs     # TEMPORARY — AgentCapabilities builders, deleted at retirement
```

- **Stream parsers and adapters stay where they are** (`stream/providers/`,
  `adapters/`). Moving them into `provider/<slug>/` would re-entangle the import cycle
  the `provider_id` leaf was created to break, churn the drift-guard allow-list, and
  couple the split (workstream 1) to a parser reorganization nobody needs. The spec's
  "behavior traits, stream parsers, adapters, configurators" enumeration is amended:
  `behavior.rs` holds the four trait impls only; parsers/adapters remain factory-wired.
- **`legacy.rs` exists so the split does not block on retirement.** Workstream 1
  mechanically relocates the LazyLock builders there; workstream 2 deletes the file.
  Nothing new may be added to any `legacy.rs` (guard: a test asserts the set of
  `legacy.rs` files only shrinks).
- `data.rs` is produced per design/catalog-generation.md (roster + facts + research +
  overrides; wholly generated; drift-tested).

## AgentCapabilities retirement (workstream 2)

Verified consumer inventory (non-test): the accessor methods on `agents/model.rs`
itself, the `agents/registry.rs` facade (already forwarding to `provider_info()`), and
`cli/src/commands/providers.rs` (the describe/matrix output).
`permissions/engine.rs`'s `.capabilities()` is the permissions surface, not this tree
— untouched.

Order:

1. Migrate `providers.rs` describe output to read `ProviderInfo` + the committed
   `catalog.json` superset (design/catalog-generation.md) — this is also the moment it
   moves onto renderable components (design/render-components.md).
2. Delete the `agents::Agent` trait, `agent_for` registry, and the 80-field struct
   together — the trait exists only to serve the tree; there is no partial retirement.
3. Delete each provider's `legacy.rs`; drop the agreement tests.

Codegen never emits the legacy tree (spec already decides this); therefore workstream
1's `legacy.rs` relocation must land **before** the first generated `data.rs`, so
generation never has to know the tree existed.

## Drift-guard allow-list edits (lib)

The lib guard (`provider/tests.rs::no_unauthorized_match_provider_in_lib`, 6-file
allow-list) needs mechanical updates for the new paths
(`provider/<slug>/behavior.rs` replacing `provider/<slug>.rs`) and nothing else. The
guard's pattern gaps (`matches!`, `if provider ==`) are fixed in the CLI-guard work
(design/pipeline-dry.md) and back-ported to the lib guard in the same change.

## Sequencing summary

```
split (mechanical, includes legacy.rs)  →  first generated data.rs (Phase 1)
        ↘ retirement (workstream 2) — may interleave after providers.rs migrates
```

The split itself is behavior-preserving and reviewable as pure code motion; the
scope-discipline rule applies (no drive-by edits inside moved code).
