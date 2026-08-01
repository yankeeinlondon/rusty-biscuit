# Phase 4 home/config discovery cluster

## Classification

The completion, MCP, skills, actions, and hooks failures were **portable test
fixture defects**. Their subprocess fixtures treated `HOME` as an injectable
user-global root. On Windows, `dirs` 6 obtains the profile through the Known
Folder API, so changing `HOME` does not redirect product discovery. Per the
source specification, that inertness is native behavior and must not be
replaced with HOME-first production discovery.

The fixtures were corrected according to the behavior under test:

- completion and skills subprocess tests now use initialized repository
  fixtures and nested working directories for repo-scoped end-to-end behavior;
- true user-global completion behavior is tested through an injected
  `ScopeContext` home for both empty and typed partials;
- user-only skills discovery remains covered by the library's explicit-root
  fixture, while a pure CLI regression verifies the non-git user-only footer
  without redirecting the native user home;
- actions load an explicit config fixture, while an internal production
  renderer test preserves the `Report` / `SessionStart` output contract;
- the selection test injects an explicit config path through its private
  loader seam; the prep-context regression calls the production constructor
  from a scoped real repository CWD and proves both the effective selection
  root and captured launch repository fallback structurally;
- hooks retain home-independent command routing and exercise the complete
  invalid-sound warning renderer with an injected config, including the
  warning header, escaped effect name, hint, and atomic-token guard;
- nine MCP command/output fixtures retain their full end-to-end assertions on
  Unix, where `$HOME` is the isolation facility they require. They are
  `#[cfg(unix)]` with that reason; cross-platform storage behavior remains
  covered through the catalog and provider-state explicit-path store tests;
- the Codex SQLite fallback test now compares against the native
  `dirs::home_dir()` result instead of assuming that a temporary `HOME`
  redirects the Windows Known Folder API.

`skills_fix_shows_summary` also exposed a separate **Windows filesystem
defect**: Windows resource links need the target kind to select
`symlink_dir` or `symlink_file`. The product fix preserves the existing Unix
`symlink` behavior and the shared collision/already-linked handling. Native
Windows temp-only tests cover file-link creation, real-file collision, and the
already-linked result.

## Verification

- Baseline cluster: 59/83 passed; 24 failed.
- Corrected native-Windows cluster: 74/74 passed with retries and fail-fast
  disabled. The remaining nine original cases are the Unix `$HOME` fixtures.
- Injected internal regressions passed for user-global completion, non-git
  user-only skills/footer reporting, actions rendering, and invalid-sound
  warning rendering.
- Residual selection, prep-context, and Codex SQLite home fixtures passed 3/3
  with explicit selection config injection, structural production-wrapper CWD
  fallback coverage, or the native-home expectation.
- Test-placement guard: 9/9 passed.
- `mcp_cli` no-run compilation emits no warnings from that test target after
  Unix-only imports and fixture helpers were gated with their consumers.
- `skills_fix_shows_summary` passed end to end on native Windows, including
  Windows directory-link creation.
- Native Windows file-link creation, collision, and already-linked regressions
  passed 3/3 using temporary roots only.
