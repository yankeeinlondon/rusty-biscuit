# Skill Linking

Claudine's `link` command synchronizes skills across agentic CLI providers using symbolic links. This document defines the provider landscape, the sync algorithm, and the data structures required.

---

## Table of Contents

1. [Problem Statement](#problem-statement)
2. [The Agent Skills Standard](#the-agent-skills-standard)
3. [Provider Skill Inventory](#provider-skill-inventory)
4. [Per-Provider Details](#per-provider-details)
5. [Variances and Gotchas](#variances-and-gotchas)
6. [Non-Destructive Sync Design](#non-destructive-sync-design)
7. [The `link` Command](#the-link-command)
8. [Data Structures](#data-structures)
9. [Edge Cases](#edge-cases)

---

## Problem Statement

Users who work with multiple agentic CLIs (Claude Code, Roo Code, OpenCode, Gemini CLI, Codex CLI) maintain duplicate skill directories across providers. A skill written for Claude Code in `~/.claude/skills/my-skill/` must be manually copied or symlinked to `~/.roo/skills/my-skill/`, `~/.gemini/skills/my-skill/`, and so on.

Claudine's `link` command automates this synchronization with two constraints:

1. **Non-destructive**: Never overwrite or modify a skill that already exists at a destination. If two providers have different content for the same skill name, report the conflict rather than clobbering.
2. **Symlink-based**: All synchronization is done via symbolic links, not file copies. This means changes to the source skill are immediately reflected in all linked providers.

---

## The Agent Skills Standard

The [Agent Skills](https://agentskills.io) format is an open standard originally developed by Anthropic and now adopted by Claude Code, Roo Code, OpenCode, Gemini CLI, Codex CLI, and many others.

### SKILL.md Format

Every skill is a directory containing a `SKILL.md` file with YAML frontmatter:

```yaml
---
name: pdf-processing
description: Extract text and tables from PDF files, fill forms, merge documents.
license: Apache-2.0
compatibility: Requires Python 3.10+
metadata:
  author: example-org
  version: "1.0"
allowed-tools: Bash(python:*) Read
---

# PDF Processing

Instructions for the agent follow here...
```

### Required Fields

| Field | Constraints |
|---|---|
| `name` | 1-64 chars; lowercase alphanumeric + hyphens; no leading/trailing/consecutive hyphens; **must match directory name** |
| `description` | 1-1024 chars; describes what the skill does and when to use it |

### Optional Fields

| Field | Purpose |
|---|---|
| `license` | License name or reference to bundled LICENSE file |
| `compatibility` | Max 500 chars; environment requirements (products, packages, network) |
| `metadata` | Arbitrary key-value map for author, version, etc. |
| `allowed-tools` | Space-delimited pre-approved tool list (experimental) |

### Directory Structure

```
skill-name/
├── SKILL.md          # Required: instructions + metadata
├── scripts/          # Optional: executable code
├── references/       # Optional: documentation
└── assets/           # Optional: templates, resources
```

### Progressive Disclosure

All conforming providers use a three-level loading strategy:

1. **Discovery**: At startup, only `name` and `description` from frontmatter are loaded
2. **Activation**: When a task matches, the full `SKILL.md` body is loaded into context
3. **Resources**: Bundled files (`scripts/`, `references/`, `assets/`) are loaded on-demand

---

## Provider Skill Inventory

### Skills Support Summary

| Provider | Supports Skills | User Skills Path | Repo Skills Path | Commands Path (User) | Commands Path (Repo) |
|---|---|---|---|---|---|
| Claude Code | Yes | `~/.claude/skills/` | `.claude/skills/` | `~/.claude/commands/` | `.claude/commands/` |
| Roo Code | Yes | `~/.roo/skills/` | `.roo/skills/` | `~/.roo/commands/` | `.roo/commands/` |
| OpenCode | Yes | `~/.config/opencode/skills/` | `.opencode/skills/` | `~/.config/opencode/commands/` | `.opencode/commands/` |
| Gemini CLI | Yes (experimental) | `~/.gemini/skills/` | `.gemini/skills/` | `~/.gemini/commands/` | `.gemini/commands/` |
| Codex CLI | Yes (feature flag) | `~/.codex/skills/` | `.codex/skills/` | — (deprecated prompts) | — |

**Additional cross-agent paths:** Both Codex and OpenCode also read from `~/.agents/skills/` and `<repo>/.agents/skills/` as a vendor-neutral convention. Claudine does not link to these paths (they serve as an interop fallback, not a primary location).

### Commands/Scripts Support

Commands (user-invoked slash commands) and scripts (skill-bundled executables) are separate concepts:

| Artifact | Claude Code | Roo Code | OpenCode | Gemini CLI | Codex CLI |
|---|---|---|---|---|---|
| **Slash commands** | `.claude/commands/*.md` | `.roo/commands/*.md` | `.opencode/commands/*.md` | `.gemini/commands/*.toml` | — |
| **Skill scripts** | `skills/*/scripts/` | `skills/*/scripts/` | `skills/*/scripts/` | `skills/*/scripts/` | `skills/*/scripts/` |

**Command format variance:** Claude Code, Roo Code, and OpenCode use Markdown (`.md`) for commands. Gemini CLI uses TOML (`.toml`) with a different syntax. Because of this format incompatibility, Claudine links commands only between Markdown-based providers (Claude, Roo, OpenCode) and skips Gemini CLI for command linking.

Claudine links both **skills** and **commands** where the provider supports a compatible format.

---

## Per-Provider Details

### Claude Code

**The reference implementation.** Claude Code originated the skill format and is the most feature-complete.

**Skill Locations:**
- User: `~/.claude/skills/{skill-name}/SKILL.md`
- Repo: `<root>/.claude/skills/{skill-name}/SKILL.md`
- Nested: Subdirectory `.claude/skills/` paths are auto-discovered (monorepo support)

**Command Locations:**
- User: `~/.claude/commands/{command-name}.md`
- Repo: `<root>/.claude/commands/{command-name}.md`

**Activation:** Model-invoked. Claude reads skill metadata at startup and loads the full SKILL.md when a request matches the description. No user confirmation required.

**Key Details:**
- No naming restrictions beyond the Agent Skills spec
- Supports nested skill discovery in monorepo subdirectories
- Skill descriptions drive activation (progressive disclosure)
- Commands are user-invoked via `/command-name`

---

### Roo Code CLI

**Roo Code** uses the Agent Skills format with added mode-specific directories.

**Skill Locations:**
- User: `~/.roo/skills/{skill-name}/SKILL.md`
- User mode-specific: `~/.roo/skills-{modeSlug}/{skill-name}/SKILL.md`
- Repo: `<root>/.roo/skills/{skill-name}/SKILL.md`
- Repo mode-specific: `<root>/.roo/skills-{modeSlug}/{skill-name}/SKILL.md`

**Override Priority** (highest to lowest):
1. Project mode-specific (`.roo/skills-code/my-skill/`)
2. Project generic (`.roo/skills/my-skill/`)
3. Global mode-specific (`~/.roo/skills-code/my-skill/`)
4. Global generic (`~/.roo/skills/my-skill/`)

Project location takes precedence over mode specificity.

**Slash Command Locations:**
- User: `~/.roo/commands/{command-name}.md`
- Repo: `<root>/.roo/commands/{command-name}.md`

Roo Code slash commands use Markdown format with optional YAML frontmatter (`description`, `argument-hint`, `mode` fields). The filename becomes the command name. Commands can be chained programmatically via the `run_slash_command` tool.

**Instruction Files:** Roo Code loads `AGENTS.md` from the workspace root (with `AGENT.md` as fallback). Also uses multi-file rules via `.roo/rules/` directories and legacy `.roorules` files.

**Note:** Roo Code is a VS Code extension, not a standalone CLI. Community CLI forks exist but are unofficial.

**Variances from Claude Code:**

| Aspect | Difference |
|---|---|
| Mode-specific skills | `skills-{modeSlug}/` directories target specific modes (code, architect, etc.) |
| Name validation | `name` in frontmatter **must** match directory name exactly |
| Slash commands | Uses `.roo/commands/*.md` (similar format to Claude's `.claude/commands/`) |
| Rules system | Multi-file rules via `.roo/rules/` directories (not a single CLAUDE.md) |
| AGENTS.md | Auto-loaded from workspace root; disabled via `roo-cline.useAgentRules: false` |
| System prompt override | `.roo/system-prompt-{mode-slug}` files **disable skills entirely** |

**Gotchas:**
1. If a user has a custom system prompt file, skills are invisible — this is a common source of confusion
2. The `name` field in SKILL.md frontmatter must exactly match the enclosing directory name; a mismatch silently prevents discovery
3. Roo Code does not read `.claude/skills/` or `.claude/commands/` — all cross-provider sharing requires explicit linking

---

### OpenCode CLI

**OpenCode** has the broadest Claude Code compatibility, reading skills from both its own paths and Claude Code's paths.

**Skill Locations:**
- User (native): `~/.config/opencode/skills/{skill-name}/SKILL.md`
- User (Claude compat): `~/.claude/skills/{skill-name}/SKILL.md`
- Repo (native): `<root>/.opencode/skills/{skill-name}/SKILL.md`
- Repo (Claude compat): `<root>/.claude/skills/{skill-name}/SKILL.md`

**Command Locations:**
- User: `~/.config/opencode/commands/{command-name}.md`
- Repo: `<root>/.opencode/commands/{command-name}.md`

**Additional Artifact Types** (not linked by Claudine, but noted for completeness):
- Agents: `~/.config/opencode/agents/` and `.opencode/agents/`
- Modes: `~/.config/opencode/modes/` and `.opencode/modes/`
- Tools: `~/.config/opencode/tools/` and `.opencode/tools/`

**Backward Compatibility:** Singular names (`agent/`, `command/`, `skill/`, `tool/`) are also supported alongside plural names.

**Variances from Claude Code:**

| Aspect | Difference |
|---|---|
| Claude path reading | Reads from `.claude/skills/` by default (disable with `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS`) |
| Command format | Commands support `$ARGUMENTS`, `$1`/`$2` positional params, `!`command`` bash injection, `@filename` file includes |
| Agent/model frontmatter | Commands support `agent` and `model` fields in frontmatter |
| Subtask commands | Commands support `subtask: true` to force subagent invocation |

**Gotchas:**
1. OpenCode reads `.claude/skills/` automatically — if Claudine also creates symlinks into OpenCode's native path, the same skill may appear twice. **Claudine must not link a skill from Claude to OpenCode's native path if the same skill already exists at `.claude/skills/`** (since OpenCode reads that directly).
2. Does **not** support `.claude/commands/` — commands must be in `.opencode/commands/`
3. The `model` field in skill frontmatter is requested but not yet implemented
4. Unknown frontmatter fields are silently ignored (not an error)
5. API credentials from Claude Code's official CLI cannot be used with OpenCode

---

### Gemini CLI

**Gemini CLI** supports the Agent Skills format with an explicit activation model requiring user approval.

**Skill Locations:**
- User: `~/.gemini/skills/{skill-name}/SKILL.md`
- Repo: `<root>/.gemini/skills/{skill-name}/SKILL.md`
- Extensions: `~/.gemini/extensions/{ext}/skills/{skill-name}/SKILL.md`

**Management CLI:**
```bash
gemini skills list               # List discovered skills
gemini skills install <source>   # Install from git, local path, or .skill file
gemini skills install <url> --path <subdir>  # Install from monorepo subdirectory
gemini skills install <path> --scope workspace|user
gemini skills uninstall <name>
gemini skills enable|disable <name>
```

**In-session management:** `/skills list|disable|enable|reload`

**Custom Command Locations:**
- User: `~/.gemini/commands/{command-name}.toml`
- Repo: `<root>/.gemini/commands/{command-name}.toml`

Gemini CLI commands use **TOML format** (not Markdown), with a different syntax:
```toml
description = "Generate a commit message from staged changes"
prompt = """Generate a commit message from:
```diff
!{git diff --staged}
```
"""
```

Dynamic content injection uses `{{args}}` for arguments, `!{shell command}` for shell output, and `@{path/to/file}` for file includes. Subdirectory namespacing is supported: `commands/git/commit.toml` becomes `/git:commit`.

**Context System:** Uses `GEMINI.md` files (hierarchical: global → project → subdirectory) with `@file.md` import syntax for modular context. The context filename is configurable — Gemini CLI can be configured to also read `CLAUDE.md`:

```json
{ "context": { "fileName": ["AGENTS.md", "CLAUDE.md", "GEMINI.md"] } }
```

**Variances from Claude Code:**

| Aspect | Difference |
|---|---|
| Skills are experimental | Must be explicitly enabled via `experimental.skills` setting |
| Activation model | Explicit: Uses `activate_skill` tool call, requires user approval before loading |
| Skill management | CLI commands for install/uninstall/enable/disable |
| Extension skills | Third source: `~/.gemini/extensions/` |
| Command format | TOML (not Markdown) — incompatible with Claude/Roo/OpenCode commands |
| Enable/disable | Skills can be disabled per-session via `/skills disable <name>` |
| Resource loading broken | Progressive disclosure Level 3 is NOT implemented — all files dumped at activation |

**Gotchas:**
1. **Skills must be explicitly enabled** via the `experimental.skills` setting or `/settings` UI
2. Gemini's activation requires user confirmation — skills behave more like tools than implicit context
3. **Progressive disclosure is broken**: When a skill activates, ALL files are loaded at once instead of on-demand. Skills designed for lazy-loading may consume excessive context in Gemini CLI.
4. Only `name` and `description` frontmatter fields are parsed; `compatibility`, `allowed-tools`, and `metadata` are silently ignored
5. The TOML command format means Claudine **cannot link commands** between Gemini and other providers
6. Gemini has its own extension system that may contain skills; Claudine should not attempt to manage extension skills
7. `gemini skills install` has reported reliability issues (GitHub issue #16703)

---

### Codex CLI

**Codex CLI** (OpenAI) has the simplest skill setup with a single user-level directory.

**Skill Locations:**
- User: `~/.codex/skills/{skill-name}/SKILL.md`
- System: `~/.codex/skills/.system/` (OpenAI-provided, should not be modified)

**Skill Locations:**
- User: `~/.codex/skills/{skill-name}/SKILL.md`
- User (cross-agent): `~/.agents/skills/{skill-name}/SKILL.md`
- Repo: `<root>/.codex/skills/{skill-name}/SKILL.md`
- Repo (cross-agent): `<root>/.agents/skills/{skill-name}/SKILL.md`
- System: `~/.codex/skills/.system/` (OpenAI-provided, **never modify**)

The `CODEX_HOME` environment variable overrides the `~/.codex/` base directory.

**Instruction File:** `AGENTS.md` (not `CODEX.md`). Codex also supports `AGENTS.override.md` for temporary overrides.

**Built-in System Skills:**
- `plan` — Lifecycle management skill
- `skill-creator` — Meta-skill for developing new skills

**Skills Feature Flag:** Skills must be explicitly enabled with `codex --enable skills`. They are currently considered experimental.

**Trust Model:** Repo-scoped `.codex/` content (including skills and config) is silently skipped for untrusted projects. Trust is configured in `~/.codex/config.toml`.

**Variances from Claude Code:**

| Aspect | Difference |
|---|---|
| Feature flag | Skills must be explicitly enabled (`codex --enable skills`) |
| Per-turn activation | Skills activate only for the current turn unless re-mentioned |
| System skills | `.system/` subdirectory contains OpenAI-provided skills (protected) |
| Trust model | Untrusted repos have their `.codex/` directory silently ignored |
| Instruction file | Uses `AGENTS.md` with `AGENTS.override.md` support |
| No commands | Custom prompts (`~/.codex/prompts/*.md`) are deprecated; no replacement |
| Config format | TOML (`~/.codex/config.toml`) instead of JSON |
| Cross-agent path | Also reads from `~/.agents/skills/` and `.agents/skills/` |

**Gotchas:**
1. **Symlink issues**: Multiple reported bugs where Codex does not follow symlinks to skill directories (GitHub issues #8943, #9365). Test symlinks after linking.
2. System skills in `.system/` must never be linked or modified by Claudine
3. Per-turn activation means skills don't persist across conversation turns — this doesn't affect linking but is worth noting for users
4. Codex is written in Rust, not Node.js
5. The trust model can silently hide repo-scoped skills — users may need to run `codex trust` if skills are not discovered
6. Duplicate skill listings have been reported on macOS (issue #8169)

---

## Variances and Gotchas

### Cross-Provider Comparison Table

| Feature | Claude | Roo | OpenCode | Gemini | Codex |
|---|---|---|---|---|---|
| **User skill path** | `~/.claude/skills/` | `~/.roo/skills/` | `~/.config/opencode/skills/` | `~/.gemini/skills/` | `~/.codex/skills/` |
| **Repo skill path** | `.claude/skills/` | `.roo/skills/` | `.opencode/skills/` | `.gemini/skills/` | `.codex/skills/` |
| **Reads `.claude/skills/`** | ✓ (native) | ✗ | ✓ (compat) | ✗ | ✗ |
| **Cross-agent path** | ✗ | ✗ | ✗ | ✗ | ✓ (`.agents/skills/`) |
| **Mode-specific dirs** | ✗ | ✓ (`skills-{mode}/`) | ✗ | ✗ | ✗ |
| **Nested discovery** | ✓ (monorepo) | ✗ | ✗ | ✗ | ✗ |
| **Slash commands** | ✓ (`.md`) | ✓ (`.md`) | ✓ (`.md`) | ✓ (`.toml`) | ✗ |
| **Command linking** | ✓ | ✓ | ✓ | ✗ (format mismatch) | ✗ |
| **Activation model** | Implicit (model) | Implicit (model) | Implicit (model) | Explicit (tool + approval) | Implicit (model, per-turn) |
| **Skills status** | Stable | Stable | Stable | Experimental | Feature flag |
| **Name must match dir** | ✓ (spec) | ✓ (enforced) | ✓ (enforced) | ✓ (spec) | ✓ (spec) |
| **Instruction file** | `CLAUDE.md` | `AGENTS.md` + rules | `AGENTS.md` / `CLAUDE.md` | `GEMINI.md` | `AGENTS.md` |
| **Config format** | JSON | JSON | JSON | JSON | TOML |
| **Symlink support** | ✓ | ✓ | ✓ | ✓ | ⚠ (reported bugs) |

### Critical Linking Considerations

1. **OpenCode reads Claude paths**: Do not create redundant symlinks from Claude to OpenCode's native path. OpenCode already reads `~/.claude/skills/`. Claudine should detect this and skip the link, noting it in the report. This behavior can be disabled by the user via `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS`.

2. **Roo mode-specific skills**: Claudine links to the **generic** skills directory (`.roo/skills/`), not mode-specific directories. Mode-specific targeting is a Roo-specific feature that should be configured by the user manually.

3. **System/protected directories**: Never link into `~/.codex/skills/.system/` or `~/.gemini/extensions/`.

4. **Codex symlink issues**: Multiple bugs have been reported where Codex does not follow symlinks (GitHub #8943, #9365). Claudine should warn about this in the link report and suggest the user verify discovery with `/skills list` in Codex.

5. **Codex trust model**: Repo-scoped skills in `.codex/skills/` are silently ignored for untrusted projects. Claudine should note this in the report.

6. **Gemini skills experimental**: Skills must be explicitly enabled in Gemini CLI. Claudine should check for this and warn if skills appear to be disabled.

7. **Command format incompatibility**: Gemini CLI uses TOML for commands while all others use Markdown. Command linking is limited to Claude ↔ Roo ↔ OpenCode.

8. **Symlink support**: Claude Code, Roo Code, OpenCode, and Gemini CLI handle symlinks correctly. Codex has reported issues — Claudine should verify after linking.

---

## Non-Destructive Sync Design

### Architecture Overview

The sync process has four phases:

```
┌─────────────┐     ┌──────────────┐     ┌───────────────┐     ┌──────────────┐
│  Discovery   │ ──▶ │   Hashing    │ ──▶ │   Conflict    │ ──▶ │   Linking    │
│              │     │              │     │  Detection     │     │              │
│ Scan all     │     │ xxHash each  │     │ Compare hashes │     │ Create       │
│ provider     │     │ skill dir    │     │ across         │     │ symlinks     │
│ skill dirs   │     │              │     │ providers      │     │              │
└─────────────┘     └──────────────┘     └───────────────┘     └──────────────┘
```

### Phase 1: Discovery

Scan each provider's skill directory to build an inventory of all skills:

```rust
/// A discovered skill from any provider.
pub struct DiscoveredSkill {
    /// Skill name extracted from SKILL.md frontmatter.
    pub name: String,

    /// Description from SKILL.md frontmatter.
    pub description: String,

    /// Which provider owns this skill directory.
    pub provider: Provider,

    /// Absolute path to the skill directory.
    pub path: PathBuf,

    /// Whether this path is a symlink.
    pub is_symlink: bool,

    /// If symlink, the resolved target path.
    pub symlink_target: Option<PathBuf>,

    /// xxHash of the skill's content (computed in Phase 2).
    pub content_hash: Option<u64>,
}
```

**Discovery rules:**
- Iterate each provider's skill directory
- For each subdirectory, check for a `SKILL.md` file
- Parse the YAML frontmatter to extract `name` and `description`
- Record whether the entry is already a symlink (and if so, resolve its target)
- Skip hidden directories (`.system/`, `.git/`, etc.)

### Phase 2: Content Hashing

For each discovered skill, compute an xxHash of its content to enable fast equality comparison. We use `biscuit_hash::xx_hash_bytes` for speed — xxHash is non-cryptographic but ideal for content comparison.

**What to hash:**

Hash the concatenation of all files in the skill directory (sorted alphabetically by relative path) to produce a single `u64` fingerprint:

```rust
use biscuit_hash::xx_hash_bytes;
use std::fs;
use std::path::Path;

/// Compute an xxHash fingerprint for a skill directory.
///
/// Walks all files in the directory (sorted by relative path),
/// concatenates their contents, and hashes the result.
///
/// Symlinks are resolved before hashing (we hash the target content,
/// not the symlink metadata).
fn hash_skill_dir(skill_dir: &Path) -> Result<u64> {
    let mut file_paths: Vec<PathBuf> = Vec::new();

    // Collect all files recursively
    collect_files(skill_dir, &mut file_paths)?;

    // Sort by relative path for deterministic ordering
    file_paths.sort();

    // Concatenate all file contents
    let mut combined = Vec::new();
    for path in &file_paths {
        let relative = path.strip_prefix(skill_dir)?;
        // Include the relative path as a separator to distinguish
        // files with identical content but different names
        combined.extend_from_slice(relative.to_string_lossy().as_bytes());
        combined.push(0); // null separator
        combined.extend_from_slice(&fs::read(path)?);
        combined.push(0); // null separator
    }

    Ok(xx_hash_bytes(&combined))
}
```

**Why xxHash over BLAKE3:**
- We need fast comparison, not cryptographic integrity
- xxHash is ~3x faster than BLAKE3 for small inputs
- A `u64` is sufficient for detecting differences (collision probability is negligible for our scale)

**Symlink handling during hashing:**
- If a skill directory is a symlink, resolve it and hash the **target** content
- Two symlinks pointing to the same target will produce the same hash
- A symlink and a real directory with identical content will produce the same hash

### Phase 3: Conflict Detection

Group discovered skills by name across providers. For each skill name:

```
Case 1: Exists in one provider only → Candidate for linking to other providers
Case 2: Exists in multiple providers with SAME hash → Already in sync (no action)
Case 3: Exists in multiple providers with DIFFERENT hashes → CONFLICT (report, do not link)
Case 4: Exists as a symlink pointing to a skill from another provider → Already linked (no action)
```

**Conflict resolution is always manual.** Claudine reports conflicts but never resolves them automatically. The user must decide which version to keep.

```rust
/// Result of analyzing a single skill name across all providers.
pub enum SkillSyncStatus {
    /// Skill exists in one provider; can be linked to others.
    LinkCandidate {
        source: DiscoveredSkill,
        targets: Vec<Provider>,
    },

    /// Skill exists in multiple providers with identical content.
    InSync {
        providers: Vec<DiscoveredSkill>,
    },

    /// Skill exists in multiple providers with different content.
    Conflict {
        skills: Vec<DiscoveredSkill>,
    },

    /// Skill is already linked via symlink from source to target(s).
    AlreadyLinked {
        source: DiscoveredSkill,
        linked_from: Vec<DiscoveredSkill>,
    },
}
```

### Phase 4: Linking

For each `LinkCandidate`, create symbolic links from the source provider to each target provider's skill directory.

**Symlink path strategy:**

| Scope | Symlink Type | Reason |
|---|---|---|
| User-scoped (`~/...`) | Absolute path | User dirs are fixed; absolute links are stable |
| Repo-scoped (`<root>/...`) | Relative path | Repo may be cloned elsewhere; relative links are portable |

**Creating relative symlinks for repo scope:**

```rust
use std::path::{Path, PathBuf};

/// Compute the relative path from `from_dir` to `target`.
///
/// Both paths must be absolute. The result is suitable for
/// use as a symlink target when created inside `from_dir`.
///
/// ## Examples
///
/// ```
/// let from = Path::new("/repo/.roo/skills/my-skill");
/// let target = Path::new("/repo/.claude/skills/my-skill");
/// assert_eq!(
///     relative_path(from, target),
///     PathBuf::from("../../.claude/skills/my-skill")
/// );
/// ```
fn relative_path(from_dir: &Path, target: &Path) -> PathBuf {
    // Walk up from `from_dir` to the common ancestor,
    // then walk down to `target`
    let from_components: Vec<_> = from_dir.components().collect();
    let target_components: Vec<_> = target.components().collect();

    // Find common prefix length
    let common_len = from_components.iter()
        .zip(target_components.iter())
        .take_while(|(a, b)| a == b)
        .count();

    // "../" for each remaining component in from_dir
    let ups = from_components.len() - common_len;
    let mut result = PathBuf::new();
    for _ in 0..ups {
        result.push("..");
    }

    // Append remaining components from target
    for component in &target_components[common_len..] {
        result.push(component);
    }

    result
}
```

**Atomic link creation:**

```rust
use std::os::unix::fs::symlink;

/// Create a symlink for a skill, with safety checks.
///
/// ## Errors
///
/// Returns an error if:
/// - The target directory already exists and is not a symlink
/// - The parent directory cannot be created
/// - The symlink cannot be created
fn create_skill_link(
    source: &Path,
    dest_dir: &Path,
    scope: LinkScope,
) -> Result<LinkResult> {
    let skill_name = source.file_name()
        .ok_or_else(|| Error::InvalidPath(source.to_path_buf()))?;
    let dest = dest_dir.join(skill_name);

    // Safety: never overwrite existing non-symlink directories
    if dest.exists() && !dest.is_symlink() {
        return Ok(LinkResult::Skipped {
            reason: format!("Real directory already exists at {}", dest.display()),
        });
    }

    // If a symlink already exists, check if it points to our source
    if dest.is_symlink() {
        let existing_target = fs::read_link(&dest)?;
        let resolved_source = match scope {
            LinkScope::User => source.to_path_buf(),
            LinkScope::Repo => relative_path(&dest.parent().unwrap(), source),
        };
        if existing_target == resolved_source {
            return Ok(LinkResult::AlreadyLinked);
        }
        // Different target — this is a conflict with an existing symlink
        return Ok(LinkResult::Skipped {
            reason: format!(
                "Symlink exists but points to {} (expected {})",
                existing_target.display(),
                resolved_source.display()
            ),
        });
    }

    // Ensure parent directory exists
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    // Create the symlink
    let link_target = match scope {
        LinkScope::User => source.to_path_buf(),
        LinkScope::Repo => relative_path(&dest.parent().unwrap(), source),
    };

    symlink(&link_target, &dest)?;

    Ok(LinkResult::Linked {
        source: source.to_path_buf(),
        dest,
        link_target,
    })
}
```

---

## The `link` Command

### Usage

```
claudine link [--dry-run] [--provider <name>] [--verbose]
```

### Behavior

**When run inside a git repository** (detected via `git rev-parse --show-toplevel`):
- Links **repo-scoped** skills using **relative** symlinks
- Scans `<root>/.claude/skills/`, `<root>/.roo/skills/`, etc.
- Creates relative symlinks between provider directories within the repo
- Skips Codex (no repo-scoped skill directory)

**When run outside a git repository:**
- Links **user-scoped** skills using **absolute** symlinks
- Scans `~/.claude/skills/`, `~/.roo/skills/`, etc.
- Creates absolute symlinks between provider directories

### Source Provider Selection

When a skill exists in only one provider, that provider is the **source** and all other installed providers are **targets**.

When a skill exists in multiple providers with identical content, the provider with the **longest history** (oldest mtime on SKILL.md) is chosen as the canonical source. The others are replaced with symlinks to the canonical source (only if the user confirms with `--replace-duplicates`; otherwise duplicates are left alone and reported).

### Command Output

```
Scanning skill directories...

  Claude Code:  ~/.claude/skills/  (12 skills)
  Roo Code:     ~/.roo/skills/     (8 skills)
  OpenCode:     ~/.config/opencode/skills/  (5 skills)
  Gemini CLI:   ~/.gemini/skills/  (3 skills)
  Codex CLI:    ~/.codex/skills/   (10 skills)

Analyzing 18 unique skill names...

  Linked:
    ✓ clap         Claude → Roo, Gemini, Codex
    ✓ tokio        Claude → Roo, Gemini, Codex
    ✓ serde        Claude → Roo, Gemini, Codex
    ✓ ratatui      Claude → Roo, Gemini, Codex

  Already in sync:
    = axum         Claude ↔ Roo ↔ OpenCode (identical content)

  Skipped (OpenCode reads .claude/skills/ directly):
    ~ chrono       Claude → OpenCode (not needed)

  Conflicts (different content):
    ✗ react        Claude (hash: a1b2c3d4) ≠ Roo (hash: e5f6g7h8)
                   Resolve: compare ~/.claude/skills/react/ vs ~/.roo/skills/react/
    ✗ typescript   OpenCode (hash: i9j0k1l2) ≠ Gemini (hash: m3n4o5p6)
                   Resolve: compare ~/.config/opencode/skills/typescript/ vs ~/.gemini/skills/typescript/

  Not linked (no repo skills directory):
    ⊘ Codex CLI does not support repo-scoped skills

Summary: 4 linked, 1 in sync, 1 skipped, 2 conflicts, 1 provider excluded
```

### Flags

| Flag | Description |
|---|---|
| `--dry-run` | Show what would happen without creating any symlinks |
| `--provider <name>` | Only link to/from a specific provider (claude, roo, opencode, gemini, codex) |
| `--verbose` | Show detailed hash values and path resolution |
| `--replace-duplicates` | When identical skills exist in multiple providers, replace copies with symlinks to the oldest source (requires confirmation) |

---

## Data Structures

### Configuration

The link command needs to know which providers are installed. This is detected automatically using the same mechanism described in `agent-configuration.md`:

```rust
/// Skill directory paths for each provider.
pub struct ProviderSkillPaths {
    pub provider: Provider,

    /// Path to the user-scoped skills directory.
    pub user_skills: PathBuf,

    /// Path to the repo-scoped skills directory (relative to repo root).
    /// `None` if the provider doesn't support repo-scoped skills (e.g., Codex).
    pub repo_skills: Option<PathBuf>,

    /// Path to the user-scoped commands directory.
    /// `None` if the provider doesn't support commands.
    pub user_commands: Option<PathBuf>,

    /// Path to the repo-scoped commands directory (relative to repo root).
    /// `None` if the provider doesn't support commands.
    pub repo_commands: Option<PathBuf>,

    /// Whether this provider also reads skills from another provider's path
    /// (e.g., OpenCode reads from `.claude/skills/`).
    pub also_reads_from: Vec<Provider>,
}

impl ProviderSkillPaths {
    pub fn all() -> Vec<Self> {
        vec![
            Self {
                provider: Provider::Claude,
                user_skills: dirs::home_dir().unwrap().join(".claude/skills"),
                repo_skills: Some(PathBuf::from(".claude/skills")),
                user_commands: Some(dirs::home_dir().unwrap().join(".claude/commands")),
                repo_commands: Some(PathBuf::from(".claude/commands")),
                also_reads_from: vec![],
            },
            Self {
                provider: Provider::RooCode,
                user_skills: dirs::home_dir().unwrap().join(".roo/skills"),
                repo_skills: Some(PathBuf::from(".roo/skills")),
                user_commands: Some(dirs::home_dir().unwrap().join(".roo/commands")),
                repo_commands: Some(PathBuf::from(".roo/commands")),
                also_reads_from: vec![],
            },
            Self {
                provider: Provider::OpenCode,
                user_skills: dirs::home_dir().unwrap()
                    .join(".config/opencode/skills"),
                repo_skills: Some(PathBuf::from(".opencode/skills")),
                user_commands: Some(
                    dirs::home_dir().unwrap()
                        .join(".config/opencode/commands"),
                ),
                repo_commands: Some(PathBuf::from(".opencode/commands")),
                also_reads_from: vec![Provider::Claude],
            },
            Self {
                provider: Provider::Gemini,
                user_skills: dirs::home_dir().unwrap().join(".gemini/skills"),
                repo_skills: Some(PathBuf::from(".gemini/skills")),
                // Gemini uses TOML commands — incompatible with Markdown format
                // Claudine does not link commands to/from Gemini
                user_commands: None,
                repo_commands: None,
                also_reads_from: vec![],
            },
            Self {
                provider: Provider::Codex,
                user_skills: dirs::home_dir().unwrap().join(".codex/skills"),
                repo_skills: Some(PathBuf::from(".codex/skills")),
                user_commands: None,
                repo_commands: None,
                also_reads_from: vec![],
            },
        ]
    }
}
```

### Link Scope

```rust
/// Whether we are linking user-scoped or repo-scoped skills.
pub enum LinkScope {
    /// User-scoped: absolute symlinks between `~/.<provider>/skills/`
    User,

    /// Repo-scoped: relative symlinks between `<root>/.<provider>/skills/`
    Repo,
}
```

### Sync Report

```rust
/// Full report from a link operation.
pub struct LinkReport {
    /// Skills that were successfully linked.
    pub linked: Vec<LinkAction>,

    /// Skills that are already in sync across providers.
    pub in_sync: Vec<SyncGroup>,

    /// Skills that were skipped (e.g., OpenCode reads Claude directly).
    pub skipped: Vec<SkipAction>,

    /// Skills with conflicting content across providers.
    pub conflicts: Vec<Conflict>,

    /// Providers that were excluded (e.g., Codex for repo scope).
    pub excluded_providers: Vec<(Provider, String)>,

    /// Errors encountered during the process.
    pub errors: Vec<LinkError>,
}

pub struct LinkAction {
    pub skill_name: String,
    pub source_provider: Provider,
    pub source_path: PathBuf,
    pub targets: Vec<(Provider, PathBuf, LinkResult)>,
}

pub struct SyncGroup {
    pub skill_name: String,
    pub hash: u64,
    pub providers: Vec<(Provider, PathBuf)>,
}

pub struct SkipAction {
    pub skill_name: String,
    pub source_provider: Provider,
    pub target_provider: Provider,
    pub reason: String,
}

pub struct Conflict {
    pub skill_name: String,
    pub versions: Vec<(Provider, PathBuf, u64)>, // (provider, path, hash)
}

pub enum LinkResult {
    /// Symlink was created.
    Linked {
        source: PathBuf,
        dest: PathBuf,
        link_target: PathBuf,
    },

    /// Symlink already exists and points to the correct target.
    AlreadyLinked,

    /// Linking was skipped for the stated reason.
    Skipped { reason: String },
}

pub struct LinkError {
    pub skill_name: String,
    pub provider: Provider,
    pub error: String,
}
```

---

## Edge Cases

### 1. Circular Symlinks

If Provider A has a symlink to Provider B and Provider B has a symlink to Provider A, we have a circular reference. **Prevention:** During discovery, always resolve symlinks and track the canonical (non-symlink) source. Never create a symlink from a provider that only has a symlink to another provider — always link to the canonical source.

### 2. Broken Symlinks

A skill directory may be a symlink whose target has been deleted. **Handling:** During discovery, detect broken symlinks via `fs::canonicalize()` failure. Report them and exclude from the sync. Optionally offer to clean up broken symlinks with `--clean`.

### 3. Permission Errors

Some provider directories may have restricted permissions. **Handling:** Catch permission errors during discovery and linking. Report the affected provider and suggest the user check permissions.

### 4. Missing Provider Directories

If a provider's skill directory doesn't exist, Claudine creates it before linking (e.g., `mkdir -p ~/.gemini/skills/`). This is safe because the directories are just organizational — the providers create them on first use anyway.

### 5. Skill Name Collisions Across Commands and Skills

A skill named `test` and a command named `test` are in different namespaces. Claudine handles them independently — skills are linked to skill directories, commands to command directories.

### 6. OpenCode Double-Discovery

As noted above, OpenCode reads from both `~/.claude/skills/` and `~/.config/opencode/skills/`. If Claudine links a Claude skill into OpenCode's native path, OpenCode discovers it twice. **Prevention:** The `also_reads_from` field in `ProviderSkillPaths` tracks cross-provider reading. When the source provider is in a target's `also_reads_from` list, skip the link and report it.

### 7. Repo-Root Detection

Claudine uses `git rev-parse --show-toplevel` to detect the repository root. If this fails (not in a git repo), Claudine falls back to user-scope linking. If the user explicitly passes `--repo` outside a git repo, Claudine exits with an error.

### 8. Skills with Scripts That Use Relative Paths

A skill's `scripts/` directory may contain scripts that reference files via relative paths. Since symlinks preserve the logical path but resolve to the physical location, relative paths within scripts work correctly — they resolve relative to the **target** (where the real files are), not the symlink location.

### 9. Large Skill Directories

Some skills may contain large asset files. The xxHash computation reads all files into memory for hashing. For very large skills (>100MB), consider streaming the hash computation:

```rust
use xxhash_rust::xxh64::Xxh64;

fn hash_skill_dir_streaming(skill_dir: &Path) -> Result<u64> {
    let mut hasher = Xxh64::new(0);
    let mut file_paths = Vec::new();
    collect_files(skill_dir, &mut file_paths)?;
    file_paths.sort();

    for path in &file_paths {
        let relative = path.strip_prefix(skill_dir)?;
        hasher.update(relative.to_string_lossy().as_bytes());
        hasher.update(&[0]);

        // Stream the file in 8KB chunks
        let mut file = fs::File::open(path)?;
        let mut buf = [0u8; 8192];
        loop {
            let n = std::io::Read::read(&mut file, &mut buf)?;
            if n == 0 { break; }
            hasher.update(&buf[..n]);
        }
        hasher.update(&[0]);
    }

    Ok(hasher.digest())
}
```

**Note:** `biscuit_hash` currently exposes `xx_hash_bytes(&[u8]) -> u64` which requires the full buffer in memory. For the streaming approach, the `xxhash-rust` crate's `Xxh64` hasher would need to be used directly. Consider adding a streaming API to `biscuit_hash` if this becomes a common need.

### 10. Command Linking

Commands follow the same linking logic as skills but only apply to providers that support the **Markdown command format**:

| Source | Target |
|---|---|
| Claude `.claude/commands/` | Roo `.roo/commands/`, OpenCode `.opencode/commands/` |
| Roo `.roo/commands/` | Claude `.claude/commands/`, OpenCode `.opencode/commands/` |
| OpenCode `.opencode/commands/` | Claude `.claude/commands/`, Roo `.roo/commands/` |

**Excluded from command linking:**
- **Gemini CLI**: Uses TOML format (`.gemini/commands/*.toml`) with different placeholder syntax (`{{args}}`, `!{cmd}`, `@{file}`). Format is incompatible with Markdown commands.
- **Codex CLI**: Custom prompts are deprecated; no current command system.

**Variance notes:**
- OpenCode commands support additional frontmatter fields (`agent`, `model`, `subtask`) and placeholder syntax (`$ARGUMENTS`, `` !`backtick commands` ``, `@file`) that Claude Code does not. Commands authored for OpenCode will work in Claude Code (extra frontmatter is ignored), but OpenCode-specific placeholders will appear as literal text in Claude Code.
- Roo Code commands support `mode` frontmatter to switch modes before execution, and the `run_slash_command` tool for programmatic chaining. These features are Roo-specific.
- Claudine should warn in the link report when provider-specific features are detected in linked commands.

### 11. Codex Trust Model

If a project is not explicitly trusted in Codex's config, all repo-scoped `.codex/` content is silently ignored. When Claudine creates symlinks in `.codex/skills/` for a repo, those links will be invisible to Codex until the project is trusted. Claudine should detect this by checking `~/.codex/config.toml` for trust status and warn accordingly.
