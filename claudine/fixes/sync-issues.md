# Symlink Fix Commands — Broken Symlink Handling Review

**Reviewer:** Claudine subsystem audit
**Date:** 2026-04-18
**Branch:** `claudine`
**Scope:** `claudine skills --fix`, `claudine commands --fix`, `claudine agents --fix`, and `claudine sync`

## 1. Executive Summary

The three `--fix` commands (`skills --fix`, `commands --fix`, `agents --fix`) are **incomplete** in their symlink remediation. They correctly create missing symlinks but **do not detect or repair broken symlinks** — symlinks that exist at the correct path and point to the correct relative/absolute target, but where the target no longer exists.

`claudine sync` is **unrelated** — it manages hook registrations only, not symlinks. Its `--fix` flag is a no-op kept for CLI compatibility.

---

## 2. Affected Code Map

| Command | CLI File | Fix Function | Symlink Backend |
|---------|----------|--------------|-----------------|
| `claudine skills --fix` | `cli/src/commands/skills.rs:56–60` | `linking/skills.rs:137–214` | `linking/symlink.rs:157–166` |
| `claudine commands --fix` | `cli/src/commands/slash_commands.rs:52–56` | `linking/commands.rs:147–240` | `linking/symlink.rs:50–144` |
| `claudine agents --fix` | `cli/src/commands/agents.rs:52–56` | `linking/agents.rs:138–217` | `linking/symlink.rs:157–166` |
| `claudine sync` | `cli/src/commands/sync.rs:207–376` | N/A (hook registration) | N/A |

---

## 3. Bug 1 — Broken Symlinks Are Not Repaired

### 3.1 Symptom

A skill/command/agent symlink exists in a provider directory, but its target has been deleted from the Claude source directory. Running `claudine skills --fix` reports the symlink as `already_linked` and makes no change. The broken symlink remains, confusing the agent provider.

### 3.2 Root Cause

**In `create_resource_link` (`symlink.rs:88–115`):**

```rust
// If a symlink already exists, check if it points to our source
if dest
    .symlink_metadata()
    .map(|m| m.file_type().is_symlink())
    .unwrap_or(false)
{
    let existing_target = fs::read_link(&dest)?;   // ← returns target path
    let expected = match scope { ... };             // ← absolute or relative path

    if existing_target == expected {
        return Ok(LinkResult::AlreadyLinked);      // ← BUG: no existence check
    }

    return Ok(LinkResult::Skipped { ... });
}
```

The function confirms the symlink **points to** the correct path, but never confirms the target **actually exists**. If the source was deleted, `read_link()` still succeeds (it only reads the symlink metadata, not the target), and the function returns `AlreadyLinked`.

### 3.3 Secondary Factor — `check_scope_missing` Does Not Detect Broken Symlinks

**In `skills.rs:508–521`:**

```rust
for name in canonical.keys() {
    let expected = provider_dir.join(name);
    if !expected.exists() || !expected.join("SKILL.md").exists() {  // ← line 510
        exceptions.push(SkillException { ... });
    }
}
```

`Path::exists()` returns `false` for broken symlinks (it follows the symlink). So broken symlinks are **silently invisible** to the `list` command — they produce no exception, no diagnostic, nothing. The user sees no indication anything is wrong until the agent provider fails to read the resource.

### 3.4 Severity

**Major.** Broken symlinks cause agent providers to silently fail to load skills/commands/agents with no user-visible error from `claudine`. The agent may emit cryptic errors or behave as if the resource doesn't exist.

---

## 4. Bug 2 — `create_skill_link` Never Overwrites Any Existing Symlink

### 4.1 Symptom

When a broken symlink exists at the destination, re-running `--fix` does not repair it.

### 4.2 Root Cause

The logic in `create_resource_link` (lines 74–85) only protects against overwriting **real files/directories**:

```rust
if dest.exists()
    && !dest.symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
{
    // Real file/dir — skip
    return Ok(LinkResult::Skipped { ... });
}

// Symlink path reaches line 88
if dest.symlink_metadata()... {
    // Existing symlink — check target path only, not existence
    ...
}
```

When `dest` is a broken symlink:
1. `dest.exists()` → `false` (follows symlink, target missing) — **first branch skipped**
2. `dest.symlink_metadata()` → `Ok(Metadata)` — **second branch entered**
3. `fs::read_link()` → succeeds, returns stale target path
4. If target path matches expected → `AlreadyLinked` — **no repair**

The function never removes and recreates a broken symlink. There is no code path that calls `fs::remove_file(&dest)` for an existing symlink.

### 4.3 Severity

**Major.** No recovery mechanism exists once a symlink becomes broken.

---

## 5. Bug 3 — `claudine sync --fix` Is a No-Op

### 5.1 Current Behavior

In `sync.rs:370–373`:

```rust
// With ClaudineConfig, events are provider-agnostic — every event applies
// to all providers. Events that a provider doesn't support simply won't
// fire for that provider. This is expected, not a configuration error.
// The --fix flag is preserved for CLI compatibility but is not actionable.
```

The `--fix` flag on `sync` was once intended to remove unsupported events from config, but this functionality was removed. The flag remains in the CLI struct (`sync.rs:35–36`) but does nothing.

### 5.2 Severity

**Minor.** The flag is documented as non-functional in a comment. However, users who run `claudine sync --fix` expecting stale event cleanup will be misled.

---

## 6. `claudine sync` Is Unrelated to Symlinks

