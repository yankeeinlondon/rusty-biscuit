# System Prompt Refactor — Implementation Plan

This plan implements the spec and tech design for the system prompt refactor. It is organized into seven phases with explicit file targets, type signatures, and test expectations.

Primary references:

- `claudine/features/2026-03-30-refactor-system-prompt/spec.md`
- `claudine/features/2026-03-30-refactor-system-prompt/tech-design.md`

---

## Phase 1 — Library Types and Launch Context

**Goal:** Introduce the `system_prompt` module in `claudine/lib` with core types and the launch-context extractor.

### 1.1 Create `claudine/lib/src/system_prompt/types.rs`

Define the core data model. These types are consumed by both the library resolution pipeline and the CLI wrapper application layer.

```rust
use std::path::PathBuf;

/// Whether a system prompt should append to or replace the provider's default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemPromptMode {
    Append,
    Replace,
}

/// Parsed CLI switch state before resolution.
#[derive(Debug, Clone, Default)]
pub struct SystemPromptArgs {
    pub append_file: Option<String>,
    pub replace_file: Option<String>,
}

/// The scope from which a standard `system-prompt.md` was discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardPromptScope {
    Package,
    PackageArea,
    Repo,
    User,
    CurrentDirectory,
}

/// Where the effective system prompt came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemPromptSource {
    /// Found via automatic `system-prompt.md` discovery.
    StandardDiscovered {
        path: PathBuf,
        scope: StandardPromptScope,
    },
    /// Provided via an explicit CLI switch.
    ExplicitFile {
        path: PathBuf,
        mode: SystemPromptMode,
    },
}

/// A system prompt that has been resolved, composed, and is ready for
/// provider-specific delivery.
#[derive(Debug, Clone)]
pub struct PreparedSystemPrompt {
    pub mode: SystemPromptMode,
    pub source: SystemPromptSource,
    /// The raw file text before Darkmatter composition.
    pub raw_text: String,
    /// The composed Markdown body (after Darkmatter pipeline).
    pub composed_markdown: String,
}

/// The outcome of the full resolve → compose pipeline.
#[derive(Debug, Clone)]
pub enum EffectiveSystemPrompt {
    /// No system prompt file was found or specified.
    None,
    /// A file was found but its composed body is empty — explicit disable.
    Disabled {
        source: SystemPromptSource,
    },
    /// A system prompt is ready for provider delivery.
    Ready(PreparedSystemPrompt),
}

impl EffectiveSystemPrompt {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn is_disabled(&self) -> bool {
        matches!(self, Self::Disabled { .. })
    }

    pub fn prepared(&self) -> Option<&PreparedSystemPrompt> {
        match self {
            Self::Ready(p) => Some(p),
            _ => None,
        }
    }
}
```

### 1.2 Create `claudine/lib/src/system_prompt/context.rs`

Extract launch-context detection from `wrap/env.rs` into shared library code. The existing `resolve_monorepo_package_context()` function in `env.rs` (lines 322–382) already does git-root, package, and package-area detection via `sniff::filesystem`. This phase extracts the _path resolution_ portion into a library struct so both env planning and system-prompt discovery share the same answer.

```rust
use std::path::{Path, PathBuf};

/// Filesystem roots resolved from the directory the user launched Claudine in.
///
/// All paths are canonical when possible. Fields are `None` when the
/// corresponding scope is not detected (e.g. not inside a git repo,
/// not inside a monorepo package).
#[derive(Debug, Clone)]
pub struct LaunchContext {
    /// The working directory Claudine was launched from.
    pub cwd: PathBuf,
    /// Git repository root, if detected.
    pub repo_root: Option<PathBuf>,
    /// Package-area root inside a monorepo.
    ///
    /// For the "root" package area this equals `repo_root`.
    pub package_area_root: Option<PathBuf>,
    /// Deepest matching workspace package root.
    pub package_root: Option<PathBuf>,
}

impl LaunchContext {
    /// Build a launch context from the given working directory.
    ///
    /// Uses `sniff::filesystem::git::detect_git` and
    /// `sniff::filesystem::repo::detect_repo` under the hood.
    pub fn from_cwd(cwd: &Path) -> Result<Self, crate::error::ClaudineError> {
        // Implementation: reuse detect_git + detect_repo + select_package_for_cwd
        // + select_package_area_for_cwd logic from env.rs:322-426.
        // Convert package_area string + repo_root into an actual PathBuf.
        todo!()
    }

    /// Deduplicated search directories in precedence order.
    ///
    /// Returns unique paths for: package root, package-area root, repo root.
    /// The user-home scope (`~/.claudine/`) is NOT included here — the
    /// caller adds it as a final fallback.
    pub fn search_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::with_capacity(3);
        let mut seen = std::collections::HashSet::new();
        for candidate in [&self.package_root, &self.package_area_root, &self.repo_root] {
            if let Some(p) = candidate {
                if seen.insert(p.clone()) {
                    dirs.push(p.clone());
                }
            }
        }
        dirs
    }
}
```

**Extraction plan for `env.rs`:**

