---
phases: 5
start_phase: 5
source_files_during_phase_5:
  - schematic/gen/tests/terminal_capture.rs
  - schematic/justfile
docs_updated_during_phase_5:
  - schematic/gen/README.md
  - schematic/docs/io/export-openapi.md
  - schematic/README.md
  - schematic/features/ergonomics-and-postman-projects/review-plan-1.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/schematic/SKILL.md
packages:
  - schematic-gen
---
# Review-1 Fix Plan: Ergonomics And Postman Collections

## Scope

This plan addresses every recommendation in
`/Users/ken/.claudine/worktrees/rusty-biscuit/schematic/schematic/features/ergonomics-and-postman-projects/review-1.md`.

The three reviewer findings, ordered by severity:

1. **High** — OpenAPI export is per-API instead of per-module, so shared-module
   APIs (`OllamaNative` + `OllamaOpenAI`, `EmqxBasic` + `EmqxBearer`) emit
   separate files (`ollamanative.json`, `ollamaopenai.json`, `emqxbasic.json`,
   `emqxbearer.json`) instead of a single grouped `ollama.json` / `emqx.json`.
2. **Medium** — Filenames are derived from `api.name.to_lowercase()` rather than
   from the resolved module path returned by
   `schematic_gen::export::resolve_module_name(api)`, so OpenAPI artifacts drift
   from the generated Rust module layout (e.g. `huggingfacehub.json` vs
   `huggingface`, `samsungsmarttv.json` vs `samsung_smart_tv`).
3. **High** — There is no Level 2 verification (real terminal capture) of the
   colored CLI output produced by `schematic-gen validate` and `generate`.

The reviewer also flagged a Low-severity observation (registry is hard-failed
during `--api all` instead of warn-and-skip). Per the tech design, the **final
state is strict failure**, and registry coverage is now complete for every API
that the generator emits — so this is the intended terminal behavior. The plan
keeps strictness but documents it via test coverage and a CLI escape hatch
(`--no-openapi`) so the migration story remains correct.

The phases are ordered to minimize rework:

- Phase 1 lands the foundational refactor (registry grouping + grouped writer)
  with unit tests.
- Phase 2 wires the new writer into the CLI driver and stops using
  `api.name.to_lowercase()` anywhere as a filename source.
- Phase 3 regenerates committed artifacts and updates drift detection.
- Phase 4 adds Level 2 terminal-capture coverage.
- Phase 5 is the lint/doc sweep and final sign-off.

Working directory for every command below:
`/Users/ken/.claudine/worktrees/rusty-biscuit/schematic`

Quick reference for affected files:

- `schematic/gen/src/main.rs` (lines 538–805 — see grouped Postman block as a
  template for grouped OpenAPI)
- `schematic/gen/src/openapi_output.rs`
- `schematic/gen/src/postman_output.rs` (`build_postman_collection_grouped` /
  `write_postman_grouped` — reference shape, lines 680–752)
- `schematic/gen/src/export/naming.rs` (`resolve_module_name`)
- `schematic/gen/tests/artifact_drift.rs`
- `schematic/gen/tests/e2e_generation.rs`
- `schematic/definitions/src/registry.rs` (add `get_registries_for_module`)
- `schematic/definitions/src/lib.rs` (`apis_by_module`)
- `schematic/openapi/*.json` (regenerated)

---

## Phase 1 — Grouped registry + grouped OpenAPI writer (foundational)

### Goal

Create the building blocks for module-grouped OpenAPI export: a registry
lookup that returns the union of registries for every API in a module, a
`write_openapi_grouped` writer in `openapi_output.rs`, and unit-test coverage
for both. No CLI behavior changes yet.

### Code changes

