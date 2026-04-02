# Preflight Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix structural gaps in the preflight shell approval system: correct discovery pipeline, transclusion provenance, harness parsing, approval cache propagation, interactive handler wiring, and error reporting.

**Architecture:** Bottom-up from darkmatter foundation (types, discovery, provenance) through claudine-lib (approval authority, cache propagation) to claudine-cli (handler wiring, error reporting). Changes in each layer unlock the next.

**Tech Stack:** Rust, darkmatter (markdown compose pipeline), claudine (harness/preflight), thiserror, tempfile (tests)

---

## File Map

### Modified Files

| File | Responsibility |
|------|---------------|
| `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs` | Discovery pipeline: collect `::shell` directives from document graph |
| `darkmatter/lib/src/markdown/compose/types.rs` | `SourceRange` type, `ComposeReport.source_map` field |
| `darkmatter/lib/src/markdown/compose/mod.rs` | Populate source map during transclusion application, add `source_file` to `ResolvedTransclusion` |
| `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs` | Enrich `NotPreApproved` error with `source_file` |
| `claudine/lib/src/composition/preflight.rs` | Populate `ShellApprovalRequest` provenance from `ShellCommandEntry` |
| `claudine/lib/src/composition/error.rs` | Enrich `ShellCommandDenied` and `PreFlightFailed` with provenance fields |
| `claudine/lib/src/harness/parse.rs` | Remove `shell_options` parameter, simplify to tokenize-only |
| `claudine/lib/src/harness/mod.rs` | Remove `parse_harness_plan_with_shell` re-export |
| `claudine/cli/src/commands/wrap/mod.rs` | Wire approval handler, remove non-interactive guard, pass shell_options to harness loop |
| `claudine/cli/src/commands/wrap/composition.rs` | Remove parse-time shell validation |
| `claudine/cli/Cargo.toml` | Add `darkmatter-cli` dependency |

---

### Task 1: Extend Discovery Pipeline Stages

**Spec refs:** Group 1a (Finding 5)

**Files:**
- Modify: `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs`

- [ ] **Step 1: Write failing test — PageBlocks excludes conditional false blocks**

Add to the `tests` module in `discovery.rs`:

```rust
#[test]
fn excludes_directives_inside_false_page_blocks() {
    let content = "\
---
include_shell: false
---
::shell echo always
::block when=\"{{include_shell}}\"
::shell echo conditional
::end-block
";
    let md: Markdown = content.into();
    let options = ComposeOptions::new();

    let entries = collect_shell_commands(&md, &options).unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].raw_command, "echo always");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter --lib -- compose::shell_expansion::discovery::tests::excludes_directives_inside_false_page_blocks`

Expected: FAIL — the conditional block directive is still collected because `PageBlocks` is not in the discovery pipeline.

- [ ] **Step 3: Write failing test — TextReplacement introduces directives**

```rust
#[test]
fn discovers_directives_introduced_by_text_replacement() {
    let content = "\
---
replace:
  PLACEHOLDER: \"echo replaced\"
---
::shell PLACEHOLDER
";
    let md: Markdown = content.into();
    let options = ComposeOptions::new();

    let entries = collect_shell_commands(&md, &options).unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].raw_command, "echo replaced");
}
```

- [ ] **Step 4: Run test to verify it fails**

Run: `cargo test -p darkmatter --lib -- compose::shell_expansion::discovery::tests::discovers_directives_introduced_by_text_replacement`

Expected: FAIL — `TextReplacement` is not in the discovery pipeline.

- [ ] **Step 5: Add TextReplacement and PageBlocks to the discovery .only() list**

In `collect_shell_commands()`, change the `.only()` call:

```rust
    let discovery_options = options.clone().only(&[
        ComposeOperation::FrontmatterInterpolation,
        ComposeOperation::TextReplacement,
        ComposeOperation::PageBlocks,
        ComposeOperation::Interpolation,
        ComposeOperation::BlockTransclusion,
        ComposeOperation::FrontmatterTransclusion,
    ]);
```

- [ ] **Step 6: Run all discovery tests to verify they pass**

Run: `cargo test -p darkmatter --lib -- compose::shell_expansion::discovery::tests`

Expected: All 8 tests PASS.

- [ ] **Step 7: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs
git commit -m "feat(darkmatter): align discovery pipeline with compose stage order

Add TextReplacement and PageBlocks to the discovery .only() list so that
conditional blocks and text-replaced directives are handled correctly."
```

---

### Task 2: Add SourceRange Type and source_map to ComposeReport

**Spec refs:** Group 1b (Finding 4) — types only

**Files:**
- Modify: `darkmatter/lib/src/markdown/compose/types.rs`

- [ ] **Step 1: Add SourceRange struct**

Add after the `ComposeWarning` type (before `ComposeReport`):

```rust
/// Maps a byte range in composed output to its originating source file.
///
/// Populated by `BlockTransclusion` when file content replaces a
/// `::file` directive. Byte positions refer to the final composed content.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceRange {
    /// Start byte offset in the composed output (inclusive).
    pub byte_start: usize,
    /// End byte offset in the composed output (exclusive).
    pub byte_end: usize,
    /// The source file whose content occupies this range.
    pub source_file: PathBuf,
    /// The starting line number in the source file (1-based).
    pub source_start_line: usize,
}
```

- [ ] **Step 2: Add source_map field to ComposeReport**

Add after `pub perf: Option<ComposePerfReport>`:

```rust
    /// Source map tracking which byte ranges came from transcluded files.
    ///
    /// Each entry maps a byte range in the composed output to the
    /// originating file. Ranges not covered default to the root document.
    pub source_map: Vec<SourceRange>,
