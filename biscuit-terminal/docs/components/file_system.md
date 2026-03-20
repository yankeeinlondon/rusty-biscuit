# FileSystem

Renders directory trees in the terminal with tree-style output using Unicode box-drawing characters (`├──`, `└──`, `│`). Supports Nerd Font icons for files and directories with Unicode fallbacks, gitignore awareness, configurable depth/entry limits, symlink detection, and color highlights.

## Programmatic Use

```rust
use biscuit_terminal::prelude::*;

// Basic usage
let mut fs = FileSystem::new(".").unwrap();
fs.ensure_tree_built();
let term = Terminal::default();
println!("{}", fs.display(&term));

// With formatting options
let mut fs = FileSystem::new_with_formatting(".")?
    .depth(5)
    .max_entries(100)
    .highlight_green("src")
    .highlight_red("TODO");
fs.ensure_tree_built();

// Access file metrics
if let Some(metrics) = fs.metrics() {
    println!("Files: {}", metrics.file_count());
    println!("Dirs: {}", metrics.dir_count());
}
```

### Key API

| Method | Description |
|--------|-------------|
| `FileSystem::new(path)` | Create from a directory path |
| `FileSystem::new_with_formatting(path)` | Create with formatting enabled (colors, icons) |
| `.depth(n)` | Limit tree depth |
| `.max_entries(n)` | Limit total entries shown |
| `.highlight_green(pattern)` | Highlight matching entries in green |
| `.highlight_red(pattern)` | Highlight matching entries in red |
| `.ensure_tree_built()` | Build the tree (must call before rendering) |

### Icon Support

Icons use Nerd Fonts by default with Unicode emoji fallback:

```rust
use biscuit_terminal::components::filesystem::icons;

// Nerd Font icons (require patched fonts)
let rust_icon = icons::nerd::ext::RUST;
let dir_icon = icons::nerd::dir::BASE;

// Unicode fallbacks (work in any terminal)
let file_emoji = icons::unicode::file::BASE; // 📄
let folder_emoji = icons::unicode::dir::BASE; // 📂
```

### Error Handling

Returns `FileSystemError` for path not found, not a directory, permission denied, I/O errors, and gitignore pattern errors.

## CLI

Exposed via `bt dir`:

```bash
bt dir .                    # Current directory
bt dir src --depth 3        # Limit depth
bt dir . --max-entries 50   # Limit entries
```
