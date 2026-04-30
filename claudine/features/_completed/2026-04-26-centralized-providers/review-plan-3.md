---
ready: false
agent: ""
model: ""
review: review-3.md
---

# Centralized Providers — Review 3 Closure Plan

This plan addresses every finding in
[`review-3.md`](./review-3.md). It is staged in three phases plus a final
verification gate. Each phase compiles independently, has its own test
green-light, and ends lint-clean for the `claudine` package area
(`claudine/lib`, `claudine/cli`).

The review found four issues:

| # | Severity | Issue |
|---|---|---|
| 1 | High | Public compatibility re-exports (`events::Provider`, `events::PROVIDERS_DISPLAY_ORDER`, `agents::AgentId`) were removed early — the deprecation window promised by spec/design has not elapsed. |
| 2 | Medium | `discover_agents_full` still hard-codes a per-provider `(Provider, config_path, AiCli)` table that duplicates `ProviderInfo` data. |
| 3 | Medium | The CLI wrapper registry uses `Provider::RooCode \| _ => None`, so a future `Provider` variant silently classifies as "no wrapper" instead of forcing a registry update. |
| 4 | Low | The `no_unauthorized_match_provider_in_lib` guard is a literal text scan that does not catch provider arrays, alternate match forms, or duplicated provider facts. |

Phase mapping:

| Phase | Closes | Description |
|---|---|---|
| 1 | Issue 1 | Restore deprecated compatibility re-exports + add tests that import them. |
| 2 | Issue 2 | Drive `discover_agents_full` from the central catalog. |
| 3 | Issues 3 + 4 | Replace the wrapper wildcard with an array-backed registry and add invariant tests that supersede the literal text scan. |
| 4 | All | Final verification gate — full `cargo test` and `cargo clippy -D warnings` across all claudine crates. |

## Assumptions

- `claudine/lib`, `claudine/cli`, and any other in-tree claudine crates are
  the targets for the final lint/test gate. `cargo metadata --no-deps` will
  be used to enumerate the exact crate list at the start of Phase 4.
- The deprecation window explicitly required by spec §Migration Plan and
  design §6.1 (one release cycle for `AgentId` and `events::Provider`) has
  not elapsed; restoring the re-exports is correct.
- `sniff::programs::AiCli` already exposes `display_name()` and
  `binary_name()` accessors used by today's `discover_agents_full`. The
  catalog field consumed in Phase 2 will keep these via `info.sniff_binding`.
- Existing tests pass on the pre-plan tree (review explicitly verified
  `provider::tests` is green at 26 tests). New tests added by this plan
  must NOT regress any existing behavior or snapshot.
- `clippy` baseline may already have warnings in claudine crates that are
  unrelated to review-3 changes. Phase 4 fixes them all to satisfy the
  "lint clean across all claudine crates with zero warnings/errors"
  requirement.

---

## Phase 1 — Restore Deprecated Compatibility Re-Exports

### Scope (Closes Issue 1)

Restore the public surfaces promised by spec §"Migration Plan" and
design §6.1 "Deprecation Mechanics":

- `claudine::events::Provider` — `#[deprecated]` re-export of
  `claudine::provider::Provider`.
- `claudine::events::PROVIDERS_DISPLAY_ORDER` — `#[deprecated]` re-export
  of `claudine::provider::PROVIDERS_DISPLAY_ORDER`.
- `claudine::events::EventSupportLevel` — `#[deprecated]` re-export of
  `claudine::provider::EventSupportLevel` (this also moved during the
  migration; restore it for symmetry with the spec).
- `claudine::agents::AgentId` — `#[deprecated] pub type AgentId =
  claudine::provider::Provider;`.
- Any other previously public provider items that moved out of `events::`
  or `agents::`. Inventory by scanning the original `events/provider.rs`
  and `agents/mod.rs` exports against today's exports.

These re-exports must compile **without** triggering the `deprecated`
lint inside this crate (`#[allow(deprecated)]` at the use sites that
exercise them). External consumers see the deprecation warning.

### Files Touched

- `claudine/lib/src/events/mod.rs` — add deprecated re-exports for
  `Provider`, `PROVIDERS_DISPLAY_ORDER`, `EventSupportLevel`.
- `claudine/lib/src/events/provider.rs` — recreate (or restore) as a
  thin module that contains nothing but `#[deprecated]` re-exports
  pointing into `crate::provider::*`. Per spec line 89, `events/provider.rs`
  becomes a deprecated re-export.
