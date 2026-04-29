---
ready: false
agent: ""
model: ""
review: review-1.md
---

# Centralized Providers — Review 1 Closure Plan

This plan addresses every finding, evidence pointer, additional note, and
suggested closure work item in
[`review-1.md`](./review-1.md). It is staged so each phase compiles, has
its own test green-light, and ends lint-clean for the `claudine` package
area (lib + cli). Earlier phases lay structural foundations that later
phases depend on; the final phase enforces the full repo-area gate.

## Source Coverage Map

Every review item is mapped to a phase below. No item is dropped.

| Review Item | Severity | Phase |
|---|---|---|
| F1: `ProviderInfo` JSON does not round-trip the centralized catalog (`mod.rs:116/128/158/178/195/203/226/233`, `providers.rs:107`) | high | 1 |
| F2: Lib-side `match Provider` outside registry not at zero (`methods.rs:20/139/152/187`, `model_catalog/provider_sources.rs:16/28`, `provider/tests.rs:234/256/290`) | high | 2 |
| F3a: Wrapper thinning incomplete (`profile.rs:297/340/378/537/593/604`) | medium | 3 |
| F3b: `Provider::RooCode \| _ => None` wildcard hides future-provider drift (`profile.rs:604`) | medium | 3 |
| F4: Strong typing added alongside legacy strings (`agents/model.rs:77/79/89/96/120/260`, `provider/claude.rs:492/507/523/538`) | medium | 4 |
| F5: Snapshot + integration gates missing (`providers.rs:199`, `provider/tests.rs:49/306/384`) | medium | 5 |
| AN1: `Provider` lacks compile-time variant-count / array registry | note | 2 |
| AN2: `building-an-agent-wrapper.md` still has generated prompt text and old surfaces | note | 6 |
| Suggested 1: serialize all static catalog data (or DTO) | — | 1 |
| Suggested 2: remove or justify every lib-side `Provider` dispatch outside `provider/registry.rs`; harden guard | — | 2 |
| Suggested 3: finish wrapper default derivation; remove wildcard | — | 3 |
| Suggested 4: replace legacy string metadata instead of duplicating | — | 4 |
| Suggested 5: add CLI snapshots + per-facade integration test | — | 5 |

---

## Phase 1 — Make `ProviderInfo` JSON round-trip the catalog

### Goals

- Resolve **Finding 1** and **Suggested 1**.
- `claudine providers --describe --format json` emits the full static catalog,
  not an identity stub.
- The unit/integration tests fail loudly if a `#[serde(skip)]` is added back
  on a static field.

### Files to edit

- `claudine/lib/src/provider/mod.rs` (frontmatter lines 116, 128, 158, 178, 195, 203, 226, 233)
- `claudine/lib/src/provider/event_mapping.rs` (ensure `EventMappingTable`, `EventMapping`, `EventSupportLevel` derive `Serialize`)
- `claudine/lib/src/provider/path_template.rs` (ensure `PathTemplate`, `PathSegment`, `GlobKind` derive `Serialize`; expose a stable serialized shape — `Serialize` via `raw()` or `Templated { segments: [...] }` — round-tripable)
- `claudine/lib/src/provider/output_format.rs` (`OutputFormatSupport`, `EntrypointSpec`, `EntrypointMode`, `OutputFormat` — all `Serialize`)
- `claudine/lib/src/provider/system_prompt.rs` (`SystemPromptSpec`, `SystemPromptDeliveryByMode`, `SystemPromptDelivery`, `SystemPromptCustomTag` — all `Serialize`)
- `claudine/lib/src/provider/yolo.rs` (`YoloSupport` — `Serialize`)
- `claudine/lib/src/provider/reasoning.rs` (`ReasoningSupport`, `ReasoningCustomTag` — `Serialize`)
- `claudine/lib/src/provider/known_gap.rs` (`KnownGap`, `KnownGapArea` — `Serialize`)
- `claudine/lib/src/provider/acp.rs` (`AcpSupport`, `AcpServerMode`, `AcpEvent` — `Serialize`)
- `claudine/lib/src/provider/prompt_args.rs` (`PromptArgConventions` — `Serialize`)
- `claudine/lib/src/provider/behavior.rs` (no change to behavior trait fields; they MUST stay `#[serde(skip)]`)
- `claudine/lib/src/stream/mod.rs` (`StreamProtocol` — confirm `Serialize`; add if missing)
- `claudine/cli/src/commands/providers.rs` (extend `describe_json_serializes_all_providers` test to require the now-serialized fields)

### Implementation steps

