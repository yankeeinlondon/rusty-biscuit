---
prompt: |-
  Review the execution flow of the 'claudine compose', 'claudine inline-compose', 'claudine sequence'.

  - identify the precise steps that each command takes to produce it's results
  - describe each step in the process
  - identify what CLI switches or prompt frontmatter has an impact on each step and what that impact is

  After fully documenting the flows add a section called `## Future Improvements` and add any improvements you think could be made to the current flow
last_updated: 2026-07-04
hash: 7738d51f4cfa73e2-83b6b84817b7ee16
---
Let me explore the composition-related source code and documentation to trace the exact execution flows.Now let me read the existing composition topic doc and the key source files for precision:Now let me read the critical execution pipeline files:Now let me read the remaining portions of the composition executor to get the full execution pipeline:Now I have a comprehensive understanding of all three flows. Let me compose the complete Markdown body.Now I have all the details needed. Let me produce the comprehensive Markdown body content:Now I have all the details. Let me compose the full Markdown document body:

The existing `composition.md` file is 540 lines. The user wants me to return the replacement Markdown body content only (no frontmatter delimiters). I'll produce the complete replacement body with the detailed execution flows and a Future Improvements section.

Here is the replacement Markdown body content:

---

Claudine supports the ability to _compose_ content by leveraging the Darkmatter library's powerful composition features and routing the result through a wrapper-grade execution pipeline.

Three canonical commands:

- **`claudine compose [flags] <arg>...`** — direct (chained) composition
- **`claudine inline-compose [flags] <arg>...`** — inline composition
- **`claudine sequence [flags] <arg>...`** — serial sequence composition

Both `compose` and `inline-compose` share the same six-stage pipeline and inherit full wrapper-grade behavior: environment setup, harness detection, structured streaming, and handler-driven recovery. `sequence` iterates that same pipeline per step with shared state.

Because composition flows through the same execution path as `claudine claude` / `codex` / etc., it inherits every behavior of the live stderr surface documented in [Non-Interactive Sessions](non-interactive-sessions.md):

- **Tool call rendering** — `→ Name(summary)` / `← Name(slot)` with shell-name prefixing for `Bash` / `shell` / `run_command` and `description → subject → prompt → task` field order for `Task`.
- **Idle flush** — buffered assistant markdown is flushed by an independent 30-second ticker whenever the block buffer has been idle for 30 s, so a dangling final paragraph never sits invisible while a slow-to-close provider waits to exit.
- **Prompt-scoped timing header** — every 10 minutes the stderr surface emits `⏱️ {HH:MM} {TZ} running the <prompt> prompt for <duration>` anchored on the prompt's start time (and a `t=0` header without the duration at run start). See [Timing Surface](#timing-surface) below.
- **Typed error rendering** — `SemanticEvent::Error` is rendered as a colored `BlockQuote` whose label and border come from `SemanticErrorKind` (`Configuration`, `AgentNative`, `ApiRemote`, `Interrupted`, `Unknown`).
- **Reasoning / thinking** — provider reasoning (Claude, Codex, OpenCode, Gemini, Qwen) renders into `Section::Thinking` as a `BlockQuote` with the wider `▌ ` border that matches the System Prompt and Agent Prompt sections.

## Positional Arguments

Each command accepts exactly one file reference plus zero or more `key=value`
setters, in any order:

```sh
claudine compose @prompts/review.md review=review.md
claudine compose review=review.md @prompts/review.md
claudine inline-compose draft=false @notes/update.md
claudine sequence @deploy.md
```

A token is a setter when it contains `=` and its key starts with an ASCII
letter or `_` and contains only letters, digits, `_`, or `-`. Dot-paths and
path-like tokens (for example `foo.bar=baz`) are not setters and are treated
as file-reference candidates.

Setter values are parsed as JSON5 first and fall back to strings when JSON5
parsing fails, so `count=3`, `enabled=true`, `tags=["a","b"]`, and
`review=review.md` all resolve to their natural types.

Inline setters override matching keys from `--set`. For `sequence`, reserved
per-step overlay keys still win over both `--set` and shorthand setters.

## Shell Completion

Dynamic completion fires at markdown-expecting argument positions on all
three composition commands and on the `--append-system-prompt` /
`--replace-system-prompt` flag values — both on compose/inline-compose/
sequence themselves and on every wrapped provider subcommand (`claude`,
`codex`, `gemini`, `goose`, `kimi`, `opencode`, `qwen`). All three
composition commands share one markdown-only contract; there is no
per-command frontmatter validation at completion time.

The generated bash/zsh/fish scripts shell out to `claudine __complete` on
every `<TAB>`. The supplement engine applies these rules in order:

- **Candidates are markdown files only** (`*.md`). Directories,
    non-markdown files, `./`/`../` traversal tokens, `!` package sigils,
    `vault:`, `/abs`, `%`, and `{{…}}` prefixes all return zero candidates.

- **Two supported entry forms**: `@`-prefixed magic paths (enumerated
    against repo root + user home) and implicit-relative paths like
    `prompts/…` (enumerated against the repo root only).

