# Argv Normalization

Claudine's CLI is parsed by `clap` via derive. The surface has grown to
the point where clap's ordinary parsing model produces rough edges:
eight boolean provider flags in a mutual-exclusion group, positional
"file plus `key=value` setters" collected with `num_args = 1..`, and
fuzzy provider matching that historically only applied to the
`--provider` value.

Claudine installs a thin **argv normalization layer** between
`std::env::args_os()` and `Cli::parse_from` so a curated set of
shorthand patterns is reshaped into the canonical form clap already
understands. clap remains the authoritative parser; the normalizer
never consults clap, never reads the filesystem, and only reshapes
input on the way in.

The implementation lives in [`claudine/cli/src/argv.rs`](../../cli/src/argv.rs),
and is wired into [`claudine/cli/src/main.rs`](../../cli/src/main.rs) as
the single pre-clap entry point.

## Pipeline placement

```text
                ┌────────────────────────────┐
std::env::args_os│  argv::normalize(...)       │  Vec<OsString>
  ──────────────▶│  - Rule 1: booleans        │──────────────▶ Cli::parse_from
                 │  - Rule 2: fuzzy value     │
                 │  - Rule 3: `--` insertion  │
                 └────────────────────────────┘
```

`main.rs` collects argv once, passes it to `argv::normalize`, and reuses
the same normalized vector for the `--plain` pre-scan and every parse
pass. Library code never sees argv and therefore never normalizes.

## Rewrite rules

Rules are applied in order, in a single left-to-right pass, and stop at
the first literal `--` token.

### Rule 1 — provider boolean → `--provider <slug>`

The eight user-facing provider booleans are rewritten to the canonical
`--provider <slug>` pair using `Provider::as_slug()`:

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

**Before**

```sh
claudine compose file.md --gemini
```

**After normalization (what clap sees)**

```sh
claudine compose file.md --provider gemini
```

Duplicates are preserved verbatim so clap keeps emitting its existing
mutual-exclusion error — `--claude --gemini` becomes
`--provider claude --provider gemini` and clap rejects it.

The normalizer does **not** fuzzy-match these flags. `--claud` is left
untouched so clap surfaces an unknown-argument error, which is the
desired outcome.

### Rule 2 — fuzzy `--provider <value>` canonicalization

Both the space form (`--provider cl`) and the equals form
(`--provider=cl`) are passed through `Provider::fuzzy_match_cli_name`.
If the helper resolves a match, the value is replaced with its canonical
slug. If it does not, the token is left untouched so clap keeps its
native "invalid value" error listing the valid variants.

**Before**

```sh
claudine compose --provider cl file.md
claudine compose --provider=oc file.md
```

**After normalization**

```sh
claudine compose --provider claude file.md
claudine compose --provider=open_code file.md
```

Edge cases intentionally left untouched:

- `--provider` with no following token — clap produces "a value is required".
- `--provider=` with an empty value — clap produces its native error.
- `--provider -x` — the next token starts with `-` and is treated as a
  flag, not a value. Leave untouched.

### Rule 3 — `--` separator insertion before trailing setters

This rule fixes the motivating bug: `claudine compose file.md --gemini name=Ken --help`
used to produce a misleading

```text
error: unexpected argument '--help' found
  tip: to pass '--help' as a value, use '-- --help'
```

because clap's greedy multi-value positional
(`#[arg(num_args = 1..)]`) absorbed `--help`. The normalizer walks the
argv after the composition subcommand and, when it detects a positional
token followed by at least one flag (or flag-and-value pair) followed by
a setter-shaped token, inserts a single `--` separator before the first
such setter.

A setter-shaped token matches the same key pattern enforced by
`parse_compose_setter` — `^[A-Za-z_][A-Za-z0-9_-]*=` — so anything the
positional parser already classifies as a shorthand setter is protected
behind the separator.

**Gates:**

- Fires only on the `compose`, `inline-compose`, and `sequence`
  subcommands.
- Never inserts more than one `--` separator.
- Never inserts `--` when the argv already contains one.
- Only fires after at least one real positional has been seen and at
  least one flag-or-flag-value has appeared between that positional and
  the candidate setter.
- Respects Claudine root-level globals that precede the subcommand
  (`--plain`, `--verbose`, `--debug [LEVEL]`, `--help`, `-h`).

**Before**

```sh
claudine compose file.md --gemini name=Ken --help
claudine sequence file.md --gemini k=v --help
claudine --plain compose file.md --gemini name=Ken --help
```

**After normalization**

```sh
claudine compose file.md --provider gemini -- name=Ken --help
claudine sequence file.md --provider gemini -- k=v --help
claudine --plain compose file.md --provider gemini -- name=Ken --help
```

Rule 3 is intentionally narrow. It does not fire when:

- the positional-plus-setter case has no flag interleaving
  (`claudine compose file.md key=val` — common case, unchanged);
- the trailing non-setter token is not a setter
  (`claudine compose file.md --gemini other.md` — clap still errors on
  the second file, which is the intended pre-existing behavior);
- the subcommand is not a composition subcommand
  (`claudine hooks file.md --gemini k=v --help` — untouched by Rule 3);
- a `--` is already present in the argv;
- no positional has been seen yet
  (`claudine compose --help` — nothing to protect).

## Pass-through guarantees

The normalizer never mutates argv when any of the following hold:

1. **Completion mode.** `clap_complete::CompleteEnv` signals completion
   through the `COMPLETE` environment variable. When set, argv is
   returned untouched so dynamic completion sees exactly what the shell
   typed.
2. **Tokens at or after `--`.** The first literal `--` terminates the
   rule scan; everything after it is copied verbatim.
3. **Non-UTF-8 tokens.** Rules are pattern-based on `&str`; `OsString`
   values that are not valid UTF-8 are left in place.
4. **Argv with fewer than two elements.** Nothing downstream needs
   parsing.
5. **Unknown subcommands.** Rule 1 and Rule 2 are subcommand-agnostic;
   Rule 3 is gated to the composition trio. Unknown-subcommand handling
   stays with clap.

Every new rule added to the normalizer MUST land with a matching
pass-through unit test so the normalizer cannot silently start
rewriting inputs it should leave alone.

## Testing

Unit tests live inside the `argv.rs` module
(`#[cfg(test)] mod tests`) and cover every rewrite rule, each
boolean-to-slug mapping, every pass-through guarantee, and a dense set
of Rule 3 corner cases (flag values, equals-form flags, short-form
flags, first-setter-only insertion, setter-before-positional, etc.).

Integration tests live in
[`claudine/cli/tests/argv_normalization.rs`](../../cli/tests/argv_normalization.rs)
and drive the compiled `claudine` binary through the three headline
cases from the feature spec plus the key pass-through cases
(`--version`, root `--help`, `hooks --describe`).

Reference: feature `2026-04-17-cli-pre-processing`.