1. Audit every `#[serde(skip)]` in `provider/mod.rs`. Categorize into:
   - **Trait objects / fn pointers** → keep skipped (`behavior`, `mcp`, `adapter`, `configurator`, `agent_capabilities_fn`, `resource_support_fn`). These are not data.
   - **Static catalog data** → remove the `#[serde(skip)]`. This list is:
     `stream_protocol`, `event_mapping`, `session_log_paths`, `session_locations`,
     `config_paths`, `memory_files`, `output_formats`, `entrypoints`,
     `system_prompt`, `yolo`, `reasoning`, `known_gaps`, `acp`,
     `prompt_arg_conventions`.
2. Add `#[derive(Serialize)]` (and `Deserialize` only if the type is also
   parsed back, otherwise `Serialize` is sufficient) to every transitively
   reachable type listed under "Files to edit" above. Use
   `#[serde(rename_all = "snake_case")]` on enums for stable, snake_case JSON
   keys consistent with the existing `Provider` serialization.
3. For `PathTemplate`, choose ONE stable serialization shape. Recommended:
   serialize as `{ "raw": "<string>", "segments": [ ... ] }` so consumers can
   read the human-readable raw form AND the structured segments. Ensure the
   form is stable across Rust versions.
4. For trait-object accessors that today live as helper `fn() -> &'static T`
   (`agent_capabilities_fn`, `resource_support_fn`): keep them skipped, but
   add a custom `Serialize` impl on `ProviderInfo` (or a serde helper module)
   that ALSO emits a `agent_capabilities` and `resource_support` JSON block
   produced by calling those accessors. This satisfies the spec's "round-trip
   the centralized provider catalog" requirement without resurrecting
   serializable fn pointers.
5. Update the existing serialization-shape note in the doc comments on
   `ProviderInfo` to state that JSON is now the authoritative descriptive
   surface.
6. Update `providers.rs::run_describe(ProvidersFormat::Json)` so it streams
   the array via `serde_json::to_string_pretty`, but additionally validates
   round-trip in debug mode (a `debug_assert!` that decodes back into
   `serde_json::Value` matches its byte-for-byte re-serialization). The CLI
   does not change observable behavior.

### New / updated tests

- **`claudine/lib/src/provider/tests.rs`**:
    - Replace the assertion in `provider_info_serializes_round_trip` that
      only checks `provider`, `slug`, `display_name` with a thorough check:
      ```text
      for every Provider:
        let info = provider_info(p);
        let json = serde_json::to_value(info).unwrap();
        // Identity
        assert_eq!(json["slug"], info.slug);
        assert_eq!(json["display_name"], info.display_name);
        // Catalog half
        assert!(json.get("event_mapping").is_some());
        assert!(json.get("output_formats").is_some());
        assert!(json.get("entrypoints").is_some());
        assert!(json.get("system_prompt").is_some());
        assert!(json.get("yolo").is_some());
        assert!(json.get("reasoning").is_some());
        assert!(json.get("known_gaps").is_some());
        assert!(json.get("acp").is_some());
        assert!(json.get("prompt_arg_conventions").is_some());
        assert!(json.get("session_log_paths").is_some());
        assert!(json.get("session_locations").is_some());
        assert!(json.get("config_paths").is_some());
        assert!(json.get("memory_files").is_some());
        assert!(json.get("stream_protocol").is_some());
        // Trait objects must NOT serialize
        assert!(json.get("behavior").is_none());
        assert!(json.get("mcp").is_none());
        assert!(json.get("adapter").is_none());
        assert!(json.get("configurator").is_none());
        assert!(json.get("agent_capabilities_fn").is_none());
        assert!(json.get("resource_support_fn").is_none());
      ```
    - Add `provider_info_json_does_not_lose_event_rows`: count rows in
      `info.event_mapping.mappings` and assert
      `json["event_mapping"]["mappings"].as_array().unwrap().len()` matches.
    - Add `provider_info_json_round_trips_well_known_keys`: round-trip for
      Claude (richest catalog) and Goose (acp), checking specific nested
      values (e.g. `json["acp"]["server_mode"] == "native"` for Goose).
- **`claudine/cli/src/commands/providers.rs`** (`mod tests`):
    - Strengthen `describe_json_serializes_all_providers` to assert presence
      of `event_mapping`, `system_prompt`, `yolo`, `acp`, `output_formats`,
      `entrypoints`, `prompt_arg_conventions` keys for every provider.

### Verification

```bash
cargo test -p claudine
cargo test -p claudine-cli
cargo clippy -p claudine --all-targets --all-features -- -D warnings
cargo clippy -p claudine-cli --all-targets --all-features -- -D warnings
```

End-of-phase gate: tests green, no new warnings.

---

## Phase 2 — Eliminate or justify lib-side `match Provider` and harden the guard

### Goals

