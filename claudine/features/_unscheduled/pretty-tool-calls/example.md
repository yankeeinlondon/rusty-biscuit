 ← Read(error File not found: /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/reviews/2026-05-04-sentrux/plan.md)
 ← Skill(successful, claudine)
 ← Glob(successful, /Users/ken/.claudine/worktrees/rusty-biscuit/claudine)
 ← Read(successful, /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/reviews/2026-05-04-sentrux/plan.md)
 ← Read(successful, /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/harness/error.rs)
 ← Read(successful, /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/harness/mod.rs)
 ← Read(successful, /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/harness/model.rs)
 ← Read(successful, /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/provider/mod.rs)
 ← Read(successful, /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/lib.rs)
 ← Read(successful, /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/stream/mod.rs)
 ← Grep(successful, /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/stream)
 ← Grep(successful, /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/stream)
 ← Read(successful, /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/provider/output_format.rs)
 ← Read(successful, /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/provider/identity.rs)
 ← Read(successful, /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/provider/registry.rs)
 ← Grep(successful, /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/stream)
 ← Grep(successful, /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/stream)
 ← Bash(successful, bash cargo build -p claudine 2>&1 | head -50)
 ← Read(successful, /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/reviews/2026-05-04-sentrux/review-1.md)
 ← Grep(successful, /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/provider)
 ← Todowrite(successful, {"todos":[{"content":"Phase 1.1: Break harness::error ⇄ harness::model
cycle","priority":"high","status":"in_progress"},{"content":"Phase 1.2: Break provider ⇄ stream
cycle","priority":"high","status":"pending"},{"content":"Run tests and lints for claudine package
area","priority":"high","status":"pending"},{"content":"Update plan frontmatter with changed
files","priority":"medium","status":"pending"}]})
 ← Write(successful, //! Validation failure types — the shared boundary between error reporting and
//! the harness data model.
//!
//! This module is intentionally a leaf: it depends on neither [`crate::harness::error`]
//! nor [`crate::harness::model`] so both of those modules can import from here
//! without creating an import cycle.

    use std::path::PathBuf;

    use serde::{Deserialize, Serialize};

    /// Stable identifier for a validation rule, preserving author declaration order.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ValidationRuleId(pub u32);

    /// Which lifecycle event this validation maps to for handler lookup.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum ValidationEvent {
        FileExists,
        DirExists,
        JsonFileExists,
        YamlFileExists,
        TomlFileExists,
        HasWritePermission,
        ShellCommand,
        NoDirtySourceCode,
        HasDirtySourceCode,
        FileChanged,
        FileUnchanged,
        FrontmatterPropChanged,
        FrontmatterPropUnchanged,
        FrontmatterPropEquals,
        ResponseLengthAtLeast,
        ResponseLengthAtMost,
        ResponseIncludes,
        ResponseMissing,
        InlineResponseEmpty,
        InlineBodyUnchanged,
    }

    impl std::fmt::Display for ValidationEvent {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let s = match self {
                Self::FileExists => "file_exists",
                Self::DirExists => "dir_exists",
                Self::JsonFileExists => "json_file_exists",
                Self::YamlFileExists => "yaml_file_exists",
                Self::TomlFileExists => "toml_file_exists",
                Self::HasWritePermission => "has_write_permission",
                Self::ShellCommand => "shell_command",
                Self::NoDirtySourceCode => "no_dirty_source_code",
                Self::HasDirtySourceCode => "has_dirty_source_code",
                Self::FileChanged => "file_changed",
                Self::FileUnchanged => "file_unchanged",
                Self::FrontmatterPropChanged => "frontmatter_prop_changed",
                Self::FrontmatterPropUnchanged => "frontmatter_prop_unchanged",
                Self::FrontmatterPropEquals => "frontmatter_prop_equals",
                Self::ResponseLengthAtLeast => "response_length_at_least",
                Self::ResponseLengthAtMost => "response_length_at_most",
                Self::ResponseIncludes => "response_includes",
                Self::ResponseMissing => "response_missing",
                Self::InlineResponseEmpty => "inline_response_empty",
                Self::InlineBodyUnchanged => "inline_body_unchanged",
            };
            write!(f, "{s}")
        }
    }
