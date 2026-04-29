# CLI Argv Pre-Processing

## Summary

Claudine's CLI is parsed by `clap` via `derive`, and the surface has grown to the point where clap's ordinary parsing model produces rough edges: eight boolean provider flags in a mutual-exclusion group, positional "file plus key=value setters" collected with `num_args = 1..`, and fuzzy provider matching that today only applies to the `--provider` value (not to the `claude` / `codex` subcommands or the `--claude` / `--codex` flags). This feature adds a small, clearly scoped **argv normalization layer** that runs *before* clap sees the argv, rewrites a curated set of shorthand patterns into the canonical forms clap already understands, and otherwise passes argv through untouched. clap remains the authoritative parser; the normalizer only reshapes input on the way in.

The goals are:

1. Eliminate the class of parsing failures caused by clap's greedy multi-value positional interacting with boolean flags (the `--help` absorbed-as-positional bug).
2. Collapse the eight provider booleans and the `--provider` flag into a single canonical representation so downstream code stops re-resolving them.
3. Let users continue typing short forms (`--claude`, `cl` for `claude`, `compose file.md key=val --gemini`) without the code paying that cost eight times over.
4. Keep every clap-native feature (`--help`, `--version`, completions, `ValueEnum`, derive, subcommand dispatch, error rendering) fully intact.

## Motivation

The trigger is this command producing a confusing error:

```
$ claudine compose @prompts/greet.md --gemini name="Ken" --help
error: unexpected argument '--help' found
  tip: to pass '--help' as a value, use '-- --help'
Usage: claudine compose --gemini <ARG>...
```

Root cause: `ComposeArgs.args: Vec<String>` with `num_args = 1..` and a boolean `--gemini` flag interleaved with positional tokens. clap enters "positional collection" mode after the first positional, suspends it for `--gemini`, resumes it for `name="Ken"`, and then refuses `--help` as an "unexpected positional" instead of treating it as the built-in help flag. The tip is actively misleading — the user did not want `--help` as a value.

Related pressure points:

- **Eight provider booleans** (`--claude`, `--codex`, `--gemini`, `--goose`, `--kimi`, `--opencode`, `--qwen`, `--roo`) appear on every composition surface (`SharedComposeArgs` in `claudine/cli/src/commands/compose.rs:25`). Each is wired into a `compose_provider` clap group, each adds a help-text line, and each must be re-resolved downstream by `explicit_provider()` (`claudine/cli/src/commands/compose.rs:150`). The `--provider` flag duplicates the same resolution.
- **Fuzzy provider matching** (`Provider::fuzzy_match_cli_name`, `claudine/lib/src/events/provider.rs:315`) already exists and works, but only for the `--provider` *value*. Typos like `--cdex` or subcommand shorthand like `claudine cl compose …` are not offered the same matching surface.
- **Positional `key=value` setters** are ergonomically valuable (`claudine compose file.md key=val`) but actively fight clap's greedy positional collection when flags appear after the first positional.
- **Future parsing needs** (e.g. file-completion integration, `--` passthrough to wrapped providers, tag-stripping for MCP tags) will keep adding pre-parse work. A single normalization layer is the cheapest place to host that work consistently.

Writing a full custom parser would cost weeks of rework and give up clap's help rendering, shell completions (see `2026-04-17-file-completion`), `ValueEnum` validation, derive ergonomics, error formatting, `--version`, and man-page generation. A thin normalization pass preserves all of that while removing the observable pain.

## Scope

### In scope for v1

- A new module at `claudine/cli/src/argv.rs` that exposes a single public function: `fn normalize(raw: Vec<OsString>) -> Vec<OsString>`.
- Wiring in `claudine/cli/src/main.rs` to call `normalize` on `std::env::args_os().collect()` before `Cli::parse_from`.
- The three rewrite rules listed in "Normalization rules" below.
- A `#[cfg(test)] mod tests` block in `argv.rs` with dense coverage of each rule plus pass-through cases.
- Documentation updates: a new topic file at `claudine/docs/topics/argv-normalization.md` that enumerates the rules and pass-through guarantees.