```

- [ ] **Step 3: Update ComposeReport::has_changes to include source_map**

In the `has_changes()` method, no change needed — source_map presence doesn't indicate "changes made."

- [ ] **Step 4: Verify merge() does NOT merge source_map**

The `merge()` method merges child reports from recursive transclusions. Source map entries have byte positions relative to their own compose output, so merging them would produce invalid positions. The parent populates its own source_map during transclusion application. Confirm `merge()` does not reference `source_map` (it won't, since it's a new field and `Default` gives `Vec::new()`).

- [ ] **Step 5: Add SourceRange to the module's public exports**

Check if `types.rs` items are re-exported in `darkmatter/lib/src/markdown/compose/mod.rs`. If `ComposeReport` is already re-exported, `SourceRange` needs to be too. Add it to the `pub use` statement.

- [ ] **Step 6: Run darkmatter tests to verify no regressions**

Run: `cargo test -p darkmatter --lib`

Expected: All existing tests PASS.

- [ ] **Step 7: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/types.rs darkmatter/lib/src/markdown/compose/mod.rs
git commit -m "feat(darkmatter): add SourceRange type and source_map to ComposeReport

Tracks which byte ranges in composed output came from transcluded files.
Populated during BlockTransclusion application (next commit)."
```

---

### Task 3: Populate source_map During BlockTransclusion

**Spec refs:** Group 1b (Finding 4) — compose pipeline

**Files:**
- Modify: `darkmatter/lib/src/markdown/compose/mod.rs`

- [ ] **Step 1: Add source_file field to ResolvedTransclusion**

At `darkmatter/lib/src/markdown/compose/mod.rs:173`, add `source_file`:

```rust
struct ResolvedTransclusion {
    order: usize,
    target: ApplyTarget,
    content: Option<String>,
    report: ComposeReport,
    /// Source file for file-based transclusions (used for source map).
    source_file: Option<PathBuf>,
}
```

- [ ] **Step 2: Set source_file on all ResolvedTransclusion construction sites**

For the `Markdown` variant in `resolve_prepared_transclusion` (around line 1243):
```rust
Ok(ResolvedTransclusion {
    order,
    target,
    content: Some(content),
    report: child_report,
    source_file: Some(path),
})
```

For all other variants (`FixedReplace`, `FixedSection`, `Code`, `Toc`), set `source_file: None`.

There are approximately 5-6 construction sites. Search for `Ok(ResolvedTransclusion {` and add `source_file: None,` or `source_file: Some(path),` as appropriate.

- [ ] **Step 3: Build source_map from replacement records after application**

In the transclusion application section (around line 737), the existing code collects replacements and applies them in reverse byte order:

```rust
if !replacements.is_empty() {
    replacements.sort_by(...);
    let mut next = self.content.clone();
    for (_, span, replacement) in replacements {
        next.replace_range(span, &replacement);
    }
    self.content = next;
}
```

Change the replacement tuple to carry the optional source file. Replace the replacement collection at line 726:

```rust
ApplyTarget::Replace(span) => {
    replacements.push((
        resolved.order,
        span,
        resolved.content.unwrap_or_default(),
        resolved.source_file,
    ));
}
```

After applying replacements (after `self.content = next;`), build the source map by walking replacements in forward byte order:

```rust
// Build source map: compute final byte positions for each file transclusion.
// Sort forward by original span start and track cumulative offset.
{
    let mut forward: Vec<_> = replacements
        .iter()
        .map(|(_, span, content, source)| (span.clone(), content.len(), source.clone()))
        .collect();
    forward.sort_by_key(|(span, _, _)| span.start);

    let mut offset: isize = 0;
    for (span, content_len, source_file) in forward {
        let final_start = (span.start as isize + offset) as usize;
        let final_end = final_start + content_len;

        if let Some(file) = source_file {
            report.source_map.push(SourceRange {
                byte_start: final_start,
                byte_end: final_end,
                source_file: file,
                source_start_line: 1,
            });
        }

        offset += content_len as isize - (span.end - span.start) as isize;
    }
}
```

Note: `replacements` was consumed by the `for` loop. To keep the data available for source map construction, collect into a Vec first or clone. The cleanest approach is to change the application loop to borrow:

```rust
if !replacements.is_empty() {
    replacements.sort_by(|left, right| {
        right
            .1
            .start
            .cmp(&left.1.start)
            .then_with(|| right.0.cmp(&left.0))
    });
    let mut next = self.content.clone();
    for (_, span, replacement, _) in &replacements {
        next.replace_range(span.clone(), replacement);
    }
    self.content = next;

    // Build source map from forward-sorted replacements
    let mut forward: Vec<_> = replacements
        .iter()
        .map(|(_, span, content, source)| {
            (span.clone(), content.len(), source.clone())
        })
        .collect();
    forward.sort_by_key(|(span, _, _)| span.start);

    let mut offset: isize = 0;
    for (span, content_len, source_file) in forward {
        let final_start = (span.start as isize + offset) as usize;
        let final_end = final_start + content_len;

        if let Some(file) = source_file {
            report.source_map.push(SourceRange {
                byte_start: final_start,
                byte_end: final_end,
                source_file: file,
                source_start_line: 1,
            });
        }

        offset += content_len as isize - (span.end - span.start) as isize;
    }
}
```

Add import at the top of `mod.rs`:
```rust
use super::types::SourceRange;
```

- [ ] **Step 4: Run darkmatter tests to verify no regressions**

Run: `cargo test -p darkmatter --lib`

Expected: All existing tests PASS.

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/mod.rs
git commit -m "feat(darkmatter): populate source_map during BlockTransclusion

