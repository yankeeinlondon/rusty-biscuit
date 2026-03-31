# System Prompt Refactor Tech Design

This document turns the system-prompt refactor spec into an implementation-ready design for Claudine's CLI, library, and wrapper runtime.

Primary inputs:

- `claudine/features/2026-03-30-refactor-system-prompt/spec.md`
- `claudine/docs/research/system-prompt/*.md`
- `claudine/docs/research/agent-cli/*.md`
- current system-prompt runtime in `claudine/cli/src/commands/wrap/mod.rs`
- current wrapper profile contract in `claudine/cli/src/commands/wrap/profile.rs`
- current composition request model in `claudine/lib/src/composition/types.rs`
- current system prompt capability docs in `claudine/docs/topics/system-prompt.md`

The core design decision is:

1. make system prompts file-based and explicit
2. treat `system-prompt.md` as a standard append-mode document discovered from the launch CWD hierarchy
3. run every selected system prompt file through Darkmatter composition before provider delivery
4. keep the canonical rendered output as Markdown, with no automatic XML wrapping in v1
5. add a richer provider application layer because the current `apply_system_prompt(&mut Vec<String>, &str)` API is too small for temp files, env vars, and session-scoped config overlays

## Summary

The refactor has five major parts:

1. replace the old universal `--system-prompt <PROMPT|FILE>` switch with two file-only switches
2. add automatic `system-prompt.md` discovery from package, package-area, repo, and user scopes
3. compose the chosen file through Darkmatter before provider launch
4. change wrapper system-prompt application from an args-only hook into a full launch-plan mutation step
5. implement provider-specific append and replace strategies without mutating the user's real project files or home directory

The result is:

- one standard prompt filename
- one deterministic search hierarchy
- one canonical Markdown output format
- explicit append versus replace behavior
- provider-specific runtime delivery that can use args, env, temp files, or session-scoped overlay homes as needed

## Goals

1. Match the spec's CLI and resolution behavior exactly.
2. Make standard `system-prompt.md` work without any CLI flag.
3. Keep prompt composition in library code and provider delivery in wrapper code.
4. Avoid mutating real repo files or real provider config in order to inject session-scoped prompt state.
5. Preserve room for provider-specific advanced formats later without locking v1 into XML.

## Non-Goals

1. Replacing provider-native memory files such as `AGENTS.md`, `GEMINI.md`, or `QWEN.md` as a product concept.
2. Redesigning Darkmatter composition semantics.
3. Making Roo Code wrappable in this refactor.
4. Defining a provider-specific prompt templating DSL beyond authored Markdown.
5. Guaranteeing that "replace" removes every other provider-owned context source; some providers still layer their own project memory after replacement.

## User-Facing Behavior

### CLI Contract

Remove the old switch:

- `-s, --system-prompt <PROMPT|FILE>`

Add two new file-only switches on wrapper and composition entrypoints:

- `--append-system-prompt <FILE>`
- `--replace-system-prompt <FILE>`

Add visible long aliases:

- `--asp` for `--append-system-prompt`
- `--rsp` for `--replace-system-prompt`

Clap rules:

- the two switches conflict with each other
- neither switch accepts inline text
- both accept file references resolved through the same file-resolution rules used elsewhere in Claudine

### Standard File

When neither CLI switch is present, Claudine attempts to resolve a standard file named `system-prompt.md`.

Resolution is based on the directory the user launched Claudine from, not the composition source file location.

Search order:

1. package root
2. package-area root
3. repo root
4. `~/.claudine/system-prompt.md`

If Claudine is not in a detected repo/monorepo, the effective local search collapses to:

1. current working directory
2. `~/.claudine/system-prompt.md`

If a CLI switch is provided, Claudine skips standard-file discovery entirely.

### Default Mode for the Standard File

The standard `system-prompt.md` is always treated as append-mode.

Rationale:

- the standard file is the equivalent of project guidance, not a provider constitution override
- most providers' native persistent memory systems are supplement-oriented
- full replacement is a high-risk escape hatch and should stay explicit

### Empty File Semantics

After Darkmatter composition, if the resulting body is empty or whitespace-only:

- Claudine treats that as an explicit "no Claudine-managed system prompt" decision
- no lower-priority `system-prompt.md` locations are consulted
- no provider-specific append or replace action is performed

This means an empty `system-prompt.md` disables Claudine's prompt injection for that scope. It does not promise removal of the provider's built-in prompt.

## Output Format Decision

The default output format is Markdown as-authored.

