# Phase 11 Editor Smoke Results

The per-editor manual smoke checklist is authored and shipped at
[`darkmatter/dmls/docs/editors/smoke-checklist.md`](../../dmls/docs/editors/smoke-checklist.md).
It exercises every v1 feature family (open, diagnostics, completion, definition,
references, hover, symbols/folding, frontmatter schema, rename, file rename,
formatting, no-side-effects) per editor.

## Automated packaging and launch coverage

The launch and packaging paths a live-editor smoke would exercise are now
guarded by executable checks that run non-interactively (no editor GUI/TTY):

| Check | What it proves | Where it runs |
|---|---|---|
| `dmls/tests/level2_stdio_subprocess.rs` (`level2_native_binary_speaks_lsp_over_real_stdio`) | The **compiled** `dmls` binary launches and drives `initialize → initialized → shutdown → exit` over real OS stdin/stdout pipes — the exact path every editor integration uses (not `Connection::memory()`). Bounded by a hard timeout that kills the child, so it can never hang the suite. | `cargo nextest run -p dmls`, `just test-l2`. Runs and passes here. |
| `dmls/tests/packaging_contract.rs` | The four per-platform archive names `just dist` produces (`dmls-<version>-{macos-universal,linux-x86_64,linux-aarch64,windows-x86_64}`) match the names the Zed extension downloads in `zed-dmls/src/lib.rs`. A rename on either side (a download 404) fails a test. | `cargo nextest run -p dmls`, `just test`. Runs and passes here. |
| `just check-zed` | The workspace-excluded `zed-dmls` WASM extension compiles against `wasm32-wasip1` (Zed's extension target). Folded into `just check`; skips cleanly when the target is absent. | `just check-zed` / `just check`. Compiled successfully here against `wasm32-wasip1`. |

`just dist` itself is not run in this environment: the macOS branch needs both
`aarch64-apple-darwin` and `x86_64-apple-darwin` release builds for the `lipo`
universal binary, and `x86_64-apple-darwin` is not installed here. The
`packaging_contract` test verifies the naming/layout contract statically instead
of doing a full cross-platform release build.

## Manual editor smoke status

Manual execution against the four live editors (VS Code, Zed, Neovim, Helix)
was **not performed in this implementation session** — it ran headless and
non-interactively, with no editor GUI/TTY available. This remains a manual QA
step to run on a workstation with the editors installed; the automated checks
above cover the launch/packaging surface, and the L2 in-memory LSP-session suite
drives a real `lsp_server` connection through the same request handlers every
editor uses:

| Checklist item | Automated proof |
|---|---|
| Open / lifecycle | `level2_lsp_session.rs` full-lifecycle sessions |
| Diagnostics (broken link, duplicate heading) | `level2_broken_link_diagnostic_updates_on_edit` |
| Navigation / symbols / folding / completion / hover | `level2_layer0_provider_round_trips` |
| Wiki links | `level2_wiki_link_navigation_diagnostics_and_completion`, `level1_wiki.rs` |
| Frontmatter schema + Claudine extension | `level2_frontmatter_schema_intelligence`, `level2_claudine_extension_is_pure_config` |
| DSL overlay (directive/transclusion/interpolation) | `level2_dsl_overlay_navigation_hover_and_diagnostics`, `level2_dsl_valid_document_has_no_dsl_diagnostics` |
| No side effects | `no_side_effects.rs` (spawn/socket proof) |
| Rename / file rename / formatting | `level2_heading_rename_rewrites_references`, `level2_rename_refuses_ambiguous_heading`, `level2_will_rename_files_updates_and_refuses`, `level2_formatting_is_byte_equivalent_to_library_cleanup` |

Client-specific rendering quirks (hover fidelity, folding fallback, snippet
expansion) are the only surface the automated suite cannot observe; the
`ClientProfile` gates for them are derived from the R-7 primary-source capability
matrix. Record any live-editor deviations here when the manual pass is run, and
fold newly discovered quirks into `ClientProfile` defaults.
