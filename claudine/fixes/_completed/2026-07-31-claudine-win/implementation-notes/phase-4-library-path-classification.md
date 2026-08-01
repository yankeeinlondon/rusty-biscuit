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