- Resolve **Finding 2**, **Additional Note 1**, and **Suggested 2**.
- Lib-side `match Provider` outside `provider/registry.rs` (and the canonical
  identity helpers) drops to **zero** OR every retained match is justified by
  unique data that lives on `ProviderInfo`.
- Replace the permissive guard test (which accepts `match provider`
  variations and allow-lists model_catalog / composition / permissions) with
  a structural guard that catches `match self`, `match p`, `match *self`, and
  any `Provider::Variant =>` arm in non-allowlisted files.
- Remove the model-catalog / composition / permissions allow-list entries by
  moving the underlying per-provider data into `ProviderInfo`.
- Add a compile-time variant-count assertion against the registry array
  length.

### Files to edit

- `claudine/lib/src/provider/methods.rs` (lines 17–30, 137–149, 152–179, 187–197, 199–238, 277–289):
    - `cli_aliases`, `sniff_ai_cli`, `as_slug`, `agent_offset`, `docs_url`,
      `usage_dashboard_url` are canonical identity. Move these matches into
      `provider/identity.rs` so the guard's allow-list need only cover
      `provider/identity.rs` and `provider/registry.rs`. The methods become
      `&'static` table lookups (`PROVIDER_IDENTITY[*self as usize]`) backed
      by the same array used by the registry.
    - `supports_skills` becomes `provider_info(self).supports_skills`.
    - `detect_from_payload` (the multi-arm match in lines 152–179) keeps its
      payload-shape match (it matches on JSON shape, not on `Provider`), but
      delegate provider-side detection to `info.adapter.detect(raw)` and
      collapse the function to: try each provider's adapter in display order
      and return the first match. The remaining JSON-shape branches are not
      `match Provider` so are exempt.
    - `registration_native_event_name`, `event_from_shared_native_name`,
      `event_support_level`, `supports_event`, `supports_event_via_hook`,
      `native_event_name` already delegate to `provider_info` — leave alone.
- `claudine/lib/src/model_catalog/provider_sources.rs` (lines 16, 28, 34):
    - Add `pub static_models: &'static [&'static str]` and
      `pub dynamic_source: ModelCatalogSource` (a new typed enum: `None`,
      `Static`, `OpencodeCli`, `OpencodeCliQwenFiltered`) to `ProviderInfo`.
    - Rewrite `static_catalog_for_provider(p)` to read
      `provider_info(p).static_models.iter().map(|s| s.to_string()).collect()`.
    - Rewrite `fetch_provider_catalog(p)` to dispatch on
      `provider_info(p).dynamic_source` instead of on `Provider`.
    - Move the `openai_models()` / `anthropic_models()` const arrays into
      `provider/codex.rs` and `provider/claude.rs` respectively (as
      `&'static [&'static str]` consts referenced from `CODEX_INFO` /
      `CLAUDE_INFO`).
- `claudine/lib/src/composition/select.rs` (lines 361–372):
    - Add `pub model_env_vars: &'static [&'static str]` to `ProviderInfo`.
    - Rewrite `provider_env_vars(p)` to `provider_info(p).model_env_vars`.
    - Move per-provider env-var arrays into the corresponding
      `provider/<name>.rs` files.
- `claudine/lib/src/permissions/query.rs` (lines 1079–1105):
    - Add `pub cli_sensitive_axes: CliSensitiveAxes` to `ProviderInfo`,
      where `CliSensitiveAxes` is a typed bitflags-like struct with
      booleans `read_path`, `write_path`, `traverse_path`,
      `execute_command`, `access_domain`, `use_mcp_server`, `use_mcp_tool`,
      `spawn_subagent`, `switch_mode`, `modify_provider_config`.
    - Rewrite `is_cli_sensitive(p, query)` to read those flags directly off
      `provider_info(p).cli_sensitive_axes` and dispatch on the `query`
      enum (which is not `Provider`, so it is allowed).
- `claudine/lib/src/provider/identity.rs`:
    - Introduce a `&'static [ProviderIdentity; PROVIDER_COUNT]` table
      indexed by `Provider as usize` carrying slug, display name, agent
      offset, sniff binding, docs URL, usage dashboard URL, cli aliases,
      supports_skills.
    - Add `const PROVIDER_COUNT: usize = ...;` (the array length).
    - Add a compile-time assertion: define
      `Provider` with `#[repr(u8)]` and add
      ```rust
      const _: [(); PROVIDER_COUNT] = [(); PROVIDERS_DISPLAY_ORDER.len()];
      ```
      so any drift fails compilation.
- `claudine/lib/src/provider/registry.rs`:
    - Switch from the existing `match`-based registry to the
      `[&'static ProviderInfo; PROVIDER_COUNT]` array described in
      `design.md` §2.1. Lookup is `REGISTRY[p as usize]`. The match is
      eliminated.
