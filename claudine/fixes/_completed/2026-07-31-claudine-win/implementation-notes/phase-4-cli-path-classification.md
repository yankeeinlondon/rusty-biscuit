# Phase 4 CLI path classification

Native-Windows residuals were rerun with the CI profile and retries disabled.
This note records the locally owned CLI resolver/path tranche; the discovery
and process tranches have separate notes.

| Test | Classification | Owning cause | Resolution |
|---|---|---|---|
| `read_tool_call_renders_path_as_cwd_relative_blue_link` | Portable test-fixture defect | The test used POSIX `/repo` as both the Windows CWD and absolute tool path, so `Path::strip_prefix` could not produce a relative label. | Use a real temporary native directory and derive the tool path from it. |
| `shorten_source_path_strips_repo_root_prefix` | Portable test-fixture defect | The test supplied POSIX absolute strings to Windows `Path` semantics even though telemetry deliberately preserves native spelling. | Build the absolute and relative paths from native `Path` values and compare as paths. |
| `sequence_resolves_a_data_file_with_offset_and_operator` | Portable test-fixture defect | The integration staged an extensionless POSIX shell script as `goose`; Windows provider discovery requires a native executable. | Compile an equivalent `goose.exe` argument-capture fixture on Windows. |
| `file_references_resolve_from_the_authoring_document` | Portable test-fixture defect | Native Windows home discovery uses the profile Known Folder, so overriding `HOME` or `USERPROFILE` in a child process cannot isolate `~/` resolution. | Keep real CLI coverage for source-relative references and exercise the home-pinned case through the same explicit `FileResolutionContext` seam with a fixture home. |
| `a_file_valued_overlay_property_resolves_through_the_targets_own_context` | Darkmatter-owned product path defect | Darkmatter's eager caller-file resolver correctly retains an absolute native path in effective frontmatter, but the same native string became body-interpolation state. CommonMark cleanup then consumed `\.` in a Windows temporary path as an escape. | Darkmatter now carries a separate portable presentation value through interpolation while preserving native effective-frontmatter identity. The focused Claudine overlay regression passes on native Windows. |

The sequence fixture passes under the focused CLI integration test. The overlay
test also passes with Darkmatter's native-identity/presentation-value split.
GitNexus could not resolve either test symbol (risk `UNKNOWN`); direct inventory
found no callers or production-flow participation.

## Post-acceptance cluster (2026-08-02)

Two CLI failures appeared on native Windows after the Phase 6 acceptance
boundary, alongside the two library failures recorded in the library path note.
All four came from `8a11bd9c4` (`fix(claudine): propagate invocation context`).
Neither is a product defect, and the acceptance record above is unchanged.

| Test | Classification | Owning cause | Resolution |
|---|---|---|---|
| `inline_compose_writes_hash_that_passes_md_diff` | Host provisioning gap in the fixture | `md_bin()` probed only the workspace target profile directory. `just init` does install `md` — root `justfile` → `darkmatter`'s `install` → `cargo install --path ./cli` — but that populates the Cargo bin directory, which the probe never consulted. Linux runs passed because a workspace build happened to be present. | Fall back to `PATH` after the target-directory probe, so an `init`-provisioned `md` satisfies the test while a workspace build still wins when one exists. |
| `isolated_fixture_can_opt_in_to_user_prompt_discovery` | Portable test-fixture defect | The same Known Folder inertness recorded in the home/config discovery note: the subprocess fixture treated `HOME`/`USERPROFILE` as an injectable user-global root, so on Windows the child read the real user profile and fell back to the built-in prompt. | Gate the subprocess assertion `#[cfg(unix)]` with that reason, matching the nine MCP fixtures, and pin the user-home leg of standard discovery on every platform through the explicit-home seam (`standard_discovery_user_home_is_injectable`). The pre-existing `standard_discovery_user_home_fallback` asserts nothing when the real profile lacks the file. |

The same commit also left two imports used only by `#[cfg(unix)]` tests —
`std::fs` in `propagated_context_fixtures.rs` and `CliProcessFixture` in
`contextual_errors.rs` — which warn on Windows and so fail the warnings-denied
lint gate. Both are gated with their consumers, as the home/config discovery
note did for `mcp_cli`.

Native Windows verification: `claudine-cli` 2,018/2,018 with 8 skipped and
`claudine` 3,891/3,891, both with retries disabled; Clippy clean across both
packages with `--all-features --tests`. Windows now runs one fewer CLI test by
design. Linux execution remains unavailable on this host (the WSL distribution
fails to start), so the Unix legs are unverified here.
