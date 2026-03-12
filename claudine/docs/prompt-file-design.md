# Prompt File Design

## Summary

This document designs a new universal wrapper switch for Claudine:

- `--prompt-file <file>`
- alias: `-p <file>`

The switch lets a wrapped provider session source its initial prompt from a Markdown file, run that file through Darkmatter's `compose` pipeline, pass the composed body to the wrapped agent, and expose any residual composed frontmatter as environment variables for the lifetime of the wrapped session.

This is a wrapper concern, not a provider concern. The wrapper resolves the file, validates it, composes it, derives environment variables, injects the resulting prompt into the provider-specific launch shape, and only then spawns the child.

## Goals

1. Provide one cross-agent way to launch a wrapped agent from a Markdown prompt file.
2. Reuse Darkmatter composition so prompt files can use interpolation, transclusion, and frontmatter-driven assembly.
3. Reuse existing monorepo context so prompt-file lookup behaves naturally inside `rusty-biscuit`.
4. Preserve deterministic behavior for CI and `--non-interactive` use.
5. Keep the prompt body and the composed frontmatter distinct:
   - body becomes the wrapped prompt
   - residual frontmatter becomes child-session env vars

## Non-Goals

1. This switch does not replace `--system-prompt`; they address different layers.
2. This switch does not implicitly make non-Markdown files valid prompt sources.
3. This switch does not silently guess between multiple repo matches in non-interactive mode.
4. This switch does not change Darkmatter's composition semantics; it consumes them.

## CLI Contract

### Syntax

```bash
claudine codex --prompt-file @claudine/prompts/acp-wrapping.md --non-interactive
claudine gemini -p ./prompts/review.md --non-interactive
claudine goose -p prompt.md --non-interactive
```

### Reserved Short Flag

`-p` becomes a Claudine-reserved wrapper flag.

This is intentional and follows the same wrapper model Claudine already uses for universal flags like `--non-interactive` / `-n`.

Some wrapped providers already use `-p` natively:

- Gemini: prompt flag
- Qwen: prompt flag
- Codex: profile/local-provider related flags

That is acceptable. When a user launches a provider through Claudine, Claudine-owned short flags take precedence over provider-native short flags.

If a user still needs the provider-native meaning of `-p`, it must go through passthrough after `--`, or they should use the provider's long-form native flag when possible. This only applies to the native `-p` meaning itself. Claudine-owned flags like `--model` still stay at the wrapper layer and do not need passthrough.

Example:

```bash
claudine gemini -p ./review.md --model flash
```

This follows the existing wrapper rule that universal flags are Claudine-owned before passthrough begins.

## High-Level Flow

The wrapper pipeline becomes:

1. Parse wrapper args.
2. Resolve repo/package context.
3. If `--prompt-file` is present:
   - resolve the file reference
   - validate Markdown type
   - compose it with Darkmatter
   - derive child env vars from residual frontmatter
   - inject the composed body as the wrapped prompt
4. Continue with existing wrapper behavior:
   - yolo/non-interactive mapping
   - model/output/system-prompt mapping
   - env sanitization
   - MCP session composition and prompt-tag cleanup
   - child spawn

The important ordering constraint is that prompt-file composition must happen before prompt extraction and MCP tag parsing, so `#tags` inside the composed body are treated the same as any other prompt source.

## Path Resolution

### Accepted Markdown Extensions

Prompt files must resolve to a Markdown path with one of these extensions:

- `.md`
- `.markdown`

Any other extension is rejected immediately with a wrapper error.

This should reuse the same Markdown-file notion Darkmatter already uses for local Markdown transclusion.

### Resolution Context

Resolution uses three roots:

- `cwd`: the shell current working directory
- `repo_root`: the detected git root
- `package_root`: the detected monorepo package root when the cwd is inside a concrete package

`repo_root` should come from the same repo detection already used by the wrapper.

`package_root` should be derived from the same monorepo package selection logic already used to populate `PACKAGE_AREA` and `PACKAGE`. If the cwd is only inside a package area and not inside a concrete package, `package_root` is considered unavailable.