1. Move `select_package_for_cwd()`, `select_package_area_for_cwd()`, and `canonical_or_self()` into `system_prompt/context.rs` (or a shared utility).
2. Add a new `package_area_root_path()` helper that converts a package-area name string (e.g. `"claudine"`) plus `repo_root` into a `PathBuf` (the `"root"` area maps to repo root itself, otherwise `repo_root.join(area)`).
3. Have `env.rs` call `LaunchContext::from_cwd()` and pull `package_context` from it, replacing its inline detection. This deduplicates the logic.
4. Alternatively, if the extraction is too invasive for phase 1, `context.rs` can call the same `sniff` functions independently and the `env.rs` refactor can be deferred to a follow-up. The key requirement is that `LaunchContext` exists and is correct.

### 1.3 Create `claudine/lib/src/system_prompt/mod.rs`

```rust
pub mod context;
pub mod resolve;
pub mod prepare;
pub mod types;

pub use context::LaunchContext;
pub use resolve::resolve_system_prompt_source;
pub use prepare::prepare_system_prompt;
pub use types::*;
```

### 1.4 Register the module in `claudine/lib/src/lib.rs`

Add `pub mod system_prompt;` to the existing module list (after `stream`).

### 1.5 Tests for Phase 1

**Unit tests in `system_prompt/context.rs`:**

| Test | Behavior |
|------|----------|
| `from_cwd_in_package` | CWD inside `claudine/cli` → package_root = `claudine/cli`, package_area_root = `claudine/`, repo_root = repo root |
| `from_cwd_in_package_area` | CWD inside `claudine/` but not a package → package_root = None, package_area_root = `claudine/` |
| `from_cwd_at_repo_root` | CWD = repo root → repo_root set, others None |
| `from_cwd_outside_repo` | CWD in `/tmp` → all None |
| `search_dirs_dedupes` | When package_root == repo_root, only one entry |
| `search_dirs_ordering` | Package before package-area before repo |

Use `tempfile::TempDir` with fabricated `.git` markers and `Cargo.toml` workspace definitions where needed.

---

## Phase 2 — Source Resolution

**Goal:** Implement the algorithm that selects which system prompt file to use based on CLI args or standard-file discovery.

### 2.1 Create `claudine/lib/src/system_prompt/resolve.rs`

```rust
use std::path::{Path, PathBuf};
use crate::system_prompt::types::*;
use crate::system_prompt::context::LaunchContext;

const STANDARD_FILENAME: &str = "system-prompt.md";

/// Resolve the effective system prompt source.
///
/// If explicit args are provided, resolves only the named file and
/// skips standard discovery. Otherwise searches the launch context
/// hierarchy for `system-prompt.md`.
pub fn resolve_system_prompt_source(
    args: &SystemPromptArgs,
    context: &LaunchContext,
) -> Result<Option<(SystemPromptSource, String)>, crate::error::ClaudineError> {
    // 1. Explicit --append-system-prompt
    if let Some(ref file) = args.append_file {
        let path = resolve_file_ref(file, &context.cwd)?;
        let text = std::fs::read_to_string(&path)?;
        return Ok(Some((
            SystemPromptSource::ExplicitFile { path, mode: SystemPromptMode::Append },
            text,
        )));
    }

    // 2. Explicit --replace-system-prompt
    if let Some(ref file) = args.replace_file {
        let path = resolve_file_ref(file, &context.cwd)?;
        let text = std::fs::read_to_string(&path)?;
        return Ok(Some((
            SystemPromptSource::ExplicitFile { path, mode: SystemPromptMode::Replace },
            text,
        )));
    }

    // 3. Standard discovery
    discover_standard_file(context)
}

fn discover_standard_file(
    context: &LaunchContext,
) -> Result<Option<(SystemPromptSource, String)>, crate::error::ClaudineError> {
    // Search local scopes in precedence order
    let scope_dirs: Vec<(PathBuf, StandardPromptScope)> = build_scope_list(context);
    for (dir, scope) in &scope_dirs {
        let candidate = dir.join(STANDARD_FILENAME);
        if candidate.is_file() {
            let text = std::fs::read_to_string(&candidate)?;
            return Ok(Some((
                SystemPromptSource::StandardDiscovered {
                    path: candidate,
                    scope: *scope,
                },
                text,
            )));
        }
    }

    // User-home fallback
    let user_home = dirs::home_dir()
        .map(|h| h.join(".claudine").join(STANDARD_FILENAME));
    if let Some(ref home_path) = user_home {
        if home_path.is_file() {
            let text = std::fs::read_to_string(home_path)?;
            return Ok(Some((
                SystemPromptSource::StandardDiscovered {
                    path: home_path.clone(),
                    scope: StandardPromptScope::User,
                },
                text,
            )));
        }
    }

    Ok(None)
}

/// Build the local scope search list from the launch context.
///
/// When inside a repo/monorepo, returns package → package-area → repo.
/// When outside a repo, returns CWD only.
fn build_scope_list(context: &LaunchContext) -> Vec<(PathBuf, StandardPromptScope)> {
    if context.repo_root.is_some() {
        let mut list = Vec::with_capacity(3);
        let mut seen = std::collections::HashSet::new();
        if let Some(ref p) = context.package_root {
            if seen.insert(p.clone()) {
                list.push((p.clone(), StandardPromptScope::Package));
            }
        }
        if let Some(ref p) = context.package_area_root {
            if seen.insert(p.clone()) {
                list.push((p.clone(), StandardPromptScope::PackageArea));
            }
        }
        if let Some(ref p) = context.repo_root {
            if seen.insert(p.clone()) {
                list.push((p.clone(), StandardPromptScope::Repo));
            }
        }
        list
    } else {
        vec![(context.cwd.clone(), StandardPromptScope::CurrentDirectory)]
    }
}

/// Resolve a file reference relative to the CWD (same rules as
/// `biscuit_file::resolve` or Claudine's existing file-ref logic).
fn resolve_file_ref(file_ref: &str, cwd: &Path) -> Result<PathBuf, crate::error::ClaudineError> {
    let path = Path::new(file_ref);
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    if !resolved.is_file() {
        return Err(crate::error::ClaudineError::SystemPromptFileNotFound(
            resolved.display().to_string(),
        ));
    }
    Ok(resolved)
}
```