- `claudine/lib/src/agents/mod.rs` — add `#[deprecated] pub type AgentId
  = crate::provider::Provider;`. Re-export `parse_agent_id` (already
  present) plus any historic `AgentId` constructors that existed before
  the merge (audit `git log -p claudine/lib/src/agents/mod.rs` and
  `claudine/lib/src/agents/registry.rs` for the pre-Phase-0 surface).
- `claudine/lib/tests/deprecated_compatibility.rs` (new integration test)
  — imports each restored deprecated path with `#[allow(deprecated)]` and
  asserts type equivalence via a function signature trick or simple
  use-site smoke. Lives in `claudine/lib/tests/` so a regression that
  removes the re-export breaks the test build immediately.

### Tasks

1. **Inventory previously public surface.** Run
   `git log -p --follow claudine/lib/src/events/provider.rs claudine/lib/src/agents/mod.rs claudine/lib/src/agents/registry.rs`
   and list every `pub` item that existed in those files prior to the
   migration. For each, decide:
    - If it still has a counterpart in `claudine::provider::*`, add a
      `#[deprecated]` re-export.
    - If it was renamed, add the deprecated old name as an alias to
      the new path.
    - If it was deleted entirely (no longer applicable), document why
      in the inline comment so a future audit understands.
2. **Recreate `events/provider.rs` as a thin deprecated re-export
   module.** Per spec line 89, the file must exist (not be deleted)
   and must consist of `#[deprecated]` `pub use` statements only.
   Example:
   ```rust
   //! Deprecated re-exports from the pre-migration provider module.
   //!
   //! The canonical home for these items is `crate::provider::*`. This
   //! module survives one release cycle so external consumers can update
   //! without breakage. Removal is scheduled for the post-Phase-8 cleanup.
   #![allow(deprecated)]

   #[deprecated(
       since = "0.9.0",
       note = "Use `claudine::provider::Provider` instead"
   )]
   pub use crate::provider::Provider;

   #[deprecated(
       since = "0.9.0",
       note = "Use `claudine::provider::PROVIDERS_DISPLAY_ORDER` instead"
   )]
   pub use crate::provider::PROVIDERS_DISPLAY_ORDER;

   #[deprecated(
       since = "0.9.0",
       note = "Use `claudine::provider::EventSupportLevel` instead"
   )]
   pub use crate::provider::EventSupportLevel;
   ```
   Wire `events/mod.rs` to declare and re-export this module:
   ```rust
   #[allow(deprecated)]
   pub mod provider;
   #[allow(deprecated)]
   pub use provider::{Provider, PROVIDERS_DISPLAY_ORDER, EventSupportLevel};
   ```
3. **Recreate `agents::AgentId`.** In `claudine/lib/src/agents/mod.rs`:
   ```rust
   #[deprecated(
       since = "0.9.0",
       note = "AgentId merged into `claudine::provider::Provider` during the centralized provider migration"
   )]
   pub type AgentId = crate::provider::Provider;
   ```
   If any other previously public `AgentId`-related items existed
   (e.g., a `parse_agent_id` returning `AgentId`, an `AgentId` builder),
   add deprecated re-exports / shims pointing at the new equivalents.
4. **Update internal call sites that use the new names.** No internal
   change needed if the rest of the lib already uses
   `crate::provider::*` (which it does, per the current state).
5. **Add a forward-looking comment** in
   `events/provider.rs` and `agents/mod.rs` linking to design §6.1 with
   the removal milestone (post-Phase-8 cleanup release).

### Test Strategy