### Out of scope for v1 (tracked follow-ups)

- Removing the eight boolean provider flags from `SharedComposeArgs` / wrapper commands. The normalizer lets us keep them as user-facing sugar today; a future cleanup can decide whether to retain them as documented aliases or retire them entirely.
- Rewriting subcommand fuzzy input (e.g. `claudine cl compose …` → `claudine claude compose …`). Subcommand-level rewriting has different semantics from flag-level rewriting and deserves its own scoping pass.
- Tag-stripping or MCP-tag pre-processing. These are handled later in the pipeline today and should not migrate into the normalizer unless we can do so without changing runtime behavior.
- Shell-completion interaction. The normalizer runs before clap; it must not run when `clap_complete::CompleteEnv::complete()` is being driven (dynamic completion path). Detection of that mode is in scope *as a guard*; rewriting completion argv is not.
- Suggesting corrections for unknown tokens (e.g. "did you mean `--claude`?"). clap already does this for recognized args; adding it to the normalizer is a separate concern.
- `--` passthrough rewrites (e.g. collecting everything after `--` into a single trailing arg for wrappers). Existing wrapper code handles this; the normalizer must not touch tokens at or after the first `--`.

## Normalization rules

Each rule is **syntactic and local**. The normalizer never consults clap, never reads the filesystem, and never inspects state beyond the argv `Vec<OsString>` itself. Ordering: rules are applied in the sequence listed below, once, in a single left-to-right pass.

### Rule 1: Provider boolean → `--provider <name>`

**Input patterns:** `--claude`, `--codex`, `--gemini`, `--goose`, `--kimi`, `--opencode`, `--qwen`, `--roo`.

**Rewrite:** replace the single token with two tokens: `--provider <canonical-slug>`, where `<canonical-slug>` is `Provider::as_slug()` for the matching provider.

**Canonical slugs** (authoritative list lives in `claudine/lib/src/events/provider.rs`):

| Boolean flag | Canonical slug |
|---|---|
| `--claude` | `claude` |
| `--codex` | `codex` |
| `--gemini` | `gemini` |
| `--goose` | `goose` |
| `--kimi` | `kimi_code` |
| `--opencode` | `open_code` |
| `--qwen` | `qwen_code` |
| `--roo` | `roo_code` |

**Multiple-occurrence handling:** if two or more provider booleans appear, each is rewritten individually. clap will then reject the duplicate via its existing `--provider` handling (the `compose_provider` group already enforces mutual exclusion for the booleans themselves; that group disappears when the booleans are removed in a follow-up). The normalizer does not attempt to merge or deduplicate.

**Interaction with `--provider`:** if both `--provider <x>` and a provider boolean are present, the normalizer still rewrites the boolean. clap then sees two `--provider` occurrences and errors. That's the correct outcome; the user was ambiguous.

**Non-goals:** the normalizer does not fuzzy-match these. `--claud` is not rewritten. clap will surface it as an unknown argument, which is the desired behavior.

### Rule 2: Fuzzy `--provider <value>` canonicalization

**Input pattern:** `--provider <VALUE>` or `--provider=<VALUE>` where `<VALUE>` is any non-empty string.

**Rewrite:** if `Provider::fuzzy_match_cli_name(&value)` returns `Some(p)`, replace `<VALUE>` with `p.as_slug()`. If it returns `None`, leave the token unchanged so clap can produce its native "invalid value" error listing the valid variants.

**Why this belongs in the normalizer:** the `provider_value_parser` (`claudine/cli/src/provider_values.rs:7`) currently accepts alias equality via `PossibleValue::alias`, but that is exact-match only. Fuzzy matching (`cl` → `claude`, `gem` → `gemini`) is not wired in. Pre-normalizing the value is strictly a syntactic rewrite — clap still enforces the canonical set downstream.

**Edge cases:**

- `--provider` with no following token: leave untouched. clap produces "a value is required".
- `--provider=`: leave untouched (empty value). clap produces its native error.
- `--provider -x`: the next token starts with `-`; do not treat it as a value. Leave untouched.

### Rule 3: `--` separator insertion before trailing setters