### 2.2 Add error variant

In `claudine/lib/src/error.rs`, add:

```rust
#[error("system prompt file not found: {0}")]
SystemPromptFileNotFound(String),
```

### 2.3 Tests for Phase 2

**Unit tests in `system_prompt/resolve.rs`:**

| Test | Behavior |
|------|----------|
| `explicit_append_file_resolves` | `--asp some/file.md` resolves and returns Append mode |
| `explicit_replace_file_resolves` | `--rsp some/file.md` resolves and returns Replace mode |
| `explicit_file_not_found_errors` | Missing file → `SystemPromptFileNotFound` |
| `explicit_skips_standard_discovery` | When explicit arg set, standard files are ignored even if they exist |
| `standard_discovery_package_wins` | `system-prompt.md` at package root beats package-area and repo |
| `standard_discovery_package_area_wins` | No package-level file → package-area wins |
| `standard_discovery_repo_fallback` | Only repo-level file → repo wins |
| `standard_discovery_user_home_fallback` | Only `~/.claudine/system-prompt.md` → User scope |
| `standard_discovery_none` | No files anywhere → `None` |
| `standard_discovery_dedupes_overlapping_roots` | When package_root == repo_root, file checked once (no duplicate read) |
| `outside_repo_searches_cwd` | No git repo → searches CWD with `CurrentDirectory` scope |

Use `tempfile::TempDir` to create fixture directory trees with selective `system-prompt.md` placement.

---

## Phase 3 — Darkmatter Composition

**Goal:** Compose the resolved system prompt file through Darkmatter and produce the final `EffectiveSystemPrompt`.

### 3.1 Create `claudine/lib/src/system_prompt/prepare.rs`

```rust
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeOptions;
use crate::system_prompt::types::*;

/// Compose a resolved system prompt source through Darkmatter and
/// return the effective result.
///
/// If the composed body is empty after trimming, returns
/// `EffectiveSystemPrompt::Disabled`. Otherwise returns `Ready`.
pub fn prepare_system_prompt(
    source: SystemPromptSource,
    raw_text: &str,
) -> Result<EffectiveSystemPrompt, crate::error::ClaudineError> {
    let source_path = match &source {
        SystemPromptSource::StandardDiscovered { path, .. } => path,
        SystemPromptSource::ExplicitFile { path, .. } => path,
    };

    let mode = match &source {
        SystemPromptSource::StandardDiscovered { .. } => SystemPromptMode::Append,
        SystemPromptSource::ExplicitFile { mode, .. } => *mode,
    };

    // Parse and compose through Darkmatter
    let md: Markdown = raw_text.into();
    let options = ComposeOptions::new()
        .with_source_file(source_path);

    let (composed, _report) = md.compose_with(options)
        .map_err(|e| crate::error::ClaudineError::SystemPromptComposition(e.to_string()))?;

    let composed_markdown = composed.content().to_string();

    // Empty-body check
    if composed_markdown.trim().is_empty() {
        return Ok(EffectiveSystemPrompt::Disabled { source });
    }

    Ok(EffectiveSystemPrompt::Ready(PreparedSystemPrompt {
        mode,
        source,
        raw_text: raw_text.to_string(),
        composed_markdown,
    }))
}

/// Top-level convenience: resolve + compose in one call.
pub fn resolve_and_prepare(
    args: &SystemPromptArgs,
    context: &crate::system_prompt::context::LaunchContext,
) -> Result<EffectiveSystemPrompt, crate::error::ClaudineError> {
    let Some((source, raw_text)) =
        crate::system_prompt::resolve::resolve_system_prompt_source(args, context)?
    else {
        return Ok(EffectiveSystemPrompt::None);
    };
    prepare_system_prompt(source, &raw_text)
}
```

### 3.2 Add error variant

In `claudine/lib/src/error.rs`, add:

```rust
#[error("system prompt composition failed: {0}")]
SystemPromptComposition(String),
```

### 3.3 Tests for Phase 3

**Unit tests in `system_prompt/prepare.rs`:**