- **Typed-length scope**: 0–2 "meaningful characters" (leading `@` and
    segments before a `/` don't count) use the curated scope only —
    `prompts/` and `sequences/` under `<repo>/`, `<package-root>/`,
    `<package-area-root>/`, `~/`, and `~/.claudine/`. 3+ characters extend
    to a `.gitignore`-aware walk of the enclosing git repo.

- **Case-insensitive substring matching** on the filename with `.md`
    stripped for matching only. `@omp<TAB>` matches `prompt.md`.

- **`KEY=<TAB>` setters** offer schema-aware completion when the prompt
    declares `$schema` (property names before `=`; enum members and
    `file`-glob paths after `=`). Without a schema, the value slot still
    supports `@`-gated file completion; plain string/number values yield
    no candidates so shell default behavior kicks in. See
    [Shell Completions](completions/shell-completions.md).

Install with `claudine completions <shell>` — regenerate and reinstall
after a Claudine upgrade that changes the callback wiring. The hidden
`claudine __complete` subprocess always tracks the running binary, so
the completion candidates themselves never go stale. PowerShell and
Elvish retain the legacy one-line `COMPLETE=<shell>` bootstrap; users
who installed an older `COMPLETE=<shell>` snippet continue to reach the
legacy completion path on every shell until they regenerate. See
[completions/shell-completions.md](completions/shell-completions.md) for the full install
matrix, supported token shapes, and the open-questions list.

## Execution Flows

### Common Bootstrap (all three commands)

All three commands share the same bootstrap path before diverging:

#### Step 0: Argv Normalization

**Source:** `cli/src/argv.rs`

Before clap parses anything, `argv::normalize()` applies four syntactic rewrite rules in fixed order:

1. **Rule 1** — Rewrites provider boolean flags (`--claude`, `--codex`, `--gemini`, `--goose`, `--kimi`, `--opencode`, `--qwen`) to `--provider <slug>` on composition subcommands only, so wrapper passthrough is preserved.
2. **Rule 2** — Canonicalizes `--provider <value>` / `--provider=<value>` via `Provider::fuzzy_match_cli_name` (e.g. `cl` → `claude`).
3. **Rule 4** — Hoists a trailing `--help` / `-h` to argv position 1 on composition subcommands so the root custom help handler fires.
4. **Rule 3** — Inserts a single `--` separator before the first `key=value` setter that follows an interleaved flag after a previously seen positional, fixing ambiguous arg boundaries.

The normalizer is a strict no-op under `COMPLETE` (shell completion), after the first literal `--`, on non-UTF-8 tokens, for argv with fewer than two elements, and on non-composition subcommands.

#### Step 1: Argument Parsing and Validation

Each entry point destructures its clap args struct, then calls the shared `parse_composition_positionals()` helper to classify positional tokens into a file reference and a map of `key=value` setters.

**Affected by:**

| Input                     | Impact                                                                                          |
|---------------------------|-------------------------------------------------------------------------------------------------|
| Positional tokens         | Classified as file ref or `key=value` setter; errors on multiple file refs or empty setter keys |
| `--set JSON`              | Parsed as JSON5 object; forms the base override map                                             |
| `key=value` setters       | Override matching keys from `--set` on conflict                                                 |
| `--step-timeout DURATION` | Validated against the `parse_timeout()` grammar; rejected when the session resolves to interactive (`--interactive` or `interactive: true` frontmatter) |

#### Step 2: Source Resolution

`composition::resolve_composition_source(&file_ref)` resolves the file reference.

**What it does:**

1. Constructs a `FileReference` with package-area magic path support
2. Calls `.resolve()` to find the actual file on disk (supports `@` magic, `!` package, repo-relative, monorepo-package-relative, absolute paths)
3. Validates `.md`/`.markdown` extension
4. Reads file content and parses into a `Markdown` struct

**Returns:** `ResolvedCompositionSource { original_ref, resolved_path, original_text, markdown }`

**Affected by:**

| Input                 | Impact                                                         |
|-----------------------|----------------------------------------------------------------|
| File reference string | Determines resolution strategy (`@` magic, relative, absolute) |

---

### `claudine compose` — Full Execution Flow

Direct composition takes a Markdown file, composes it through Darkmatter, and sends the composed content as a prompt to an agentic CLI. No files are mutated.

```sh
claudine compose @commit.md
claudine compose --codex @commit.md
```

#### Step 3: Pre-flight Shell Approval (compose)

Scans all shell command sources in the document before execution begins.

**What it does:**

1. Builds `ComposeOptions` with source file and set overrides
2. Creates a shared approval cache (`Arc<Mutex<HashMap>>`) for reuse across harness commands
3. Builds `ShellApprovalOptions` with interactive approval handler (when TTY is available) and policy root from git root or source directory
4. Calls `composition::resolve_shell_approvals()`:

    - **Source 1**: Template `::shell` directives via Darkmatter's document graph walker
    - **Source 2**: Harness pre/post checks and handlers (none at this stage for compose — only template directives)
    - Deduplicates commands
    - Each command is validated against whitelist via `validate_and_approve_command_parts()`
    - Interactive approval handler prompts user for non-whitelisted commands

5. Returns `PreFlightResult { approved_commands, total_discovered, ... }`

**Affected by:**

| Input                         | Impact                                                                                         |
|-------------------------------|------------------------------------------------------------------------------------------------|
| `--set` / `key=value` setters | Template variables influence which `::shell` directives execute and what commands they produce |
| `--silent`                    | Suppresses interactive approval prompts (may cause hard errors for unapproved commands)        |

#### Step 4: Composition Preparation (compose)

`composition::prepare_direct(&source, PrepareOptions)` composes the document through Darkmatter.

**What it does:**

1. Creates `ComposeContext::capture()` with env overrides
2. Builds `ComposeOptions` with source file, set overrides, pre-approved commands, perf flag
3. Calls `source.markdown.compose_with(compose_opts)` through Darkmatter:

    - Expands transclusions
    - Runs `::shell` directives (using pre-approved commands)
    - Performs template variable interpolation
    - Evaluates conditionals

4. Extracts effective frontmatter from the composed result
5. Parses `agent` hint → `AgentHint::Single` or `AgentHint::List` (fuzzy-matched against known providers; unknown names error immediately)
6. Parses `model` hint → `ModelHint::Single` or `ModelHint::List`
7. Parses lifecycle config (`start`, `success`, `blocked`, `failure`)
8. Finds git root via `find_git_root_from_path()`

**Returns:** `PreparedComposition { mode: ChainedDocument, resolved_path, source_repo_root, prompt, effective_frontmatter, selection_hints, closure: Direct, lifecycle, compose_perf }`

**Affected by:**

| Input                                             | Impact                                                               |
|---------------------------------------------------|----------------------------------------------------------------------|
| Frontmatter `agent`                               | Parsed as `AgentHint`; used by provider selection                    |
| Frontmatter `model`                               | Parsed as `ModelHint`; used by model resolution                      |
| Frontmatter `start`/`success`/`blocked`/`failure` | Parsed as lifecycle config for notification emissions                |
| `--set` / `key=value` setters                     | Override template variables before Darkmatter processes the document |
| `--perf`                                          | Enables Darkmatter composition performance collection                |

#### Step 4b: Schema Validation (compose)

After `prepare_direct` produces the effective frontmatter, the schema-aware
wrapper translates Darkmatter's structural failures into typed claudine errors:

1. **No `$schema`** — pass through unchanged.
2. **Required value missing** — return `CompositionError::MissingProperties`
   carrying the prompt path, the missing names in declaration order, their
   type labels and descriptions, and the frontmatter `description` for context.
   When Interactive Mode is allowed (config `prompt_for_missing` + stdin/stderr
   TTYs + not `--silent`) the CLI consumes this error to drive a `biscuit-tui`
   prompt loop and re-runs preparation with the collected overrides.
3. **Required value present but invalid** — return
   `CompositionError::SchemaValidation` as a hard abort; no prompting.
4. **Optional value present but invalid** — drop the key from a clone of the
   markdown, emit a `tracing::warn!`, and retry preparation once. Residual
   problems (typically a missing required) surface after the retry.
5. **`$schema` itself unresolvable / uncompilable** — return
   `CompositionError::SchemaLoad`.

This stage runs before provider/model selection so a schema failure aborts
without launching anything.

**Affected by:**

| Input                              | Impact                                                                                                  |
|------------------------------------|---------------------------------------------------------------------------------------------------------|
| Frontmatter `$schema`              | Activates schema validation; controls required vs optional classification and Interactive Mode mapping  |
| `--set` / `key=value` setters      | Applied before validation so users can satisfy required values without entering the interactive prompt  |
| Config `prompt_for_missing`        | Required (along with TTY/silent checks) to enable Interactive Mode for missing-required collection      |
| `--silent`                         | Forces Interactive Mode off; missing required values surface as `MissingProperties` instead of prompts  |

#### Step 5: Build Execution Request (compose)

Constructs a `CompositionExecutionRequest` with `mode: ChainedDocument`, the prepared composition, all CLI flags, and an empty env override map. `resolved_target` is set to `None` — the executor will resolve provider and model.

#### Step 6: Execute Composition Request (compose)

`execute_composition_request_inner()` runs the full wrapper-grade pipeline:

##### 6a. Provider and Model Selection

1. Detects installed providers via `InstalledAiClients::new()`
2. Builds installed provider snapshot, filtering excluded providers
3. Loads selection config (favorite provider, model overrides) from Claudine config
4. Builds model catalog via `ModelCatalogService` (no upfront refresh; refresh is provider-scoped and deferred until after a provider is selected)
5. Resolves execution target:

    - **If `resolved_target` is pre-set** (from sequence): uses it directly
    - **TTY mode**: explicit `--provider` flag wins unconditionally; otherwise shows interactive picker via `biscuit-tui::ChooseOne`
    - **Non-TTY mode**: `resolve_target_non_tty_with_catalog()` applies strict chain: explicit flag → frontmatter `agent` (single or first-installed-from-list) → config favorite → hard error

6. Resolves model: CLI `--model` → provider-specific env vars → generic `MODEL` → frontmatter `model` (validated against catalog) → provider default. When the frontmatter `model` hint is the only source, the selected provider's catalog is refreshed via `refresh_provider_blocking(provider)` before validation; CLI/env model wins skip the refresh entirely.

**Affected by:**

| Input                                      | Impact                                                              |
|--------------------------------------------|---------------------------------------------------------------------|
| `--provider` / `--claude` / `--codex` etc. | Unconditional provider selection; bypasses all resolution chains    |
| `--exclude`                                | Removes providers from candidate pool                               |
| `--model` / `-m`                           | Overrides model selection; highest priority in model chain          |
| Frontmatter `agent`                        | Hint for provider selection (single string or ordered list)         |
| Frontmatter `model`                        | Hint for model selection (validated against catalog when available) |
| Config `favorite_agent`                    | Fallback provider when no explicit flag or frontmatter hint         |

##### 6b. Profile and Binary Resolution

Loads the `WrapperProfile` for the resolved provider and locates the binary on disk.

##### 6c. Early Header Emission

Emits the execution line to stderr with `ComposeDisplay::Compose`, showing provider, file reference, operation, and flags.

**Affected by:**

| Input            | Impact                                   |
|------------------|------------------------------------------|
| `--quiet` / `-q` | Suppresses env details and info messages |
| `--silent`       | Suppresses all preflight output          |

##### 6d. Environment Setup

Builds child process environment via `env::build_child_env()`:

- Constructs `EnvPlan` with env vars, shadow HOME, sensitive var stripping
- Sets `OPERATION` env if specified
- Applies request-level env overrides

**Affected by:**

| Input                  | Impact                                                    |
|------------------------|-----------------------------------------------------------|
| `--operation` / `--op` | Sets `OPERATION` env var for the child process            |
| `--repo`               | Uses shadow HOME for repo-scoped resources                |
| `--include`            | Preserves named env vars that would otherwise be stripped |

##### 6e. MCP Session Setup

When `--mcp` or `--mcp-use` is active:

1. Loads MCP catalog from `~/.claudine/mcp/catalog.json`
2. Extracts `#tags` from the prompt and strips them
3. Computes session set (resolves tags, handles ambiguous/missing)
4. Injects provider-specific MCP servers (Codex/Gemini use shadow HOME; OpenCode uses `OPENCODE_CONFIG_CONTENT`)

**Affected by:**

| Input       | Impact                                                  |
|-------------|---------------------------------------------------------|
| `--mcp`     | Enables MCP session composition with effective defaults |
| `--mcp-use` | Activates specific MCP servers by ID or alias           |
| `--strict`  | Treats unresolved or ambiguous MCP tags as hard errors  |

##### 6f. Provider Flag Assembly

Applies provider-specific flags to child args:

- YOLO/auto-approval mode via `profile.apply_yolo_for_mode()`
- Entrypoint via `profile.apply_entrypoint()`
- Non-interactive flags via `profile.apply_non_interactive_flags()`
- OpenCode model resolution (special handling)
- Model flag via `profile.apply_model()`
- Output format via `profile.apply_output_format()`
- Sandbox via `profile.apply_sandbox()`

**Affected by:**

| Input                  | Impact                                                    |
|------------------------|-----------------------------------------------------------|
| `--yolo` / `-y`        | Enables provider-specific auto-approval mode              |
| `--interactive` / `-i` | Switches from non-interactive to interactive session mode |
| `--no-interactive`     | Forces non-interactive mode, overriding `interactive: true` frontmatter |
| `--output` / `-o`      | Sets output format (json, text, stream)                   |
| `--sandbox`            | Enables provider-specific sandboxing                      |
| `--model`              | Applied to provider-specific flags                        |

##### 6g. System Prompt Resolution

1. Builds `LaunchContext::from_cwd()` for workspace detection
2. Calls `resolve_and_prepare_for_session()`:

    - Resolves system prompt source: `--append-system-prompt` > `--replace-system-prompt` > standard discovery (package > package-area > repo > user home)
    - Composes through Darkmatter
    - For non-interactive: appends non-interactive safety instructions

3. Applies system prompt to child args via `profile.apply_system_prompt()`

**Affected by:**

| Input                               | Impact                                                          |
|-------------------------------------|-----------------------------------------------------------------|
| `--append-system-prompt` / `--asp`  | Appends a system prompt file                                    |
| `--replace-system-prompt` / `--rsp` | Replaces the provider's system prompt                           |
| (neither flag)                      | Standard `system-prompt.md` discovery from launch CWD hierarchy |

##### 6h. Prompt Delivery

Delivers the composed prompt via `profile.prompt_delivery()` (places prompt in args, stdin, or wire RPC depending on provider).

##### 6i. Dry Run Check

If `--dry-run`: prints what would be executed and exits with code 0.

##### 6j. Preflight Checks

1. Switches process CWD to child working directory via `switch_process_cwd` (in `claudine/cli/src/commands/wrap/mod.rs`). This is **intentional and not restored** for two reasons:
    - Agents should always start at the repo root for permission-grant scope and consistent context — many provider sandboxes treat CWD as the trust boundary.
    - The original launch CWD is preserved separately on `LaunchContext`/`LaunchWorkspaceContext` so downstream stages (system-prompt discovery, package-area detection) still know where the user actually invoked Claudine from.

    **Implication for loops, sequences, and `ctx.*`:** because the CWD mutation persists for the parent claudine process across iterations and across sequence steps, any code that re-derives state from `std::env::current_dir()` between iterations (notably `ComposeContext::capture()` for `ctx.current_package_area` and friends) will see the post-switch CWD on iteration 2+ / step 2+. The intended fix is a Claudine-owned launch-CWD scratchpad captured once at process startup and used as the base directory for every `ComposeContext::capture_for_dir(...)` call — not re-reading the process CWD, and not anchoring to the source file's parent (which would change the user-facing meaning of `ctx.current_package_area`).

2. Detects harness from effective frontmatter via `has_harness_properties()`
3. Builds lifecycle context and creates `LifecycleRunGuard`
4. Parses the harness plan from the effective frontmatter; for bare documents (no harness properties) this yields the empty/bare plan. Prepend the inline writability pre-check if the mode is inline.
5. Resolves shell approvals for harness commands (and any system-owned inline writability check) via `resolve_shell_approvals`
6. Emits preflight-complete status

##### 6k. Execution

All non-dry-run composition runs flow through `run_harness_loop()` with `HarnessPromptMode::Compose` or `HarnessPromptMode::Inline`. The loop re-parses the plan each attempt, runs pre-checks (including the system-owned inline writability rule), spawns the provider, streams the response, runs post-checks, and invokes handler-driven recovery on failure.

##### 6l. Post-Execution

Emits terminal lifecycle signal (`Success` or `Failure`), renders performance report if `--perf`.

**Returns:** Exit code 0 on success, non-zero on failure.

---

### `claudine inline-compose` — Full Execution Flow

Inline composition uses the `prompt` frontmatter property as input and replaces the document's body with the provider's output.

```sh
claudine inline-compose @research.md
claudine inline-compose --claude @research.md
```

#### Step 3: Source Resolution with Reporting (inline-compose)

Same resolution as compose, but additionally reports source file resolution to terminal via `report::report_source_file()` and creates a tracing span named `inline_compose`.

#### Step 4: Prompt Property Validation (inline-compose — unique)

Extracts the `prompt` frontmatter property and validates:

- `has_prompt`: property exists in frontmatter
- `is_non_empty`: string value is non-whitespace

Reports validation status to terminal. This check runs before any Darkmatter composition to fail fast on documents that don't declare a prompt.

**Affected by:**

| Input                | Impact                                                                                     |
|----------------------|--------------------------------------------------------------------------------------------|
| Frontmatter `prompt` | Must be present and non-empty; the prompt text that gets composed and sent to the provider |

#### Step 5: Pre-flight Shell Approval (inline-compose)

Identical to compose Step 3.

#### Step 6: Inline Composition Preparation (inline-compose — differs from compose)

`composition::prepare_inline(&source, PrepareOptions)` prepares the prompt differently:

**What it does:**

1. Extracts `prompt` frontmatter property (required; errors if missing or non-string)
2. Builds a temporary `Markdown` with frontmatter + prompt text as body
3. Creates `ComposeContext` and `ComposeOptions` (same as direct)
4. Calls `temp_md.compose_with(compose_opts)` through Darkmatter on the temporary document
5. Extracts effective frontmatter, agent/model hints, lifecycle config
6. **Appends guardrails** via `load_or_create_guardrails(source_repo_root)` — adds instructions like "Return the replacement Markdown body content only". Guardrails are loaded from `.claudine/inline-compose.md` in the repo root (created on first use), falling back to a built-in default.
7. **Captures pre-execution Simple hash** via `source.markdown.compute_hash(MdHashKind::Simple, &inline_hash_options())` for closure validation

**Returns:** `PreparedComposition { mode: InlineFrontmatterPrompt, ..., closure: Inline(InlineClosurePlan { original_document_text, original_hash }) }`

**Affected by:**

| Input                         | Impact                                                             |
|-------------------------------|--------------------------------------------------------------------|
| Frontmatter `prompt`          | Required; the text that gets composed and sent as the agent's task |
| `.claudine/inline-compose.md` | Custom guardrails file; overrides default instructions             |

#### Step 6b: Schema Validation (inline-compose)

Schema validation runs **after** the `prompt` property check so a missing or
non-string `prompt` still surfaces as `PromptPropertyMissing` /
`PromptPropertyWrongType` instead of a generic schema error. After that
guard, the same typed translation as direct compose applies — `SchemaLoad`,
`SchemaValidation`, `MissingProperties`, automatic drop-and-retry for invalid
optionals. The original `$schema` declaration is preserved byte-for-byte by
the inline rewrite, and interactive values collected during the run are never
persisted to the source file.

#### Step 7: Build Execution Request (inline-compose)

Constructs `CompositionExecutionRequest` with `mode: InlineFrontmatterPrompt` and `closure: Inline(InlineClosurePlan)`.

#### Step 8: Execute Composition Request (inline-compose)

Runs the same pipeline as compose Step 6, with these differences:

- Header shows `ComposeDisplay::InlineCompose` instead of `Compose`
- Inline + interactive check: rejects providers that don't support interactive inline closure recovery
- All non-dry-run runs call `run_harness_loop()` with `HarnessPromptMode::Inline`

**Affected by:**

| Input                  | Impact                                                                                                             |
|------------------------|--------------------------------------------------------------------------------------------------------------------|
| `--interactive` / `-i` | Provider-gated; only allowed when the provider can recover the final assistant message for the inline rewrite path |

#### Step 9: Inline Closure (inline-compose — unique)

After the provider completes, the inline closure pipeline rewrites the target file:

##### 9a. Exit Code Validation

- Exit 130/143 (interrupted): reports interruption, returns error
- Exit 0: reports agent completed
- Non-zero: reports agent error

##### 9b. Body Extraction

`closure::extract_replacement_body(&final_response)`:

1. Trims whitespace, rejects empty
2. Strips accidental frontmatter fences from provider output
3. Validates body is non-empty

##### 9c. Document Reconstruction

`closure::apply_inline_closure(plan, body, path, today, post_run_fm)`:

1. Validates replacement body is non-empty
2. Runs `darkmatter::markdown::cleanup::cleanup_content()` on the body so the cleaned body is what is hashed, stamped, and written (one atomic write; `result.body_cleaned` records whether cleanup changed anything)
3. Rejects when the **body segment** of the Simple hash of the *cleaned* body matches the original
4. Compares frontmatter to detect new and modified properties
5. Calls `rewrite_inline_document()`:

    - Splits frontmatter from source text (preserving byte-for-byte layout including block scalars)
    - Updates `last_updated` to today's date (local time, `YYYY-MM-DD`)
    - Stamps a Darkmatter `Simple` content hash into the `hash:` frontmatter property (see [Composition — `hash` property](composition.md#hash-property-auto-stamped))
    - Merges new frontmatter properties from agent (inserted before `last_updated`)
    - Preserves original frontmatter values (reverts agent modifications with a warning)

6. Writes atomically via `atomic_write()`

##### 9d. Summary Emission

Summary is deferred until after closure validation messages (unlike compose which emits immediately), so the section separator does not split the validation block.

**Affected by:**

| Input                      | Impact                                                                                            |
|----------------------------|---------------------------------------------------------------------------------------------------|
| Frontmatter `last_updated` | Auto-updated by Claudine on each successful write                                                 |
| Frontmatter `hash`         | Auto-stamped Darkmatter `Simple` content hash on each successful write                            |
| Provider output            | Must be replacement body content only (no frontmatter); guardrails instruct the agent accordingly |
| `--silent`                 | Suppresses check/validation messages                                                              |

---

### `claudine sequence` — Full Execution Flow

Sequence composition runs a single source document multiple times, once per step in a defined list, with step-specific state injected into the composition context on each run.

```sh
claudine sequence @deploy.md
claudine sequence --fail-fast false @batch.md
```

#### Step 3: Sequence Plan Resolution (sequence — unique)

`composition::resolve_sequence_plan(&source)` parses the sequence definition:

1. Reads `sequence` frontmatter key
2. Parses `fail_fast` frontmatter (defaults to `true`)
3. If `sequence` is an array: normalizes via `normalize_inline_list()` → `SequenceStep` array (scalar strings or objects with required `name`)
4. If `sequence` is a string: resolves as external YAML file reference via `resolve_sequence_reference()`:

    - Supports `@`, `!`, `vault:`, `%`, `{{ENV}}`, `~`, absolute, and relative paths
    - External file form 1: `{ sequence: [...] }`
    - External file form 2: `{ kind: "sequence", list: [...], template?: {...} }` with `{{key}}` template rendering

**Returns:** `SequencePlan { source, steps, document_fail_fast }`

**Affected by:**

| Input                   | Impact                                                        |
|-------------------------|---------------------------------------------------------------|
| Frontmatter `sequence`  | Required; array of steps or string path to external YAML file |
| Frontmatter `fail_fast` | Document-level default; `true` when absent                    |
| `--fail-fast`           | CLI override for document default                             |

#### Step 4: Execute Sequence

`execute_sequence()` runs the full orchestration in three phases:

##### Phase 1a: Build Overlays and Prepare Each Step

For each step (0..total_steps):

1. **Build step overlay** via `build_step_overlay(&plan, step_index)` — computes `state`, `previous_state`, `next_state`, `is_first`, `is_last`, `step`, `total_steps`
2. **Build set overrides**: overlay keys (reserved per-step variables) merged with user overrides; reserved keys always win
3. **Inject `FAIL_FAST` env var** into env overrides
4. **Template pre-flight**: `resolve_shell_approvals()` for `::shell` directives using step-specific overrides
5. **Prepare composition**: `composition::prepare_direct(source, prepare_options)` with step-specific overrides and cumulative approved commands
6. **Harness pre-flight** (if applicable): parse harness plan and resolve shell approvals for harness commands
7. **Accumulate approved commands** in `cumulative_approved` set (shared across steps)
8. Store in `StepContext { env_overrides, prepared }`

**Affected by:**

| Input                                          | Impact                                                                           |
|------------------------------------------------|----------------------------------------------------------------------------------|
| Step overlay variables (`state`, `step`, etc.) | Injected as set overrides; cannot be overridden by `--set` or `key=value`        |
| `FAIL_FAST` env var                            | Injected per step so `{{env.FAIL_FAST}}` and `::shell` directives see the policy |
| `--set` / `key=value` setters                  | Override template variables but lose to reserved overlay keys                    |

##### Phase 1a.5: Aggregate Schema Validation Across Steps

Every step is validated against its `$schema` during Phase 1a, before any
provider session is launched. When multiple steps share the same missing
required property with the same shape and description, the interactive prompt
fires once and the answer is reused for the remaining steps (unless a step
overlay supplies a different value). When Interactive Mode is denied or one
or more steps have invalid required values, claudine returns a single
aggregated `CompositionError::SequenceMissingProperties` listing every
failing step so the user can fix the entire sequence in one edit pass.

##### Phase 1b: Resolve Provider/Model for Every Step

1. Detects installed providers and builds snapshot
2. Loads selection config and model catalog
3. For each step: resolves provider (explicit flag or non-TTY chain), resolves model
4. Builds `SequenceStepDraft` for each step with provider plan and model info

**Phase 1c: Review or validate:**

- **If failures exist**: returns `SequenceSelectionFailed` aggregate error
- **TTY + no explicit provider**: shows review screen via `biscuit-tui::InputTable` where user can edit per-step provider and model; `Ctrl+S` confirms, `Esc` aborts
- **Non-TTY**: converts drafts directly to `ResolvedExecutionTarget` array

**Affected by:**

| Input                          | Impact                                                     |
|--------------------------------|------------------------------------------------------------|
| `--provider` / `--claude` etc. | Locks the provider cell for every step in the review table |
| `--model`                      | Locks the model cell for every step in the review table    |
| Per-step frontmatter `agent`   | Influences default provider for that step                  |
| Per-step frontmatter `model`   | Influences default model for that step                     |

##### Phase 1d: Shell Pre-flight Completion

Emits preflight-complete status for all steps. Shell approvals were already collected during Phase 1a.

##### Phase 2: Execute Each Step

For each step in `step_contexts`:

1. **Check interrupt flag** — if SIGINT observed, break
2. **Print step start status**: `[1/N] starting step-name`
3. Clone prepared composition from step context
4. Get pre-resolved target from Phase 1b
5. Build `CompositionExecutionRequest` with:

    - `mode: ChainedDocument`
    - `resolved_target`: pre-resolved (avoids re-resolution)
    - `sequence: true` (suppresses per-step preflight messaging)
    - `env_overrides`: includes `FAIL_FAST`
    - `shared_approval_cache`: shared across all steps

6. Execute step via `execute_composition_request_inner()` — the same full pipeline as compose Steps 6a–6l
7. Handle result:

    - **Success (exit 0)**: increment succeeded, record `SequenceStepResult`
    - **Interrupted (exit 130)**: record failure, set interrupt flag, break
    - **Failure (non-zero)**: increment failed, record result; if `effective_fail_fast`, break
    - **Error from executor**: record failure; if `effective_fail_fast`, break

##### Final Summary

1. Prints "Sequence finished: X succeeded, Y failed"
2. Emits perf report if `--perf` (aggregated across all steps)
3. Returns exit code: 0 if all succeeded, 130 if interrupted, 1 if any failed

**Affected by:**

| Input                   | Impact                                                   |
|-------------------------|----------------------------------------------------------|
| `--fail-fast`           | CLI override; stops on first failure when true (default) |
| Frontmatter `fail_fast` | Document default when `--fail-fast` not specified        |
| SIGINT                  | Tracked across all phases; aborts between steps          |
| `--perf`                | Aggregated report covering all steps                     |
| `--silent`              | Suppresses per-step status messages                      |

### Per-step Template Variables

Each step runs the source document through Darkmatter with these reserved variables injected as overrides:

| Variable         | Type                    | Description                                           |
|------------------|-------------------------|-------------------------------------------------------|
| `state`          | string or object        | The current step value                                |
| `previous_state` | string, object, or null | The previous step's value, or null for the first step |
| `next_state`     | string, object, or null | The next step's value, or null for the last step      |
| `is_first`       | boolean                 | `true` when this is the first step                    |
| `is_last`        | boolean                 | `true` when this is the last step                     |
| `step`           | integer                 | One-based index of the current step                   |
| `total_steps`    | integer                 | Total number of steps                                 |

## Provider Selection

Provider selection behaves differently in **TTY** (interactive terminal) and **non-TTY** (piped or CI) modes. In both modes, explicit `--<provider>` flags always win unconditionally.

### TTY Mode

When stdout is a terminal and no explicit `--<provider>` flag is given:

1. **Interactive picker** — a `biscuit-tui` one-shot picker shows all installed providers. Frontmatter `agent` and config `favorite_agent` only influence the **default index** and **row ordering**; they do not bypass the picker.

### Non-TTY Mode

When stdout is not a terminal (e.g., CI, scripts), resolution follows a strict chain with no interactive fallback:

1. **Explicit flag** (`--provider <slug>`, or the shorthand booleans `--claude`, `--codex`, `--gemini`, `--opencode`, `--qwen`, `--goose`, `--kimi`) — highest priority
2. **Singular frontmatter `agent`** — a single provider name in the effective (composed) frontmatter, fuzzy-matched against known providers
3. **List-valued frontmatter `agent`** — an ordered list of provider names; the first installed provider in the list is chosen
4. **Config favorite** — `favorite_agent` from `~/.claudine/config.json`
5. **Hard error** — if none of the above resolve, the command fails with a structured error

The old "single installed" auto-selection shortcut has been removed. Even when only one provider is installed, non-TTY sessions still require an explicit signal (flag, frontmatter, or favorite).

### Frontmatter `agent` and `model`

Both `agent` and `model` frontmatter properties accept either a single string or an ordered list of strings:

```yaml
agent: codex
agent: [gemini, codex, claude]
model: gpt-4o
model: [gpt-4o, o3-mini]
```

List-valued `agent` is treated as author preference order: the first installed provider wins. List-valued `model` is validated against the provider's model catalog; the first valid entry wins. When a catalog is unavailable (e.g., Gemini, Kimi, Goose in v1), frontmatter `model` is gracefully skipped rather than treated as an error.

### Model Resolution

Model selection follows a single chain independent of TTY mode:

1. **CLI `--model`**
2. **Provider-specific env var** (`CODEX_MODEL`, `CLAUDE_MODEL`, `OPENCODE_MODEL`, etc.)
3. **Generic `MODEL` env var**
4. **Frontmatter `model`** (validated against catalog when available)
5. **Provider default** (`None` — let the provider choose)

### OpenCode Non-TTY Requirement

OpenCode requires a model in non-interactive mode. If no model survives the resolution chain when running OpenCode in non-TTY mode, Claudine emits a hard error before launching the provider:

```text
OpenCode requires a model in non-interactive mode; set --model, OPENCODE_MODEL, or MODEL
```

### Shorthand Flags

The shorthand booleans and the `--provider` value both accept fuzzy input (`cl` → `claude`, `gem` → `gemini`, `oc` → `opencode`). The [argv normalizer](argv-normalization.md) rewrites every shorthand into a canonical `--provider <slug>` pair before clap runs, so runtime provider selection only ever reads the single `--provider` field.

### The `--interactive` and `--no-interactive` Flags

`-i` / `--interactive` and `--no-interactive` control the **provider session mode**, not provider selection. The composed prompt is still prepared first, then passed as the initial message for the session. The resolved mode follows a fixed precedence: `--no-interactive` > `-i` / `--interactive` > `interactive` frontmatter property > default (non-interactive). The two flags are mutually exclusive. `compose` and `inline-compose` honor the `interactive` frontmatter property; `sequence` rejects `interactive: true`. See [Composition](composition.md#the---interactive-and---no-interactive-flags) for the full contract.

> **Note:** `inline-compose -i` is provider-gated. Claudine allows it only when the selected provider can recover the final assistant message for the inline rewrite path. When the interactive intent comes from `interactive: true` frontmatter, the diagnostic names `frontmatter` as the source.

### The `--exclude` Flag

`--exclude <PROVIDER>` removes a provider from automatic selection (repeatable). Explicit flags (`--codex`, etc.) override exclusions.

## Migrating from the Retired Harness DSL

Earlier Claudine releases let composed documents declare `pre_checks`, `post_checks`, `handle_*` handlers, a programmatic `handle`, and `deviate` recovery commands in their frontmatter. That validation-and-handler DSL has been **removed**; its gating, verification, and recovery roles are now expressed through the [lifecycle stack](lifecycle.md). A document that still declares any of these keys fails composition with a typed `RemovedValidationKey` diagnostic naming the offending key and its replacement surface:

| Removed key | Replacement |
|-------------|-------------|
| `pre_checks` | the `initialize` or `start` lifecycle stack |
| `post_checks` | the `success` or `finalize` lifecycle stack |
| `handle_<event>` (e.g. `handle_timeout`, `handle_inline_body_unchanged`) | the `blocked` or `failure` lifecycle recovery actions |
| `handle` | a lifecycle `shell` action or other lifecycle action |
| `deviate` | a lifecycle `shell` action plus a recovery action (`retry`, `resume`, etc.) |

See [Composition — Migrating from the Retired Harness DSL](composition.md#migrating-from-the-retired-harness-dsl) for the full guidance and [lifecycle.md](lifecycle.md) for the replacement action catalog.

### Timeouts

Claudine supports two independent timeout properties that share the same
human-readable duration syntax (`s`/`sec`/`seconds`, `m`/`min`/`minutes`,
`h`/`hr`/`hours`):

```yaml
timeout: 5m
step_timeout: 30s
```

- **`timeout`** is a wall-clock deadline for the entire provider run. When the
    elapsed time since launch exceeds this budget, Claudine sends SIGTERM (then
    SIGKILL after a short grace period).

- **`step_timeout`** is a silence deadline. The timer resets on every
    `SemanticEvent` observed on the provider's structured stream (tool call,
    tool result, reasoning chunk, assistant text, info/warning, etc.). If no
    event is observed for longer than the budget, Claudine kills the child.

Both timeouts surface as the same timeout failure and route to the
`failure` lifecycle event, where a `Retry` or `Resume` action can recover.

**Streaming-only.** `step_timeout` requires structured streaming. If the
selected provider runs in capture or passthrough mode, Claudine emits a
warning and ignores the field. The non-streaming Goose wrapper is the
primary example.

**Relational validation.** When both properties are present,
`step_timeout` must be less than or equal to `timeout`. Documents that
violate this invariant fail parse-time validation with
`HarnessError::InvalidTimeout`.

**Precedence.** The CLI flags `--timeout` and `--step-timeout` override
frontmatter when provided. Wall-clock enforcement is checked before
step-silence on each poll, so if both budgets expire in the same tick the
run is reported as the wall-clock timeout.

```yaml
timeout: 10m          # hard ceiling on total runtime
step_timeout: 45s     # kill the child if it goes silent for 45s
```

## Timing Surface

Every composition run (whether harness-enabled or not) shares a single
user-visible timing surface rendered to stderr. Two emitters drive it:

1. **Periodic prompt-scoped header.** Emitted at `t=0` when the prompt
   begins and then at monotonic offsets from `t=0` (`t=10m`, `t=20m`,
   `t=30m`, …). Ticks are anchored on the prompt's start time, not on
   wall-clock `:00 :10 :20` boundaries.

    - The `t=0` header reads `⏱️ {HH:MM} {TZ} running the {prompt} prompt`
     (no duration segment).

    - Subsequent ticks read `⏱️ {HH:MM} {TZ} running the {prompt} prompt for {duration}`.
    - `{prompt}` is an OSC8 link whose visible text is the path relative
     to the repo root (falling back to CWD, `$HOME`, then absolute).

2. **Fire-once warnings** (harness frontmatter only):

    - **`timeout_warn`** — prompt-scoped. Fires once when the prompt has
     been running for this long. Message variants:

        - *With `timeout` also set:* `the {prompt} has been running for {elapsed}, this is longer than we'd expect it to take but we won't timeout this prompt until we reach {HH:MM} in {remaining}.`
        - *Without `timeout`:* `the {prompt} has been running for {elapsed}, this is longer than we'd expect it to take. Press CTRL+C to terminate this prompt if you're convinced that the prompt has hung.`

    - **`step_timeout_warn`** — step-scoped, fires once per stall episode
     using the same silence clock as `step_timeout`. Message variants:

        - *With `step_timeout` also set:* `the {prompt} has not produced output for {silence}, this is longer than we'd expect, but we won't abort this step until we reach {HH:MM} in {remaining}.`
        - *Without `step_timeout`:* `the {prompt} has not produced output for {silence}, this is longer than we'd expect. Press CTRL+C to terminate this prompt if you're convinced that the prompt has hung.`

```yaml
timeout: 10m
timeout_warn: 5m         # warn at 5m; still kill at 10m
step_timeout: 45s
step_timeout_warn: 20s   # warn at 20s silence; still kill at 45s silence
```

**Duration rendering.** All user-visible durations use a single format:
`{N}s` under 60 seconds (e.g. `45s`), `{N}m` between 1 and 59 minutes
(e.g. `12m`), `{H}h {M}m` at 60 minutes or more (e.g. `1h 30m`, `2h 5m`).
No zero-padding, no colon-separated forms.

**Preflight validation.** Each `*_warn` must be strictly less than its
corresponding hard threshold when both are present. `timeout_warn >= timeout` or `step_timeout_warn >= step_timeout` is rejected at parse
time with `HarnessError::InvalidTimeout`. `*_warn` values `<= 0` are
also rejected (the underlying duration parser requires positive values).
A warn set without its corresponding hard threshold is legal — the
"without hard threshold" message variant applies.

**Fire-once semantics.** `timeout_warn` fires at most once per prompt
run. `step_timeout_warn` fires at most once per stall episode; once
activity resumes it re-arms for the next stall. Neither emission blocks
the provider or affects hard-timeout behavior.

### Recovery

Recovery is expressed through the lifecycle stacks — `failure` and `blocked` are its natural homes, but flow control is universal and every event may recover. The available lifecycle recovery actions are `retry`, `resume`, `proxy`, and `defer` (`defer` is parse-valid but not yet implemented). See [lifecycle.md](lifecycle.md) for the full reference and the [migration table](#migrating-from-the-retired-harness-dsl) for the mapping from the removed `handle_*` keys.

### Shell Policy

All shell commands — `::shell` directives in the template, top-level frontmatter `$(cmd)` expressions, and lifecycle `shell` stack actions — are approved upfront during the pre-flight phase, before the provider session starts. See [Pre-Flight Shell Approval](pre-flight-checks.md) for the full flow.

## Retired Interfaces

The following interfaces have been removed and replaced by the two canonical commands above:

| Removed                                        | Replacement                                              |
|------------------------------------------------|----------------------------------------------------------|
| `claudine <agent> --compose <file>`            | `claudine compose --<agent> <file>`                      |
| `claudine <agent> --frontmatter-prompt <file>` | `claudine inline-compose --<agent> <file>`               |
| `claudine compose inline <file>`               | `claudine inline-compose <file>`                         |
| `claudine compose-inline <file>`               | `claudine inline-compose <file>`                         |
| `AGENT` environment variable                   | `--claude`, `--codex`, etc. flags or `agent` frontmatter |

**Removed without replacement:**

| Removed                                 | Reason                                                                                                                                                                                                                                                                   |
|-----------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `claudine <agent> --prompt-file <file>` | Sent file content verbatim as a prompt. `claudine compose` performs full Markdown composition (frontmatter, template substitution, `::shell` directives) so it is not a drop-in replacement. Callers that need raw prompt delivery should use the provider CLI directly. |

## Architecture

All three commands follow the same six-stage pipeline:

```text
Resolve → Pre-Flight → Prepare → Select Provider → Launch → Closure
```

- **Resolve**: `composition::resolve_composition_source()` loads the Markdown file
- **Pre-Flight**: `composition::resolve_shell_approvals()` discovers every shell command in the document graph — template `::shell` directives, top-level frontmatter `$(...)` expressions, and lifecycle `shell` stack actions — checks whitelists, and prompts the user to approve any unapproved commands before proceeding (see [Pre-Flight Shell Approval](pre-flight-checks.md))
- **Prepare**: `composition::prepare_direct()` or `composition::prepare_inline()` composes through Darkmatter with the pre-approved command set and produces a `PreparedComposition` with `effective_frontmatter`
- **Select**: Provider selection applies the precedence chain described in [Provider Selection](#provider-selection)
- **Launch**: `wrap::composition::execute_composition_request()` runs the provider through the full wrapper pipeline (env, MCP, harness, streaming)
- **Closure**: `composition::closure::rewrite_inline_document()` reconstructs the document for inline mode; direct mode outputs to stdout; sequence iterates the pipeline per step

## Performance Reporting

Composition commands support an opt-in `--perf` flag that prints a detailed performance breakdown to stderr after execution completes. The report includes:

- **CLI Overhead** — arg parsing, config loading, tracing init, and environment setup.
- **Composition Report** — when `compose` or `inline-compose` (or each step of a `sequence`) triggers document preparation, the Darkmatter composition timings are shown (total time plus per-stage breakdown: interpolation, shell expansion, transclusion apply, etc.).
- **Agent Execution** — launches, first-response latency, total execution time, and provider-reported API duration when available.

For `sequence`, the report is aggregated across all steps: launches and total execution time are summed, first-response latencies are averaged (with the minimum shown in a note), and composition metrics are merged. The report appears exactly once at the end of the run, after the sequence summary.

`--perf` is emitted unconditionally when passed, even alongside `--silent` or `--quiet`, because it is an explicit opt-in.

> **Note:** `provider_api_duration` is only populated for structured-streaming providers. Legacy providers (e.g., Goose) omit this line.

## Future Improvements

### 1. Shared Preparation Cache for Sequence

Sequence Phase 1a calls `prepare_direct()` for every step, which means Darkmatter's full composition pipeline (transclusion, interpolation, shell expansion) runs N times during pre-flight even though the source document is the same. A hash-keyed cache of the base composition (keyed on source file + overrides) could avoid redundant work when steps differ only in overlay variables that are injected late in the pipeline.

### 2. Parallel Sequence Execution

The sequence orchestrator runs steps strictly serially. For workloads where steps are independent (no `previous_state`/`next_state` dependency), an opt-in `parallel: N` frontmatter property could execute steps concurrently with a configurable concurrency limit, dramatically reducing wall-clock time for large batches.

### 3. Lazy Shell Approval for Sequence

Shell approval currently runs during Phase 1a for every step, which means the user must approve all commands before any step starts. A lazy approval model that prompts only when a step is about to run (and caches approvals across steps) would reduce upfront latency for long sequences with diverse shell commands.

### 4. Inline Closure Diff Preview

The inline closure pipeline writes the replacement body atomically with no preview. Adding an optional `--diff` flag that opens a diff view (terminal or editor) before write-back would give users a chance to review and approve the agent's changes, reducing the risk of destructive overwrites from poorly-behaved agent output.

### 5. Compose Result Caching

When the same file is composed with the same set of overrides multiple times (e.g., during development iterations), a content-addressed cache of composed prompts could skip the Darkmatter pipeline entirely. A `--no-cache` flag would force recomposition.

### 6. Sequence Step-Level Harness Overrides

Harness properties currently apply uniformly across all sequence steps. Allowing per-step harness overrides (e.g., different timeouts or post-checks for specific steps) would enable more fine-grained control without requiring separate template files.

### 7. Streaming Provider Support for Inline Closure

The legacy (non-structured) capture path for inline-compose reads the entire provider response into memory before writing. For large documents, a streaming closure that incrementally writes the body as it arrives (with rollback on failure) would reduce peak memory and improve perceived latency.

### 8. Structured Error Recovery for Sequence

When a sequence step fails in non-fail-fast mode, the only information preserved is the exit code and error message. Capturing structured failure metadata (which harness check failed, what the agent's partial response was, etc.) would enable more intelligent retry or skip decisions in downstream tooling.

### 9. Frontmatter Schema Validation During Prepare

Agent and model hints are validated during preparation (unknown providers error immediately), but other frontmatter properties (lifecycle config, harness checks, sequence definitions) are parsed without schema validation beyond basic type checks. A schema-driven validation pass during preparation would surface configuration errors earlier and with better error messages.

### 10. Unified Telemetry Span Across All Three Commands

Each command creates its own tracing span hierarchy, making it difficult to correlate telemetry across compose, inline-compose, and sequence runs in the same session. A shared span convention with a correlation ID would improve observability in logging and tracing backends.