**Problem:** `claudine compose file.md --gemini key=val --help` (which this feature exists to fix).

**Rewrite:** on `compose`, `inline-compose`, and `sequence` subcommands, detect the pattern "positional token followed by one or more flag-and-value pairs followed by another positional-looking token that is a setter" and insert a `--` separator before the first post-flag positional token. Concretely, the normalizer:

1. Locates the subcommand token in argv (first token after `claudine` that matches `compose`, `inline-compose`, or `sequence`).
2. After the subcommand token, tracks whether it has already seen a positional.
3. When it encounters a token that (a) looks like a shorthand setter (matches `^[A-Za-z_][A-Za-z0-9_-]*=` — the same regex governed by `parse_compose_setter` in `claudine/cli/src/commands/compose.rs:458`) and (b) follows at least one flag-or-flag-value token *after* a previous positional, insert `--` immediately before that token.
4. Never inserts more than one `--` in a given argv.
5. Never inserts `--` if one is already present in the argv.

**Effect:** clap stops greedy-positional collection at the `--`, treats the remainder as trailing raw values for `args`, and `--help` earlier in the argv is still recognized as the help flag. Downstream `parse_composition_positionals` (`claudine/cli/src/commands/compose.rs:498`) is already capable of handling setters and files in any order, so the user-visible behavior is unchanged except that errors stop happening.

**Why not just require `--` today:** existing documented usage (`claudine compose file.md key=val`, with no flags between them) must continue to work. Rule 3 only fires when flags interleave positionals, so the common case is untouched.

**Alternative considered:** making the `args` positional `trailing_var_arg = true` + `allow_hyphen_values = true`. Rejected because it disables `--help` entirely for the subcommand (the problem we are trying to solve) and because it forces users to put `--` in manually for every call that mixes flags with setters.

## Pass-through guarantees

The normalizer **must not** modify argv in any of the following cases:

1. **Completion mode.** If `clap_complete::CompleteEnv` is active (detected via the `COMPLETE` environment variable being set), the normalizer returns argv unchanged. Dynamic completion depends on seeing the raw argv.
2. **Unknown subcommand.** If the subcommand token does not match any known subcommand, Rules 1 and 3 still apply (they are subcommand-agnostic or subcommand-filtered, respectively); Rule 2 still applies (it is flag-driven). Unknown-subcommand handling stays with clap.
3. **Tokens at or after `--`.** The normalizer scans until the first `--` and stops applying rules to anything after it.
4. **Non-UTF-8 tokens.** `OsString` values that are not valid UTF-8 are left untouched. Rules are pattern-based and require UTF-8 decoding; non-UTF-8 tokens are by definition not matching any pattern.
5. **Empty argv or single-element argv (`["claudine"]`).** Return unchanged; there is nothing to rewrite.

## Pipeline placement

```text
                ┌────────────────────────────┐
std::env::args_os │  normalize(Vec<OsString>)  │  Vec<OsString>
  ───────────────▶│  - Rule 1: booleans        │───────────────▶ Cli::parse_from
                 │  - Rule 2: fuzzy value     │
                 │  - Rule 3: `--` insertion  │
                 └────────────────────────────┘
```

- `main.rs` calls `let argv = argv::normalize(std::env::args_os().collect()); let cli = Cli::parse_from(argv);`.
- No other call site should invoke `normalize`. Library code (`claudine` crate) never sees argv and therefore never needs to normalize.
- Tests can call `normalize` directly with constructed `Vec<OsString>` inputs; tests do **not** need to spawn processes.

## Testing

### Unit tests (in `argv.rs`)

Coverage must include every rule plus representative pass-through cases. Recommended cases:

**Rule 1 (booleans):**

- `["claudine", "compose", "file.md", "--claude"]` → `["claudine", "compose", "file.md", "--provider", "claude"]`.
- `["claudine", "compose", "--gemini", "file.md"]` → `["claudine", "compose", "--provider", "gemini", "file.md"]`.
- `["claudine", "compose", "--kimi"]` → `["claudine", "compose", "--provider", "kimi_code"]` (canonical slug preserved).
- `["claudine", "compose", "--claude", "--gemini"]` → `["claudine", "compose", "--provider", "claude", "--provider", "gemini"]` (both rewritten; clap rejects later).