### Resolution Rules

| Input shape | Resolution |
|-------------|------------|
| `@foo/bar.md` | Strip `@`, then join against `repo_root` |
| `./foo/bar.md` | Strip `./`, then join against `package_root` |
| absolute path | Use as-is |
| other path with directory separators | Use as-is, relative to `cwd` if relative |
| bare filename | Use the bare-name fallback search order below |

### Special Prefix Semantics

#### `@`

`@` means repo-root-relative lookup.

Example:

```text
@claudine/prompts/acp-wrapping.md
```

If Claudine cannot determine a repo root, resolution fails with an actionable error.

#### `./`

`./` means package-root-relative lookup, not cwd-relative lookup.

Example from `claudine/cli`:

```text
./prompts/review.md
```

would resolve against the current package root, not against the nested directory from which the command was invoked.

If the cwd is not inside a concrete package, resolution fails with an actionable error that explains that package-relative lookup is unavailable from the current directory.

### Bare Filename Fallback

If the input has no directory structure, Claudine resolves in this order:

1. `cwd/<filename>`
2. `package_root/<filename>`
3. `repo_root/<filename>`
4. repo-wide filename search

The repo-wide search is only attempted when a repo root exists.

### Repo-Wide Search Behavior

The repo-wide search looks for exact leaf-name matches under the repo root.

Examples:

- input: `review.md`
- matches: `claudine/prompts/review.md`, `darkmatter/example-docs/review.md`

Behavior:

- `0` matches: error
- `1` match:
  - interactive wrapper session: ask for confirmation
  - non-interactive wrapper session: error and print the single suggested path
- `>1` matches:
  - interactive wrapper session: show a `Select` prompt
  - non-interactive wrapper session: error and print the candidate list

This preserves deterministic behavior in scripts while still being ergonomic in terminals.

### Interactive Gate

Confirmation and selection are only allowed when all of the following are true:

1. stdin is a terminal
2. stdout is a terminal
3. the wrapper is not in `--non-interactive` mode

Otherwise Claudine must fail fast and print the exact candidate paths the user should choose from explicitly.

## Prompt Source Rules

`--prompt-file` is a prompt source, so it must not silently merge with another prompt source.

The wrapper should treat these as mutually exclusive with `--prompt-file`:

- a provider prompt already present in passthrough args

If both are present, Claudine should error instead of guessing precedence.

Allowed combination:

- `--prompt-file` with `--system-prompt`

Disallowed combination:

- `--prompt-file` plus an explicit prompt positional already passed to the provider
- `--prompt-file` plus `--prompt/-p` already passed through to Gemini or Qwen

## Darkmatter Composition Contract

Once a file is matched:

1. Load it as Markdown from disk.
2. Run it through Darkmatter's `compose` pipeline.
3. Set the source file on `TransformOptions` so relative transclusions resolve from the matched file path.
4. Treat the composed result as:
   - `body = transformed.content()`
   - `frontmatter = transformed.frontmatter()`

This design relies on the library form of composition, not the CLI subprocess form, because Claudine needs both the composed body and the residual composed frontmatter.

### Why Use the Library API

Darkmatter's CLI `compose` command intentionally prints only the composed content. Claudine needs more:

- the composed body for the prompt
- the residual frontmatter map for env injection
- direct access to typed errors

So the wrapper should call the Darkmatter library directly.

## Frontmatter To Environment Variables

If composed frontmatter remains after Darkmatter finishes, Claudine exports it into the child environment for the duration of the wrapped session.

### Name Normalization

Frontmatter keys become env vars with this normalization:

1. uppercase the key
2. replace any non-`[A-Z0-9]` character with `_`
3. collapse repeated `_`
4. trim leading and trailing `_`
5. if the result starts with a digit or becomes empty, prefix `PROMPT_FILE_`

Examples:

| Frontmatter key | Env var |
|-----------------|---------|
| `model` | `MODEL` |
| `agent-type` | `AGENT_TYPE` |
| `target.env` | `TARGET_ENV` |
| `2026_plan` | `PROMPT_FILE_2026_PLAN` |

### Value Serialization

Frontmatter values become env-var strings like this:

| Value type | Encoding |
|------------|----------|
| string | raw string |
| number | decimal string |
| bool | `true` / `false` |
| null | empty string |
| array/object | compact JSON string |

### Collision Rules

Two classes of collision must be rejected:

1. normalized frontmatter-name collisions
   - `foo-bar` and `foo_bar` both become `FOO_BAR`
2. protected env collisions
   - wrapper-reserved names such as `HOME`, `PATH`, `AGENT`, `YOLO`, `INTERACTIVE`, `AGENT_PARAMS`, `CLAUDINE_SESSION_ID`

These should be hard errors, not warnings, because silent overrides would make wrapper behavior unpredictable.

### Precedence

For non-protected names, prompt-file env vars are added as child-session overrides.

Precedence should be:

1. explicit wrapper flags
2. prompt-file-derived env vars
3. inherited process env
4. provider config defaults

This ordering is important for values like `MODEL`, because some providers already consult env vars when applying wrapper defaults.

## Provider Prompt Delivery Strategy

The wrapper should not hard-code prompt injection in `mod.rs`. It should introduce a provider-level prompt delivery abstraction, parallel to the existing model/output/system-prompt mappings.

Recommended trait addition:

```rust
fn apply_prompt_body(
    &self,
    args: &mut Vec<String>,
    prompt: &str,
    non_interactive: bool,
) -> Result<()>;
```

Recommended strategy table:

| Provider | Delivery strategy |
|----------|-------------------|
| Claude | normal initial prompt path for the session mode being launched |
| Kimi | normal initial prompt path for the session mode being launched |
| Gemini | `--prompt <body>` |
| Qwen | `--prompt <body>` |
| Codex | prompt positional after `exec` |
| OpenCode | prompt positional after `run` |
| Goose | provider-native initial prompt argument for the launched session |

Notes:

- The wrapper should inject prompt-file content through the same provider-native initial prompt path a user would normally use by hand.
- `--prompt-file` should not require `--non-interactive` merely because of delivery mechanics.
- Interactive sessions should be able to start with an initial prompt from a prompt file and then continue normally.
- Gemini and Qwen already normalize non-interactive prompts around `--prompt`.
- Codex and OpenCode already have wrapper logic that treats the first non-flag positional after their entrypoint as the prompt location.
- Goose should use whichever explicit argument path best represents "start this session with this initial prompt" rather than relying on stdin seeding.

## Interaction With Existing Wrapper Features

### `--non-interactive`

`--prompt-file` should work with both interactive and non-interactive launches where the provider supports an initial prompt.

It should not force `--non-interactive` solely as a delivery workaround. If the wrapped provider supports starting an interactive session with an initial prompt, `--prompt-file` should support that mode too.

### `--system-prompt`

Allowed together.

The prompt file supplies the user/task prompt. `--system-prompt` still maps to the provider's system prompt channel.

### MCP Tag Extraction

MCP tag extraction should run after prompt-file injection, not before.

That guarantees `#tag` references that come from composed Markdown behave exactly like inline prompts already do.

### `--dry-run`

Dry-run output should include:

- resolved prompt-file path
- which provider prompt channel was used
- env var names derived from frontmatter

The full prompt body does not need to be echoed by default.

## Error Behavior

### Hard Errors

These should exit immediately:

1. resolved file is not Markdown
2. file does not exist after resolution
3. `@...` used without a repo root
4. `./...` used without a concrete package root
5. multiple repo matches in non-interactive mode
6. single repo match requiring confirmation in non-interactive mode
7. prompt-file combined with another prompt source
8. frontmatter env-name collision
9. protected env collision
10. Darkmatter compose failure

### Actionable Error Style

Errors should be phrased the same way the wrapper already phrases binary/preflight failures:

- state what failed
- show the relevant path or candidate paths
- tell the user what to do next

Examples:

```text
prompt file 'notes.txt' is not a Markdown file; expected .md or .markdown
```

```text
'./review.md' is package-relative, but the current directory is not inside a concrete package
```

```text
prompt file 'review.md' matched multiple files in the repo:
  - claudine/prompts/review.md
  - darkmatter/example-docs/review.md
choose one explicitly
```

## Proposed Module Shape

Add a dedicated wrapper helper module:

```text
claudine/cli/src/commands/wrap/prompt_file.rs
```

Suggested responsibilities:

- `resolve_prompt_file(...) -> ResolvedPromptFile`
- `compose_prompt_file(...) -> ComposedPrompt`
- `frontmatter_to_env(...) -> Vec<(String, String)>`
- `detect_existing_prompt_source(...)`

Suggested data structures:

```rust
struct PromptResolutionContext {
    cwd: PathBuf,
    repo_root: Option<PathBuf>,
    package_root: Option<PathBuf>,
    interactive: bool,
    non_interactive: bool,
}

struct ResolvedPromptFile {
    original: String,
    resolved_path: PathBuf,
}

struct ComposedPrompt {
    resolved_path: PathBuf,
    body: String,
    env_overrides: Vec<(String, String)>,
    env_names: Vec<String>,
}
```

`run_provider_wrapper_inner` should stay orchestration-focused and delegate the resolution/composition details into this module.

## Implementation Notes

1. Reuse the wrapper's existing monorepo detection instead of inventing a second repo/package resolver.
2. Add a prompt-delivery hook to `WrapperProfile` rather than building provider-specific `match` arms in `mod.rs`.
3. Deliver prompt-file content through provider-native initial prompt arguments instead of introducing stdin seeding as a wrapper requirement.
4. Keep prompt-file env vars separate from sanitized inherited env so they are clearly visible in dry-run and verbose output.
5. Keep the resolved prompt file path in runtime state so dry-run and future observability can report it.

## Test Plan

### Unit Tests

1. `@` resolution against repo root
2. `./` resolution against package root
3. bare filename fallback order
4. non-Markdown rejection
5. repo-wide single-match confirm behavior
6. repo-wide multi-match selection behavior
7. non-interactive ambiguity failure
8. frontmatter key normalization
9. frontmatter collision detection
10. protected env-name rejection
11. Darkmatter compose returns body plus residual frontmatter

### Integration Tests

1. `claudine codex -p ... -n` injects the composed body as the exec prompt
2. `claudine gemini -p ... -n` maps to `--prompt`
3. `claudine qwen -p ... -n` maps to `--prompt`
4. `claudine goose -p ... -n` maps to Goose's chosen explicit initial-prompt argument path
5. `claudine claude -p ...` starts an interactive Claude session with the composed prompt as the initial prompt
6. `claudine kimi -p ...` starts an interactive Kimi session with the composed prompt as the initial prompt
7. prompt-file-derived env vars appear in the child environment
8. MCP tag cleanup still works on prompts sourced from composed Markdown
9. `--dry-run` reports the resolved prompt file and injected env names

### Snapshot / Help Tests

1. wrapper help output includes `--prompt-file` and `-p`
2. any wrapper documentation snapshots are updated

## Recommended Follow-On Docs

When implementation lands, the same change should update:

- `claudine/cli/README.md`
- `claudine/docs/cli/commands.md`
- any provider-wrapper docs that describe universal wrapper flags

## Final Design Position

`--prompt-file` should be treated as a first-class wrapper input source, not as a thin alias for `read_to_string(path)`.

The feature is only coherent if Claudine does all of the following:

1. resolves the path in repo/package-aware ways
2. enforces Markdown-only inputs
3. composes with Darkmatter before launch
4. exports residual frontmatter into the child env safely
5. injects the composed body through a provider-specific prompt-delivery contract
6. preserves deterministic failure behavior outside interactive terminal sessions

That gives Claudine a real prompt-file system instead of a file-reading shortcut.
