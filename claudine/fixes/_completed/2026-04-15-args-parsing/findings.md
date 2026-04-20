# Arg-Forwarding `--model` Drop / Re-Order Bug

**Date**: 2026-04-15
**Status**: Findings only — no changes made
**Affected providers**: OpenCode (primary), potentially Goose (same `--` + append pattern)
**Symptom**: `--model` supplied before the prompt on the Claudine command line is invisible to OpenCode's yargs parser.

## Root Cause

`prompt_delivery` places `--` before the prompt, and `apply_model` appends `--model` *after* that boundary. OpenCode's yargs parser treats everything after `--` as positional arguments.

## The three steps and their effect on `child_args`

| Step | Line | What it does to `child_args` |
|---|---|---|
| `prompt_delivery` | `mod.rs:738-744` | `AppendArgs(["--", <prompt>])` — appends a `--` separator then the prompt text |
| `apply_entrypoint` | `mod.rs:768` | `args.insert(0, "run")` — prepends the `run` subcommand |
| `apply_model` | `mod.rs:780-784` | `args.push("--model"); args.push(model)` — appends to the **end** |

## Trace: `claudine opencode --model some-model "my prompt"`

1. **Clap** parses `--model some-model` into `args.model = Some("some-model")` (before the positional). `args.passthrough = ["my prompt"]`.
2. **Line 672**: `child_args = ["my prompt"]`
3. **Line 696** `extract_prompt_source`: strips the prompt out → `child_args = []`
4. **Line 738** `prompt_delivery`: appends `["--", "my prompt"]` → `child_args = ["--", "my prompt"]`
5. **Line 768** `apply_entrypoint`: inserts `"run"` at 0 → `child_args = ["run", "--", "my prompt"]`
6. **Line 780** `apply_model`: pushes `["--model", "some-model"]` to end → `child_args = ["run", "--", "my prompt", "--model", "some-model"]`

**Final argv**: `opencode run -- "my prompt" --model some-model`

`--model` is after the `--` boundary. OpenCode's yargs parser treats everything after `--` as positional arguments, so `--model` is invisible as a flag.

## Why the `--` separator exists

`OpencodeWrapper::prompt_delivery` (`profile.rs:1506-1535`) intentionally places `--` before the prompt because composed prompts commonly start with `-`-prefixed tokens (bullet lists like `- some item`), which yargs would reject as unknown options.

## Why the reverse order works

`claudine opencode "my prompt" --model some-model`:

- With `trailing_var_arg = true`, clap dumps everything after the first positional into `passthrough`, so `args.model = None` and `--model` stays in the passthrough bucket.
- `extract_prompt_source` removes the prompt → `child_args = ["--model", "some-model"]`
- `prompt_delivery` appends `["--", "my prompt"]` → `child_args = ["--model", "some-model", "--", "my prompt"]`
- `apply_entrypoint` inserts `"run"` at 0 → `["run", "--model", "some-model", "--", "my prompt"]`
- `apply_model` is skipped (`args.model` is `None`), `apply_non_interactive_defaults` sees `--model` already present and skips, and the `MODEL` env-var fallback at line 787 picks it up.

**Final argv**: `opencode run --model some-model -- "my prompt"` — correct.

## `apply_structured_stream` has the same class of bug

`OpencodeWrapper::apply_structured_stream` (`profile.rs:1569-1572`) pushes `["--format", "json"]` to the end of `child_args` at line 1157 — also after `--`. For the `--model`-before-prompt case, the full argv becomes:

```
opencode run -- "my prompt" --model some-model --format json
```

Both `--model` and `--format` are positionalized. The `MODEL` env var (`apply_model` sets it at `profile.rs:1487`) may partially compensate for `--model`, but `--format json` has no env-var fallback — meaning the structured stream parser may receive non-JSON output and fail.

## `env.rs` sanitization is NOT involved

`sanitize_process_env` (`env.rs:233-280`) only strips env vars matching sensitive-key patterns (`API_KEY`, `TOKEN`, `PASSWORD`, `SECRET`, etc.). `MODEL` does not match any of those patterns and passes through untouched. The `redact_sensitive_args` function (`env.rs:298-345`) only redacts values for logging into `AGENT_PARAMS`; it does not mutate `child_args`.

## Summary

| Factor | Contribution |
|---|---|
| `apply_entrypoint` ordering | Inserts `"run"` at index 0 — not harmful by itself |
| `prompt_delivery` `--` separator | The `--` is intentional for bullet-list prompts but creates a "dead zone" after it |
| `apply_model` appending to end | **The core issue** — pushes `--model` past the `--` boundary where yargs ignores it |
| `apply_structured_stream` appending to end | Same class of bug — `--format json` lands after `--` |
| `env.rs` sanitization | Not involved — only filters sensitive-key env vars |

## Key files

- `claudine/cli/src/commands/wrap/mod.rs` — `run_provider_wrapper_inner` (lines 640-1379), `extract_wrapper_flags_from_passthrough` (lines 3295-3375), `model_value_from_args` (lines 2861-2874)
- `claudine/cli/src/commands/wrap/profile.rs` — `OpencodeWrapper::apply_entrypoint` (line 1445), `OpencodeWrapper::apply_model` (line 1478), `OpencodeWrapper::apply_structured_stream` (line 1569), `OpencodeWrapper::prompt_delivery` (line 1506)
- `claudine/cli/src/commands/wrap/env.rs` — `build_child_env_with_launch` (line 86), `sanitize_process_env` (line 233), `redact_sensitive_args` (line 298)
- `claudine/cli/src/main.rs` — `parse_cli` two-pass wrapper parsing (line 66)