**Rule 2 (fuzzy value):**

- `["claudine", "compose", "--provider", "cl"]` → `["claudine", "compose", "--provider", "claude"]`.
- `["claudine", "compose", "--provider", "gem"]` → `["claudine", "compose", "--provider", "gemini"]`.
- `["claudine", "compose", "--provider=oc"]` → `["claudine", "compose", "--provider=open_code"]` (`=` form preserved).
- `["claudine", "compose", "--provider", "nonesuch"]` → unchanged (clap will reject).
- `["claudine", "compose", "--provider"]` → unchanged (no value).

**Rule 3 (`--` insertion):**

- `["claudine", "compose", "file.md", "--gemini", "name=Ken", "--help"]` → `["claudine", "compose", "file.md", "--provider", "gemini", "--", "name=Ken", "--help"]`.
- `["claudine", "compose", "file.md", "key=val"]` → unchanged (no flag interleaves; common case).
- `["claudine", "compose", "key=val", "file.md"]` → unchanged (still no flag interleaves).
- `["claudine", "compose", "file.md", "--gemini", "other.md"]` → unchanged by Rule 3 (the trailing token is not a setter); clap will error on two files, which is the correct pre-existing behavior.
- `["claudine", "compose", "file.md", "--gemini", "--", "name=Ken"]` → unchanged by Rule 3 (`--` already present).
- `["claudine", "sequence", "file.md", "--gemini", "k=v", "--help"]` → `--` inserted (sequence included).
- `["claudine", "inline-compose", "file.md", "--gemini", "k=v", "--help"]` → `--` inserted.
- `["claudine", "hooks", "file.md", "--gemini", "k=v", "--help"]` → unchanged (Rule 3 only fires on composition subcommands).

**Pass-through:**