1. `schematic/definitions/src/registry.rs`
    - Add `pub fn get_registries_for_module(module_name: &str)
      -> Option<SchemaRegistry>`.
        - Iterate `apis_by_module()` to find the matching module bucket
          (case-insensitive on `module_name`).
        - For each `RestApi` in that bucket, compute the registry key the same
          way the existing `api_names` table in `main.rs` does, and look up the
          per-API registry via `get_registry`.
        - Merge all returned `SchemaRegistry` instances into one. Add a small
          private `SchemaRegistry::merge(self, other: SchemaRegistry) -> Self`
          (or a `pub(crate) fn extend(&mut self, other: SchemaRegistry)`) that
          inserts every entry from `other.types` into `self.types`, preserving
          insertion order (`IndexMap::insert` already overwrites duplicates,
          which is the desired union behavior — the same response type shared
          across two module siblings collapses to one entry).
        - Return `None` only when the module name is unknown OR when **every**
          member API has a missing registry. If at least one member registry
          exists, return `Some(merged)` so callers can decide policy.
    - Inline the per-API name → registry-key lookup table that currently lives
      in `main.rs` so callers do not need to re-encode it. Recommended shape:
      `fn registry_key_for(api_name: &str) -> &'static str` that mirrors the
      table at `main.rs:687–703`. Re-export it via the `registry` module.