- **`claudine/lib/tests/deprecated_compatibility.rs`** (new) — single
  integration test crate that imports every restored deprecated path
  inside `#[allow(deprecated)]` blocks and asserts type identity:
  ```rust
  #![allow(deprecated)]

  use claudine::events::{Provider as DeprecatedEventsProvider, PROVIDERS_DISPLAY_ORDER as DEPRECATED_DISPLAY_ORDER, EventSupportLevel as DeprecatedEventSupportLevel};
  use claudine::agents::AgentId as DeprecatedAgentId;
  use claudine::provider::{Provider, PROVIDERS_DISPLAY_ORDER as PROVIDER_DISPLAY_ORDER, EventSupportLevel};

  #[test]
  fn events_provider_re_export_matches_canonical() {
      let canonical: Provider = Provider::Claude;
      let deprecated: DeprecatedEventsProvider = canonical;
      assert_eq!(canonical, deprecated);
  }

  #[test]
  fn events_providers_display_order_re_export_matches() {
      assert_eq!(DEPRECATED_DISPLAY_ORDER.len(), PROVIDER_DISPLAY_ORDER.len());
      for (deprecated, canonical) in DEPRECATED_DISPLAY_ORDER.iter().zip(PROVIDER_DISPLAY_ORDER.iter()) {
          assert_eq!(deprecated, canonical);
      }
  }

  #[test]
  fn events_event_support_level_re_export_compiles() {
      // Just exercising the type alias keeps compile-time symmetry.
      let _: DeprecatedEventSupportLevel = EventSupportLevel::NotSupported;
  }

  #[test]
  fn agents_agent_id_alias_is_provider() {
      let id: DeprecatedAgentId = Provider::Codex;
      assert_eq!(id, Provider::Codex);
  }
  ```
- **Existing test suites** must remain green; the re-exports do not
  alter behavior.

### Verification Commands

```bash
cargo test -p claudine --tests
cargo test -p claudine --doc
cargo test -p claudine --test deprecated_compatibility
cargo clippy -p claudine --all-targets --all-features -- -D warnings
```

End-of-phase gate: tests green; deprecation warnings fire ONLY for
external usage (the new test deliberately suppresses them with
`#![allow(deprecated)]`); no new clippy warnings introduced.

---

## Phase 2 — Drive `discover_agents_full` From the Central Catalog

### Scope (Closes Issue 2)

Eliminate the hand-maintained `[(Provider, PathBuf, AiCli); 8]` table in
`claudine/lib/src/config/mod.rs:83`. The function must read every fact
it needs from `provider_info(p)`:

- `provider` → from `info.provider`.
- `AiCli` (sniff binding) → from `info.sniff_binding`.
- Display name → from `info.sniff_binding.display_name()` OR
  `info.display_name` (decide one canonical source — see Tasks).
- Binary name → from `info.sniff_binding.binary_name()` OR `info.binary`.
- Primary user-level config path → derive from
  `info.config_paths` (a `&'static [PathTemplate]`) by selecting the
  first `User` template and resolving it against the current `PathContext`
  (HOME, etc.).

### Files Touched

- `claudine/lib/src/config/mod.rs` — rewrite `discover_agents_full` and
  any helper it uses (e.g., a private `primary_config_path_for(Provider)`
  helper if it makes the function readable).
- `claudine/lib/src/provider/path_template.rs` — confirm
  `PathTemplate::resolve(&PathContext) -> PathBuf` exists. If it does
  not, add a minimal resolver that handles the segments needed for
  user-level config (HomeDir + literal segments). Phase 5 of the
  original plan introduced richer template support; for this phase only
  the `HomeDir + Literal` path is required.
- `claudine/lib/src/provider/<provider>.rs` (potentially) — if any
  provider's `config_paths` does not currently contain a user-level
  template that resolves to today's hard-coded path (e.g.,
  `~/.claude/settings.json` for Claude), add it. Audit each provider
  against the current `discover_agents_full` table:
    - Claude → `~/.claude/settings.json`
    - Codex → `~/.codex/config.toml`
    - Gemini → `~/.gemini/settings.json`
    - Goose → `~/.config/goose/config.yaml`
    - KimiCode → `~/.kimi/config.json`
    - OpenCode → `~/.config/opencode/opencode.json`
    - QwenCode → `~/.qwen/settings.json`
    - RooCode → `~/.roo/settings.json`
- `claudine/lib/src/provider/mod.rs` — if `config_paths` items need a
  classification (User vs Project vs Local), add a typed wrapper or
  enum on `PathTemplate` (e.g., `ConfigPathScope::User`). If today the
  field is just `&'static [PathTemplate]`, introduce a sibling
  classification helper or a thin newtype. Use the simplest design that
  unambiguously identifies "primary user-level config".

### Tasks

1. **Audit `info.config_paths` per provider** — for each provider,
   confirm the current `&'static [PathTemplate]` array contains a
   template that resolves to the historical primary config path.
   If missing, add it as the first element so the catalog is
   the authoritative source.