- `["claudine"]` → unchanged.
- `["claudine", "--version"]` → unchanged.
- `["claudine", "hooks"]` → unchanged.
- `["claudine", "compose", "--help"]` → unchanged (no positional yet; Rule 3 doesn't trigger).
- Completion mode (`COMPLETE=zsh` in env): argv unchanged regardless of content. Use `std::env::set_var` + `remove_var` or inject the detection as a parameter for testability.

**Non-UTF-8:**

- Construct an `OsString` from invalid UTF-8 bytes (e.g. `OsString::from_vec(vec![0xff])` on Unix) and assert the normalizer returns it untouched.

### Integration tests (`claudine/cli/tests/`)

A small `assert_cmd` test suite that runs the binary end-to-end for the three headline cases:

1. `claudine compose <fixture> --gemini name=Ken --help` exits successfully with help text (the bug that motivated the feature).
2. `claudine compose <fixture> --provider cl --dry-run` resolves to the `claude` provider (the fuzzy-match case).
3. `claudine compose <fixture> key=val` (no flags) behaves identically to today (regression guard).

Fixture: a minimal `.md` with frontmatter that composition can accept. Reuse an existing fixture if one exists in `claudine/cli/tests/`.

### Snapshot tests

None required. The normalizer output is trivially comparable as `Vec<OsString>`; prefer direct equality assertions over snapshots.

## Acceptance criteria

- `claudine compose @file.md --gemini name=Ken --help` exits with the compose help text, not a clap "unexpected argument" error.
- `claudine compose --provider cl file.md` resolves to the `claude` provider end-to-end (normalizer rewrites `cl` → `claude`; clap accepts; composition uses Claude).
- `claudine compose --claude file.md` and `claudine compose --provider claude file.md` produce identical composition runs (structural equality of the resolved `CompositionExecutionRequest` up to `explicit_provider`).
- `claudine compose file.md key=val` (no flags between tokens) continues to work exactly as today; argv is unchanged.
- `claudine --version`, `claudine --help`, `claudine hooks --describe`, and every other non-composition command produce identical argv to today (regression guard via pass-through tests).
- Shell completion via `COMPLETE=zsh claudine` produces the same candidate set as before this feature lands. The normalizer is a no-op under `COMPLETE`.
- Running `claudine compose --kimi file.md` internally results in clap seeing `--provider kimi_code`, and the resolved provider is `Provider::KimiCode`.
- Unit test module in `argv.rs` covers every case in "Testing → Unit tests" above and compiles under `cargo test -p claudine-cli`.

## Open questions and tracked follow-ups

- **Retiring the provider booleans.** Once the normalizer ships, the boolean flags in `SharedComposeArgs` and wrapper commands become pure syntactic sugar with no code benefit. A follow-up can evaluate whether to retain them as documented aliases or remove them entirely in favor of `--provider`. Removing them shrinks the `compose_provider` clap group and simplifies `explicit_provider()`.
- **Subcommand-level fuzzy matching.** `claudine cl compose …` currently errors ("unrecognized subcommand"). Adding subcommand-level fuzzy matching is attractive but requires deciding how to handle ambiguity (e.g. `co` matching both `codex` and `compose`). Deferred to its own feature.
- **Rule 3 scope expansion.** Should Rule 3 apply to non-composition subcommands that also have multi-positional args? The current trio (`compose`, `inline-compose`, `sequence`) is the only place the bug has been reproduced. Expand if another subcommand exhibits the same shape.
- **Alternative to Rule 3: `trailing_var_arg`.** If clap's `trailing_var_arg` semantics evolve to preserve `--help` recognition, Rule 3 could be replaced by a pure clap-side fix. Track the upstream issue.
- **Error messaging when a rewritten token fails.** If Rule 1 rewrites `--kimi` → `--provider kimi_code` and clap later errors on the value, the error points at `kimi_code`, which the user did not type. Consider whether the normalizer should emit a debug-mode trace (`tracing::debug!`) of every rewrite so that users running with `--debug` can see what happened.
- **Mutation signal in help output.** Should `--help` output for composition surfaces advertise the rewrite rules (e.g. note that `--claude` is equivalent to `--provider claude`)? Low priority; the help already lists the booleans.
- **Normalization of wrapper subcommand args.** The direct wrappers (`claudine claude`, `claudine codex`, etc.) forward most flags to the child CLI. The normalizer currently only touches Claudine's own flags. Verify that no rewrite rule can accidentally corrupt tokens intended for the wrapped child process — in particular, ensure Rule 3 never fires for wrapper subcommands (it is gated on composition subcommands only, so this should hold by construction, but add a regression test).

## Dependencies and risks

- **No new crate dependencies.** The normalizer is pure standard library plus existing `claudine::events::Provider` for fuzzy matching.
- **Coupling to `Provider::as_slug` and `Provider::fuzzy_match_cli_name`.** If either signature changes, the normalizer must be updated in lockstep. Both APIs are already load-bearing for the `--provider` value parser, so coupling cost is near-zero.
- **Subtle interaction with `disable_help_flag`.** The root `Cli` struct sets `disable_help_flag = true` (`claudine/cli/src/args.rs:31`) and defines its own `--help` boolean. Rule 3 intentionally preserves `--help` as a standalone token so Claudine's custom help handler still fires. Verify that the existing help handler works under the rewritten argv — the help flag comes *before* the inserted `--`, which means it is always on the non-trailing side and clap recognizes it normally.
- **Completion-mode detection fragility.** Relying on the `COMPLETE` env var is the contract `clap_complete` documents, but future versions may introduce additional environment signals. Re-verify on each `clap_complete` upgrade. Track this alongside the `unstable-dynamic` guidance in `2026-04-17-file-completion`.
- **Test surface growth.** Every rule adds a small matrix of cases. The risk is that a future rule is added without the corresponding pass-through tests, so the normalizer silently starts rewriting inputs it should leave alone. Mitigation: document the "pass-through tests required" contract in `argv.rs` module docs and enforce it in code review.
- **User surprise from hidden rewrites.** Normalization is invisible by default. If a user reports a bug, the real argv clap saw may differ from what they typed. Mitigation: emit `tracing::debug!("argv rewrite: {old:?} -> {new:?}")` on every mutation, surfaceable via `--debug` (see the open question above).