| Test | Behavior |
|------|----------|
| `plain_markdown_composes_as_is` | Simple Markdown passes through unchanged |
| `transclusion_resolves_relative` | `::file ./included.md` resolves relative to the prompt file |
| `shell_directive_executes` | `::shell echo hello` expands to `hello` |
| `interpolation_expands` | `{{today}}` replaced with current date |
| `empty_body_produces_disabled` | File with only frontmatter (empty body) → `Disabled` |
| `whitespace_only_body_produces_disabled` | Body with only whitespace → `Disabled` |
| `frontmatter_not_forwarded` | Output is body only; frontmatter stripped |
| `standard_file_always_append` | Standard-discovered file → mode is `Append` |
| `explicit_replace_preserves_mode` | Explicit replace file → mode is `Replace` |
| `resolve_and_prepare_none` | No file anywhere → `EffectiveSystemPrompt::None` |

---

## Phase 4 — CLI Switch Replacement

**Goal:** Remove the old `--system-prompt` switch and add `--append-system-prompt` / `--replace-system-prompt` with their short aliases.

### 4.1 Update `WrapperArgs` in `claudine/cli/src/commands/wrap/mod.rs`

**Remove** (line 636–637):

```rust
/// Set or append a system prompt (string or file path).
#[arg(short = 's', long = "system-prompt", value_name = "PROMPT|FILE")]
pub system_prompt: Option<String>,
```

**Add:**

```rust
/// Append a system prompt from a file.
#[arg(
    long = "append-system-prompt",
    visible_alias = "asp",
    value_name = "FILE",
    conflicts_with = "replace_system_prompt",
)]
pub append_system_prompt: Option<String>,

/// Replace the provider's system prompt with contents from a file.
#[arg(
    long = "replace-system-prompt",
    visible_alias = "rsp",
    value_name = "FILE",
    conflicts_with = "append_system_prompt",
)]
pub replace_system_prompt: Option<String>,
```

### 4.2 Update `ComposeArgs` in `claudine/cli/src/commands/compose.rs`

Apply the identical field replacement (lines 100–102). Same field names, same clap attributes.

### 4.3 Update `InlineComposeArgs` in `claudine/cli/src/commands/compose.rs`

Apply the identical field replacement (around lines 180–181).

### 4.4 Update `CompositionExecutionRequest` in `claudine/lib/src/composition/types.rs`

**Remove** (line 124–125):

```rust
pub system_prompt: Option<String>,
```

**Add:**

```rust
/// Parsed system prompt CLI args for the session.
pub system_prompt_args: SystemPromptArgs,
```

Import `use crate::system_prompt::SystemPromptArgs;` at the top of the file.

### 4.5 Update all sites that construct `CompositionExecutionRequest`

Search for every place that sets `system_prompt:` on the request and replace with:

```rust
system_prompt_args: SystemPromptArgs {
    append_file: args.append_system_prompt.clone(),
    replace_file: args.replace_system_prompt.clone(),
},
```

**Affected files:**

- `claudine/cli/src/commands/compose.rs` — `run_compose()` and `run_inline_compose()`
- Any other site that builds `CompositionExecutionRequest`

### 4.6 Update wrapper flow in `wrap/mod.rs`

**Remove** the old resolve-and-apply block (lines 806–812):

```rust
// Universal --system-prompt flag
if let Some(ref prompt) = args.system_prompt {
    let resolved = resolve_system_prompt(prompt)?;
    if let Some(warn) = profile.apply_system_prompt(&mut child_args, &resolved) {
        deferred_warnings.push(warn);
    }
}
```

**Replace with** a call to the new pipeline (detailed in Phase 6 when `apply_system_prompt` trait changes land). For now in Phase 4, wire up a temporary bridge:

```rust
let sp_args = SystemPromptArgs {
    append_file: args.append_system_prompt.clone(),
    replace_file: args.replace_system_prompt.clone(),
};
// Full integration deferred to Phase 6
```

### 4.7 Remove `resolve_system_prompt()` function

Delete the old `resolve_system_prompt()` function at `wrap/mod.rs:3139–3146`. Update any callers in `composition.rs` that reference it.

### 4.8 Tests for Phase 4

- `just test -p claudine-cli` — verify existing tests still compile after field renames
- `just test -p claudine` — verify lib tests pass with new types
- Manual or snapshot test: `claudine claude --help` shows new switches and hides old one
- Clap conflict test: passing both `--asp` and `--rsp` produces a clap error

---

## Phase 5 — Provider Profile Contract Change

**Goal:** Replace the narrow `apply_system_prompt(&mut Vec<String>, &str)` hook with the richer `apply_system_prompt` method that returns a full `SystemPromptApplication`.

### 5.1 Define wrapper-side types in `claudine/cli/src/commands/wrap/system_prompt.rs`

Create this new file:

```rust
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::Path;
use tempfile::{NamedTempFile, TempDir};
use claudine::system_prompt::{EffectiveSystemPrompt, PreparedSystemPrompt};

/// Artifacts that must remain alive until the child process exits.
pub(crate) enum SystemPromptArtifact {
    TempFile(NamedTempFile),
    TempDir(TempDir),
}

/// The result of applying a system prompt to a provider launch plan.
pub(crate) struct SystemPromptApplication {
    /// Additional CLI args to append.
    pub args: Vec<String>,
    /// Additional or overridden env vars.
    pub env: Vec<(OsString, OsString)>,
    /// Temp resources that must outlive the child process.
    pub artifacts: Vec<SystemPromptArtifact>,
    /// Non-fatal warnings to display.
    pub warnings: Vec<String>,
}

impl SystemPromptApplication {
    pub fn empty() -> Self {
        Self { args: vec![], env: vec![], artifacts: vec![], warnings: vec![] }
    }
}
```

### 5.2 Update `WrapperProfile` trait in `profile.rs`

**Remove** the old method (lines 121–126):

```rust
fn apply_system_prompt(&self, _args: &mut Vec<String>, _prompt: &str) -> Option<String> { ... }
```

**Add** the new method:

```rust
fn apply_system_prompt(
    &self,
    prompt: &PreparedSystemPrompt,
    interactive: bool,
    cwd: &Path,
) -> Result<SystemPromptApplication> {
    Ok(SystemPromptApplication {
        args: vec![],
        env: vec![],
        artifacts: vec![],
        warnings: vec![format!(
            "{} does not support {} system prompt; this flag was skipped",
            self.provider(),
            match prompt.mode {
                SystemPromptMode::Append => "append",
                SystemPromptMode::Replace => "replace",
            },
        )],
    })
}
```

The default implementation warns and returns empty args/env, matching the current behavior for unsupported providers.

### 5.3 Add `tempfile` as a runtime dependency

In `claudine/cli/Cargo.toml`, promote `tempfile` from `[dev-dependencies]` to `[dependencies]`:

```toml
[dependencies]
tempfile = "3"
```

### 5.4 Register the new module

In `claudine/cli/src/commands/wrap/mod.rs`, add:

```rust
pub(crate) mod system_prompt;
```

### 5.5 Tests for Phase 5

- Compile check: all providers that use the default still compile
- Existing tests pass with the signature change

---

## Phase 6 — Provider-Specific Implementations

**Goal:** Implement the `apply_system_prompt` method for each provider per the tech design's runtime support matrix.

### Provider Support Matrix (Phase 1 Rollout)

| Provider | Append | Replace | Strategy |
|----------|--------|---------|----------|
| Claude | Yes | Yes | Native CLI flags |
| Codex | Yes | Yes | Append: ephemeral home + `AGENTS.override.md`. Replace: temp file + `-c model_instructions_file=...` |
| Gemini | Yes | Yes | Append: ephemeral home + `GEMINI.md`. Replace: temp file + `GEMINI_SYSTEM_MD` env |
| Goose | Yes | No | Append: `goose run --system <text>`. Replace: warn + skip |
| Kimi | No | Yes | Replace: temp agent YAML + `--agent-file`. Append: warn + skip |
| OpenCode | Yes | Yes | Append: temp file + `OPENCODE_CONFIG_CONTENT`. Replace: `--system <path>` |
| Qwen | Yes | No | Append: ephemeral home + `QWEN.md`. Replace: warn + skip |

### 6.1 Claude implementation

```rust
// In ClaudeWrapper::apply_system_prompt
fn apply_system_prompt(
    &self,
    prompt: &PreparedSystemPrompt,
    interactive: bool,
    cwd: &Path,
) -> Result<SystemPromptApplication> {
    let mut app = SystemPromptApplication::empty();
    match prompt.mode {
        SystemPromptMode::Append => {
            if interactive {
                app.args.push("--append-system-prompt".into());
                app.args.push(prompt.composed_markdown.clone());
            } else {
                // Prefer file to avoid argv length limits
                let mut tmp = NamedTempFile::new()?;
                std::io::Write::write_all(&mut tmp, prompt.composed_markdown.as_bytes())?;
                app.args.push("--append-system-prompt-file".into());
                app.args.push(tmp.path().display().to_string());
                app.artifacts.push(SystemPromptArtifact::TempFile(tmp));
            }
        }
        SystemPromptMode::Replace => {
            if interactive {
                app.args.push("--system-prompt".into());
                app.args.push(prompt.composed_markdown.clone());
            } else {
                let mut tmp = NamedTempFile::new()?;
                std::io::Write::write_all(&mut tmp, prompt.composed_markdown.as_bytes())?;
                app.args.push("--system-prompt-file".into());
                app.args.push(tmp.path().display().to_string());
                app.artifacts.push(SystemPromptArtifact::TempFile(tmp));
            }
        }
    }
    Ok(app)
}
```

### 6.2 Codex implementation

**Append:** Create an ephemeral session home via `TempDir`. Copy minimal Codex config from the real home. If `~/.codex/AGENTS.override.md` exists, copy its contents and append the Claudine prompt after it. Otherwise create a new `AGENTS.override.md` with just the Claudine prompt. Set `HOME` to the temp dir.

**Replace:** Write composed Markdown to a `NamedTempFile`. Push `-c model_instructions_file=/absolute/path`.

