# Use Cases

crossterm is used for building text-based user interfaces and command-line applications. Here are the most common scenarios where it excels.

## Terminal User Interfaces (TUIs)

crossterm is frequently used as the backend for TUI frameworks, especially Ratatui (formerly tui-rs).

### Dashboards

Real-time monitoring and visualization tools:

- **System monitors** - CPU, memory, network usage displays
- **Log viewers** - Real-time log aggregation and filtering
- **Server health dashboards** - Multi-service status monitoring
- **Stock/crypto tickers** - Live price feeds and charts
- **Build/CI monitors** - Pipeline status and test results

**Why crossterm:** Double buffering via alternate screen prevents flicker, event handling enables real-time updates, cross-platform support ensures consistency.

### File Managers

Visual directory navigation and file operations:

- **Directory browsers** - Visual tree navigation (like broot or ranger)
- **Archive explorers** - Navigate ZIP/TAR contents without extraction
- **Git file pickers** - Stage/unstage files interactively
- **Media galleries** - Browse images/videos with previews

**Why crossterm:** Cursor control for selection highlighting, mouse support for clicking files, keyboard events for vim-style navigation.

### Data Visualization

Rendering charts and tables directly in the terminal:

- **Sparklines and line charts** - Trend visualization for metrics
- **Bar/histogram charts** - Distribution displays
- **Tables with sorting/filtering** - Interactive data exploration
- **ASCII art graphs** - Network topology, dependency trees

**Why crossterm:** Precise cursor positioning for drawing, styling for color-coded data, resize events for responsive layouts.

## Interactive CLI Tools

Making command-line applications feel like "apps" rather than scripts.

### Custom Prompts

Interactive selection and input:

- **Multi-select menus** - Choose multiple options from a list
- **Autocomplete inputs** - Type-ahead suggestions as you type
- **Form wizards** - Multi-step configuration workflows
- **Confirmation dialogs** - Yes/no prompts with validation

**Why crossterm:** Raw mode for immediate keystroke capture, cursor control for live updates, styling for visual feedback.

### Progress Indicators

Advanced progress visualization:

- **Multi-bar progress** - Parallel task tracking
- **Persistent status lines** - Progress bar fixed at bottom while logs scroll
- **Percentage + ETA displays** - Time remaining calculations
- **Spinner animations** - Indefinite progress indication

**Why crossterm:** Cursor save/restore for updating fixed positions, alternate screen for complex layouts, queueing for smooth animations.

### Syntax Highlighting

Terminal-based code viewers and editors:

- **Code pagers** - Syntax-highlighted `less` alternatives
- **REPL interfaces** - Language-specific input with highlighting
- **Diff viewers** - Side-by-side file comparison
- **Log colorizers** - Semantic coloring of log levels

**Why crossterm:** Rich color support (RGB, 256-color), styling attributes (bold, italic, underline), cursor control for multi-column layouts.

## Terminal Games

Games that run entirely in the terminal.

### Classic Games

- **Snake** - Real-time movement and collision detection
- **Tetris** - Block rotation and line clearing
- **Pong** - Paddle control and ball physics
- **Chess/Checkers** - Board rendering and piece movement

**Why crossterm:** Raw mode disables line buffering for instant input, alternate screen preserves terminal state, mouse support for drag-and-drop pieces.

### Roguelikes

Dungeon crawlers and procedural exploration:

- **ASCII dungeons** - Tile-based world rendering
- **Turn-based combat** - Command menus and status displays
- **Inventory management** - Interactive item selection
- **Map exploration** - Fog of war and visibility

**Why crossterm:** Efficient screen updates via queueing, event handling for movement/actions, save/restore cursor for UI overlays.

### Puzzle Games

Logic and strategy games:

- **Minesweeper** - Click-to-reveal cells
- **Sudoku** - Number input and validation
- **2048** - Directional tile merging
- **Sokoban** - Box-pushing puzzles

**Why crossterm:** Mouse events for clicking cells, keyboard events for arrow key input, styling for game state visualization.

## Cross-Platform System Utilities

Tools that need consistent behavior on Windows and UNIX.

### Consistent Terminal Styling

Before crossterm, Windows support for ANSI colors was problematic:

- **Color output** - Uniform color support across platforms
- **Progress bars** - Consistent rendering on Windows Command Prompt
- **Formatted tables** - Aligned columns with box-drawing characters
- **Error highlighting** - Red errors, yellow warnings everywhere

**Why crossterm:** Abstracts platform differences (WinAPI vs ANSI escape codes), ensures identical appearance across OS.

### Terminal Manipulation

Screen control that works everywhere:

- **Screen clearing** - Consistent clear behavior
- **Cursor positioning** - Absolute positioning for status displays
- **Cursor visibility** - Hide during operations, show for input
- **Terminal resizing** - Handle resize events uniformly

**Why crossterm:** Cross-platform API, no conditional compilation needed, works on Windows 7+.

## Technical Capabilities Summary

| Feature | Use Case |
|---------|----------|
| **Cursor Movement** | Positioning content at specific coordinates, creating multi-column layouts |
| **Styling** | Color-coding data, emphasizing important information, syntax highlighting |
| **Terminal Control** | Alternate screen for TUIs, clearing regions, handling resizes |
| **Event Handling** | Real-time input processing, mouse interaction, focus detection |
| **Raw Mode** | Character-by-character input without Enter key, game controls |
| **Queueing** | Efficient batch updates, flicker-free animations |

## When NOT to Use crossterm

Consider alternatives when:

- **Linux-only apps** - termion offers better performance
- **Simple output-only tools** - Overhead may not be worth it
- **Web-based UIs** - Use actual web frameworks instead
- **macOS /dev/tty apps** - crossterm has known issues with piped input on macOS
- **Very size-constrained** - Pure ANSI escape codes may be smaller

## Related

- [Ecosystem](./ecosystem.md) - Companion crates for common use cases
- [Platform Issues](./platform-issues.md) - Platform-specific considerations
