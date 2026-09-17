---
created: 2026-09-12
status: draft
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-09-12
implemented: true
implemented_by: claude/opus
review_iterations: 6
area: claudine
packages:
    - claudine
    - claudine-cli
---

# Preserve Provider Overlays Without Replacing the User Home

## Outcome

Claudine continues to isolate repository-scoped provider resources and inject
session-scoped MCP configuration, but it no longer changes the process-wide
user-home identity to do so.

A wrapped provider and every tool it launches inherit the same ordinary home
environment that the user supplied to Claudine. Provider configuration is
redirected only through a provider-owned, documented configuration or state
root. If a provider cannot satisfy the requested isolation through a verified
provider-owned mechanism, Claudine refuses the launch before spawning the
provider instead of changing `HOME`, weakening the requested isolation, or
falling back to a null home.

This preserves the useful overlay while keeping Git, OpenPGP, the GitHub CLI,
SSH, package managers, and other nested development tools in the user's normal
identity and credential context.

> **Reader's note:** The prior implementation treated the provider's config
> lookup root and the OS user's home as one control surface. This specification
> separates them. The directory currently called a “shadow home” remains useful
> as provider-overlay storage, but it must not become the child's global home.
> The distinction is the central design decision in this fix.

## Problem

Claudine uses a shadow home to control provider configuration and expose
repository-scoped resources. The current implementation sets the provider
process's `HOME` to that shadow root. Since environment variables are inherited,
tools launched by the provider see the same replacement value.

That can make a valid user setup appear broken. A nested tool may resolve a
different config file, miss an OS-keyring account whose lookup is home-sensitive,
or fail to find public signing keys. The resulting error is easily
misdiagnosed as missing or invalid user credentials even though the same tool
works in the user's terminal.

The existing failure fallback is worse: when overlay creation fails, Claudine
sets `HOME=/dev/null` and continues. That launch cannot honor either the
provider-isolation contract or the normal user-context contract.

## Observed Incident

During the CI cleanup review-7 closure work on 2026-09-12, a session wrapped by
Claudine observed `HOME=/Users/ken/.claudine` instead of the user's actual home,
`/Users/ken`.

| Observation | Result |
|---|---|
| `gh auth status` in the agent's default environment | Reported no authenticated account. |
| Selecting only `GH_CONFIG_DIR=/Users/ken/.config/gh` | Reported an invalid token; an API request returned HTTP 401. |
| `gh auth status` in the user's terminal | Reported the existing authenticated keyring account. |
| `env HOME=/Users/ken gh auth status` in the agent session | Found the existing authenticated keyring account. |
| `env HOME=/Users/ken gh api user --jq .login` | Returned `yankeeinlondon`. |
| `git verify-commit` in the agent's default environment | Could not check the signature because no public key was found. |
| The same verification with `GNUPGHOME=/Users/ken/.gnupg` | Verified Ken's existing OpenPGP signature successfully. |

No new GitHub login, token replacement, or signing-key provisioning was needed.
SSH remote inspection also succeeded. Local Git signing, Git transport, GitHub
API authentication, and permission to create a commit are separate concerns;
diagnostics must not collapse them into a single “credentials are broken”
claim.

The OpenPGP evidence establishes a signature-verification lookup difference,
not a reproduced failure to create a signed commit. Likewise, the successful
`HOME` override establishes an environment-dependent GitHub authentication
failure; it does not establish the GitHub CLI's internal keyring lookup path.

## Current Triggers and Contracts

`claudine/cli/src/commands/wrap/repo_home.rs` currently builds a provider tree
under `~/.claudine`, then `build_repo_home_env` assigns the tree's parent to
child `HOME`.

The overlay is activated by more than `--repo`:

- `--repo` requests repository-biased resource isolation for every provider;
- Codex repository prompt discovery requests an overlay even without `--repo`;
- Codex and Gemini MCP runtime injection request an overlay for `--mcp` or
  `--use`; and
- composition, sequence, proxy, retry, and resume can rebuild a launch plan or
  transition to a provider with a different overlay requirement.

Codex already separates SQLite state from the overlay with
`CODEX_SQLITE_HOME`. That completed contract remains in force. This fix removes
the need for the global `HOME` override; it does not permit SQLite files or
their sidecars to be copied or linked into the overlay.