```rust
fn apply_system_prompt(
    &self,
    prompt: &PreparedSystemPrompt,
    _interactive: bool,
    _cwd: &Path,
) -> Result<SystemPromptApplication> {
    let mut app = SystemPromptApplication::empty();
    match prompt.mode {
        SystemPromptMode::Append => {
            let (tmp_home, overlay_path) = create_ephemeral_overlay_home(
                ".codex",
                "AGENTS.override.md",
                &prompt.composed_markdown,
            )?;
            app.env.push((
                "HOME".into(),
                tmp_home.path().as_os_str().to_owned(),
            ));
            app.artifacts.push(SystemPromptArtifact::TempDir(tmp_home));
        }
        SystemPromptMode::Replace => {
            let mut tmp = NamedTempFile::new()?;
            std::io::Write::write_all(&mut tmp, prompt.composed_markdown.as_bytes())?;
            app.args.push("-c".into());
            app.args.push(format!(
                "model_instructions_file={}",
                tmp.path().display()
            ));
            app.artifacts.push(SystemPromptArtifact::TempFile(tmp));
        }
    }
    Ok(app)
}
```

### 6.3 Gemini implementation

**Append:** Ephemeral home with `~/.gemini/GEMINI.md`. Preserve existing user content, append Claudine prompt.

**Replace:** Write to temp file, set `GEMINI_SYSTEM_MD` env var.

### 6.4 Goose implementation

**Append:** Push `--system` with the prompt text.

**Replace:** Warn and return empty (unsupported).

### 6.5 Kimi implementation

**Replace:** Generate temp Markdown prompt file. Generate temp YAML agent spec with `extend: default` and `system_prompt_path` pointing to the temp Markdown file. Push `--agent-file <temp-agent.yaml>`.

**Append:** Warn and return empty (deferred).

### 6.6 OpenCode implementation

**Append:** Write composed Markdown to temp file. Build JSON override for `OPENCODE_CONFIG_CONTENT` with `instructions` array containing the temp file path.

**Replace:** Push `--system <tempfile-path>`.

### 6.7 Qwen implementation

**Append:** Ephemeral home with `~/.qwen/QWEN.md`. Same pattern as Codex/Gemini.

**Replace:** Warn and return empty (unsupported).

### 6.8 Shared ephemeral overlay home helper

Create a shared utility function in `wrap/system_prompt.rs`:

```rust
/// Create an ephemeral home directory with a provider config overlay.
///
/// 1. Create a TempDir
/// 2. Copy minimal provider config from real home
/// 3. Preserve existing overlay file content (if any)
/// 4. Append Claudine prompt content
/// 5. Return the TempDir and path to the overlay file
pub(crate) fn create_ephemeral_overlay_home(
    provider_dir: &str,     // e.g. ".codex", ".gemini", ".qwen"
    overlay_file: &str,     // e.g. "AGENTS.override.md", "GEMINI.md", "QWEN.md"
    prompt_content: &str,
) -> Result<(TempDir, PathBuf)> {
    let tmp_home = TempDir::new()?;
    let provider_path = tmp_home.path().join(provider_dir);
    std::fs::create_dir_all(&provider_path)?;

    // Copy existing provider config directory (minimal)
    let real_provider_dir = dirs::home_dir()
        .map(|h| h.join(provider_dir));
    if let Some(ref real) = real_provider_dir {
        if real.is_dir() {
            // Copy only the overlay file if it exists
            let existing = real.join(overlay_file);
            if existing.is_file() {
                let existing_content = std::fs::read_to_string(&existing)?;
                let combined = format!("{}\n\n{}", existing_content.trim_end(), prompt_content);
                std::fs::write(provider_path.join(overlay_file), combined)?;
                return Ok((tmp_home, provider_path.join(overlay_file)));
            }
        }
    }

    // No existing file — write prompt content only
    let overlay_path = provider_path.join(overlay_file);
    std::fs::write(&overlay_path, prompt_content)?;
    Ok((tmp_home, overlay_path))
}
```

### 6.9 Tests for Phase 6

**Provider-specific tests in `wrap/system_prompt.rs` or a dedicated test module:**

| Test | Provider | Behavior |
|------|----------|----------|
| `claude_append_interactive` | Claude | Produces `--append-system-prompt <text>` |
| `claude_append_non_interactive` | Claude | Produces `--append-system-prompt-file <tmpfile>` |
| `claude_replace_interactive` | Claude | Produces `--system-prompt <text>` |
| `claude_replace_non_interactive` | Claude | Produces `--system-prompt-file <tmpfile>` |
| `codex_append_creates_overlay_home` | Codex | HOME env overridden, `AGENTS.override.md` exists in tmp |
| `codex_append_preserves_existing_override` | Codex | Existing file content precedes Claudine content |
| `codex_replace_emits_config_flag` | Codex | `-c model_instructions_file=<path>` |
| `gemini_append_creates_gemini_md` | Gemini | `GEMINI.md` in ephemeral home |
| `gemini_replace_sets_env` | Gemini | `GEMINI_SYSTEM_MD` env set |
| `goose_append_passes_system_flag` | Goose | `--system <text>` |
| `goose_replace_warns` | Goose | Warning returned, no args |
| `kimi_replace_generates_agent_file` | Kimi | `--agent-file <yaml>` with `extend: default` |
| `kimi_append_warns` | Kimi | Warning returned |
| `opencode_append_sets_config_content` | OpenCode | `OPENCODE_CONFIG_CONTENT` env set |
| `opencode_replace_passes_system_flag` | OpenCode | `--system <path>` |
| `qwen_append_creates_qwen_md` | Qwen | `QWEN.md` in ephemeral home |
| `qwen_replace_warns` | Qwen | Warning returned |
| `ephemeral_home_preserves_existing` | Shared | Existing overlay content appears before Claudine content |
| `ephemeral_home_creates_fresh` | Shared | No existing → only Claudine content |

