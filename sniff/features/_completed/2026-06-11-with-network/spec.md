# Specification: Remove the Unused `--with-network` Flag

## Problem Statement

`sniff` currently exposes `--with-network` as a global option, so it appears in every subcommand's help. The help text says it includes network-dependent supplemental data, but the parsed field is never read by the CLI:

- `sniff/cli/src/args/mod.rs` defines `Cli::with_network`.
- `sniff/cli/src/commands/mod.rs` never branches on `cli.with_network`.
- The only current tests assert that the flag parses before `repo name`; they do not assert an output difference.

This creates a misleading CLI contract: users can pass `--with-network` to commands where it cannot affect behavior, and even on commands with network-capable variants, the current behavior is driven by other command-local flags.

## Current Network-Affecting CLI Surface

This audit uses the current source as authoritative.

| Command surface | Current network behavior | Output variant today | Existing flag |
| --- | --- | --- | --- |
| `sniff network` | Runs `NetworkRequest::full()` through the `OutputFilter::Network` plan, including WAN IP lookup when available. | No local-only vs network-enriched variant; network data is always attempted. | None |
| `sniff` with `--json` and no subcommand | Runs the default full `DetectionPlan`, including network detection. | No variant controlled by `--with-network`. | None |
| `sniff filesystem` | Can refresh git remote-tracking refs and can enrich dependency versions. | Yes. Remote sync fields and dependency update fields are present only when requested. | `--refresh-remotes`, `--latest-versions` |
| `sniff repo git-status` | Can fetch configured git remotes before reporting branch sync and remote containment. | Yes. Remote branch/sync/containment details differ when requested. | `--refresh-remotes` |
| `sniff repo structure` | Can query package registries for latest dependency versions. | Yes. Dependency update summaries and JSON fields differ when requested. | `--latest-versions` |
| `sniff repo remote [REMOTE]` | Calls remote hosting provider APIs. | Not a supplemental variant of local host data; the command's primary purpose is network remote inspection. | None |
| `sniff repo pr` | Calls remote hosting provider APIs. | Not a supplemental variant of local host data; the command's primary purpose is network PR inspection. | None |
| Program install flows | May use remote-bash consent gates as part of installation planning. | Not a reporting enrichment variant for `--with-network`. | Existing install flags/consent |
| OS, hardware, CPU, GPU, memory, storage, audio, services, files, docs, just, blast-radius, most repo leaf commands, program listing commands | Local/system detection only. | No network variant. | None |

## Target Behavior

`--with-network` must be removed from the CLI. Existing command-local flags already describe the network operations they perform more clearly.

### 1. Remove the global flag

Remove `Cli::with_network` from the top-level `Cli` struct and from global help.

After this change, commands that do not explicitly define `--with-network` must reject it with clap's normal usage error. Examples:

```sh
sniff --with-network repo name
sniff repo name --with-network
sniff cpu --with-network
```

### 2. Keep existing specific flags for repository enrichments

Do not replace these flags with `--with-network`:

- `sniff repo git-status --refresh-remotes`
- `sniff filesystem --refresh-remotes`
- `sniff repo structure --latest-versions`
- `sniff filesystem --latest-versions`

These names are more precise than `--with-network`: they tell the user what network operation will occur and what class of output can change. Keeping them also avoids collapsing two different operations, git fetches and registry lookups, into one vague switch.

### 3. Fix documentation drift around `repo --latest-versions`

The README currently documents `sniff repo --latest-versions`, but the clap shape accepts `--latest-versions` on `repo structure`, not on the `repo` parent. Verified behavior:

```sh
sniff repo --latest-versions --help
# error: unexpected argument '--latest-versions' found
```

Documentation should say `sniff repo structure --latest-versions` unless implementation intentionally adds a parent-level aggregate flag in a separate feature.

## Implementation Notes

- Remove `Cli::with_network`.
- Remove tests that assert global parsing.
- Update help snapshots.
- Update docs that mention `--with-network`.
- Leave `sniff network` behavior unchanged in this cleanup. Changing WAN IP lookup policy is a separate behavior change.

## Tests

Update or add CLI tests for:

- `sniff --help` no longer contains `--with-network`.
- `sniff cpu --help` and `sniff repo name --help` do not contain `--with-network`.
- `sniff --with-network repo name` fails with a clap usage error.
- `sniff repo name --with-network` fails with a clap usage error.
- Existing `--refresh-remotes` and `--latest-versions` scoped-help tests continue to pass.

Remove the current tests that only assert global parsing:

- `with_network_flag_parses`
- `with_network_flag_parses_before_json`

## Documentation

Update:

- `sniff/cli/README.md`: ensure scoped enrichment examples use the existing command-local flags.
- `sniff/README.md`: use `sniff repo structure --latest-versions` for registry enrichment.
- `sniff/docs/topics/json-output.md`: revise the Golden Exception so it does not imply a global switch. State that network supplemental data is only included when the relevant command-local opt-in is present.
- `sniff/docs/topics/terminal-output.md`: update the TODO wording to "announce command-local network opt-ins when a variant is available."
- `.claude/skills/sniff/SKILL.md`: update CLI examples and the network request description after implementation.

## Non-Goals

- Do not rename `--refresh-remotes` or `--latest-versions`.
- Do not add `--with-network` to any command as part of this work.
- Do not change remote provider command semantics.
- Do not add new network detection capabilities.
- Do not change library request types.
