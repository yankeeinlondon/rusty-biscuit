---
ready: false
agent: ""
model: ""
---

# Centralized Providers Review 1

## Verdict

Not ready for production. The implementation moves a lot of provider data into `claudine::provider`, but several explicit success criteria from the spec/design are either not implemented or are guarded by tests that intentionally allow the remaining drift.

## Findings

### 1. `ProviderInfo` JSON does not round-trip the centralized provider catalog

Severity: high

The spec's success criteria require `claudine providers --describe --format json` to serialize `ProviderInfo` without information loss. The implementation skips almost every field that makes the catalog centralized: `stream_protocol`, `event_mapping`, all behavior traits, agent/resource capability accessors, typed path templates, output formats, entrypoints, system prompt delivery, yolo, reasoning, known gaps, ACP, and prompt arg conventions are all `#[serde(skip)]` in `claudine/lib/src/provider/mod.rs`.

Evidence:

- `claudine/lib/src/provider/mod.rs:116`
- `claudine/lib/src/provider/mod.rs:128`
- `claudine/lib/src/provider/mod.rs:158`
- `claudine/lib/src/provider/mod.rs:178`
- `claudine/lib/src/provider/mod.rs:195`
- `claudine/lib/src/provider/mod.rs:203`
- `claudine/lib/src/provider/mod.rs:226`
- `claudine/lib/src/provider/mod.rs:233`
- `claudine/cli/src/commands/providers.rs:107`

The CLI command does serialize `Vec<&ProviderInfo>`, but the output is an identity stub, not the catalog. The unit test only asserts `provider`, `display_name`, `slug`, and `docs_url`, so it cannot catch the loss. Either expose a separate serializable DTO containing the static catalog fields, or make the static fields serialize and explicitly skip only trait objects/callbacks.

### 2. The "zero lib-side `match Provider` outside registry" goal is not met

Severity: high

The spec says lib-side `match Provider` outside `claudine::provider::registry` should drop to zero. The implementation still has direct provider dispatch in `provider/methods.rs`, `model_catalog/provider_sources.rs`, `composition/select.rs`, `permissions/query.rs`, and test fixtures. The guard test also allow-lists several of those files instead of failing them, and it only searches for literal `match provider`, so it misses `match self`, `match p`, and many `Provider::... =>` arms.

Evidence:

- `claudine/lib/src/provider/methods.rs:20`
- `claudine/lib/src/provider/methods.rs:139`
- `claudine/lib/src/provider/methods.rs:152`
- `claudine/lib/src/provider/methods.rs:187`
- `claudine/lib/src/model_catalog/provider_sources.rs:16`
- `claudine/lib/src/model_catalog/provider_sources.rs:28`
- `claudine/lib/src/provider/tests.rs:234`
- `claudine/lib/src/provider/tests.rs:256`
- `claudine/lib/src/provider/tests.rs:290`

Some identity helpers may be reasonable to keep, but the current state does not match the design's production bar. At minimum, replace the permissive source-string scan with a stronger structural guard, remove the compatibility allow-list for model catalog/composition/permissions, and move model/env/permission metadata onto `ProviderInfo`.

### 3. Wrapper thinning is incomplete and the wrapper registry still has a hidden future-provider failure mode

Severity: medium

The spec calls for `WrapperProfile` default implementations to consume typed catalog data for yolo, output formats, entrypoints, model flags, resume support, and prompt conventions, with the file shrinking by at least 40%. The implementation only centralizes a small subset of defaults (`binary`, `agent_env`, stream protocol, prompt conventions). `apply_yolo`, `apply_entrypoint`, `apply_output_format`, and `supports_resume` still require per-provider overrides/defaults that do not consume the typed catalog.

Evidence:

- `claudine/cli/src/commands/wrap/profile.rs:297`
- `claudine/cli/src/commands/wrap/profile.rs:340`
- `claudine/cli/src/commands/wrap/profile.rs:378`
- `claudine/cli/src/commands/wrap/profile.rs:537`
- `claudine/cli/src/commands/wrap/profile.rs:593`
- `claudine/cli/src/commands/wrap/profile.rs:604`

The file is still 3018 lines, effectively unchanged from the design's stated 3057-line starting point. Also, `Provider::RooCode | _ => None` means any future provider added inside the crate will quietly return `None` instead of forcing an explicit wrapper decision. Replace that wildcard with a precise Roo arm and derive the trivial wrapper operations from `ProviderInfo`.

### 4. Strong typing was added alongside legacy string metadata rather than replacing it

Severity: medium

Phase 5 says descriptive `Vec<&'static str>` fields should be replaced by typed equivalents except for `notes`. The implementation adds typed fields on `ProviderInfo`, but the legacy `AgentCapabilities` model still carries stringly typed entrypoints, output formats, replacement mechanisms, yolo equivalent, session/log locations, and free-form gaps. Provider modules still populate those legacy fields.

Evidence:

- `claudine/lib/src/agents/model.rs:77`
- `claudine/lib/src/agents/model.rs:79`
- `claudine/lib/src/agents/model.rs:89`
- `claudine/lib/src/agents/model.rs:96`
- `claudine/lib/src/agents/model.rs:120`
- `claudine/lib/src/agents/model.rs:260`
- `claudine/lib/src/provider/claude.rs:492`
- `claudine/lib/src/provider/claude.rs:507`
- `claudine/lib/src/provider/claude.rs:523`
- `claudine/lib/src/provider/claude.rs:538`

This keeps two sources of truth alive: typed data on `ProviderInfo`, and descriptive runtime capability strings in `AgentCapabilities`. Finish the migration by either replacing the legacy fields in `AgentCapabilities` or deriving the legacy display shape from the typed catalog until the public compatibility surface is removed.

### 5. Test coverage is mostly invariant/unit-level; required snapshot and integration gates are missing

Severity: medium

The design requires snapshot tests for `claudine providers --describe --format json`, `claudine hooks --support`, `claudine hooks --mapping`, and `claudine hooks --describe`, plus smoke coverage for wrappers and `init --quick`. I found only narrow serialization/unit checks for providers and hook-count assertions, not golden snapshots of the actual CLI output surfaces.

Evidence:

- `claudine/cli/src/commands/providers.rs:199`
- `claudine/lib/src/provider/tests.rs:49`
- `claudine/lib/src/provider/tests.rs:306`
- `claudine/lib/src/provider/tests.rs:384`

This is a risky refactor because the main promise is "same behavior, centralized metadata." Add command-level snapshot tests for the named surfaces and a focused integration test that compares legacy facade outputs (`agents`, linking, event matrices, model catalog, wrapper args) against the new catalog-derived outputs.

## Additional Notes

- `Provider` is still `#[non_exhaustive]` and lacks a compile-time variant-count/registry-length check. The current registry is an exhaustive `match`, not the array-backed shape from the design, so new-provider drift is still mostly runtime/test-time.
- `building-an-agent-wrapper.md` now has a migration history, but the document still contains generated prompt/frontmatter text and still documents some old split-brain surfaces. It should be cleaned before this feature is considered closed.

## Suggested Closure Work

1. Make `ProviderInfo` serialization useful: serialize all static data or introduce a serializable catalog DTO.
2. Remove or justify every lib-side `Provider` dispatch outside `provider/registry.rs`; strengthen the guard test so it cannot be bypassed by naming the variable differently.
3. Finish wrapper default derivation from typed catalog data and remove the wildcard in `profile_for_provider`.
4. Replace legacy string metadata instead of duplicating it.
5. Add the required CLI snapshots and at least one integration test per migrated facade.