---

## Phase 7 — Wrapper Integration and Launch Plan Merge

**Goal:** Wire the full pipeline into the wrapper and composition execution flows. Handle artifact lifetimes, dry-run output, and logging.

### 7.1 Update `run_provider_wrapper_inner()` in `wrap/mod.rs`

Replace the removed system-prompt block (from Phase 4.6) with:

```rust
// -- System prompt pipeline ------------------------------------------------
let launch_context = LaunchContext::from_cwd(&cwd)?;
let sp_args = SystemPromptArgs {
    append_file: args.append_system_prompt.clone(),
    replace_file: args.replace_system_prompt.clone(),
};
let effective_sp = resolve_and_prepare(&sp_args, &launch_context)?;

// Artifacts must outlive the child process — bind them here.
let mut sp_artifacts: Vec<SystemPromptArtifact> = Vec::new();

match &effective_sp {
    EffectiveSystemPrompt::None => {}
    EffectiveSystemPrompt::Disabled { source } => {
        if !args.quiet && !args.silent {
            log::info(&format!(
                "system prompt disabled by empty {}",
                describe_source(source),
            ));
        }
    }
    EffectiveSystemPrompt::Ready(prepared) => {
        let application = profile.apply_system_prompt(
            prepared,
            !non_interactive_requested,
            &cwd,
        )?;
        child_args.extend(application.args);
        env_overrides.extend(
            application.env.into_iter()
                .map(|(k, v)| (k.to_string_lossy().into(), v.to_string_lossy().into()))
        );
        sp_artifacts = application.artifacts;
        for warn in application.warnings {
            deferred_warnings.push(warn);
        }
    }
}
```

**Important:** `sp_artifacts` must be declared _before_ the child spawn and remain in scope until the child exits. The existing code structure (child spawn → wait in the same function) naturally handles this. Verify that `sp_artifacts` is not dropped before `child.wait()`.

### 7.2 Update `execute_composition_request()` in `wrap/composition.rs`

Apply the same pattern. The composition path currently has its own system-prompt application block (around line 368–376). Replace it with the new pipeline.

Key difference: in the composition path, the launch context may need to use `prepared.source_repo_root` as the CWD hint (since composition files may live outside the CWD's repo).

### 7.3 Dry-run and verbose output

When `args.dry_run` is true, surface:

```
System prompt:
  source: standard discovery (package: claudine/cli/system-prompt.md)
  mode: append
  composed length: 1,247 chars
```

When verbose > 0, additionally log:

```
  raw length: 1,100 chars
  transclusions applied: 2
  shell expansions: 0
```

Update the dry-run output section in `wrap/mod.rs` (search for the existing dry-run block) to include system prompt details.

### 7.4 Update the composition flow's env_overrides handling

The `SystemPromptApplication.env` returns `(OsString, OsString)` pairs. These need to merge into the `EnvPlan`. Currently `env_overrides` in `run_provider_wrapper_inner()` is `Vec<(String, String)>`. Either:

- Convert the OsString pairs to String pairs (with lossy conversion) before pushing into env_overrides, or
- Widen env_overrides to accept OsString pairs

The simpler approach is lossy conversion since all provider env var names and values are ASCII/UTF-8.

### 7.5 Artifact lifetime handling for the composition path

In `execute_composition_request()`, the child process is spawned via `run_child()` or `run_child_capture()`. The `sp_artifacts` binding must remain in scope until the child wait completes. This should naturally work if declared in the same function scope.

For providers that use ephemeral homes (Codex, Gemini, Qwen append), the `TempDir` artifact contains the `HOME` override. When the child exits and the TempDir drops, the ephemeral home is cleaned up automatically.

### 7.6 Tests for Phase 7

**Integration tests (add to `claudine/cli/tests/wrap_commands.rs` or a new `system_prompt_integration.rs`):**

| Test | Behavior |
|------|----------|
| `wrapper_discovers_standard_file` | Create temp repo with `system-prompt.md` at root, verify it's passed to Claude |
| `wrapper_package_level_wins` | `system-prompt.md` at both package and repo level → package wins |
| `wrapper_empty_file_disables` | Empty `system-prompt.md` → no system prompt passed |
| `wrapper_explicit_append_skips_discovery` | `--asp file.md` ignores standard files |
| `wrapper_explicit_replace_skips_discovery` | `--rsp file.md` ignores standard files |
| `wrapper_asp_rsp_conflict` | Both flags → clap error |
| `wrapper_dry_run_shows_prompt_info` | Dry-run output includes system prompt section |
| `composition_inherits_system_prompt` | `claudine compose --asp file.md doc.md` passes system prompt to provider |

**Recommended integration fixtures (tempfile-based):**

1. Monorepo with CWD inside package
2. Monorepo with CWD in package area but outside a package
3. Repo root with only repo-level `system-prompt.md`
4. User-home fallback only (no repo files)
5. Empty package-level file suppressing repo-level fallback

---

## Phase 8 — Documentation and Cleanup

**Goal:** Update all documentation to reflect the new behavior and remove stale references.

### 8.1 Update `claudine/docs/topics/system-prompt.md`

Rewrite to reflect:

- Old `--system-prompt` is removed
- New `--append-system-prompt` / `--replace-system-prompt` switches
- Standard `system-prompt.md` discovery behavior
- Darkmatter composition support
- Empty-file disable semantics
- Updated provider support matrix (append vs replace)
- Ephemeral overlay home strategy

### 8.2 Update `claudine/cli/README.md`

- Remove references to `-s, --system-prompt`
- Add `--asp` and `--rsp` to the CLI reference
- Mention standard file discovery

### 8.3 Update `claudine/lib/README.md`

- Document the new `system_prompt` module
- Note the `LaunchContext` type for shared path detection

### 8.4 Update skill docs

Check `.claude/skills/claudine/` for any references to `--system-prompt` and update them.

### 8.5 Update agent capabilities model

In `claudine/lib/src/agents/` — the `SystemPromptCapabilities` struct and per-agent definitions don't need to change (they describe _provider_ capabilities, not Claudine's switches). But verify the doc comments are still accurate.