2. **Add a classification or first-template convention.** Two options:
    - **Option A (preferred, lowest churn):** Document and enforce
      that `info.config_paths[0]` is always the primary user-level
      config path. Add a `#[doc]` comment to the field in
      `provider/mod.rs`. Add a unit test in `provider/tests.rs`:
      ```rust
      #[test]
      fn config_paths_have_primary_user_entry() {
          for provider in PROVIDERS_DISPLAY_ORDER {
              let info = provider_info(provider);
              assert!(
                  !info.config_paths.is_empty(),
                  "{provider:?}: config_paths must contain at least one entry"
              );
          }
      }
      ```
    - **Option B (more explicit):** Add a new typed
      `ConfigPathEntry { template: PathTemplate, scope: ConfigScope }`
      and switch `config_paths` to `&'static [ConfigPathEntry]`. More
      churn but more self-documenting.
   Pick Option A unless a downstream consumer demands explicit scope.
3. **Implement `PathTemplate::resolve_with_home(&Path) -> PathBuf`**
   if not present. Must handle the existing segment kinds used by the
   listed config paths (`HomeDir` + `Literal`). Defer richer segment
   support (`EncodedCwd`, `SessionId`, etc.) to existing code paths;
   the discovery use case does not need them.
4. **Rewrite `discover_agents_full`**:
   ```rust
   pub fn discover_agents_full() -> Vec<AgentInfo> {
       let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
       let ai_clients = InstalledAiClients::new();

       crate::provider::PROVIDERS_DISPLAY_ORDER
           .into_iter()
           .map(|provider| {
               let info = crate::provider::provider_info(provider);
               let primary = info
                   .config_paths
                   .first()
                   .expect("every provider must declare at least one config path")
                   .resolve_with_home(&home);
               let config_exists = primary.exists();
               let on_path = ai_clients.is_installed(info.sniff_binding);

               AgentInfo {
                   provider,
                   config_exists,
                   on_path,
                   display_name: info.sniff_binding.display_name(),
                   binary_name: info.sniff_binding.binary_name(),
                   config_path: if config_exists { Some(primary) } else { None },
               }
           })
           .collect()
   }
   ```
5. **Confirm `get_configurator` and `detect_agents` are unchanged**
   — they already delegate via `provider_info(provider).configurator`.

### Test Strategy

- **`claudine/lib/src/config/mod.rs::tests::discover_agents_full_returns_all_eight`**
  — already exists. Confirm still passes.
- **New test** in `claudine/lib/src/config/mod.rs::tests`:
  ```rust
  #[test]
  fn discover_agents_full_uses_catalog_sniff_binding() {
      let agents = discover_agents_full();
      for agent in &agents {
          let info = crate::provider::provider_info(agent.provider);
          assert_eq!(
              agent.display_name,
              info.sniff_binding.display_name(),
              "{:?}: display_name must come from catalog sniff binding",
              agent.provider,
          );
          assert_eq!(
              agent.binary_name,
              info.sniff_binding.binary_name(),
              "{:?}: binary_name must come from catalog sniff binding",
              agent.provider,
          );
      }
  }

  #[test]
  fn discover_agents_full_covers_every_catalog_provider_exactly_once() {
      let agents = discover_agents_full();
      let observed: std::collections::HashSet<_> = agents.iter().map(|a| a.provider).collect();
      let expected: std::collections::HashSet<_> =
          crate::provider::PROVIDERS_DISPLAY_ORDER.into_iter().collect();
      assert_eq!(observed, expected);
      assert_eq!(agents.len(), crate::provider::PROVIDER_COUNT);
  }

  #[test]
  fn discover_agents_full_primary_config_path_matches_catalog_template() {
      let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
      let agents = discover_agents_full();
      for agent in &agents {
          let info = crate::provider::provider_info(agent.provider);
          let expected = info
              .config_paths
              .first()
              .unwrap()
              .resolve_with_home(&home);
          // config_path is None when the file doesn't exist; in that
          // case we still want the resolution to match the catalog,
          // so reconstruct it.
          let actual = agent
              .config_path
              .clone()
              .unwrap_or_else(|| expected.clone());
          assert_eq!(actual, expected, "{:?}", agent.provider);
      }
  }
  ```
- **Provider-side test** in `claudine/lib/src/provider/tests.rs`:
  ```rust
  #[test]
  fn config_paths_have_primary_user_entry() {
      for provider in PROVIDERS_DISPLAY_ORDER {
          let info = provider_info(provider);
          assert!(
              !info.config_paths.is_empty(),
              "{provider:?}: must declare at least one user-level config path"
          );
      }
  }
  ```
