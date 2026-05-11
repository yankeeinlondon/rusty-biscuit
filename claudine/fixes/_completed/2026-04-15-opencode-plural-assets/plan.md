# Plan: Make OpenCode Plugin Directory Naming Plural-Consistent

## Summary

Rename the singular `plugin` directory references to plural `plugins` in the OpenCode configurator to match OpenCode's canonical naming convention.

## Confidence: HIGH

Verified against:
- OpenCode official docs: `~/.config/opencode/plugins/` and `.opencode/plugins/` are the documented paths
- OpenCode config docs explicitly state: *"The `.opencode` and `~/.config/opencode` directories use **plural names** for subdirectories: `agents/`, `commands/`, `modes/`, `plugins/`, `skills/`, `tools/`, and `themes/`. Singular names (e.g., `agent/`) are also supported for backwards compatibility."*
- In-repo `agents/opencode.rs:189` already uses plural: `plugin_dirs: path_vec(&[".opencode/plugins", "~/.config/opencode/plugins"])`

## Scope

**Single file**: `claudine/lib/src/config/opencode.rs`

No other files in the codebase use the singular `plugin` path for OpenCode. The `agents/opencode.rs` file already uses plural correctly.

## Changes

### 1. Rename function `plugin_dir` → `plugins_dir` (line 263)

```rust
// Before
fn plugin_dir(config_dir: Option<&Path>) -> PathBuf {
    match config_dir {
        Some(dir) => dir.join("plugin"),
        None => {
            let home = dirs::home_dir().unwrap_or_default();
            home.join(".config").join("opencode").join("plugin")
        }
    }
}

// After
fn plugins_dir(config_dir: Option<&Path>) -> PathBuf {
    match config_dir {
        Some(dir) => dir.join("plugins"),
        None => {
            let home = dirs::home_dir().unwrap_or_default();
            home.join(".config").join("opencode").join("plugins")
        }
    }
}
```

### 2. Update all call sites (4 occurrences)

Replace `plugin_dir(` with `plugins_dir(` at:
- Line 159: `let plugin_path = plugin_dir(config_dir).join(PLUGIN_FILENAME);`
- Line 182: `let plugin_path = plugin_dir(config_dir).join(PLUGIN_FILENAME);`
- Line 191: `let plugin_path = plugin_dir(config_dir).join(PLUGIN_FILENAME);`
- Line 197: `let bridge_path = plugin_dir(config_dir).join(PLUGIN_FILENAME);`

### 3. Update doc comment on function (line 262)

```rust
// Before
/// Resolve the plugin directory path.

// After
/// Resolve the plugins directory path.
```

### 4. Update inline comments (3 locations)

- Line 156: `"in the plugin directory"` → `"in the plugins directory"`
- Line 180: `"from the plugin directory"` → `"from the plugins directory"`
- Line 189: `"from the plugin directory automatically"` → `"from the plugins directory automatically"`

### 5. Update tests (4 locations)

Replace `tmp.path().join("plugin")` with `tmp.path().join("plugins")` at:
- Line 343: `let plugin_path = tmp.path().join("plugin").join("claudine-bridge.ts");`
- Line 368: `let bridge_path = tmp.path().join("plugin").join("claudine-bridge.ts");`
- Line 391: `let plugin_dir = tmp.path().join("plugin");`
- Line 405: `let plugin_dir = tmp.path().join("plugin");`

### 6. Update test-local variable name (optional, low priority)

In tests that use `let plugin_dir = ...`, rename to `let plugins_dir = ...` for consistency. This affects lines 391 and 405 where the local variable shadows the now-renamed function.

## NOT in scope

- **`PLUGIN_FILENAME` stays as `"claudine-bridge.ts"`** — This is a single bridge file, not a directory. The plural convention applies to directories only.
- **`opencode.json` config key `"plugin": [...]`** — OpenCode's own config schema uses singular `plugin` for the npm packages array. This is correct as-is.
- **The `BRIDGE_TEMPLATE` content** — Internal TS identifiers like `ClaudineBridge` are code identifiers, not filesystem paths.
- **`agents/opencode.rs`** — Already uses plural correctly.

## Verification

```bash
cargo test -p claudine --lib -- config::opencode
```

All existing tests pass after the rename because they assert against the same `plugin_dir`/`plugins_dir` function that produces the paths.
