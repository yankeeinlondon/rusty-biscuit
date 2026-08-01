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