`claudine sync` manages **hook registrations** — it registers and deregisters event hooks with agent providers. It reads `ClaudineConfig`, calls `configurator.register()` / `configurator.deregister()`, and reports what changed.

It does **not** interact with:
- Skill symlinks
- Command symlinks
- Agent symlinks
- The `linking/` module at all

The only shared component is the `ProviderSkillPaths` used to resolve config locations.

---

## 7. Design: How Broken Symlink Detection Should Work

### 7.1 Detecting Broken Symlinks

`check_scope_missing` (and its counterparts for commands/agents) should use `symlink_metadata().is_ok_and(|m| m.file_type().is_symlink())` followed by checking whether the link target **exists**, not just whether the symlink itself exists:

```rust
// In check_scope_missing for skills:
let expected = provider_dir.join(name);
if !expected.exists() {
    // Two cases: truly missing, or broken symlink
    if expected.symlink_metadata().is_ok_and(|m| m.file_type().is_symlink()) {
        // It's a symlink — check if target exists
        if let Ok(target) = fs::read_link(&expected) {
            if !target.exists() {
                // Broken symlink — report it differently from "truly missing"
                exceptions.push(SkillException {
                    exception_type: ExceptionType::BrokenSymlink,  // new variant needed
                    ...
                });
            }
        }
    } else {
        // Truly missing
        exceptions.push(SkillException {
            exception_type: ExceptionType::Missing,
            ...
        });
    }
}
```

### 7.2 New ExceptionType Variant

`ExceptionType::BrokenSymlink` (and equivalents in commands/agents) would distinguish "target deleted" from "never linked":

```rust
// In skills.rs
ExceptionType::BrokenSymlink,  // symlink exists but target is gone
```

### 7.3 Fixing Broken Symlinks

`create_resource_link` should detect when an existing symlink is broken and repair it:

```rust
if dest.symlink_metadata()
    .is_ok_and(|m| m.file_type().is_symlink())
{
    let existing_target = fs::read_link(&dest)?;
    if existing_target == expected {
        // Symlink points to correct path — but is it valid?
        if !expected.exists() {
            // Broken but correct — remove and recreate
            fs::remove_file(&dest)?;
            // fall through to creation at line 117
        } else {
            return Ok(LinkResult::AlreadyLinked);
        }
    } else {
        return Ok(LinkResult::Skipped { ... });
    }
}
```

### 7.4 Summary Fix Counters

A new counter `links_repaired` should be added to `*FixSummary` structs:

```rust
pub struct SkillFixSummary {
    // ... existing fields ...
    /// Symlinks that were broken and repaired.
    pub links_repaired: usize,
}
```

---

## 8. Test Gaps

No existing tests cover broken symlink scenarios. The following test cases are missing:

1. **`create_skill_link` with broken symlink returns `AlreadyLinked`** (current behavior, should change)
2. **`fix_missing_skills` repairs broken symlinks** (desired behavior, unimplemented)
3. **`list_skills` reports `BrokenSymlink` exception for broken symlinks** (desired behavior, unimplemented)
4. **`fix_missing_commands` repairs broken command symlinks**
5. **`fix_missing_agents` repairs broken agent symlinks**

---

## 9. Recommendations

### 9.1 Immediate (Low Risk)

1. **Add `BrokenSymlink` exception type** to `ExceptionType` / `CommandExceptionType` / `AgentExceptionType` — distinguishable from `Missing`
2. **Update `check_scope_missing`** to detect broken symlinks using `symlink_metadata` + `read_link` + existence check
3. **Document** that `sync --fix` is a no-op (add to help text or remove)

### 9.2 Short Term (Medium Risk)

4. **Modify `create_resource_link`** to detect and repair broken symlinks (remove + recreate)
5. **Add `links_repaired` counter** to fix summaries
6. **Add comprehensive tests** for broken symlink scenarios in all three fix functions

### 9.3 Out of Scope

- `claudine sync` hook registration (unrelated to symlinks)
- The `sync --fix` flag cleanup (cosmetic only)

---

## 10. Files Requiring Changes

| File | Change |
|------|--------|
| `lib/src/linking/skills.rs` | Add `BrokenSymlink` to `ExceptionType`; update `check_scope_missing`; add `links_repaired` to `SkillFixSummary` |
| `lib/src/linking/commands.rs` | Add `BrokenSymlink` to `CommandExceptionType`; update `check_scope_missing`; add `links_repaired` to `CommandFixSummary` |
| `lib/src/linking/agents.rs` | Add `BrokenSymlink` to `AgentExceptionType`; update `check_scope_missing`; add `links_repaired` to `AgentFixSummary` |
| `lib/src/linking/symlink.rs` | Modify `create_resource_link` to repair broken symlinks |
| `cli/src/commands/sync.rs` | Add note in help text that `--fix` is a no-op, or remove the flag |

---

## 11. Open Questions

1. **Should `--fix` also remove symlinks for resources that no longer exist in Claude's source?** Currently it only creates; it could also delete orphaned symlinks in provider directories.

2. **Should `list` commands show broken symlink exceptions by default, or only when `--verbose`?** Currently broken symlinks are invisible. They could be surfaced as a warning-level exception so users notice without running `--fix`.

3. **Should `sync` invoke `skills/commands/agents --fix` automatically?** If a user runs `claudine sync` expecting comprehensive cleanup, they may be surprised that broken symlinks persist. A combined `--fix-all` flag could address this.
