# Phase 4 library path classification

Date: 2026-08-01
Branch: `fix/claudine-windows`

| Failure | Classification | Resolution |
| --- | --- | --- |
| Lifecycle proxy package reference | Portable fixture defect | Normalize the canonical temporary root back to the host's ordinary path spelling before comparing it with resolver output. |
| Lifecycle prepared snapshot | Portable fixture defect | Normalize canonical temporary repository roots before string comparison. |
| Lifecycle `ctx_base_dir` capture | Portable fixture defect | Normalize canonical temporary repository roots before string comparison. |
| Direct composition shell CWD | Portable fixture defect | Compile a native probe that reports its working directory instead of invoking `pwd`, whose availability and path spelling are shell-dependent. |
| Nested preflight resolution inputs | Portable fixture defect | Canonicalize the temporary root with `dunce` so native identity is preserved without a display-string round trip. |
| Sequence tilde reference | Portable fixture defect | Supply the test home through `FileResolutionContext`; Windows home discovery does not use the `HOME` variable. |
| Claude hook re-registration: add event | Product defect | Quote whitespace-containing executable paths when producing shared hook commands, then parse the bounded leading executable token and recognize native `claudine.exe` commands case-insensitively without matching unrelated executable basenames. |
| Claude hook re-registration: remove event | Product defect | Same `.exe` recognition defect. |
| Claude hook registration already in sync | Product defect | Same `.exe` recognition defect. |
| Codex missing wrapper sync | Portable fixture defect | Serialize the native wrapper path as a TOML string instead of embedding unescaped backslashes. |
| Harness implicit no-match detail | Portable fixture expectation defect | Compare diagnostic paths using the portable path-text contract. |
| Messaging absolute image path | Portable fixture defect | Use a host-native absolute path instead of a POSIX-rooted literal. |
| Codex workspace-write permissions | Portable fixture defect | Use native temporary writable/outside roots and TOML-safe serialization. |
| Explicit system-prompt tilde reference | Portable fixture defect | Add a private explicit-home resolver seam and keep the public entry point bound to OS home discovery. |
| Non-interactive prompt candidate order | Portable fixture defect | Exercise the same explicit-home seam rather than mutating `HOME`. |

No behavior is gated by platform. Public system-prompt resolution continues to use `dirs::home_dir()`, preserving native home-directory semantics on Unix and Windows.

## Post-acceptance cluster (2026-08-02)

Two library failures appeared on native Windows after the Phase 6 acceptance
boundary. Both were introduced by `8a11bd9c4` (`fix(claudine): propagate
invocation context`), whose new fixtures were authored where the defect is
invisible, and both are fixture defects rather than product defects. The
acceptance record above is unchanged.

| Failure | Classification | Resolution |
| --- | --- | --- |
| `cross_repo_task_nested_reference_uses_its_own_repository_context` | Portable fixture defect | Compare the projected document path with `fs::canonicalize`, which is the spelling preflight's `canonical()` key produces. `dunce` belongs at the display boundary (`biscuit_file::to_portable_string`), not to internal path identity; the sibling `nested_references_reuse_all_request_resolution_inputs` already compared this way. |
| `non_repository_session_runs_shell_in_launch_cwd` | Portable fixture defect | Replace the `cmd /C cd` branch with the compiled cwd probe the direct-composition test already uses. Darkmatter's built-in blacklist rejects every Windows shell able to report a working directory and no whitelist overrides it, so the fixture could never have run. The probe reports portable text, keeping the assertion a full-path comparison. |

The probe reports portable text for the reason recorded against
`a_file_valued_overlay_property_resolves_through_the_targets_own_context` in
the CLI path note: CommonMark cleanup consumes the `\` of a `\.` sequence, so a
native Windows path emitted into composed Markdown loses the separator of any
segment beginning with punctuation. Darkmatter's native-identity/presentation
split covers effective-frontmatter interpolation; `::shell` output reaches the
document through a different path and is not covered by it. Recorded as an
observation only — no product change was made here.