2. `schematic/gen/src/openapi_output.rs`
    - Add `pub fn write_openapi_grouped<R: SchemaRegistryLike>(apis: &[&RestApi],
      module_name: &str, registry: &R, options: &ExportOptions, dir: &Path)
      -> Result<PathBuf, GeneratorError>`.
    - For each API in `apis`, call the existing
      `schematic_define::openapi::export(api, registry, options)` to get an
      `openapiv3::OpenAPI` document.
    - Merge the documents into a single output document, using the **first
      API in the slice as the seed** for `info`, `servers`, `external_docs`,
      and `extensions`, then:
        - Override `info.title` with a humanized form of `module_name`
          (e.g. capitalise first letter; for multi-word names, prefer the
          first member API's `name` if it differs only by suffix). A simple
          `module_name.to_string()` is acceptable for now — the title is
          informational, not a contract — but document the choice in the
          `///` comment.
        - Union `paths` (each operation must remain attached to its method;
          if two members emit the same `path` + `method`, fail with
          `GeneratorError::ConfigError` and name both source APIs).
        - Union `components.schemas` (`IndexMap::insert` overwrites; identical
          schemas collapse harmlessly).
        - Union `components.security_schemes` from every member document. If
          two members use the same scheme name with **different** definitions,
          rename the second to `<scheme>_<api_name_snake>` and rewrite that
          API's per-operation `security` requirements to reference the renamed
          scheme. (Single-scheme, single-name modules — the common case —
          require no special handling.)
        - Union top-level `security` requirements; deduplicate identical
          entries, preserving insertion order.
        - Union `tags` (currently always empty — left for future use).
        - For the `x-schematic` extension on the document, prefer the
          first API's value but record the list of contributing API names
          under a new sibling key `x-schematic-grouped-from` so the artifact
          is self-describing. (The existing
          `SchematicDocExtension::module_path` already gives module identity;
          this just lists contributors.)
    - Filename: always
      `dir.join(format!("{module_name}.{ext}"))` where `ext` comes from
      `options.format` (json/yaml). Do **not** touch `api.name`.
    - Update the existing `write_openapi` `///` doc comment to mention that
      `write_openapi_grouped` is the preferred entry point and that
      `write_openapi` is retained for the single-API case (it should also be
      switched to `resolve_module_name(api)` for the filename — see Phase 2).

3. `schematic/gen/src/lib.rs`
    - Re-export `pub use openapi_output::{write_openapi, write_openapi_grouped};`
      so downstream tests can import the new symbol the same way they import
      `write_postman_grouped`.

### Test changes

All tests live alongside the production code (Rust convention) unless noted.

1. `schematic/definitions/src/registry.rs` (existing test module)
    - `get_registries_for_module_returns_union_for_ollama` — assert the
      returned registry contains every type that `ollama-native` and
      `ollama-openai` registries register; assert
      `validate_completeness` succeeds against both `define_ollama_native_api()`
      and `define_ollama_openai_api()`.
    - `get_registries_for_module_returns_union_for_emqx` — same, for
      `emqx-basic` + `emqx-bearer`.
    - `get_registries_for_module_returns_single_for_singleton_module` —
      e.g. `openai`, asserting the result equals `get_registry("openai")`
      content-wise (compare key sets).
    - `get_registries_for_module_unknown_returns_none`.
    - `registry_key_for_known_apis_matches_table` — table-driven test that
      iterates the previous table from `main.rs` to ensure no key drift.

2. `schematic/gen/src/openapi_output.rs` (existing test module)
    - `write_openapi_grouped_creates_module_named_file` — pass two synthetic
      single-endpoint `RestApi` instances with `module_path = Some("foo")`,
      assert the output filename is `foo.json`.
    - `write_openapi_grouped_unions_paths` — two APIs with disjoint paths;
      assert the resulting document contains both paths.
    - `write_openapi_grouped_collides_on_duplicate_method_path` — two APIs
      both emitting `GET /foo`; assert the function returns
      `GeneratorError::ConfigError` and the message contains both API names.
    - `write_openapi_grouped_unions_security_schemes_distinct` — two APIs
      with different `AuthStrategy` values; assert both schemes appear in
      `components.securitySchemes`.
    - `write_openapi_grouped_uses_module_name_in_info_title`.
    - `write_openapi_grouped_emits_grouped_extension` — assert the document
      carries `x-schematic-grouped-from` listing both contributors.
    - `write_openapi_grouped_yaml_round_trips` — same as JSON test but with
      `ExportFormat::Yaml`.
    - `write_openapi_grouped_real_ollama_module` — uses
      `define_ollama_native_api()` + `define_ollama_openai_api()` and
      `get_registries_for_module("ollama")`, then re-parses the JSON via
      `serde_json::from_str::<openapiv3::OpenAPI>` and asserts:
        - `info.title` contains `Ollama` (case-insensitive),
        - `paths` contains at least one path from each member API,
        - `components.schemas` is non-empty.

### Verification

```bash
cargo test -p schematic-definitions registry::tests
cargo test -p schematic-gen openapi_output::tests
cargo test -p schematic-gen --doc
cargo clippy -p schematic-definitions --all-targets -- -D warnings
cargo clippy -p schematic-gen --all-targets -- -D warnings
```

Phase 1 is complete when all five commands succeed and the new symbols
(`get_registries_for_module`, `registry_key_for`, `write_openapi_grouped`)
exist with green tests. The CLI behavior has not changed yet.

---

## Phase 2 — Wire grouped OpenAPI into the CLI driver, kill `api.name` filenames

### Goal

Replace the per-API OpenAPI loop in `run_generate_all` with a per-module loop
mirroring the existing Postman block, and fix every other site that derives a
filename from `api.name.to_lowercase()` so artifacts are named exclusively
from `resolve_module_name`. After this phase the generator emits
`schematic/openapi/{ollama,emqx,huggingface,samsung_smart_tv,unfolded_circle_core_rest}.json`
in place of the current per-API files.

### Code changes

1. `schematic/gen/src/main.rs` (lines 538–735)
    - Delete the `api_names` table at lines 687–703 (it now lives in
      `schematic_definitions::registry::registry_key_for`).
    - Replace the `for api in &apis` OpenAPI loop with the grouped pattern
      already used for Postman:
        ```rust
        let grouped = apis_by_module();
        for (module_name, module_apis) in grouped.iter() {
            // Track expected filename
            let extension = match opts.openapi_format {
                OpenApiFormat::Json => "json",
                OpenApiFormat::Yaml => "yaml",
            };
            openapi_files.insert(format!("{module_name}.{extension}"));

            // Look up the merged registry for this module
            let Some(registry) =
                schematic_definitions::registry::get_registries_for_module(module_name)
            else {
                return Err(GeneratorError::ConfigError(format!(
                    "Missing schema registry for module \"{module_name}\". \
                     Add openapi_registry() to schematic-definitions or skip with --no-openapi."
                )));
            };

            run_openapi_export_grouped(
                module_name,
                module_apis,
                &registry,
                openapi_dir,
                opts.openapi_format,
                opts.openapi_version,
                opts.dry_run,
                opts.verbose,
            )?;
        }
        ```
    - Refactor `run_openapi_export` (currently lines 539–604) into
      `run_openapi_export_grouped` that takes `module_name: &str`,
      `apis: &[RestApi]`, and a pre-resolved registry. Single-API modules are
      handled by passing a one-element slice — no separate code path.
    - Inside the new function, call `write_openapi_grouped` instead of
      `write_openapi`. Replace `api.name.to_lowercase()` (line 588) with
      `module_name`.
    - Drop `run_openapi_export`'s lines 588–593 dry-run print and replace
      with a print that uses `module_name`.
    - Keep the per-`generate-one` path (`run_generate_one`) working: it should
      also call `run_openapi_export_grouped` with a one-element slice and the
      module name resolved via `schematic_gen::export::resolve_module_name`.
      This avoids two divergent code paths.

2. `schematic/gen/src/openapi_output.rs`
    - Update `write_openapi` (line 79) to derive the filename from
      `resolve_module_name(api)` instead of `api.name.to_lowercase()`.
      Update the existing tests `write_openapi_creates_json_file`,
      `write_openapi_creates_yaml_file`, and `write_openapi_file_name_lowercase`
      so their assertions still pass (the OpenAI sample sets
      `module_path = None` so the filename remains `openai.json`; add one
      explicit test using a `module_path = Some("foo_bar")` API to assert the
      filename becomes `foo_bar.json` not `foo.json` or
      `<api.name>.json`).

3. `schematic/gen/src/main.rs` (single-API generate path)
    - Find any remaining `api.name.to_lowercase()` references inside `run_generate_one` /
      `run_postman_export` and verify they already go through
      `resolve_module_name` (Postman path at line 519 already does — confirm,
      do not touch). Add a `// Filename derived from module name, NOT api.name`
      comment on each such line.

4. `schematic/gen/src/postman_output.rs`
    - Audit `build_postman_collection_grouped` to confirm `info.name` uses
      `module_name` (not the first API's `name`); fix if not. This prevents
      Postman-side filename/info drift symmetric to the OpenAPI fix.

### Test changes

1. `schematic/gen/src/main.rs`
    - Add a small in-process integration test (already imports
      `tempfile::TempDir` indirectly) that calls `run_generate_all` with
      `dry_run = true, no_openapi = false, no_postman = true` and asserts
      the printed dry-run output contains `ollama.json` exactly once and
      does **not** contain `ollamanative.json` or `ollamaopenai.json`.
      Capture stdout with the existing `print_to_string` helper if present;
      otherwise, refactor the inner write step to return the filename list
      and assert against that. (Choose whichever is least invasive.)

2. `schematic/gen/tests/e2e_generation.rs`
    - Add `#[test] fn generated_openapi_filenames_use_module_names()`:
        - run `generate_and_write_all` then call the new grouped writer for
          every module via `apis_by_module()` into a `TempDir`,
        - assert the temp dir contains `ollama.json`, `emqx.json`,
          `huggingface.json`, `samsung_smart_tv.json`,
          `unfolded_circle_core_rest.json`,
        - assert it does **not** contain `ollamanative.json`,
          `ollamaopenai.json`, `emqxbasic.json`, `emqxbearer.json`,
          `huggingfacehub.json`, `samsungsmarttv.json`,
          `unfoldedcirclecorerest.json`,
        - parse `ollama.json` via `serde_json::from_str::<openapiv3::OpenAPI>`
          and assert it contains paths owned by both Ollama variants.
    - Add `#[test] fn generate_all_fails_when_module_registry_missing()`:
        - construct a synthetic two-API module whose registry is `None`,
        - call `run_generate_all` (or its inner helper),
        - assert the returned error is `GeneratorError::ConfigError` and
          its message names the module.

3. `schematic/gen/tests/openapi_import_test.rs`
    - If any of its tests reference `ollamanative.json` or similar, update
      them to the new module-named files. (Re-read the file during execution;
      no edits needed if it does not.)

### Verification

```bash
cargo test -p schematic-gen
cargo test -p schematic-definitions
cargo clippy -p schematic-gen --all-targets -- -D warnings
cargo clippy -p schematic-definitions --all-targets -- -D warnings
```

Phase 2 is complete when all four commands succeed and the new in-process
test confirms grouped filenames in the dry-run output.

---

## Phase 3 — Regenerate committed artifacts + tighten drift detection

### Goal

Bring the committed `schematic/openapi/` directory in line with the new
naming, delete the old per-API files, and ensure the drift detection tests
treat module-grouped output as canonical.

### Code changes

1. `schematic/openapi/`
    - Delete: `emqxbasic.json`, `emqxbearer.json`, `huggingfacehub.json`,
      `ollamanative.json`, `ollamaopenai.json`, `samsungsmarttv.json`,
      `unfoldedcirclecorerest.json`.
    - Regenerate by running:
        ```bash
        just -f schematic/justfile generate
        cargo check -p schematic-schema
        cargo check --manifest-path schematic/schema/Cargo.toml
        ```
    - Confirm the new files appear: `emqx.json`, `huggingface.json`,
      `ollama.json`, `samsung_smart_tv.json`,
      `unfolded_circle_core_rest.json`. Existing single-module files
      (`openai.json`, `anthropic.json`, etc.) should remain byte-identical
      except for any change introduced by `write_openapi_grouped`'s shared
      code path — which is expected and acceptable.
    - Do not commit any leftover `*.json` files that are no longer produced.
      The cleanup pass at `main.rs:732` (`cleanup_stale_artifacts`) should
      remove them automatically when running `just generate`; verify with
      `git status` afterwards.

2. `schematic/postman/`
    - Re-run generation (covered by `just generate`); diff should be empty
      because Postman grouping is already correct.

3. `schematic/gen/tests/artifact_drift.rs`
    - Delete the `api_names` table at lines 26–43 — it lives in
      `schematic_definitions::registry::registry_key_for` now.
    - Rewrite `generated_openapi_artifacts_are_up_to_date` to iterate
      `apis_by_module()` and call `write_openapi_grouped` per module
      (mirroring the Postman test), then compare against
      `schematic/openapi/<module>.json`.
    - Make both drift tests **non-`#[ignore]`** for fast modules (≤ 16
      modules, all in-process — no cargo invocation), so they run in
      `cargo test` by default and CI catches drift on every PR. Keep them
      runnable from the package root by reading `CARGO_MANIFEST_DIR` to
      anchor the `openapi/` and `postman/` paths (currently they assume
      cwd == `schematic/gen`).
    - Add `generated_openapi_no_legacy_per_api_files`: walk `schematic/openapi`
      and assert no file matches the deleted-name allow-list above; this
      pins the regression so it cannot silently come back.

### Test changes

(Covered above — drift tests become unignored and use module grouping.)

### Verification

```bash
just -f schematic/justfile generate
cargo check -p schematic-schema
cargo check --manifest-path schematic/schema/Cargo.toml
cargo test -p schematic-gen
cargo test -p schematic-gen --test artifact_drift
git status schematic/openapi schematic/postman   # expect: deletions of legacy files + adds of module-named files; nothing else unexpected
```

Phase 3 is complete when `git status` shows only the expected
add/delete/modify set, all generated files round-trip through
`openapiv3::OpenAPI`, and the drift tests pass without `--ignored`.

---

## Phase 4 — Level 2 terminal-capture verification

### Goal

Add real-terminal capture tests for the user-observable, colored CLI output
of `schematic-gen validate` and `schematic-gen generate --dry-run`. Both
`tmux` and `wezterm` are installed on the dev host
(`/opt/homebrew/bin/tmux`, `/opt/homebrew/bin/wezterm`); the tests must
gracefully skip when neither is available so CI environments without a TTY
remain green.

### Code changes

None to production code. Tests only.

### Test changes

1. `schematic/gen/tests/terminal_capture.rs` (new)
    - Module-level doc:
      `//! Level 2 terminal-capture tests for schematic-gen CLI output.`
      `//! Requires tmux to be on PATH; otherwise tests are skipped.`
    - Helper: `fn tmux_available() -> bool { Command::new("tmux").arg("-V").output().map(|o| o.status.success()).unwrap_or(false) }`
    - Helper: `fn schematic_gen_bin() -> PathBuf` — return
      `env!("CARGO_BIN_EXE_schematic-gen")` (Cargo provides this path for
      bin crates). This avoids needing to rebuild during the test.
    - Helper: `fn run_in_tmux(cmd: &str) -> String`:
        - create a unique session name (`format!("schematic-gen-{}", uuid_or_pid)`),
        - `tmux new-session -d -s <name> -x 200 -y 50 <shell>` so the pane
          width is wide enough to avoid wrap noise,
        - `tmux send-keys -t <name> '<cmd>; printf DONE-MARKER\n' Enter`,
        - poll `tmux capture-pane -t <name> -p -e` (the `-e` flag includes
          escape sequences) until the buffer contains `DONE-MARKER`
          (or 10s timeout — fail with the captured buffer for diagnosis),
        - `tmux kill-session -t <name>`,
        - return the captured string.
    - Tests:
        - `validate_emits_ansi_styled_ok_on_success`:
            - skip if `!tmux_available()`,
            - run `<bin> validate --api openai` in tmux,
            - assert the captured buffer contains the SGR sequence
              `\x1b[1;32m` (bold green) and the text `OK`,
            - assert it contains `\x1b[2m` or `\x1b[22m` (dim) for the
              "Validating ..." status line.
        - `generate_dry_run_emits_module_grouped_filenames`:
            - skip if `!tmux_available()`,
            - run `<bin> generate --api all --output schematic/schema/src --openapi-out /tmp/sg-test-openapi --postman-out /tmp/sg-test-postman --dry-run` in tmux,
            - assert the captured buffer contains `ollama.json` and
              `emqx.json`,
            - assert it does **not** contain `ollamanative.json` or
              `emqxbasic.json` (Phase 2 regression guard from the user's
              point of view),
            - assert it contains the SGR sequence `\x1b[1;32m` for
              `[OK]` markers.
        - `generate_strict_failure_emits_red_error_when_registry_missing`:
            - skip if `!tmux_available()`,
            - point the binary at a synthetic module that has no registry
              (use a tiny test fixture written to a temp file, or
              parameterise via env var if simpler),
            - assert the captured buffer contains `\x1b[31m` (red),
              `Error:`, and the module name.
    - Mark all three with `#[test]` (no `#[ignore]`) — they are fast
      (< 1s each) and stable, but they self-skip when tmux is missing.

2. Optional: `schematic/gen/tests/wezterm_capture.rs` (new, optional but
   recommended for parity with the existing `biscuit-tui` Level-2 patterns)
    - Same shape as above but using `wezterm cli spawn` /
      `wezterm cli get-text`. Skip if `wezterm --version` fails. This
      provides a second SGR/layout engine cross-check.

### Verification

```bash
cargo test -p schematic-gen --test terminal_capture
# Expect: 3 passed (or "ignored: tmux not available" on TTY-less hosts)

# Optional second engine:
cargo test -p schematic-gen --test wezterm_capture
```

Phase 4 is complete when at least one terminal-capture test file passes
locally with non-empty SGR assertions and self-skips on hosts without
the relevant emulator.

---

## Phase 5 — Lint, docs, and final sign-off

### Goal

Eliminate every clippy warning across the schematic area (including the
out-of-workspace `schematic/schema` crate), refresh user-facing docs, and
perform the final acceptance check.

### Code changes

1. Documentation
    - `schematic/gen/README.md`:
        - Replace any per-API filename examples (e.g. `ollamanative.json`)
          with the module-grouped names.
        - Document `write_openapi_grouped` in the public-API table.
    - `schematic/docs/io/export-openapi.md`:
        - Add a "Shared modules" subsection explaining that
          `OllamaNative + OllamaOpenAI -> ollama.json`,
          `EmqxBasic + EmqxBearer -> emqx.json`.
        - Note the strict registry policy and the `--no-openapi` escape
          hatch.
    - `schematic/README.md`:
        - Update any artifact-listing snippets to the new filenames.
    - `schematic/CLAUDE.md` (if it exists in the schematic subtree — check
      first; do not create one):
        - Note the module-grouped output convention if not already captured.

2. Code lint sweep
    - Run clippy across every schematic package (see verification block).
    - Fix any warnings introduced by Phases 1–4. Common expected suspects:
      unused imports after deleting the `api_names` table, missing
      `#[must_use]` on the new `get_registries_for_module`, missing `Errors`
      sections on new `pub fn`s.

### Test changes

None new; this phase only re-runs the full suite.

### Verification

```bash
# Workspace-member packages
for pkg in schematic-define schematic-definitions schematic-gen schematic-oauth; do
  cargo test  -p "$pkg"
  cargo clippy -p "$pkg" --all-targets -- -D warnings
done

# Out-of-workspace schema crate
cargo test  --manifest-path schematic/schema/Cargo.toml
cargo clippy --manifest-path schematic/schema/Cargo.toml --all-targets -- -D warnings

# Justfile end-to-end
just -f schematic/justfile generate
just -f schematic/justfile lint
just -f schematic/justfile test
```

Phase 5 is complete when every command above exits 0 with no warnings.

---

## Done When

- [ ] `get_registries_for_module` exists in `schematic-definitions::registry`
      and is unit-tested for `ollama`, `emqx`, and a singleton module.
- [ ] `write_openapi_grouped` exists in `schematic-gen::openapi_output`,
      handles path/scheme unioning, and is unit-tested.
- [ ] `run_generate_all` iterates `apis_by_module()` for OpenAPI export
      (no more per-API loop, no more `api_names` table in `main.rs`).
- [ ] Every OpenAPI filename — both in source and on disk — derives from
      `resolve_module_name(api)` / `module_name`; no remaining call site uses
      `api.name.to_lowercase()` for filename construction.
- [ ] `schematic/openapi/` contains `ollama.json`, `emqx.json`,
      `huggingface.json`, `samsung_smart_tv.json`,
      `unfolded_circle_core_rest.json` and **does not** contain the seven
      legacy per-API files.
- [ ] `cargo test -p schematic-gen --test artifact_drift` passes without
      `--ignored`, and includes a regression guard for the legacy filenames.
- [ ] At least one Level 2 terminal-capture test (tmux) verifies SGR output
      from `schematic-gen validate` and from `schematic-gen generate
      --dry-run`, with the test self-skipping when tmux is unavailable.
- [ ] `cargo test -p schematic-define -p schematic-definitions -p
      schematic-gen -p schematic-oauth` passes.
- [ ] `cargo test --manifest-path schematic/schema/Cargo.toml` passes.
- [ ] `cargo clippy -p <pkg> --all-targets -- -D warnings` passes for every
      schematic package.
- [ ] `cargo clippy --manifest-path schematic/schema/Cargo.toml --all-targets
      -- -D warnings` passes.
- [ ] `just -f schematic/justfile generate` produces a clean `git status`
      (no unexpected drift).
- [ ] `schematic/gen/README.md`, `schematic/docs/io/export-openapi.md`, and
      `schematic/README.md` reference the new module-grouped filenames.
