---
created: 2026-09-12
status: draft
reviewed: false
implemented: false
area: claudine
packages:
    - claudine
    - claudine-cli
---

# Preserve Shadow-Home Functionality Without Confusing User Identity and Credentials

## Problem

Claudine uses a shadow home to control provider configuration and expose
repository-scoped resources. That functionality remains valuable and must be
preserved in some form. However, changing the provider process's `HOME` also
changes the environment inherited by tools the agent invokes. Those tools may
then fail to discover the user's existing configuration or credentials.

The resulting failures look like missing signing keys, an unauthenticated
GitHub account, or an invalid token. An agent can mistake this environment
mismatch for a problem the user must fix, repeatedly interrupting authorized
work with unnecessary authentication instructions. The user sees working
credentials in their terminal while the agent insists they are unavailable.

This draft records the problem and the behavior a future design must preserve.
It does not select a replacement mechanism or authorize implementation.

## Observed Incident

During the CI cleanup review-7 closure work on 2026-09-12, a session wrapped
by Claudine observed `HOME=/Users/ken/.claudine` instead of the user's actual
home, `/Users/ken`.

| Observation | Result |
|---|---|
| `gh auth status` in the agent's default environment | Reported no authenticated account. |
| Selecting only `GH_CONFIG_DIR=/Users/ken/.config/gh` | Reported an invalid token; an API request returned HTTP 401. |
| `gh auth status` in the user's terminal | Reported the existing authenticated keyring account. |
| `env HOME=/Users/ken gh auth status` in the agent session | Found the existing authenticated keyring account. |
| `env HOME=/Users/ken gh api user --jq .login` | Returned `yankeeinlondon`. |
| `git verify-commit` in the agent's default environment | Could not check the signature because no public key was found. |
| The same verification with `GNUPGHOME=/Users/ken/.gnupg` | Verified Ken's existing OpenPGP signature successfully. |

No new GitHub login, token replacement, or signing-key provisioning was needed
to resolve these observations. SSH remote inspection also succeeded. Local
Git signing, Git transport, GitHub API authentication, and permission to create
a commit are separate concerns; the agent's explanations repeatedly blurred
those distinctions.

The OpenPGP evidence here is signature verification, not a reproduced failure
to create a signed commit. Likewise, the successful HOME override establishes
an environment-dependent GitHub authentication failure; it does not establish
the exact internal keyring lookup that produced the earlier invalid-token
diagnosis.

## Relevant Implementation

- `claudine/cli/src/commands/wrap/repo_home.rs`:
  `build_repo_home_env` explicitly sets child `HOME` to the shadow-home root,
  normally `~/.claudine`. This matches the path observed in the incident.
- The same module's `needs_shadow_home` enables shadow-home behavior for
  repository-only mode and for Codex repository prompt discovery.
- `claudine/cli/src/commands/wrap/mod.rs` also requests a shadow home for
  Codex or Gemini when MCP mode is enabled through `--mcp` or `--use`.
- Codex already receives special handling for its pre-shadow SQLite location.
  This illustrates that provider state and the replacement home do not always
  belong at the same location.

The exact trigger that enabled shadow-home behavior in this particular launch
has not been established. The source confirms a mechanism consistent with the
observed environment; a complete launch trace remains follow-up work.

The worktree happened to be under `~/.claudine/worktrees/`, but its location
does not itself assign `HOME`. Moving worktrees to
`/Volumes/coding/wt/{repo}/{worktree}` will not by itself change this
shadow-home assignment.

## Desired Outcomes and Constraints

- Retain the ability to isolate or overlay provider configuration and expose
  the intended repository resources without modifying the user's ordinary
  provider configuration.
- Preserve the expected user context for ordinary development tools, including
  Git, OpenPGP, and the GitHub CLI, wherever that access is intended.
- Make intentional isolation and its consequences understandable to both the
  agent and the user. A shadow-home lookup failure must not be presented as
  proof that the user's credentials are absent or invalid.
- Avoid requiring users to authenticate again, duplicate credentials, or
  explain the same environment mismatch each session.
- Account for nested tool processes and provider transitions, rather than
  considering only the initial provider launch.
- Define behavior for macOS, Linux, native Windows, and WSL2. This incident
  provides macOS evidence only; equivalent failures elsewhere remain untested.

## Open Design Questions

1. Which resources actually require a shadow home, and which should remain
   associated with the real user home?
2. How should provider configuration isolation coexist with normal credential
   and configuration discovery in child tools?
3. How can an agent distinguish intentional isolation from a broken credential
   context before asking the user to repair authentication?
4. What evidence should establish that provider isolation still works while
   existing development-tool authentication remains available as intended?

No mechanism is chosen here. The successful environment overrides above are
diagnostic evidence, not the proposed permanent implementation. Source changes,
credential changes, and migration decisions belong to a later design pass.