Claudine will not automatically wrap the rendered prompt in XML in v1.

Rationale:

1. Markdown is the only cross-provider common denominator across append-mode memory files and most replacement mechanisms.
2. Gemini and Kimi replacement paths rely on provider-native Markdown templates and variable expansion. Automatic XML wrapping risks breaking those templates.
3. Qwen is explicitly Markdown-first.
4. XML can still be authored manually inside `system-prompt.md` when a user wants stronger segmentation for a specific provider.
5. Automatic XML wrapping would make prompt shape implicit and harder to reason about during debugging.

Future extension point:

- a prompt-rendering enum may be added later if Claudine wants provider-specific wrapping strategies
- that is deliberately deferred from this refactor

## Current Baseline

Today the runtime is still centered on one field:

- `WrapperArgs.system_prompt: Option<String>`
- `ComposeArgs.system_prompt: Option<String>`
- `CompositionExecutionRequest.system_prompt: Option<String>`

The old flow is:

1. `resolve_system_prompt()` returns either file contents or inline text
2. `WrapperProfile::apply_system_prompt(&mut Vec<String>, &str)` mutates child args
3. only Claude overrides that method

That design is no longer sufficient because the new feature needs:

- file-only explicit switches
- auto discovery
- Darkmatter composition
- session-scoped home/config overlays
- env-var based replacement
- temp prompt files whose lifetimes must survive until child exit

## Target Architecture

The target architecture is a four-stage pipeline shared by wrappers and composition execution:

1. discover launch context
2. resolve effective system prompt source
3. compose the selected file through Darkmatter
4. apply provider-specific runtime delivery

### Recommended Module Layout

Library:

```txt
claudine/lib/src/system_prompt/
├── mod.rs
├── context.rs
├── resolve.rs
├── prepare.rs
└── types.rs
```

Wrapper runtime:

```txt
claudine/cli/src/commands/wrap/
├── mod.rs
├── composition.rs
├── profile.rs
├── env.rs
├── repo_home.rs
└── system_prompt.rs
```

Responsibilities:

- `system_prompt/context.rs`
  - detect repo root, package-area root, and package root from the launch CWD
  - reuse or extract the monorepo logic currently embedded in `wrap/env.rs`
- `system_prompt/resolve.rs`
  - resolve explicit file arguments
  - find standard `system-prompt.md`
  - return `None`, `Disabled`, or a selected source
- `system_prompt/prepare.rs`
  - run Darkmatter composition on the selected file
  - return composed Markdown body only
- `wrap/system_prompt.rs`
  - own temp-file, temp-home, and inline-config application
  - merge provider-specific prompt changes into the launch plan

## Core Data Model

```rust
pub enum SystemPromptMode {
    Append,
    Replace,
}

pub struct SystemPromptArgs {
    pub append_file: Option<String>,
    pub replace_file: Option<String>,
}

pub enum EffectiveSystemPrompt {
    None,
    Disabled {
        source: SystemPromptSource,
    },
    Ready(PreparedSystemPrompt),
}

pub enum SystemPromptSource {
    StandardDiscovered {
        path: PathBuf,
        scope: StandardPromptScope,
    },
    ExplicitFile {
        path: PathBuf,
        mode: SystemPromptMode,
    },
}

pub enum StandardPromptScope {
    Package,
    PackageArea,
    Repo,
    User,
    CurrentDirectory,
}

pub struct PreparedSystemPrompt {
    pub mode: SystemPromptMode,
    pub source: SystemPromptSource,
    pub raw_text: String,
    pub composed_markdown: String,
}
```

Launch-context helper:

```rust
pub struct LaunchContext {
    pub cwd: PathBuf,
    pub repo_root: Option<PathBuf>,
    pub package_area_root: Option<PathBuf>,
    pub package_root: Option<PathBuf>,
}
```

Wrapper-side application output:

```rust
pub struct SystemPromptApplication {
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub artifacts: Vec<SystemPromptArtifact>,
    pub warnings: Vec<String>,
}

pub enum SystemPromptArtifact {
    TempFile(tempfile::NamedTempFile),
    TempDir(tempfile::TempDir),
}
```

The important change is that system prompt application is no longer "push args only". It becomes a full launch-plan mutation step.

## Discovery and Preparation Algorithm

### 1. Launch Context

Extract the monorepo/package-root detection logic from `wrap/env.rs` into shared library code so system-prompt resolution and env planning can both use the same answer.

Expected behavior:

- package root comes from the deepest matching workspace package
- package-area root comes from repo root plus `package_area`, with `root` mapping to repo root
- repo root comes from git detection
- duplicate paths are deduplicated before search

### 2. Effective Source Selection

Algorithm:

1. if `--append-system-prompt` is set:
   - resolve only that file
   - mark mode `Append`
   - skip standard discovery
2. else if `--replace-system-prompt` is set:
   - resolve only that file
   - mark mode `Replace`
   - skip standard discovery
3. else:
   - search standard locations in precedence order
   - select the first file that exists
   - assign mode `Append`
4. if no file exists:
   - return `EffectiveSystemPrompt::None`

### 3. Composition

Compose the selected prompt file using Darkmatter with `with_source_file(selected_path)` so transclusion, shell directives, and relative imports behave relative to the prompt file itself.

Only the composed body is sent to providers.

Frontmatter is not forwarded in v1, but it remains available for future metadata if needed.

### 4. Empty-Body Handling

If `composed_markdown.trim().is_empty()`:

- return `EffectiveSystemPrompt::Disabled`
- do not consult lower-priority standard files
- do not synthesize placeholder prompt text

## Wrapper Integration

### Shared CLI Changes

Update:

- `claudine/cli/src/commands/wrap/mod.rs`
- `claudine/cli/src/commands/compose.rs`
- `claudine/lib/src/composition/types.rs`

Replace the old single field with:

```rust
pub append_system_prompt: Option<String>,
pub replace_system_prompt: Option<String>,
```

### Execution Order

The wrapper and composition launch paths should do this in order:

1. determine launch context from CWD
2. resolve and prepare the effective system prompt
3. build the base env plan
4. ask the provider profile to apply the effective system prompt
5. merge returned args/env/artifacts into the child launch

Artifacts must stay alive until the child process exits.

## Provider Runtime Support Matrix

This design separates native provider capability from Claudine runtime support.

### Phase 1 Runtime Support

| Provider | Append | Replace | Planned runtime strategy |
|----------|--------|---------|--------------------------|
| Claude | Yes | Yes | native CLI flags |
| Codex | Yes | Yes | append via session home overlay, replace via temp file + config override |
| Gemini | Yes | Yes | append via session home overlay, replace via `GEMINI_SYSTEM_MD` |
| Goose | Yes | No | append via `goose run --system`, replace warns and skips |
| Kimi | No | Yes | replace via generated `--agent-file`; append deferred |
| OpenCode | Yes | Yes | append via `OPENCODE_CONFIG_CONTENT.instructions`, replace via `--system` |
| Qwen | Yes | No | append via session home overlay, replace warns and skips |
| Roo | No | No | no wrapper runtime support |

### Provider-Specific Design

#### Claude

Append:

- interactive: `--append-system-prompt <text>`
- non-interactive: prefer `--append-system-prompt-file <tempfile>` to avoid argv length limits

Replace:

- interactive: `--system-prompt <text>`
- non-interactive: prefer `--system-prompt-file <tempfile>`

#### Codex

Append:

- create an ephemeral session home
- materialize `~/.codex/AGENTS.override.md`
- if an existing override file is present in the copied home, preserve it and append the Claudine content after it

Replace:

- write the composed Markdown to a temp file
- pass `-c model_instructions_file=/absolute/path/to/file`

Reason not to use `developer_instructions` for append:

- it is inline-only
- it is more vulnerable to argv size limits
- `AGENTS.override.md` better matches Codex's native memory hierarchy

#### Gemini

Append:

- create an ephemeral session home
- materialize `~/.gemini/GEMINI.md`
- preserve copied user content first, then append Claudine content

Replace:

- write the composed Markdown to a temp file
- set `GEMINI_SYSTEM_MD=/absolute/path/to/file`

#### Goose

Append:

- pass `goose run --system <text>`

Replace:

- unsupported
- emit a warning and continue without applying a Claudine system prompt

#### Kimi

Replace:

- generate a temp Markdown prompt file
- generate a temp YAML agent spec
- launch with `--agent-file <temp-agent.yaml>`
- use `extend: default` so tools and built-in behavior inherit from the default agent

Append:

- deferred from phase 1
- Kimi's native supplement path is `AGENTS.md`, but session-scoped append without mutating the real worktree is not a faithful fit today

This is the one provider where Claudine should choose correctness over fake parity.

#### OpenCode

Append:

- write the composed Markdown to a temp file
- set `OPENCODE_CONFIG_CONTENT` to an inline JSON override that appends that file path to the `instructions` array
- rely on OpenCode's documented config-merge behavior