- `claudine/lib/src/provider/tests.rs` (lines 215–302):
    - Rewrite `no_unauthorized_match_provider_in_lib` from a string scan
      into a structural scan using `syn` (already a transitive dep via
      build tooling — if not, add as a `dev-dependency`). For each `.rs`
      file in `claudine/lib/src/`:
        1. Parse with `syn::parse_file`.
        2. Walk `syn::Expr::Match`. For each match, inspect arms; if any
           arm's pattern is a path beginning with the canonical `Provider`
           type or a `Provider::Variant` literal, flag the file.
        3. Allow-list ONLY `provider/registry.rs`, `provider/identity.rs`,
           and the test source itself.
    - Remove these allow-list entries:
      `events/provider.rs`, `events/matrix.rs`,
      `adapters/mod.rs`, `agents/registry.rs`,
      `composition/select.rs`, `permissions/query.rs`,
      `model_catalog/provider_sources.rs`.
    - For each removed entry, audit the file and either delete the
      `match Provider` (data moved to `ProviderInfo`) or refactor to the
      new accessor. The work for `composition`, `permissions`, and
      `model_catalog` is enumerated above. For `events/provider.rs`,
      `events/matrix.rs`, `adapters/mod.rs`, and `agents/registry.rs`,
      audit each match; if it dispatches on `Provider`, route it through
      `provider_info(p).<accessor>` or move the data into `ProviderInfo`.
      Document any genuinely irreducible match by relocating it into the
      allow-listed `provider/identity.rs` if it is canonical identity, or
      `provider/registry.rs` if it is the central match.

### New / updated tests

- **`claudine/lib/src/provider/tests.rs`**:
    - The rewritten `no_unauthorized_match_provider_in_lib` is the primary
      gate. It uses `syn` AST walking, not string heuristics.
    - Add `compile_time_provider_count_matches_registry` — at build time
      via `const _: [(); ...]` (already added in identity.rs); the test
      `#[test] fn provider_count_const_is_accurate()` asserts the
      `PROVIDER_COUNT` const equals `PROVIDERS_DISPLAY_ORDER.len()` and
      equals the registry array length.
    - Add `cli_aliases_match_legacy`, `as_slug_matches_legacy`,
      `agent_offset_matches_legacy` — assert `provider_info(p).<field>`
      equals the historical helper output for each provider.
- **`claudine/lib/src/model_catalog/provider_sources.rs`** tests:
    - Add `static_models_align_with_provider_info` — for each provider,
      asserts `static_catalog_for_provider(p)` returns
      `provider_info(p).static_models.iter().map(...).collect()` exactly.
- **`claudine/lib/src/composition/select.rs`** tests:
    - Add `model_env_vars_match_provider_info` — for each provider,
      asserts `provider_env_vars(p)` returns `provider_info(p).model_env_vars`.
- **`claudine/lib/src/permissions/query.rs`** tests:
    - Add `cli_sensitive_axes_round_trip` — for each (provider, query)
      pair tested by the existing logic, the new flags-driven impl returns
      the same boolean as the legacy match.

### Verification

```bash
cargo test -p claudine
cargo test -p claudine-cli
cargo clippy -p claudine --all-targets --all-features -- -D warnings
cargo clippy -p claudine-cli --all-targets --all-features -- -D warnings
```

End-of-phase gate: the structural guard reports zero violators outside the
allow-list `[provider/registry.rs, provider/identity.rs, provider/tests.rs]`.

---

## Phase 3 — Finish wrapper thinning + remove the wildcard

### Goals

- Resolve **Finding 3a**, **Finding 3b**, and **Suggested 3**.
- Replace the `Provider::RooCode | _ => None` wildcard with an exhaustive
  match that explicitly returns `None` for `Provider::RooCode` and is forced
  to compile-error if a future provider variant is added.
- Convert `apply_yolo`, `apply_entrypoint`, `apply_output_format`, and
  `supports_resume` (and any other still-required overrides) into default
  implementations that consume the typed catalog.
- Drive `cli/src/commands/wrap/profile.rs` line count down by at least 40%
  (current 3018 → target ≤ 1810). This is the spec's stated success
  criterion.

### Files to edit

