# Current State: Skill and Command Linking

This document describes the current implementation of cross-provider skill and command linking in Claudine.

## Overview

Claudine can create symlinks to share skills and commands across multiple AI CLI providers (Claude, Codex, Gemini, OpenCode). The goal is "write once, use everywhere" - maintain a single source of truth for each skill/command.

## Supported Providers

| Provider | Skills | Commands | Notes |
|----------|--------|----------|-------|
| Claude   | ✅     | ✅       | Primary provider, Markdown commands |
| Codex    | ✅     | ❌       | Skills only, no command system (per current impl) |
| Gemini   | ✅     | ❌       | Skills only, commands marked "TOML format incompatible" |
| OpenCode | ✅     | ✅       | Also reads Claude's directories natively |

## Directory Paths

### User Scope (Home Directory)

| Provider | Skills | Commands |
|----------|--------|----------|
| Claude   | `~/.claude/skills/` | `~/.claude/commands/` |
| Codex    | `~/.codex/skills/` | N/A |
| Gemini   | `~/.gemini/skills/` | N/A |
| OpenCode | `~/.config/opencode/skills/` | `~/.config/opencode/commands/` |

### Repo Scope (Repository-Local)

| Provider | Skills | Commands |
|----------|--------|----------|
| Claude   | `.claude/skills/` | `.claude/commands/` |
| Codex    | `.codex/skills/` | N/A |
| Gemini   | `.gemini/skills/` | N/A |
| OpenCode | `.opencode/skills/` | `.opencode/commands/` |

## Linking Algorithm

### 4-Phase Process

1. **Discovery** - Scan all provider skill/command directories
2. **Hashing** - Compute xxHash content hash for each skill directory
3. **Analysis** - Classify each skill by sync status
4. **Linking** - Create symlinks for candidates

### Sync Status Classification

| Status | Condition | Action |
|--------|-----------|--------|
| `LinkCandidate` | Skill exists in exactly one provider | Create symlinks to other providers |
| `InSync` | Same skill in multiple providers, identical hash | No action needed |
| `Conflict` | Same skill in multiple providers, different hashes | Report conflict, no linking |
| `AlreadyLinked` | One real directory + symlinks in others | No action needed |

### Symlink Types

- **User Scope**: Absolute symlinks (e.g., `~/.codex/skills/foo` → `~/.claude/skills/foo`)
- **Repo Scope**: Relative symlinks (e.g., `.codex/skills/foo` → `../../.claude/skills/foo`)

Relative symlinks for repo scope ensure the links work when the repository is cloned to different locations.

## Special Cases

### OpenCode Reads Claude's Directories

OpenCode natively reads from Claude's skill directories. This is configured via `also_reads_from`:

```rust
opencode: ProviderPaths {
    also_reads_from: vec![claude_user_skills, claude_repo_skills],
    ...
}
```

When a skill exists in Claude, OpenCode is **excluded** from target providers since it already has access. This prevents redundant symlinks.

### Content Hashing

Skills are hashed by:
1. Collecting all files in the skill directory recursively
2. Sorting by relative path (deterministic order)
3. Hashing filename + contents for each file
4. Combining into a single xxHash

This means:
- Identical content = identical hash, even across machines
- Any file change (including nested files) = different hash
- File ordering doesn't affect the hash

## Current Limitations

### 1. Hardcoded to User Scope

```rust
// In link.rs
let scope = LinkScope::User;
```

**Problem**: The CLI only operates on user-level directories. Repo-level linking is not exposed.

**Impact**: Cannot share repo-local skills across providers within a project.

### 2. Commands Limited to Claude + OpenCode

```rust
gemini: ProviderPaths {
    user_commands: None, // Gemini uses TOML format — incompatible
    ...
},
codex: ProviderPaths {
    user_commands: None, // Codex has no command system
    ...
},
```

**Problem**: Gemini and Codex are excluded from command linking based on assumptions that may be incorrect.

**Question**: Do Gemini and Codex actually support Markdown commands? If the content/semantics are the same, format shouldn't matter for symlinking.

### 3. No Scripts Linking

There is no separate "scripts" linking feature. The hashing code handles scripts within skill directories (they're included in the hash), but there's no standalone scripts linking.

### 4. No CLI Control Over Source Provider

The algorithm picks the first provider with a skill as the source. There's no way to specify "always use Claude as the source" or similar preferences.

### 5. Conflict Resolution is Manual

When conflicts are detected (same skill, different content), the user must manually resolve. No merge or diff tooling is provided.

## CLI Usage

```bash
# Link skills (user scope only, currently)
claudine link

# Preview without making changes
claudine link --dry-run

# Filter to specific skill
claudine link --provider my-skill

# Verbose output
claudine link --verbose
```

## Report Output

The link command produces a report with:

- **Linked**: Skills/commands where symlinks were created
- **In Sync**: Already identical across providers
- **Already Linked**: Symlinks already exist
- **Conflicts**: Different content, requires manual resolution
- **Skipped**: Could not link (e.g., target is a real directory)

Commands are prefixed with `cmd:` in the report to distinguish from skills.

## Code Location

- Entry point: `claudine/lib/src/linking/mod.rs`
- Path definitions: `claudine/lib/src/linking/paths.rs`
- Conflict analysis: `claudine/lib/src/linking/conflict.rs`
- Skill discovery: `claudine/lib/src/linking/discovery.rs`
- Content hashing: `claudine/lib/src/linking/hashing.rs`
- Symlink creation: `claudine/lib/src/linking/symlink.rs`
- CLI command: `claudine/cli/src/commands/link.rs`