- **Negative test** (optional but recommended) — comment-out a provider's
  `config_paths` first element locally and confirm the new tests fail
  with a clear message; then restore.

### Verification Commands

```bash
cargo test -p claudine --tests
cargo test -p claudine --doc
cargo clippy -p claudine --all-targets --all-features -- -D warnings
```

End-of-phase gate: `discover_agents_full` contains zero hard-coded
provider facts; all new tests pass; existing matrix and dispatch tests
unchanged.

---

## Phase 3 — Catalog-Backed Wrapper Registry + Hardened Drift Guard

### Scope (Closes Issues 3 + 4)

Two related changes:

1. **Replace the wildcard wrapper match** in
   `claudine/cli/src/commands/wrap/profile.rs:711` with an array-backed
   registry whose length is tied to `PROVIDER_COUNT`. A future
   `Provider` variant must force a compile error or test failure rather
   than silently falling through to `None`.
2. **Augment the drift guard** in
   `claudine/lib/src/provider/tests.rs:319` with structural invariants
   that catch the kinds of drift the literal text scan cannot:
    - Provider arrays (the `discover_agents_full` table this plan
      removed in Phase 2 is the exact case the guard missed).
    - `match p`, `match self.provider`, `match *self`, etc.
    - Duplicated provider facts (the catalog-backed Phase 2 tests
      partially cover this; add the explicit invariants here).

### Files Touched

- `claudine/cli/src/commands/wrap/profile.rs` — replace
  `profile_for_provider` with an array-backed registry.
- `claudine/cli/src/commands/wrap/profile.rs` (test module) — extend
  the existing `wrapper_for` exhaustiveness test (lines 2426, 2442,
  2457, 2470).