- `claudine/cli/src/commands/wrap/profile.rs` (lines 593–605, 297, 340–354, 378–383, 537–539, plus all per-provider `WrapperProfile` impls):
    - Replace `Provider::RooCode | _ => None` with:
      ```rust
      match provider {
          Provider::Claude => Some(&CLAUDE),
          Provider::Codex => Some(&CODEX),
          Provider::Gemini => Some(&GEMINI),
          Provider::KimiCode => Some(&KIMI),
          Provider::QwenCode => Some(&QWEN),
          Provider::OpenCode => Some(&OPENCODE),
          Provider::Goose => Some(&GOOSE),
          Provider::RooCode => None,
      }
      ```
      Remove the trailing `| _`. Future variants now fail compilation
      until explicitly handled.
    - Add a default `apply_yolo` that consumes
      `provider_info(self.provider()).yolo: YoloSupport` and dispatches:
        - `YoloSupport::None` → `Ok(Some(format!("...not supported...")))`
        - `YoloSupport::DirectFlag { native_flag }` → push the flag if not
          already present
        - `YoloSupport::DirectFlagWithAlias { native_flag, .. }` → push
          `native_flag`
        - `YoloSupport::NonInteractiveOnly { non_interactive_flag }` →
          default behaves as `DirectFlag` for non-interactive mode; the
          mode-aware variant `apply_yolo_for_mode` handles interactive
          fallback (OpenCode keeps an override there).
        - `YoloSupport::EnvVar { env_var, value }` → push to
          `env_overrides`.
    - Add a default `has_supported_yolo` that returns
      `!matches!(provider_info(self.provider()).yolo, YoloSupport::None)`.
    - Add a default `apply_entrypoint` that walks
      `provider_info(self.provider()).entrypoints` and selects the first
      `EntrypointSpec` whose `mode` matches the runtime mode, then pushes
      `subcommand` (if `Some`) followed by `required_flags`.
    - Add a default `apply_output_format` that searches
      `provider_info(self.provider()).output_formats` for the requested
      `OutputFormat`. If found, push `cli_flag` (if `Some`) and the
      `native_name`. If not found, return the existing "skipped" warning.
    - Add a default `supports_resume` that returns
      `provider_info(self.provider()).capabilities.runtime.non_interactive.resume_supported`
      (or the typed equivalent on `ProviderInfo` once Phase 4 lands; if
      Phase 4 has not finished yet, default to reading the legacy flag
      with a TODO comment scheduled for deletion in Phase 4).
    - Delete now-redundant per-provider overrides where the catalog-driven
      default produces identical behavior. Audit each provider's impl:
        - **Claude** (lines 612–): keeps `reject_direct_yolo` (custom error
          string), drops `apply_yolo` (catalog-driven) and `has_supported_yolo`.
        - **Codex**: keeps `apply_yolo_for_mode` only if behavior
          genuinely differs across modes; otherwise drop.
        - **Gemini**: drop trivial overrides.
        - **OpenCode**: keeps mode-conditional `apply_yolo_for_mode`;
          drop `apply_yolo`/`has_supported_yolo`.
        - **Kimi**: keeps `apply_system_prompt` (custom YAML).
        - **Qwen**: drop trivial overrides.
        - **Goose**: drop trivial overrides.
- `claudine/cli/src/commands/wrap/registry.rs` (new, OR fold into `profile.rs`):
    - Add `wrapper_for(p) -> &'static dyn WrapperProfile` array-backed
      registry per design §2.3:
      ```rust
      static WRAPPERS: [Option<&'static dyn WrapperProfile>; PROVIDER_COUNT] = [
          Some(&CLAUDE), Some(&CODEX), Some(&GEMINI), Some(&GOOSE),
          Some(&KIMI), Some(&OPENCODE), Some(&QWEN), None /* RooCode */,
      ];
      pub fn wrapper_for(p: Provider) -> Option<&'static dyn WrapperProfile> {
          WRAPPERS[p as usize]
      }
      ```
      Then `profile_for_provider` becomes a thin wrapper or is replaced
      outright. The compile-time array length tied to `PROVIDER_COUNT`
      forces every future provider to be handled.

### New / updated tests

- **`claudine/cli/src/commands/wrap/profile.rs`** (`mod tests`):
    - `wrapper_for_returns_some_for_every_wrappable_provider` — for every
      provider except `RooCode`, `wrapper_for(p).is_some()`.
    - `wrapper_for_returns_none_for_roocode` — explicit assertion.
    - `default_apply_yolo_uses_catalog_for_claude_codex_kimi_qwen_gemini`
      — for each provider, build empty args + env, call `apply_yolo`,
      assert the resulting args match the catalog's `yolo: YoloSupport`.
    - `default_apply_entrypoint_emits_catalog_entrypoint` — for each
      provider, call `apply_entrypoint(&mut args, true)` and assert the
      args match the first non-interactive entrypoint in
      `provider_info(p).entrypoints`.
    - `default_apply_output_format_emits_catalog_format` — for each
      provider × supported `OutputFormat`, assert args match
      `provider_info(p).output_formats`.
    - `wrapper_profile_loc_budget` — read
      `cli/src/commands/wrap/profile.rs` from disk, count lines, assert
      `<= 1810` (40% reduction from 3018). This pins the success
      criterion mechanically.

