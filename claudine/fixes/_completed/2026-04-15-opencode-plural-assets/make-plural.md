# Making OpenCode Plural Naming Consistent

OpenCode supports both plural and singular names for agents, plugins, commands, and skills, but the **plural naming is considered the right one**. This document tracks singular references that should be updated to plural for OpenCode consistency.

## Critical: Source Code Issues

### 1. `claudine/lib/src/config/opencode.rs`

**Issue**: Uses singular `plugin` directory and `bridge` filename

| Line | Current (Singular) | Should Be (Plural) | Description |
|------|-------------------|-------------------|-------------|
| 12 | `const PLUGIN_FILENAME: &str = "claudine-bridge.ts";` | `const PLUGIN_FILENAME: &str = "claudine-bridges.ts";` | OpenCode uses plural "bridges" directory |
| 263-270 | `fn plugin_dir(...)` returning `plugin` | `fn plugins_dir(...)` returning `plugins` | Function and path should use plural |

**Details**:
- Line 265: `Some(dir) => dir.join("plugin"),` → `dir.join("plugins")`
- Line 268: `home.join(".config").join("opencode").join("plugin")` → `.join("plugins")`
- Line 159: Uses `plugin_dir(config_dir).join(PLUGIN_FILENAME)` - affected by rename
- Line 182: Uses `plugin_dir(config_dir).join(PLUGIN_FILENAME)` - affected by rename
- Line 191: Uses `plugin_dir(config_dir).join(PLUGIN_FILENAME)` - affected by rename
- Line 197: Uses `plugin_dir(config_dir).join(PLUGIN_FILENAME)` - affected by rename

**Comments needing update**:
- Line 156: "Generate the bridge plugin TypeScript file" → "Generate the bridge plugins TypeScript file"
- Line 180: "OpenCode discovers plugins from the plugin directory" → "from the plugins directory"
- Line 188: "OpenCode discovers plugins from the plugin directory" (in deregister comment)

### 2. Test files using singular paths

**`claudine/lib/src/config/opencode.rs` tests**:
- Line 391: `let plugin_dir = tmp.path().join("plugin");` → `join("plugins")`
- Line 393: `fs::write(plugin_dir.join("claudine-bridge.ts"), "// bridge")` → affected by filename change
- Line 399: `assert!(!plugin_dir.join("claudine-bridge.ts").exists())` → affected
- Line 405: `let plugin_dir = tmp.path().join("plugin");` → `join("plugins")`
- Line 407: `fs::write(plugin_dir.join("claudine-bridge.ts"), "// bridge")` → affected

## Reference: Correct Plural Usage

**`claudine/lib/src/agents/opencode.rs`** (Line 189) shows the correct pattern:

```rust
plugin_dirs: path_vec(&[".opencode/plugins", "~/.config/opencode/plugins"]),
```

This correctly uses:
- `plugins` (plural) in directory paths
- `plugin_dirs` (plural) field name

## Verification Needed

1. Confirm OpenCode's actual directory naming convention (is it `plugins` or `bridges`?)
2. Check if `claudine-bridges.ts` is the correct filename or if it should remain `claudine-bridge.ts` as a single bridge plugin file
3. Verify if OpenCode uses `~/.config/opencode/plugin/` or `~/.config/opencode/plugins/`

## Notes

- The `agents/opencode.rs` file uses `plugin_dirs` with plural "plugins" correctly
- The `opencode.rs` configurator uses singular `plugin_dir` inconsistently
- Documentation in `provider-quirks.md` already uses plural correctly: "for agents, commands, modes, and plugins"