The worktree's location under `~/.claudine/worktrees/` did not assign `HOME`.
Moving worktrees elsewhere cannot fix this issue.

## Terminology

- **User-home identity** is the launch-time OS user context and its ordinary
  home-related environment. On Unix this includes `HOME`; on native Windows it
  includes `USERPROFILE`, `HOMEDRIVE`, and `HOMEPATH`, plus `HOME` when the
  caller supplied it.
- **Provider source root** is the provider config/state location the same
  invocation would use without a Claudine overlay, including any explicit
  provider-specific override supplied by the user.
- **Provider overlay** is Claudine-controlled storage containing the stable
  provider config and resource view for one launch. It may continue to live
  under the Claudine data directory; its location does not make it an OS home.
- **Provider-owned selector** is a documented provider environment variable,
  CLI option, or inline-config mechanism whose effects are limited to that
  provider.

Code and updated documentation should use “provider overlay” for the new
abstraction. Existing on-disk paths do not need migration solely to rename the
concept.

## Required Invariants

1. Claudine never writes a provider-overlay path, `/dev/null`, `NUL`, or an
   equivalent sentinel into a process-wide home variable.
2. The child receives the launch-time user-home identity unchanged, subject
   only to the existing environment-sanitization contract. An inherited home
   variable must not be silently synthesized from another variable.
3. Provider redirection uses only a verified provider-owned selector and the
   path shape that selector expects.
4. The overlay preserves the provider authentication and stable settings that
   the wrapper requires, while repo-only resource classes remain masked to the
   same degree documented for that provider.
5. Claudine never silently converts requested isolation into a less isolated
   launch.
6. Mutable databases, journals, lock files, sockets, and equivalent live state
   are not mirrored with per-file links. Existing provider-native state-root
   exceptions, including Codex SQLite, remain authoritative.
7. Provider transitions restore the invocation baseline before applying the
   target provider's environment patch. No selector owned by the previous
   provider leaks into the next provider attempt.
8. Diagnostics and logs may name variables and paths after normal path
   scrubbing, but never print credential values or secret file contents.

Preserving the real home does not bypass Claudine's sensitive-environment
filter. A missing API-key environment variable and a home-context mismatch are
different diagnostic causes and remain distinguishable.

## Design

### Capture one immutable launch baseline

Extend the existing invocation-level environment snapshot rather than reading
home variables again during a retry or provider transition. The snapshot owns:

- the resolved user home used for `~` and provider-source discovery; and
- the exact presence or absence and raw `OsString` value of each platform home
  variable.

Use the existing focused home-resolution authority already carried by
`InvocationContext`; do not run broad host discovery to obtain this one fact.
All path handling remains non-UTF-8-safe where Rust and the target OS permit it.

The snapshot is captured before sanitization and before any provider selector
is applied. The child-environment builder projects the allowed baseline; an
attempt rebuild never consults the ambient process again.

### Replace the boolean shadow-home decision with a typed overlay plan

Replace the combination of `force_shadow_home`, `needs_shadow_home`, a raw
optional path, and ad hoc provider checks with a plan that records at least:

- why an overlay is required (`repo_resources`, `repo_prompt`, or `mcp`);
- the selected provider;
- the pre-overlay provider source root;
- the overlay storage root and the provider-visible root expected by its
  selector;
- the provider-owned environment/argument patch;
- resource classes to exclude or materialize;
- state that must remain at its pre-overlay location; and
- whether the requested set of reasons is supported.

The provider wrapper profile supplies this strategy. Do not introduce another
central `match Provider` dispatch site; the existing dispatch-inventory guard
must remain clean.

An inline-config mechanism can satisfy a reason without a filesystem overlay.
For example, OpenCode's existing `OPENCODE_CONFIG_CONTENT` MCP injection should
not acquire a filesystem or home override merely for uniformity.

### Verify provider selectors instead of inferring them