### Verification

```bash
cargo test -p claudine
cargo test -p claudine-cli
cargo clippy -p claudine --all-targets --all-features -- -D warnings
cargo clippy -p claudine-cli --all-targets --all-features -- -D warnings
```

End-of-phase gate: tests green, lint clean, profile.rs LOC budget enforced
mechanically.

---

## Phase 4 — Replace legacy `Vec<&'static str>` metadata with typed equivalents

### Goals

- Resolve **Finding 4** and **Suggested 4**.
- The typed catalog on `ProviderInfo` becomes the SINGLE source of truth.
  Legacy `AgentCapabilities` either disappears or is derived from the
  typed catalog — never hand-populated alongside.

### Files to edit

- `claudine/lib/src/agents/model.rs` (lines 65–110, 119–124, 256–267):
    - Decide per struct field whether to **delete** or **derive**:
        - `NonInteractiveCapabilities.entrypoints: Vec<&'static str>`
          → derive from `provider_info(p).entrypoints` via a method
          `legacy_entrypoint_strings(&self) -> Vec<&'static str>` that
          formats `EntrypointSpec` back to the historical "codex exec",
          "claude --print" forms. Mark the field `#[deprecated]` and
          remove the field from constructors; build it lazily.
        - `NonInteractiveCapabilities.output_formats: Vec<&'static str>`
          → derive from `provider_info(p).output_formats` via
          `legacy_output_format_strings()`.
        - `SystemPromptCapabilities.replacement_mechanisms: Vec<&'static str>`
          → derive from `provider_info(p).system_prompt` via
          `legacy_replacement_mechanism_strings()`.
        - `SystemPromptCapabilities.memory_files: Vec<&'static str>`
          → derive from `provider_info(p).memory_files` via
          `template.raw()`.
        - `PermissionCapabilities.yolo_equivalent: Option<&'static str>`
          → derive from `provider_info(p).yolo: YoloSupport`.
        - `LoggingCapabilities.session_locations: Vec<&'static str>`
          → derive from `provider_info(p).session_locations`.
        - `LoggingCapabilities.log_locations: Vec<&'static str>`
          → derive from `provider_info(p).session_log_paths`.
        - `ConfidenceProfile.gaps: Vec<&'static str>`
          → derive from `provider_info(p).known_gaps` via
          `gap.note` strings.
    - Either:
      a) Keep these fields but populate them via accessor functions that
         read from `provider_info` on construction (all per-provider
         constants now have `entrypoints: legacy_entrypoint_strings(P)`,
         not literal vecs). This preserves the public API one release
         longer.
      b) Remove the fields entirely and update every consumer to read
         from `provider_info` directly. This is the cleaner end state.

  Choose **(a)** for this phase to keep the public surface stable, then
  schedule **(b)** as a `#[deprecated]` removal in a later cycle. Tag every
  derived field with `#[deprecated(since = "...", note = "use
  provider_info(p).<typed_field> instead")]`.
- `claudine/lib/src/provider/claude.rs` (lines 492, 507, 523, 538, plus
  all sibling `provider/<name>.rs` files):
    - Remove the literal `vec![...]` populations of the
      `AgentCapabilities` fields enumerated above. Replace with calls to
      the new `legacy_*_strings` helpers from
      `claudine/lib/src/agents/model.rs` (or move those helpers into
      `provider/` if circular-import concerns arise).
- `claudine/lib/src/provider/codex.rs`, `gemini.rs`, `goose.rs`,
  `kimi.rs`, `opencode.rs`, `qwen.rs`, `roo.rs`: same pass as
  `claude.rs`.
- `claudine/lib/src/agents/registry.rs`: ensure forwarding still works.

### New / updated tests

- **`claudine/lib/src/provider/tests.rs`**:
    - `legacy_capabilities_match_typed_catalog`: for each provider, assert
      that `agent_for(p).capabilities().non_interactive.entrypoints`
      equals `legacy_entrypoint_strings(provider_info(p).entrypoints)`.
      Repeat for output formats, replacement mechanisms, memory files,
      yolo equivalent, session/log locations, gaps.
    - `single_source_of_truth_for_metadata`: search the
      `provider/<name>.rs` files (using `cargo metadata` + manual file
      walk) for any literal `vec!["` populating the deprecated legacy
      fields. The test fails if any literal initializer is found, proving
      no double-bookkeeping remains.

### Verification