Replace:

- prefer `--system <tempfile-path>`

This avoids mutating project `AGENTS.md` while still using a documented project-instruction mechanism for append.

#### Qwen

Append:

- create an ephemeral session home
- materialize `~/.qwen/QWEN.md`
- preserve copied user content first, then append Claudine content

Replace:

- unsupported
- emit a warning and continue without applying a Claudine system prompt

#### Roo

Roo remains out of wrapper scope in this refactor. Capability metadata can keep describing Roo's native files, but the runtime should not promise support until Claudine has a real Roo launch path.

## Ephemeral Overlay Homes

Codex, Gemini, and Qwen append-mode support all need session-scoped home files.

Do not write these prompt overlays into:

- the user's real home directory
- Claudine's persistent repo shadow-home cache

Instead, add an ephemeral overlay-home concept:

1. choose the base home source
   - real home by default
   - repo shadow home when `--repo` or other wrapper logic already requires it
2. create a session `TempDir`
3. copy only the minimal provider-owned files/directories needed for startup
4. write the Claudine overlay file into that temp home
5. point `HOME` at the temp home for the child process

This keeps prompt injection session-scoped and cleanup automatic.

## Provider Profile Contract Change

Replace the current narrow hook:

```rust
fn apply_system_prompt(&self, args: &mut Vec<String>, prompt: &str) -> Option<String>
```

With something closer to:

```rust
fn apply_system_prompt(
    &self,
    prompt: &PreparedSystemPrompt,
    session_interactive: bool,
    cwd: &Path,
    base_env_plan: &EnvPlan,
) -> Result<SystemPromptApplication>;
```

Why this change is necessary:

- some providers need env vars
- some need temp files
- some need temp homes
- some need different behavior in interactive versus non-interactive mode
- some need to return warnings without failing the launch

## Logging and Dry-Run

Dry-run and verbose output should surface:

- whether a system prompt was selected
- whether it came from standard discovery or explicit CLI flags
- which file won the search
- whether the final mode is append or replace
- whether the composed body resolved to Disabled
- provider-specific fallback warnings

This matters because automatic discovery makes prompt behavior less visible unless Claudine reports it clearly.

## Documentation Updates

Update these docs alongside implementation:

- `claudine/docs/topics/system-prompt.md`
- `claudine/cli/README.md`
- `claudine/lib/README.md`
- any skill docs that still describe the old `--system-prompt` switch

Important doc changes:

- old inline-string behavior is gone
- standard-file discovery is now first-class
- provider support matrix must distinguish append and replace
- Roo remains metadata-only for now

## Testing

### Unit Tests

Add unit coverage for:

- launch-context root detection
- standard discovery precedence
- deduping overlapping package/package-area/repo roots
- explicit flags bypassing discovery
- empty composed file producing `Disabled`
- Darkmatter transclusion relative to the selected prompt file

### Wrapper Tests

Add provider-specific tests for:

- Claude append versus replace in interactive and non-interactive modes
- Codex append creating `AGENTS.override.md` in an ephemeral home
- Codex replace emitting `model_instructions_file`
- Gemini replace setting `GEMINI_SYSTEM_MD`
- Goose replace warning path
- OpenCode append setting `OPENCODE_CONFIG_CONTENT`
- Qwen append creating `QWEN.md` in an ephemeral home
- Kimi replace generating the temp agent spec

### Integration Tests

Run:

- `just test -p claudine`
- `just test -p claudine-cli`

Recommended new integration fixtures:

- monorepo cwd inside package
- monorepo cwd inside package area but outside a package
- repo root with only repo-level `system-prompt.md`
- user-home fallback only
- empty package-level file suppressing repo-level fallback

## Open Risks

1. Kimi append is not a faithful native capability without mutating project `AGENTS.md`; this design intentionally defers it.
2. OpenCode append depends on the documented `instructions` config merge path rather than the experimental system-transform hook, which is the safer choice but still needs runtime verification.
3. Some providers continue to load their own project memory after replacement. Claudine should document that replace means "use the provider's replacement mechanism", not "guarantee isolation from every other prompt source".

## Recommended Rollout

1. land the shared library types and discovery/composition pipeline
2. replace CLI args and remove the old `--system-prompt`
3. implement Claude, Codex, Gemini, Goose, OpenCode, and Qwen
4. implement Kimi replace
5. update docs and support matrix
6. decide separately whether Kimi append emulation is worth a follow-up feature