- `claudine/lib/src/provider/tests.rs` — add new invariant tests:
    - `agent_discovery_covers_catalog_exactly_once` (already added in
      Phase 2 — confirm still passing here).
    - `every_provider_has_documented_wrapper_or_no_wrapper_marker`
      (lib-side: assert that every `Provider` either has a wrapper or
      is in a documented "no wrapper" allow-list — uses a small
      lib-visible helper or a #[cfg(test)] export from the CLI crate).
    - Hardened `no_unauthorized_match_provider_in_lib` — broaden the
      pattern set as described in Tasks below.
- `claudine/cli/src/commands/wrap/profile.rs` (test module) — add a
  CLI-side companion to the registry exhaustiveness invariant.

### Tasks

1. **Convert `profile_for_provider` to an array-backed registry.**
   Replace the existing match with:
   ```rust
   use claudine::provider::{PROVIDER_COUNT, Provider};
   use std::sync::OnceLock;

   /// CLI-side wrapper registry. Indexed by `Provider as usize`.
   ///
   /// `None` slots are documented "no wrapper" exceptions (e.g. RooCode,
   /// which is a VS Code extension and has no standalone CLI to wrap).
   static WRAPPER_REGISTRY: OnceLock<[Option<&'static dyn WrapperProfile>; PROVIDER_COUNT]> =
       OnceLock::new();

   pub(crate) fn profile_for_provider(provider: Provider) -> Option<&'static dyn WrapperProfile> {
       let registry = WRAPPER_REGISTRY.get_or_init(|| [
           /* Claude   */ Some(&CLAUDE),
           /* Codex    */ Some(&CODEX),
           /* Gemini   */ Some(&GEMINI),
           /* Goose    */ Some(&GOOSE),
           /* KimiCode */ Some(&KIMI),
           /* OpenCode */ Some(&OPENCODE),
           /* QwenCode */ Some(&QWEN),
           /* RooCode  */ None,
       ]);
       registry[provider as usize]
   }
   ```
   The compile-time array length tied to `PROVIDER_COUNT` forces every
   future provider to be addressed. The slot order must match
   `Provider as usize` (`Claude=0, Codex=1, Gemini=2, Goose=3,
   KimiCode=4, OpenCode=5, QwenCode=6, RooCode=7`).
2. **Remove the wildcard.** Delete `Provider::RooCode | _ => None`
   entirely. There is no `_` arm in the new code.
3. **Document "no wrapper" exceptions explicitly.** Add a doc comment
   above `WRAPPER_REGISTRY` listing each `None` slot and why
   (currently only RooCode). If a future provider is intentionally
   unwrapped, the diff that adds it must update both the registry and
   this doc.
4. **Add a CLI-side test** that the registry's `Some`/`None` decisions
   match an explicit allow-list. Replace the existing test (lines
   2440–2476) with:
   ```rust
   #[test]
   fn wrapper_registry_covers_every_provider_and_documents_exceptions() {
       use claudine::provider::PROVIDERS_DISPLAY_ORDER;

       const NO_WRAPPER: &[Provider] = &[Provider::RooCode];

       for provider in PROVIDERS_DISPLAY_ORDER {
           let result = profile_for_provider(provider);
           if NO_WRAPPER.contains(&provider) {
               assert!(
                   result.is_none(),
                   "{:?}: documented as no-wrapper but registry returned Some",
                   provider
               );
           } else {
               let profile = result.expect(&format!(
                   "{:?}: registry must provide a wrapper unless explicitly listed in NO_WRAPPER",
                   provider
               ));
               assert_eq!(
                   profile.provider(),
                   provider,
                   "{:?}: registry slot returned a profile for the wrong provider",
                   provider
               );
           }
       }
   }
   ```
5. **Harden the lib drift guard.** Replace the literal-text scan in
   `claudine/lib/src/provider/tests.rs::no_unauthorized_match_provider_in_lib`
   with a regex-based scan that catches additional patterns. The
   pattern set must include:
    - `match\s+(provider|p|self\.provider|self|\*self|&\*self)\s*\{`
    - `Provider::[A-Z][A-Za-z]+\s*=>`
    - `\[\s*\(\s*Provider::` — provider tuple arrays (catches
      `[(Provider::Claude, ...), ...]` like the discovery table).
    - `\[\s*Provider::[A-Z]` — provider arrays (already legitimate
      uses are restricted to `provider/identity.rs` /
      `provider/registry.rs`).
   Strip Rust line and block comments before scanning so commented-out
   examples don't trip the guard.
   Allow-list (kept narrow): `provider/registry.rs`,
   `provider/identity.rs`, `provider/tests.rs` itself,
   `provider/methods.rs` (if it still hosts canonical method dispatch
   — confirm during implementation), and any other file whose match
   has been justified by a relocation comment in the code.
6. **Add narrow positive invariant tests** that supersede the broad
   text scan as the primary safety net (per review-3 Suggestion 4):
    - `agent_discovery_covers_catalog_exactly_once_using_sniff_binding`
      — already added in Phase 2.
    - `wrapper_registry_covers_every_provider_and_documents_exceptions`
      — added in Task 4.
    - `every_config_path_first_entry_is_user_scope` — the Phase 2
      `config_paths_have_primary_user_entry` test, lifted as a more
      assertive variant if scope classification was added.
    - `deprecated_paths_compile` — the integration test from Phase 1
      already enforces this.
7. **Revisit the `events/provider.rs` and `events/matrix.rs` allow-list
   entries.** These were allow-listed in the original guard
   (`provider/tests.rs:350-351`) when the migration was in flight.
   Phase 1's restoration of `events/provider.rs` as a thin
   `#[deprecated]` re-export module should mean it no longer has any
   `match Provider` content. If so, remove from the allow-list. If
   `events/matrix.rs` still has a legitimate match, leave on the
   allow-list with an updated explanatory comment.

### Test Strategy

- New CLI test `wrapper_registry_covers_every_provider_and_documents_exceptions`
  (replaces or augments the existing exhaustiveness test). Must
  compile-fail if `NO_WRAPPER` drifts from the registry's `None` slots.
- Hardened lib test `no_unauthorized_match_provider_in_lib` runs all
  patterns over comment-stripped source. Must report violators by
  file path.
- Existing `provider::tests` (26 tests) plus all new tests added
  in Phases 1 + 2 must remain green.

### Verification Commands

```bash
cargo test -p claudine --tests
cargo test -p claudine --test deprecated_compatibility
cargo test -p claudine-cli --tests
cargo test -p claudine-cli --doc
cargo clippy -p claudine --all-targets --all-features -- -D warnings
cargo clippy -p claudine-cli --all-targets --all-features -- -D warnings
```

End-of-phase gate:
- Wildcard `Provider::RooCode | _ => None` is gone; future variants
  fail compilation if missed.
- Hardened drift guard catches the four new pattern classes.
- All new positive invariant tests pass.

---

## Phase 4 — Final `cargo test` + `cargo clippy` Gate Across All Claudine Crates

### Scope

Confirm full claudine package area is green and lint-clean. Fix any
clippy warnings or test failures that surface, even if unrelated to
review-3. Document any deferred items in the final checklist.

### Tasks

1. **Enumerate all claudine crates.**
   ```bash
   cargo metadata --no-deps --format-version 1 \
     | jq -r '.packages[] | select(.manifest_path | contains("/claudine/")) | .name'
   ```
   Expected (subject to confirmation): `claudine` (lib), `claudine-cli`
   (cli). If additional sub-crates exist (per repo convention some
   areas have `lib`/`cli`/`server` etc.), include them in the gate.
2. **Run the full test suite per crate.**
   ```bash
   for pkg in <list-from-step-1>; do
       cargo test -p "$pkg" --all-features --tests
       cargo test -p "$pkg" --all-features --doc
   done
   ```
3. **Run the lint pass per crate.**
   ```bash
   for pkg in <list-from-step-1>; do
       cargo clippy -p "$pkg" --all-targets --all-features -- -D warnings
   done
   ```
4. **Triage and fix any clippy warnings**:
    - Apply `cargo clippy --fix --allow-dirty --allow-staged -p $pkg`
      first for low-risk auto-fixes.
    - Review each remaining warning manually. Prefer fixing the
      underlying issue over `#[allow(...)]`. Where `#[allow(...)]` is
      genuinely correct (false positive, intentional API), document
      the rationale in a code comment.
    - Maintain the rustdoc convention from the project CLAUDE.md
      (no H1 inside `///`, H2 sections only, recommended order).
5. **Triage and fix any test failures.** If a test is broken by an
   environmental factor (e.g., missing `dirs::home_dir()`), make the
   test resilient to that environment.
6. **Run the area-level `just` recipes** when present.
   ```bash
   just test claudine 2>/dev/null || true
   just lint claudine 2>/dev/null || true
   just doctest claudine 2>/dev/null || true
   ```
   Per the project CLAUDE.md, the root `just` may not cover every
   workspace member; supplement with direct `cargo` invocations from
   step 2/3.
7. **Capture the final state**:
    - Record any test counts, snapshot assertions, and clippy zero
      warnings/errors.
    - Update `claudine/features/2026-04-26-centralized-providers/review-3.md`
      front-matter `ready` to `true` ONLY IF the user requests it
      (this plan does not auto-flip the ready flag).

### Verification Commands

```bash
# Tests across all claudine crates
cargo test -p claudine --all-features --tests
cargo test -p claudine --all-features --doc
cargo test -p claudine-cli --all-features --tests
cargo test -p claudine-cli --all-features --doc

# Lint across all claudine crates — must be ZERO warnings/errors
cargo clippy -p claudine --all-targets --all-features -- -D warnings
cargo clippy -p claudine-cli --all-targets --all-features -- -D warnings

# Workspace member discovery — re-verify nothing missed
cargo metadata --no-deps --format-version 1 \
  | jq -r '.packages[] | select(.manifest_path | contains("/claudine/")) | .name'

# Optional area `just` recipes
just test claudine 2>/dev/null || true
just lint claudine 2>/dev/null || true
just doctest claudine 2>/dev/null || true
```

End-of-phase gate (final feature-closure gate):

- [ ] `cargo test -p claudine` (tests + doctests) all pass.
- [ ] `cargo test -p claudine-cli` (tests + doctests) all pass.
- [ ] `cargo clippy -p claudine --all-targets --all-features -- -D warnings` clean.
- [ ] `cargo clippy -p claudine-cli --all-targets --all-features -- -D warnings` clean.
- [ ] All sub-crates discovered via `cargo metadata` are tested + linted.
- [ ] Phase 1 deprecated re-exports compile and the
      `deprecated_compatibility` integration test passes.
- [ ] Phase 2 `discover_agents_full` is catalog-driven and the
      "covers every catalog provider exactly once" test passes.
- [ ] Phase 3 wrapper registry is array-backed with no `_` wildcard
      and the new positive invariants pass.

---

## Final Verification Checklist

| Item | Phase | How verified |
|---|---|---|
| Issue 1 — `events::Provider` deprecated re-export restored | 1 | `deprecated_compatibility::events_provider_re_export_matches_canonical` passes |
| Issue 1 — `events::PROVIDERS_DISPLAY_ORDER` deprecated re-export restored | 1 | `deprecated_compatibility::events_providers_display_order_re_export_matches` passes |
| Issue 1 — `events::EventSupportLevel` deprecated re-export restored | 1 | `deprecated_compatibility::events_event_support_level_re_export_compiles` passes |
| Issue 1 — `agents::AgentId = Provider` deprecated alias restored | 1 | `deprecated_compatibility::agents_agent_id_alias_is_provider` passes |
| Issue 2 — `discover_agents_full` reads from `provider_info` only | 2 | `discover_agents_full_uses_catalog_sniff_binding` passes; the function source contains no provider literals |
| Issue 2 — Discovery covers every catalog provider exactly once | 2 | `discover_agents_full_covers_every_catalog_provider_exactly_once` passes |
| Issue 2 — Primary config path matches catalog templates | 2 | `discover_agents_full_primary_config_path_matches_catalog_template` passes |
| Issue 3 — Wildcard removed from `profile_for_provider` | 3 | `cli/src/commands/wrap/profile.rs` source has no `| _ =>` arm in the registry; registry uses array indexing tied to `PROVIDER_COUNT` |
| Issue 3 — Wrapper registry exhaustive against `PROVIDER_COUNT` | 3 | `wrapper_registry_covers_every_provider_and_documents_exceptions` passes; future variants fail compilation |
| Issue 4 — Drift guard catches arrays + alternate match forms | 3 | Hardened `no_unauthorized_match_provider_in_lib` reports zero violators with new pattern set |
| Issue 4 — Positive invariants supplant text scan as primary safety net | 3 | All discovery + wrapper + config-path invariants pass |
| All claudine tests pass | 4 | `cargo test -p claudine` + `cargo test -p claudine-cli` both green |
| All claudine crates lint clean | 4 | `cargo clippy ... -D warnings` returns success for every claudine crate |

---

## Cross-Phase Risks and Notes

- **Phase 1 cosmetic risk** — restoring `events/provider.rs` as a
  deprecated re-export module may collide with the current
  `events/mod.rs:28` comment "Phase 8: `Provider` … previously lived
  in `crate::events::provider`." Update that comment to reflect that
  the deprecated re-export module still exists for the migration
  window.
- **Phase 2 risk: `dirs::home_dir()` returning `None`** — the existing
  function uses `unwrap_or_else(|| PathBuf::from("~"))`. Preserve that
  fallback to avoid behavior regression. The new test asserting
  `primary_config_path_matches_catalog_template` must use the same
  `home` value as production code so it does not diverge under CI
  environments without `HOME`.
- **Phase 2 deferred decision** — Option A (first-element convention)
  vs Option B (typed scope wrapper). Plan defaults to Option A; if
  reviewers prefer explicit scope, implementing Option B incurs a
  per-provider `ConfigPathEntry` migration in every
  `provider/<name>.rs` file, which is bigger but more
  self-documenting.
- **Phase 3 regex limitations** — without `syn`, the comment-stripping
  + regex approach can produce false positives on unusual code. If the
  hardened guard flags legitimate code, allow-list with an inline
  comment explaining the relocation (preferred) or convert the guard
  to a `syn`-based AST walk (more invasive — defer unless tests
  expose the need).
- **Phase 4 scope creep** — fixing pre-existing clippy warnings
  unrelated to review-3 is required by the user prompt. If the
  warning count is large, batch fixes by category (e.g., one commit
  per lint family) so review remains tractable.
- **Crate inventory** — assumed `claudine` (lib) and `claudine-cli`
  (cli). Phase 4 task 1 verifies this with `cargo metadata`. If
  additional sub-crates are present (e.g., a server crate), the
  `for pkg in ...` loops cover them automatically.

---

## Mapping Back to Review-3 Findings

| Review Finding | Severity | Phase | Closure Mechanism |
|---|---|---|---|
| 1. Public compatibility re-exports removed early (`events::Provider`, `events::PROVIDERS_DISPLAY_ORDER`, `agents::AgentId`) | High | 1 | Restore `#[deprecated]` re-exports + integration test that imports them |
| 2. Agent discovery uses hand-maintained provider table | Medium | 2 | Drive `discover_agents_full` from `all_providers()` and `info.config_paths[0]` + invariant test |
| 3. Wrapper registry weaker than designed (wildcard) | Medium | 3 | Array-backed `[Option<&dyn WrapperProfile>; PROVIDER_COUNT]` registry, no `_` arm |
| 4. Dispatch drift guard too narrow | Low | 3 | Broaden pattern set to include arrays and alternate match forms; add positive invariants for discovery, wrapper, and config-path coverage |