Implementation must audit the current provider research and executable
behavior for every compiled provider. Known candidate selectors include
`CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `GEMINI_CLI_HOME`, `GOOSE_PATH_ROOT`,
`KIMI_CODE_HOME`, `OPENCODE_CONFIG_DIR`/`OPENCODE_CONFIG_CONTENT`, `QWEN_HOME`,
`PI_CODING_AGENT_DIR`, and Kilo's config overlays.

These names are not permission to treat them as equivalent:

- some selectors name the provider directory itself while others name a parent
  beneath which the provider creates its normal directory;
- some relocate config, authentication, sessions, and cache together while
  others affect only one class;
- OpenCode and Kilo config directories are additive in some discovery paths,
  so setting them alone does not prove that user resources are masked; and
- Antigravity currently has no verified safe replacement selector, and its
  provider facts explicitly warn that changing `HOME` breaks keyring auth.

For each provider and activation reason, add a test-backed capability verdict:
`native_root`, `composable_injection`, or `unsupported`. A provider is supported
only when the mechanism preserves the reason's existing behavior. In
particular, an additive config directory cannot claim repo-only isolation until
the remaining user discovery paths are explicitly disabled or filtered.

If current research is insufficient, update the authoritative provider facts
and regenerate provider metadata through `claudine-gen`; do not hand-edit a
generated `data.rs` file. Provider-specific path construction and side effects
remain in the hand-written wrapper profile.

### Preserve user intent and provider state

Resolve the provider source root before applying Claudine's selector. An
explicit user-provided provider root is the source of truth from which the
overlay is built; it must not be mistaken for the destination or replaced with
a hard-coded default.

Overlay materialization must classify entries rather than recursively mirroring
everything:

- stable settings and credential references needed by the provider are
  preserved under the existing repo-isolation policy;
- resource classes selected by `--repo` are omitted and repo-scoped
  replacements are materialized where needed;
- live state uses a provider-native external state selector when one exists;
  and
- mutable secret or state files are never hard-linked merely to make native
  Windows match Unix symlink behavior.

On Unix, symbolic links may remain appropriate for stable entries. Native
Windows must use an explicit provider-safe copy or supported link strategy and
must handle nested directories recursively; directory hard links do not exist.
Atomic replacement and cleanup must follow existing Claudine config-write
contracts.

Codex keeps its pre-overlay `CODEX_SQLITE_HOME` resolution and volatile-family
exclusions. Existing regular legacy databases under the old overlay remain
recoverable and are not deleted by this fix.

### Preserve the user home for descendants

The provider process receives the provider-owned selector and the unmodified
user-home identity. Ordinary nested tools therefore resolve the same home they
would resolve when launched directly by the user.

A provider-specific selector is intentionally inherited by descendants. This
means a same-provider CLI manually launched as a tool participates in the same
overlay. That is narrower and more predictable than changing every descendant's
OS home. Claudine does not attempt shell-specific environment rewriting around
arbitrary tool invocations.

Claudine-managed provider transitions are different: each attempt starts from
the immutable baseline, removes or restores all selectors owned by the previous
provider, then applies the target provider's plan. Retry and resume compatibility
keys include the complete overlay plan so the recorded key cannot describe a
different config/resource view than the child receives.

### Fail closed before spawn

Delete the `HOME=/dev/null` fallback.

Overlay planning or materialization failure returns a typed diagnostic before
the provider is spawned. The diagnostic identifies the provider, activation
reason, selector/capability that failed, and a safe next action. It must not
claim that the user's credentials are invalid.

When a provider has no safe mechanism for a requested reason, refuse only that
mode. A normal wrapper launch that needs no overlay remains available. Examples:

- if Antigravity still lacks a replacement selector, `claudine antigravity
  --repo` is unsupported rather than implemented by replacing `HOME`;
- an MCP provider with a complete inline injection path can still use MCP
  without repo-resource isolation; and
- an additive config selector can support a targeted overlay only when that
  feature does not promise to hide the provider's other user-scoped resources.

This is an intentional tightening of behavior. A visible pre-spawn refusal is
preferable to a successful-looking launch that either loses user identity or
violates the requested isolation contract.

## Scope

Implementation includes:

- direct wrapper launches;
- `compose`, `inline-compose`, and `sequence` launches;
- repo-only resource isolation;
- Codex repository prompt overlays;
- Codex and Gemini filesystem-backed MCP injection plus OpenCode inline MCP
  injection;
- lifecycle proxy, retry, and resume launch-plan rebuilds;
- provider-transition environment restoration;
- typed diagnostics and non-secret debug/performance reporting; and
- documentation that currently describes `HOME` replacement, including the
  Claudine skill snapshots.

Implementation does not include:

- changing the sensitive-environment allowlist;
- guaranteeing that credentials are present or authorized;
- starting login, key import, or credential-repair flows;
- turning `--repo` into a filesystem sandbox;
- broadening any provider's documented resource-isolation classes;
- deleting legacy overlay state; or
- relocating existing worktrees.

## Testing

### L1 contract tests

Use hermetic child fixtures and platform-neutral path construction to prove:

1. every overlay activation reason leaves the launch baseline's home variables
   unchanged;
2. no launch path inserts `/dev/null`, `NUL`, or an overlay path into a global
   home variable;
3. explicit provider-root overrides are captured as source roots and the
   Claudine selector points at the correct provider-visible overlay shape;
4. repo resources remain isolated to the provider's previously documented
   level;
5. Codex prompt and MCP overlays still work and SQLite remains outside them;
6. Gemini MCP injection writes the location selected by `GEMINI_CLI_HOME`
   rather than relying on `HOME`;
7. OpenCode inline MCP injection does not create an unnecessary overlay;
8. unsupported provider/reason combinations fail before the fake provider
   records a spawn;
9. materialization failure produces the typed diagnostic and never falls back
   to a null home;
10. a simulated nested `git`, `gpg`, and `gh` process observes the original
    home variables without invoking real credential stores; and
11. proxy/retry/resume provider transitions remove the prior selector, restore
    an explicit ambient value when one existed, and apply only the target
    provider's plan.

Tests must cover absent variables, non-UTF-8 Unix values where supported, paths
with spaces, and native Windows path forms. Environment-mutating unit tests use
the existing serialized environment guards.

### L2 and cross-platform evidence

Add non-interactive L2 coverage through the real `claudine` binary with fake
providers. The fixture must use a private home and config tree, must not open or
focus a terminal window, and must not read the developer's real credentials.

Required evidence covers macOS, Linux, native Windows, and WSL2. At minimum,
each environment proves the home-preservation contract and one filesystem-backed
overlay. Native Windows additionally proves recursive overlay materialization
without directory hard links. Provider-specific real-CLI tests are appropriate
only where the repository already has an opt-in real-test tier; they must not be
added to ordinary L1.

## Documentation and Migration

Update `claudine/docs/topics/repo-isolation.md`, MCP-mode documentation, CLI
reference text, architecture documentation, and the mirrored Claudine skill
documents in the same change. Documentation must distinguish provider overlay,
user-home identity, provider authentication preservation, and environment
credential admission.

Keep the existing on-disk `~/.claudine/<provider>` data in place when its shape
is compatible with the provider-owned selector. If a selector expects a
different shape, use a versioned or provider-specific overlay path and leave
the old data untouched. No automatic destructive migration is authorized.

## Acceptance Criteria

- [ ] No production wrapper or composition path replaces a global user-home
      variable to implement provider isolation or MCP injection.
- [ ] The `HOME=/dev/null` fallback is removed and overlay failures are typed,
      pre-spawn failures.
- [ ] Every compiled provider has a test-backed capability verdict for each
      overlay activation reason it can receive.
- [ ] Supported providers use only verified provider-owned selectors with the
      correct path shape; unsupported combinations are refused explicitly.
- [ ] Existing repo-resource masking, Codex prompt overlay, and runtime MCP
      behavior remain intact for combinations marked supported.
- [ ] Codex SQLite state continues to obey the completed separate-state contract.
- [ ] Direct, composition, sequence, retry, resume, and proxy launches share one
      overlay-planning path and restore provider-owned environment correctly.
- [ ] Nested ordinary tools observe the original user-home environment.
- [ ] macOS, Linux, native Windows, and WSL2 evidence satisfies the platform
      matrix, including native Windows directory materialization.
- [ ] User-facing diagnostics never infer broken credentials from an isolated
      provider/config lookup and never expose secret values.
- [ ] Repo-isolation, MCP, architecture, CLI, and Claudine skill documentation
      describe the new provider-overlay contract.
- [ ] `just test`, `just test-l2`, and `just lint` pass in the `claudine` package
      area, with real-provider tests remaining opt-in.

## Open Questions

None. The implementation audit may classify a provider/reason combination as
unsupported, but it may not reintroduce global home replacement or silently
weaken isolation to avoid that verdict.