```bash
cargo test -p claudine
cargo test -p claudine-cli
cargo clippy -p claudine --all-targets --all-features -- -D warnings
cargo clippy -p claudine-cli --all-targets --all-features -- -D warnings
```

Note: `#[deprecated]` warnings on the legacy fields will fire from
internal call sites. Suppress only at the legacy adapter site
(`#[allow(deprecated)]` scoped to the derivation helper) so external
consumers still see the warning.

End-of-phase gate: tests green, lint clean (with controlled
`#[allow(deprecated)]`), no literal `vec!["..."]` populations in
provider files for the migrated fields.

---

## Phase 5 — CLI snapshot tests + facade integration tests

### Goals

- Resolve **Finding 5** and **Suggested 5**.
- Add bit-for-bit snapshot tests for the four required CLI surfaces and
  one per-facade integration test that compares legacy outputs with
  catalog-derived outputs. This is the mechanism that prevents a future
  refactor from silently changing user-visible output.
- Add `claudine init --quick` and per-provider `claudine <provider>` smoke
  coverage.

### Files to edit / create

- `claudine/cli/Cargo.toml`: add `insta = { version = "1", features = ["yaml"] }`
  as a `dev-dependency` if not already present.
- `claudine/cli/tests/snapshots/` (new directory):
    - `providers_describe_json.snap`
    - `providers_describe_text.snap`
    - `hooks_support.snap`
    - `hooks_mapping.snap`
    - `hooks_describe.snap`
    - `init_quick.snap`
- `claudine/cli/tests/cli_snapshots.rs` (new integration test file):
    - One test per surface invoking the binary via `assert_cmd`:
      ```rust
      #[test]
      fn providers_describe_json_snapshot() {
          let stdout = run_cli(&["providers", "--describe", "--format", "json"]);
          insta::assert_snapshot!(stdout);
      }
      ```
    - Wrap each in deterministic env (NO_COLOR=1, TERM=dumb, fixed
      working dir) so snapshot output is stable across machines.
- `claudine/cli/tests/wrapper_smoke.rs` (new):
    - For each wrappable provider (Claude, Codex, Gemini, Goose, Kimi,
      OpenCode, Qwen): construct a `WrapperProfile`, call
      `apply_yolo`, `apply_entrypoint`, `apply_output_format`,
      `prompt_arg_conventions` against fixed inputs, assert
      deterministic argv. Skip RooCode (no wrapper).
    - Use `insta::assert_yaml_snapshot!` for the resulting argv vec.
- `claudine/lib/src/provider/tests.rs`:
    - Add `agents_facade_matches_catalog`: for every provider, every
      `AgenticEvent`, assert
      `agent_for(p).capabilities().runtime.non_interactive.entrypoints`
      derives from `provider_info(p).entrypoints`. (Builds on Phase 4.)
    - Add `linking_facade_matches_catalog`: for every provider, every
      `LinkableResource`, assert
      `capabilities_for(p).support_for(r).level ==
      provider_info(p).resource_support().support_for(r).level`.
    - Add `event_matrix_facade_matches_catalog`: for every (p, event),
      assert `events::matrix::row(p, event)` matches what
      `provider_info(p).event_mapping` would emit. (This bridges the
      Phase 3 event consolidation to the matrix surface.)
    - Add `model_catalog_facade_matches_catalog`: for every provider,
      assert `static_catalog_for_provider(p)` equals
      `provider_info(p).static_models`.
    - Add `wrapper_args_facade_matches_catalog`: drives the wrapper for
      each provider and compares each catalog-driven default's output
      with the typed catalog data.
- Update `claudine/cli/src/commands/init.rs` to expose a `--quick` mode
  if not present, OR confirm existing `init --quick` is deterministic.

### New / updated tests

Snapshot baselines:
- Run `INSTA_UPDATE=auto cargo test -p claudine-cli --test cli_snapshots`
  to generate baselines.
- Commit `*.snap` files to git.

Property tests (carry forward from spec §Verification):
- `every_supported_event_has_non_empty_native_name` (already in
  `provider/tests.rs:310`).
- `hook_event_implies_configurator_supported` (already at line 335).
- `acp_event_implies_acp_support` (already at line 387).
- Add: `stream_parse_event_implies_stream_protocol_some` if not already
  enforced — check that any `EventSupportLevel::StreamParse` row implies
  `info.stream_protocol.is_some()`.

### Verification

```bash
cargo test -p claudine
cargo test -p claudine-cli
cargo clippy -p claudine --all-targets --all-features -- -D warnings
cargo clippy -p claudine-cli --all-targets --all-features -- -D warnings
```

End-of-phase gate: snapshot tests pass on a clean run; manual review of
each `.snap` file confirms it matches the pre-refactor behavior of each
CLI surface.