Records byte ranges for file transclusions in the ComposeReport so that
downstream consumers can attribute content to originating files."
```

---

### Task 4: Use source_map in collect_shell_commands for Provenance

**Spec refs:** Group 1b (Finding 4) — discovery consumer

**Files:**
- Modify: `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs`

- [ ] **Step 1: Write failing test — transclusion provenance fidelity**

```rust
#[test]
fn transclusion_provenance_attributes_to_child_file() {
    let temp_dir = TempDir::new().unwrap();

    // Child document with a shell directive
    let child_path = temp_dir.path().join("child.md");
    let mut child_file = std::fs::File::create(&child_path).unwrap();
    writeln!(child_file, "# Child").unwrap();
    writeln!(child_file, "::shell echo from-child").unwrap();

    // Root document with its own directive and a transclusion
    let root_path = temp_dir.path().join("root.md");
    let mut root_file = std::fs::File::create(&root_path).unwrap();
    writeln!(root_file, "::shell echo from-root").unwrap();
    writeln!(root_file, "::file ./child.md").unwrap();

    let root_content = std::fs::read_to_string(&root_path).unwrap();
    let md: Markdown = root_content.into();
    let options = ComposeOptions::new().with_source_file(&root_path);

    let entries = collect_shell_commands(&md, &options).unwrap();

    assert_eq!(entries.len(), 2);

    let root_entry = entries.iter().find(|e| e.raw_command == "echo from-root").unwrap();
    assert_eq!(root_entry.source_file, root_path);

    let child_entry = entries.iter().find(|e| e.raw_command == "echo from-child").unwrap();
    assert_eq!(child_entry.source_file, child_path);
    assert!(child_entry.line > 0, "line should be a valid line number in the child file");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter --lib -- compose::shell_expansion::discovery::tests::transclusion_provenance_attributes_to_child_file`

Expected: FAIL — child entry's `source_file` is `root_path` instead of `child_path`.

- [ ] **Step 3: Update collect_shell_commands to use source_map**

Replace the body of `collect_shell_commands()`:

```rust
pub fn collect_shell_commands(
    markdown: &Markdown,
    options: &ComposeOptions,
) -> MarkdownResult<Vec<ShellCommandEntry>> {
    // Run compose with only interpolation + transclusion (no shell execution).
    // ::shell directives remain as text in the composed output.
    let discovery_options = options.clone().only(&[
        ComposeOperation::FrontmatterInterpolation,
        ComposeOperation::TextReplacement,
        ComposeOperation::PageBlocks,
        ComposeOperation::Interpolation,
        ComposeOperation::BlockTransclusion,
        ComposeOperation::FrontmatterTransclusion,
    ]);

    let (composed, report) = markdown.compose_with(discovery_options)?;

    // Parse ::shell directives from the fully-resolved content.
    let directives = parse_directives(composed.content())?;

    let default_source = match &options.source {
        ComposeSource::File(p) => p.clone(),
        _ => PathBuf::from("<unknown>"),
    };

    // Build entries, resolving aliases and deduplicating by normalized form.
    let mut seen = HashSet::new();
    let mut entries = Vec::new();

    for directive in directives {
        let (executable, args) = if which::which(&directive.executable).is_ok() {
            (directive.executable.clone(), directive.args.clone())
        } else if let Some(resolved) = resolve_alias(&directive.executable) {
            let mut merged_args = resolved.args;
            merged_args.extend_from_slice(&directive.args);
            (resolved.executable, merged_args)
        } else {
            (directive.executable.clone(), directive.args.clone())
        };

        let normalized = normalize_command(&executable, &args);

        if seen.insert(normalized.clone()) {
            // Look up provenance from the source map
            let (source_file, line) = lookup_provenance(
                directive.span.start,
                directive.line,
                &report.source_map,
                composed.content(),
                &default_source,
            );

            entries.push(ShellCommandEntry {
                raw_command: directive.raw_command,
                executable,
                args,
                normalized,
                source_file,
                line,
            });
        }
    }

    Ok(entries)
}

/// Looks up the originating source file for a byte position in composed output.
///
/// Checks the source map for a range containing `byte_pos`. If found, computes
/// the line within the transcluded region by counting newlines. Otherwise falls
/// back to the root document.
fn lookup_provenance(
    byte_pos: usize,
    composed_line: usize,
    source_map: &[crate::markdown::compose::types::SourceRange],
    composed_content: &str,
    default_source: &std::path::Path,
) -> (PathBuf, usize) {
    for range in source_map {
        if byte_pos >= range.byte_start && byte_pos < range.byte_end {
            // Count newlines from the start of the transcluded region to the
            // directive position to get the line number within the source file.
            let region = &composed_content[range.byte_start..byte_pos];
            let relative_line = region.chars().filter(|&c| c == '\n').count();
            return (
                range.source_file.clone(),
                range.source_start_line + relative_line,
            );
        }
    }
    (default_source.to_path_buf(), composed_line)
}
```

Add import at the top of the file:
```rust
use crate::markdown::compose::types::SourceRange;
```

(Only needed if `SourceRange` isn't re-exported through the compose module — check the import path.)

- [ ] **Step 4: Run all discovery tests**

Run: `cargo test -p darkmatter --lib -- compose::shell_expansion::discovery::tests`

Expected: All 9 tests PASS (including the new provenance test).

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs
git commit -m "feat(darkmatter): use source_map for transclusion provenance in discovery

Directives from transcluded files are now attributed to their originating
source file and line, not the root document."
```

---

### Task 5: Preserve Provenance in Claudine Preflight

**Spec refs:** Group 1c (Improvement Idea 1)

**Files:**
- Modify: `claudine/lib/src/composition/preflight.rs`

- [ ] **Step 1: Verify current provenance stripping**

The current code in `resolve_shell_approvals()` only uses `entry.normalized` from `ShellCommandEntry` — it discards `source_file` and `line`. The `ShellApprovalRequest` built in `validate_and_approve_command_parts()` (in `harness/shell.rs`) constructs `source` and `line` from its own context, not from the discovery entry.

For preflight, the approval validation happens in the `for normalized in &unique` loop. The current approach splits the normalized string back into parts and calls `validate_and_approve_command_parts`. This function doesn't receive provenance.

- [ ] **Step 2: Refactor to preserve ShellCommandEntry provenance**

Change the deduplication and validation loop to carry provenance alongside the normalized form:

```rust
pub fn resolve_shell_approvals(
    markdown: Option<&Markdown>,
    compose_options: Option<&ComposeOptions>,
    harness_plan: Option<&HarnessPlan>,
    approval_options: &ShellApprovalOptions,
) -> Result<PreFlightResult, CompositionError> {
    // Collect (normalized, source_file, line) tuples from all sources.
    let mut all_commands: Vec<(String, PathBuf, usize)> = Vec::new();

    // -- Source 1: Template ::shell directives ---------------------------------
    if let (Some(md), Some(opts)) = (markdown, compose_options) {
        let entries = collect_shell_commands(md, opts)
            .map_err(|e| CompositionError::PreFlightDiscoveryFailed(e.to_string()))?;
        for entry in entries {
            all_commands.push((entry.normalized, entry.source_file, entry.line));
        }
    }

    // -- Source 2 & 3: Harness commands ----------------------------------------
    if let Some(plan) = harness_plan {
        let auditable = collect_auditable_commands(plan, None)
            .map_err(|e| CompositionError::PreFlightFailed(e.to_string()))?;
        for cmd in auditable {
            let normalized = normalize_command(&cmd.executable, &cmd.args);
            let source_file = plan.source_path.clone();
            all_commands.push((normalized, source_file, 0));
        }
    }

    // -- Deduplicate -----------------------------------------------------------
    let unique: Vec<(String, PathBuf, usize)> = {
        let mut seen = HashSet::new();
        all_commands
            .into_iter()
            .filter(|(n, _, _)| seen.insert(n.clone()))
            .collect()
    };

    let total_discovered = unique.len();
    let mut approved = HashSet::new();

    let cache_size_before = approval_options
        .approval_cache
        .lock()
        .map(|c| c.len())
        .unwrap_or(0);

    // -- Check each command against policy -------------------------------------
    for (normalized, source_file, line) in &unique {
        let parts: Vec<String> =
            tokenize(normalized).unwrap_or_else(|_| vec![normalized.clone()]);

        match crate::harness::shell::validate_and_approve_command_parts(&parts, approval_options) {
            Ok(_) => {
                approved.insert(normalized.clone());
            }
            Err(crate::harness::error::HarnessError::ShellCommandDenied { command }) => {
                if approval_options.approval_handler.is_some() {
                    return Err(CompositionError::ShellCommandDenied {
                        command,
                        source_file: source_file.clone(),
                        line: *line,
                    });
                }
                return Err(CompositionError::PreFlightFailed(format!(
                    "Shell command '{command}' in {} requires approval but no approval handler \
                     is available. Add to whitelist or run interactively.",
                    source_file.display()
                )));
            }
            Err(crate::harness::error::HarnessError::ShellCommandBlacklisted {
                command,
                reason,
            }) => {
                return Err(CompositionError::PreFlightFailed(format!(
                    "Shell command '{command}' in {} is blacklisted: {reason}",
                    source_file.display()
                )));
            }
            Err(e) => {
                return Err(CompositionError::PreFlightFailed(e.to_string()));
            }
        }
    }

    let cache_size_after = approval_options
        .approval_cache
        .lock()
        .map(|c| c.len())
        .unwrap_or(0);
    let user_approved = cache_size_after.saturating_sub(cache_size_before);
    let already_whitelisted = total_discovered.saturating_sub(user_approved);

    Ok(PreFlightResult {
        approved_commands: approved,
        total_discovered,
        already_whitelisted,
        user_approved,
    })
}
```

Note: This requires updating `CompositionError::ShellCommandDenied` to carry `source_file` and `line` (done in Task 10).

**For now**, keep the existing `ShellCommandDenied { command }` form and add the fields in Task 10. The provenance data is now flowing through correctly; error enrichment comes later.

- [ ] **Step 3: Run claudine-lib tests**

Run: `cargo test -p claudine --lib`

Expected: All tests PASS. Some tests may need minor updates if they match on error messages that now include file paths.

- [ ] **Step 4: Commit**

```bash
git add claudine/lib/src/composition/preflight.rs
git commit -m "feat(claudine): preserve provenance through preflight approval loop

ShellCommandEntry source_file and line are now carried through dedup and
error reporting instead of being discarded."
```

---

### Task 6: Make Harness Parsing Discovery-Only

**Spec refs:** Group 2a (Finding 2, Improvement 3)

**Files:**
- Modify: `claudine/lib/src/harness/parse.rs`
- Modify: `claudine/lib/src/harness/mod.rs`
- Modify: `claudine/cli/src/commands/wrap/mod.rs`
- Modify: `claudine/cli/src/commands/wrap/composition.rs`

- [ ] **Step 1: Remove parse_runtime_command and parse_runtime_command_parts**

In `parse.rs`, delete the two functions (lines 912-935) and replace all their call sites with `tokenize_to_approved_command`:

**Line 426** — inside a check parsing function:
```rust
// Before:
let command = parse_runtime_command(&raw, source_path, shell_options)?;
// After:
let command = tokenize_to_approved_command(&raw, source_path)?;
```

**Line 620** — inside handler/parts parsing:
```rust
// Before:
parse_runtime_command_parts(&parts, shell_options)
// After:
Ok(ApprovedRuntimeCommand {
    raw: parts.join(" "),
    executable: parts[0].clone(),
    args: parts[1..].to_vec(),
})
```

**Line 622** — string variant:
```rust
// Before:
Value::String(s) => parse_runtime_command(s, source_path, shell_options),
// After:
Value::String(s) => tokenize_to_approved_command(s, source_path),
```

**Line 853** — handler command parsing:
```rust
// Before:
let command = parse_runtime_command(&cmd_str, source_path, shell_options)?;
// After:
let command = tokenize_to_approved_command(&cmd_str, source_path)?;
```

- [ ] **Step 2: Remove shell_options parameter from parse_checks and internal functions**

Remove the `shell_options: Option<&ShellApprovalOptions>` parameter from:
- `parse_checks()`
- Any internal function that accepted it only to pass to `parse_runtime_command`/`parse_runtime_command_parts`

Update all call sites within `parse.rs` to stop passing `shell_options`.

- [ ] **Step 3: Merge parse_harness_plan_with_shell into parse_harness_plan**

Replace the two-function pattern with a single function:

```rust
/// Parse composed frontmatter into a [`HarnessPlan`].
pub fn parse_harness_plan(
    frontmatter: &Value,
    source_path: &Path,
    ctx: &HarnessResolutionContext<'_>,
) -> Result<HarnessPlan, HarnessError> {
    let obj = frontmatter
        .as_object()
        .ok_or_else(|| HarnessError::InvalidFrontmatter {
            source_path: source_path.to_path_buf(),
            property: "(root)".to_string(),
            detail: "frontmatter must be an object".to_string(),
        })?;

    // ... rest of the body from parse_harness_plan_with_shell,
    // but without shell_options parameter and without passing it to parse_checks ...
}
```

Delete `parse_harness_plan_with_shell`.

- [ ] **Step 4: Remove unused imports**

In `parse.rs`, remove:
```rust
use crate::harness::shell::{
    ShellApprovalOptions, validate_and_approve_command, validate_and_approve_command_parts,
};
```

Keep only what's still needed (likely nothing from `shell` module).

- [ ] **Step 5: Update mod.rs re-exports**

In `claudine/lib/src/harness/mod.rs`, remove `parse_harness_plan_with_shell` from the `pub use parse::{...}` block:

```rust
pub use parse::{
    has_harness_properties, inline_writability_pre_check, parse_harness_plan,
};
```

- [ ] **Step 6: Update CLI call sites**

In `claudine/cli/src/commands/wrap/mod.rs`, change all `parse_harness_plan_with_shell` calls to `parse_harness_plan`:

**Line 1126:**
```rust
// Before:
let plan = claudine::harness::parse_harness_plan_with_shell(
    &seed.frontmatter,
    &source_path,
    &resolve_ctx,
    Some(&shell_options),
)
// After:
let plan = claudine::harness::parse_harness_plan(
    &seed.frontmatter,
    &source_path,
    &resolve_ctx,
)
```

**Line 2196:**
```rust
// Before:
let mut plan = claudine::harness::parse_harness_plan_with_shell(
    &materialized.frontmatter,
    &prompt_state.source_path,
    &resolve_ctx,
    Some(harness_context.shell_options()),
)
// After:
let mut plan = claudine::harness::parse_harness_plan(
    &materialized.frontmatter,
    &prompt_state.source_path,
    &resolve_ctx,
)
```

In `claudine/cli/src/commands/wrap/composition.rs`, line 429:
```rust
// Before:
let mut plan = claudine::harness::parse_harness_plan_with_shell(
    &request.prepared.effective_frontmatter,
    &request.prepared.resolved_path,
    &resolve_ctx,
    Some(&shell_options),
)
// After:
let mut plan = claudine::harness::parse_harness_plan(
    &request.prepared.effective_frontmatter,
    &request.prepared.resolved_path,
    &resolve_ctx,
)
```

- [ ] **Step 7: Run all claudine tests**

Run: `cargo test -p claudine --lib && cargo test -p claudine-cli`

Expected: All tests PASS. Parse tests still work because `parse_harness_plan` (without shell options) was the path they already exercised.

- [ ] **Step 8: Commit**

```bash
git add claudine/lib/src/harness/parse.rs claudine/lib/src/harness/mod.rs \
        claudine/cli/src/commands/wrap/mod.rs claudine/cli/src/commands/wrap/composition.rs
git commit -m "refactor(claudine): make harness parsing discovery-only

Remove shell_options from parse_harness_plan. All approval decisions now
happen in resolve_shell_approvals, the single preflight authority."
```

---

### Task 7: Wire Interactive Approval Handler

**Spec refs:** Group 2c (Finding 1, Improvement 2)

**Files:**
- Modify: `claudine/cli/Cargo.toml`
- Modify: `claudine/cli/src/commands/wrap/mod.rs`

- [ ] **Step 1: Add darkmatter-cli dependency**

In `claudine/cli/Cargo.toml`, add under `[dependencies]`:

```toml
darkmatter-cli = { path = "../../darkmatter/cli" }
```

- [ ] **Step 2: Update build_harness_shell_options to accept interactive flag**

```rust
pub(crate) fn build_harness_shell_options(
    source_path: &Path,
    repo_root: Option<&Path>,
    interactive: bool,
) -> claudine::harness::ShellApprovalOptions {
    claudine::harness::ShellApprovalOptions {
        policy_root: harness_policy_root(source_path, repo_root),
        approval_handler: if interactive {
            Some(std::sync::Arc::new(
                darkmatter_cli::approval::CliShellApprovalHandler,
            ))
        } else {
            None
        },
        ..Default::default()
    }
}
```

- [ ] **Step 3: Update all call sites of build_harness_shell_options**

**In `CachedHarnessLoopContext::new()` (line 280):**
```rust
fn new(source_path: &Path, repo_root: Option<&Path>, interactive: bool) -> Self {
    Self {
        source_path: source_path.to_path_buf(),
        repo_root: repo_root.map(Path::to_path_buf),
        shell_options: build_harness_shell_options(source_path, repo_root, interactive),
    }
}
```

**In `CachedHarnessLoopContext::refresh()` (line 284):**

When refreshing, only update `policy_root` — keep the existing `approval_handler` and `approval_cache`:

```rust
fn refresh(&mut self, source_path: &Path, repo_root: Option<&Path>) {
    let repo_root = repo_root.map(Path::to_path_buf);
    if self.source_path != source_path || self.repo_root != repo_root {
        self.source_path = source_path.to_path_buf();
        self.repo_root = repo_root;
        self.shell_options.policy_root =
            harness_policy_root(&self.source_path, self.repo_root.as_deref());
    }
}
```

**In the passthrough wrapper path (line 1124):**
```rust
let shell_options = build_harness_shell_options(
    &source_path,
    env_plan.repo_root.as_deref(),
    !effective_non_interactive,
);
```

**In `composition.rs` (line 426):**
```rust
let shell_options = build_harness_shell_options(
    &request.prepared.resolved_path,
    effective_repo_root,
    request.session_interactive,
);
```

- [ ] **Step 4: Run build to verify**

Run: `cargo build -p claudine-cli`

Expected: Build succeeds.

- [ ] **Step 5: Commit**

```bash
git add claudine/cli/Cargo.toml claudine/cli/src/commands/wrap/mod.rs \
        claudine/cli/src/commands/wrap/composition.rs
git commit -m "feat(claudine): wire interactive approval handler for shell commands

Interactive sessions use CliShellApprovalHandler to prompt for unapproved
commands. Non-interactive sessions hard-fail as before."
```

---

### Task 8: Carry Preflight Approvals Through to Runtime

**Spec refs:** Group 2b (Finding 3)

**Files:**
- Modify: `claudine/cli/src/commands/wrap/mod.rs`

- [ ] **Step 1: Pass preflight shell_options into run_harness_loop**

Add `shell_options: claudine::harness::ShellApprovalOptions` parameter to `run_harness_loop`:

```rust
pub(crate) fn run_harness_loop(
    provider: Provider,
    profile: &dyn WrapperProfile,
    binary_path: &Path,
    child_cwd: &Path,
    effective_non_interactive: bool,
    cli_timeout: Option<u64>,
    base_args: &[String],
    base_env: &HashMap<OsString, OsString>,
    prompt_state: &mut HarnessPromptState,
    repo_root: Option<&Path>,
    shell_options: claudine::harness::ShellApprovalOptions,  // <-- new
    use_structured: bool,
    // ... rest unchanged
```

- [ ] **Step 2: Use provided shell_options in CachedHarnessLoopContext**

At the top of `run_harness_loop` (line 2184), change:

```rust
// Before:
let mut harness_context = CachedHarnessLoopContext::new(&prompt_state.source_path, repo_root);
// After:
let mut harness_context = CachedHarnessLoopContext::with_shell_options(
    &prompt_state.source_path,
    repo_root,
    shell_options,
);
```

Add a new constructor to `CachedHarnessLoopContext`:

```rust
fn with_shell_options(
    source_path: &Path,
    repo_root: Option<&Path>,
    shell_options: claudine::harness::ShellApprovalOptions,
) -> Self {
    Self {
        source_path: source_path.to_path_buf(),
        repo_root: repo_root.map(Path::to_path_buf),
        shell_options,
    }
}
```

- [ ] **Step 3: Update passthrough wrapper call site to pass shell_options**

In the passthrough wrapper path (around line 1178), the `shell_options` was built earlier for preflight. Pass it through:

```rust
run_harness_loop(
    provider,
    profile,
    binary_path.as_path(),
    child_cwd,
    effective_non_interactive,
    args.timeout,
    &harness_base_args,
    &env_plan.env,
    &mut prompt_state,
    env_plan.repo_root.as_deref(),
    shell_options,  // <-- the same instance used for preflight
    use_structured,
    // ... rest unchanged
```

Ensure `shell_options` is not consumed before this point (it may need to be cloned if used by reference earlier, but since `ShellApprovalOptions` is `Clone` and `approval_cache` is `Arc`, cloning preserves the shared cache).

- [ ] **Step 4: Update the composition harness loop call site if it calls run_harness_loop**

Check if `run_harness_loop` is called from anywhere else. If so, pass the appropriate `shell_options`.

- [ ] **Step 5: Run build and tests**

Run: `cargo build -p claudine-cli && cargo test -p claudine-cli`

Expected: Build and tests pass.

- [ ] **Step 6: Commit**

```bash
git add claudine/cli/src/commands/wrap/mod.rs
git commit -m "feat(claudine): carry preflight approval cache through to runtime

The same ShellApprovalOptions instance used for preflight is passed to
run_harness_loop, so AllowOnce decisions survive into the harness loop."
```

---

### Task 9: Enable Harness Preflight for Interactive Wrappers

**Spec refs:** Group 3a (Finding 6)

**Files:**
- Modify: `claudine/cli/src/commands/wrap/mod.rs`

- [ ] **Step 1: Remove effective_non_interactive guard**

At line 1109, the harness preflight is guarded by `if effective_non_interactive`. Remove the guard so it runs unconditionally:

```rust
// Before:
let wrapper_harness = if effective_non_interactive {
    // ... harness detection, parse, preflight ...
} else {
    None
};

// After:
let wrapper_harness = {
    let base_prompt =
        extract_prompt_from_child_args(provider, &child_args, stdin_seed.as_deref());
    let harness_source = base_prompt.as_ref().and_then(|_| {
        find_wrapper_harness_source(provider, env_plan.repo_root.as_deref(), &cwd)
    });

    if let (Some(base_prompt), Some(source_path)) = (base_prompt, harness_source) {
        let seed = materialize_passthrough_harness_seed(&source_path, base_prompt.clone())?;
        let harness_enabled = claudine::harness::has_harness_properties(&seed.frontmatter);
        if harness_enabled {
            let resolve_ctx = claudine::harness::HarnessResolutionContext {
                source_path: &source_path,
                repo_root: env_plan.repo_root.as_deref(),
            };
            let shell_options = build_harness_shell_options(
                &source_path,
                env_plan.repo_root.as_deref(),
                !effective_non_interactive,  // interactive when NOT non-interactive
            );
            let plan = claudine::harness::parse_harness_plan(
                &seed.frontmatter,
                &source_path,
                &resolve_ctx,
            )
            .map_err(|e| eyre!("{e}"))?;

            // Pre-flight harness shell commands — runs for both interactive
            // and non-interactive sessions.
            let _harness_preflight = claudine::composition::resolve_shell_approvals(
                None,
                None,
                Some(&plan),
                &shell_options,
            )
            .map_err(|e| eyre!("{e}"))?;

            drop(plan);

            Some((source_path, base_prompt, seed, shell_options))
        } else {
            None
        }
    } else {
        None
    }
};
```

Note: the tuple now carries `shell_options` so it can be passed to `run_harness_loop`. Update the destructuring at the `if let Some(...)` below (around line 1159):

```rust
let exit_code = if let Some((source_path, base_prompt, initial_materialized, shell_options)) = wrapper_harness
{
    // ... existing code ...
    run_harness_loop(
        // ... pass shell_options ...
    )
```

- [ ] **Step 2: Handle the case where no prompt is provided (interactive sessions)**

Interactive sessions may not have a base_prompt in args. The `extract_prompt_from_child_args` may return `None`. In that case, `wrapper_harness` will be `None` and the interactive session proceeds without harness preflight. This is correct — harness preflight requires a prompt to detect the harness source file.

- [ ] **Step 3: Run tests**

Run: `cargo test -p claudine-cli`

Expected: All tests PASS.

- [ ] **Step 4: Commit**

```bash
git add claudine/cli/src/commands/wrap/mod.rs
git commit -m "feat(claudine): run harness preflight for interactive wrapper sessions

Preflight now runs unconditionally when a harness is detected, regardless
of interactive/non-interactive mode. Interactive sessions prompt via
CliShellApprovalHandler."
```

---

### Task 10: Enrich Error Reporting

**Spec refs:** Group 3b (Finding 7)

**Files:**
- Modify: `claudine/lib/src/composition/error.rs`
- Modify: `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`
- Modify: `claudine/lib/src/composition/preflight.rs`

- [ ] **Step 1: Enrich ShellCommandDenied in composition error**

In `claudine/lib/src/composition/error.rs`:

```rust
    /// The user denied a shell command during pre-flight approval.
    #[error(
        "Aborted: shell command '{command}' was denied during pre-flight approval \
         (source: {source_file}, line {line}). No provider session was started."
    )]
    ShellCommandDenied {
        command: String,
        source_file: PathBuf,
        line: usize,
    },
```

Add `use std::path::PathBuf;` at the top of `error.rs`.

- [ ] **Step 2: Update ShellCommandDenied construction in preflight.rs**

The construction in `resolve_shell_approvals()` (Task 5 already updated) should now pass `source_file` and `line`:

```rust
return Err(CompositionError::ShellCommandDenied {
    command,
    source_file: source_file.clone(),
    line: *line,
});
```

- [ ] **Step 3: Enrich NotPreApproved in darkmatter**

In `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`:

```rust
    #[error(
        "Command '{command}' on line {line} was not pre-approved{source_desc}. \
         This is a bug in the pre-flight scanner -- please report it."
    )]
    NotPreApproved {
        command: String,
        line: usize,
        source_desc: String,
    },
```

Update the construction site in `shell_expansion/mod.rs` (around line 107):

```rust
return Err(ShellExpansionError::NotPreApproved {
    command: display_command(directive, alias_name.as_deref()),
    line: directive.line,
    source_desc: match &options.source {
        Some(ComposeSource::File(p)) => format!(" (in {})", p.display()),
        _ => String::new(),
    },
});
```

Note: check what `options` is available at that site. It should be the `ComposeOptions` or `ShellExpansionOptions`. Look up the enclosing function signature to determine the source.

- [ ] **Step 4: Update any tests that match on error message strings**

Search for tests matching on `ShellCommandDenied` or `NotPreApproved` error text and update the expected patterns to account for new fields.

- [ ] **Step 5: Run all tests across both packages**

Run: `cargo test -p darkmatter --lib && cargo test -p claudine --lib && cargo test -p claudine-cli`

Expected: All tests PASS.

- [ ] **Step 6: Commit**

```bash
git add claudine/lib/src/composition/error.rs claudine/lib/src/composition/preflight.rs \
        darkmatter/lib/src/markdown/compose/shell_expansion/types.rs \
        darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
git commit -m "feat(claudine): enrich error reporting with source provenance

ShellCommandDenied includes source_file and line. NotPreApproved includes
source description for better debugging of scanner bugs."
```

---

### Task 11: Preflight Approval Decision Tests

**Spec refs:** Group 4b

**Files:**
- Modify: `claudine/lib/src/composition/preflight.rs` (tests module)

- [ ] **Step 1: Add mock approval handler**

In the tests module:

```rust
use std::sync::{Arc, Mutex};
use darkmatter::markdown::compose::shell_expansion::types::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
};

struct MockApprovalHandler {
    decision: ShellApprovalDecision,
    call_count: Arc<Mutex<usize>>,
}

impl MockApprovalHandler {
    fn new(decision: ShellApprovalDecision) -> Self {
        Self {
            decision,
            call_count: Arc::new(Mutex::new(0)),
        }
    }

    fn calls(&self) -> usize {
        *self.call_count.lock().unwrap()
    }
}

impl ShellApprovalHandler for MockApprovalHandler {
    fn approve(
        &self,
        _request: ShellApprovalRequest,
    ) -> Result<ShellApprovalDecision, ShellExpansionError> {
        *self.call_count.lock().unwrap() += 1;
        Ok(self.decision.clone())
    }
}
```

- [ ] **Step 2: Test AllowOnce populates cache but does not persist**

```rust
#[test]
fn allow_once_populates_cache_without_persisting() {
    let md: Markdown = "# Test\n::shell echo test-once\n".into();
    let compose_options = ComposeOptions::new();

    let dir = tempfile::TempDir::new().unwrap();
    let handler = Arc::new(MockApprovalHandler::new(ShellApprovalDecision::AllowOnce));
    let options = ShellApprovalOptions {
        policy_root: Some(dir.path().to_path_buf()),
        approval_handler: Some(handler.clone()),
        ..Default::default()
    };

    let result =
        resolve_shell_approvals(Some(&md), Some(&compose_options), None, &options).unwrap();

    assert_eq!(result.total_discovered, 1);
    assert_eq!(result.user_approved, 1);
    assert!(result.approved_commands.contains("echo test-once"));

    // Verify no whitelist file was created/modified
    let whitelist_path = dir.path().join(".darkmatter-shell-whitelist");
    assert!(!whitelist_path.exists() || std::fs::read_to_string(&whitelist_path).unwrap().is_empty());
}
```

- [ ] **Step 3: Test Deny produces ShellCommandDenied**

```rust
#[test]
fn deny_returns_shell_command_denied_error() {
    let md: Markdown = "# Test\n::shell echo test-deny\n".into();
    let compose_options = ComposeOptions::new();

    let dir = tempfile::TempDir::new().unwrap();
    let handler = Arc::new(MockApprovalHandler::new(ShellApprovalDecision::Deny));
    let options = ShellApprovalOptions {
        policy_root: Some(dir.path().to_path_buf()),
        approval_handler: Some(handler.clone()),
        ..Default::default()
    };

    let result =
        resolve_shell_approvals(Some(&md), Some(&compose_options), None, &options);

    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), CompositionError::ShellCommandDenied { .. }),
        "expected ShellCommandDenied"
    );
}
```

- [ ] **Step 4: Test warm cache propagation**

```rust
#[test]
fn warm_cache_prevents_second_handler_invocation() {
    let md: Markdown = "# Test\n::shell echo cached\n".into();
    let compose_options = ComposeOptions::new();

    let dir = tempfile::TempDir::new().unwrap();
    let handler = Arc::new(MockApprovalHandler::new(ShellApprovalDecision::AllowOnce));
    let options = ShellApprovalOptions {
        policy_root: Some(dir.path().to_path_buf()),
        approval_handler: Some(handler.clone()),
        ..Default::default()
    };

    // First call: handler invoked, command cached
    let result1 =
        resolve_shell_approvals(Some(&md), Some(&compose_options), None, &options).unwrap();
    assert_eq!(result1.user_approved, 1);
    assert_eq!(handler.calls(), 1);

    // Second call with same options: cache hit, handler NOT invoked
    let result2 =
        resolve_shell_approvals(Some(&md), Some(&compose_options), None, &options).unwrap();
    assert_eq!(result2.total_discovered, 1);
    assert_eq!(handler.calls(), 1, "handler should not be called again — cache hit");
}
```

- [ ] **Step 5: Run preflight tests**

Run: `cargo test -p claudine --lib -- composition::preflight::tests`

Expected: All tests PASS.

- [ ] **Step 6: Commit**

```bash
git add claudine/lib/src/composition/preflight.rs
git commit -m "test(claudine): add preflight approval decision and cache tests

Covers AllowOnce, Deny, and warm cache propagation via mock handler."
```

---

### Task 12: Final Integration Verification

- [ ] **Step 1: Run full darkmatter test suite**

Run: `just test` from `darkmatter/`

Expected: All tests PASS.

- [ ] **Step 2: Run full claudine test suite**

Run: `just test` from `claudine/`

Expected: All tests PASS.

- [ ] **Step 3: Run lint**

Run: `just lint` from repo root (or `cargo clippy -p darkmatter -p claudine -p claudine-cli`)

Expected: No warnings.

- [ ] **Step 4: Commit any fixups**

If lint or tests revealed issues, fix and commit.

---

## Deferred (Out of Scope)

Per the spec:

- **Retry/redirect/deviate flows** preserving preflight shell approvals across harness attempts — deeper harness lifecycle concern.
- **Cached document graph** sharing between `collect_shell_commands()` and the subsequent compose pass — performance optimization gated on observed startup latency.