### 8.6 Remove stale code

- Delete `resolve_system_prompt()` from `wrap/mod.rs` (if not already removed in Phase 4)
- Remove any dead imports related to the old system prompt flow
- Run `cargo clippy -p claudine -p claudine-cli` to catch unused code

---

## Execution Order and Dependencies

```
Phase 1 (lib types + context)
    ↓
Phase 2 (resolution)
    ↓
Phase 3 (Darkmatter composition)
    ↓
Phase 4 (CLI switch replacement)  ←  can start in parallel with Phase 5
    ↓
Phase 5 (profile contract change)
    ↓
Phase 6 (provider implementations)
    ↓
Phase 7 (wrapper integration)
    ↓
Phase 8 (docs + cleanup)
```

Phases 1–3 are pure library additions with no breaking changes — existing code continues to work. Phase 4 is the first breaking change (CLI args). Phase 5 changes the trait. Phase 6 implements per-provider. Phase 7 wires everything together. Phase 8 is documentation.

### Suggested Commit Sequence

1. **Phase 1:** `feat(claudine): add system_prompt types and LaunchContext`
2. **Phase 2:** `feat(claudine): add system prompt source resolution`
3. **Phase 3:** `feat(claudine): add Darkmatter composition for system prompts`
4. **Phase 4:** `feat(claudine): replace --system-prompt with --asp/--rsp CLI switches`
5. **Phase 5:** `refactor(claudine): widen apply_system_prompt profile contract`
6. **Phase 6:** `feat(claudine): implement provider-specific system prompt delivery`
7. **Phase 7:** `feat(claudine): wire system prompt pipeline into wrapper launch`
8. **Phase 8:** `docs(claudine): update system prompt documentation`

---

## Risk Mitigations

| Risk | Mitigation |
|------|------------|
| `LaunchContext` extraction from `env.rs` is invasive | Phase 1 allows `context.rs` to call sniff independently; env.rs refactor can be deferred |
| Ephemeral home cleanup races with child process | `TempDir` is bound in function scope; Rust drop order guarantees cleanup after child wait |
| `tempfile` promotion to runtime dep increases CLI binary size | `tempfile` is already a dev-dep and is lightweight; minimal impact |
| Kimi append is intentionally unsupported | Warn clearly; don't attempt workarounds |
| Provider CLI flags may change between releases | Each provider implementation is isolated; flag changes affect only one `apply_system_prompt` impl |
| Existing `--system-prompt` users will break | This is an intentional breaking change per the spec; document migration clearly |

---

## Validation Checklist

After all phases:

- [ ] `just test -p claudine` passes
- [ ] `just test -p claudine-cli` passes
- [ ] `just lint` passes (clippy + fmt)
- [ ] `claudine claude --help` shows `--append-system-prompt` and `--replace-system-prompt`
- [ ] `claudine claude --help` does NOT show `--system-prompt`
- [ ] `claudine claude --asp` and `claudine claude --rsp` are recognized aliases
- [ ] `claudine claude --asp x --rsp y` produces a clap conflict error
- [ ] Standard `system-prompt.md` discovered and applied without CLI flags
- [ ] Empty `system-prompt.md` disables prompt injection
- [ ] Dry-run output shows system prompt source and mode
- [ ] Verbose output shows composition details
- [ ] Each provider's append/replace tested per the matrix