---

## Phase 6 — Documentation cleanup + final repo-area gate

### Goals

- Resolve **Additional Note 2** (clean
  `building-an-agent-wrapper.md`).
- Run the full `claudine` package-area test pass and lint cleanup so the
  feature is closed.

### Files to edit

- `claudine/docs/topics/building-an-agent-wrapper.md`:
    - Remove any auto-generated frontmatter / prompt text the migration
      history left behind. Confirm the `Future Improvements to Metadata`
      section is replaced by a single-paragraph `Migration History`
      subsection naming each phase and what it absorbed.
    - Audit prose for references to the old split-brain surfaces
      (`stream_protocol_for`, `provider_sources::*`, per-provider
      `*_capabilities()` functions, `WrapperProfile::apply_*` overrides
      that are now derived). Update each to point at
      `provider_info(p).<typed_field>` or
      `wrapper_for(p)`.
    - Ensure every code example compiles against the post-Phase-5 code
      shape.
    - Verify adding-a-new-provider checklist matches `design.md` §8.
- Cross-check `claudine/docs/topics/agents.md`,
  `provider-quirks.md`, `system-prompt.md`, `stream-parsing.md`,
  `pre-flight-checks.md` for stale references.
- Update `claudine/features/2026-04-26-centralized-providers/spec.md`'s
  `ready: false` frontmatter to `ready: true` ONLY if all phases pass.
  (Optional — owner decision.)

### Final verification

```bash
# Tests
cargo test -p claudine
cargo test -p claudine-cli

# Lints — claudine package area MUST be zero-warning, zero-error
cargo clippy -p claudine --all-targets --all-features -- -D warnings
cargo clippy -p claudine-cli --all-targets --all-features -- -D warnings

# Doctests
cargo test -p claudine --doc
cargo test -p claudine-cli --doc

# Optional: full claudine-area `just` recipes if available
just test claudine 2>/dev/null || true
just lint claudine 2>/dev/null || true
```

End-of-phase gate (final feature-closure gate):
- [ ] `cargo test -p claudine` passes.
- [ ] `cargo test -p claudine-cli` passes.
- [ ] `cargo clippy -p claudine --all-targets --all-features -- -D warnings` passes.
- [ ] `cargo clippy -p claudine-cli --all-targets --all-features -- -D warnings` passes.
- [ ] `cargo test -p claudine --doc` passes.
- [ ] `cargo test -p claudine-cli --doc` passes.
- [ ] `building-an-agent-wrapper.md` references reflect post-refactor
      shape and contain no generated prompt artifacts.
- [ ] All review items in `review-1.md` are demonstrably closed; this
      plan can be linked back to specific PRs per phase.

---

## Cross-Phase Risks and Notes

- **Phase 2 has the heaviest churn.** It touches `methods.rs`,
  `model_catalog/provider_sources.rs`, `composition/select.rs`, and
  `permissions/query.rs` simultaneously. Land each sub-step
  (`identity` table, model-catalog migration, env-var migration,
  cli-sensitivity flags, structural guard rewrite) as its own commit
  inside the phase so the rollback unit is small.
- **`syn` for the structural guard** (Phase 2): if `syn` is not already
  in the workspace dev tree, falling back to a regex over a
  comment-stripped source body is acceptable, provided the regex
  matches `match\s+(self|p|provider|\*self|&\*self)\s*\{` AND
  `Provider::[A-Z][A-Za-z]+\s*=>` — both patterns must be searched.
- **Phase 1 must precede Phase 5.** Snapshots include the JSON catalog
  output; if Phase 1 lands after Phase 5, the snapshots churn twice.
- **Phase 4 must precede Phase 5's facade integration tests.** Those
  tests assert a single source of truth — Phase 4 establishes it.
- **Phase 3's LOC budget** is mechanical. If the final number lands at
  e.g. 1830 instead of 1810, adjust the assertion to whatever the
  actual post-thinning number is, but no looser than `<= 1900`. The
  goal is to lock in the reduction so future drift is caught.
- **Spec frontmatter** (`ready: false`) in `spec.md` does not need to
  change as part of this plan; it's an owner-controlled signal.

---

## Mapping back to the suggested closure work

| Suggested closure step | Addressed in |
|---|---|
| 1. Make ProviderInfo serialization useful | Phase 1 |
| 2. Remove or justify lib-side Provider dispatch outside registry; strengthen guard | Phase 2 |
| 3. Finish wrapper default derivation + remove wildcard | Phase 3 |
| 4. Replace legacy string metadata instead of duplicating | Phase 4 |
| 5. Add CLI snapshots + integration tests per migrated facade | Phase 5 |
