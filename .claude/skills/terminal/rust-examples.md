# Rust Terminal Examples

Practical Rust examples using crossterm, termcolor, and raw escape sequences.

## Dependencies

```toml
[dependencies]
crossterm = "0.27"
termcolor = "1.4"
unicode-width = "0.1"
```

## Basic Output with crossterm

### Styled Text

```rust
use crossterm::style::{Color, Stylize, Attribute};

fn main() {
    // Simple styling
    println!("{}", "Bold Red".red().bold());
    println!("{}", "Underlined Blue".blue().underlined());
    println!("{}", "Italic Green".green().italic());

    // Combined styles
    println!("{}", "Warning".yellow().bold().on_dark_red());

    // RGB colors
    println!("{}", "Custom Color".with(Color::Rgb { r: 255, g: 128, b: 0 }));
}
```

### Using Commands

```rust
use std::io::{stdout, Write};
use crossterm::{
    execute,
    style::{Color, Print, SetForegroundColor, SetBackgroundColor, ResetColor, SetAttribute, Attribute},
    cursor::{MoveTo, Hide, Show},
    terminal::{Clear, ClearType},
};

fn main() -> std::io::Result<()> {
    let mut stdout = stdout();

    execute!(
        stdout,
        SetForegroundColor(Color::Red),
        SetAttribute(Attribute::Bold),
        Print("Hello "),
        SetForegroundColor(Color::Green),
        Print("World!"),
        ResetColor,
        Print("\n"),
    )?;

    Ok(())
}
```

## Using termcolor

```rust
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

fn main() -> std::io::Result<()> {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    // Red bold text
    stdout.set_color(ColorSpec::new()
        .set_fg(Some(Color::Red))
        .set_bold(true))?;
    writeln!(&mut stdout, "Error: Something went wrong")?;

    // Green text
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
    writeln!(&mut stdout, "Success!")?;

    // Reset
    stdout.reset()?;

    Ok(())
}
```

## Progress Bar

### Simple Progress Bar

```rust
use std::io::{stdout, Write};
use std::thread;
use std::time::Duration;

fn progress_bar(percent: f64, width: usize) {
    let filled = (width as f64 * percent / 100.0).round() as usize;
    let empty = width - filled;

    print!("\r[");
    print!("{}", "█".repeat(filled));
    print!("{}", "░".repeat(empty));
    print!("] {:>3.0}%", percent);

    stdout().flush().unwrap();
}

fn main() {
    for i in 0..=100 {
        progress_bar(i as f64, 40);
        thread::sleep(Duration::from_millis(30));
    }
    println!();
}
```

### Colored Progress Bar with ETA

```rust
use std::io::{stdout, Write};
use std::time::Instant;

struct ProgressBar {
    total: u64,
    width: usize,
    start_time: Instant,
}

impl ProgressBar {
    fn new(total: u64, width: usize) -> Self {
        Self {
            total,
            width,
            start_time: Instant::now(),
        }
    }

    fn update(&self, current: u64) {
        let percent = (current as f64 / self.total as f64) * 100.0;
        let filled = (self.width as f64 * percent / 100.0).round() as usize;
        let empty = self.width - filled;

        // Color gradient: red -> yellow -> green
        let (r, g) = if percent < 50.0 {
            (255, (percent * 5.1) as u8)
        } else {
            ((255.0 - (percent - 50.0) * 5.1) as u8, 255)
        };

        // Calculate ETA
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let eta = if current > 0 {
            let rate = current as f64 / elapsed;
            ((self.total - current) as f64 / rate) as u64
        } else {
            0
        };

        print!("\r\x1b[38;2;{};{};0m{}\x1b[90m{}\x1b[0m {:>3.0}% ETA: {}s",
            r, g,
            "█".repeat(filled),
            "░".repeat(empty),
            percent,
            eta
        );

        stdout().flush().unwrap();

        if current >= self.total {
            println!();
        }
    }
}
```

## Spinner

```rust
use std::io::{stdout, Write};
use std::thread;
use std::time::Duration;

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

struct Spinner {
    message: String,
    frame: usize,
}

impl Spinner {
    fn new(message: &str) -> Self {
        // Hide cursor
        print!("\x1b[?25l");
        Self {
            message: message.to_string(),
            frame: 0,
        }
    }

    fn tick(&mut self) {
        print!("\r{} {}", SPINNER_FRAMES[self.frame], self.message);
        stdout().flush().unwrap();
        self.frame = (self.frame + 1) % SPINNER_FRAMES.len();
    }

    fn success(self, msg: &str) {
        println!("\r\x1b[2K\x1b[32m✓\x1b[0m {}", msg);
        print!("\x1b[?25h"); // Show cursor
    }

    fn fail(self, msg: &str) {
        println!("\r\x1b[2K\x1b[31m✗\x1b[0m {}", msg);
        print!("\x1b[?25h"); // Show cursor
    }
}

fn main() {
    let mut spinner = Spinner::new("Loading...");

    for _ in 0..30 {
        spinner.tick();
        thread::sleep(Duration::from_millis(80));
    }

    spinner.success("Done!");
}
```

## Raw Mode and Key Events

```rust
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};

fn main() -> std::io::Result<()> {
    enable_raw_mode()?;

    println!("Press 'q' or Ctrl+C to quit");

    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
                match (code, modifiers) {
                    (KeyCode::Char('q'), _) |
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        break;
                    }
                    (KeyCode::Char(c), _) => {
                        print!("{}", c);
                    }
                    (KeyCode::Enter, _) => {
                        println!();
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    Ok(())
}
```

## Terminal Size and Alternate Screen

```rust
use crossterm::{
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen, Clear, ClearType},
    cursor::MoveTo,
    style::Print,
};
use std::io::stdout;

fn main() -> std::io::Result<()> {
    let (cols, rows) = terminal::size()?;
    println!("Terminal size: {}x{}", cols, rows);

    // Alternate screen buffer
    execute!(stdout(), EnterAlternateScreen)?;
    execute!(stdout(), Clear(ClearType::All))?;
    execute!(stdout(), MoveTo(0, 0))?;
    execute!(stdout(), Print("Press Enter to exit..."))?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    execute!(stdout(), LeaveAlternateScreen)?;

    Ok(())
}
```

## Hyperlinks

```rust
fn hyperlink(text: &str, url: &str) -> String {
    format!("\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\", url, text)
}

fn hyperlink_with_id(text: &str, url: &str, id: &str) -> String {
    format!("\x1b]8;id={};{}\x1b\\{}\x1b]8;;\x1b\\", id, url, text)
}

fn main() {
    println!("Visit {}", hyperlink("Rust website", "https://rust-lang.org"));

    // Multi-line link with same ID
    let id = "docs";
    println!("{}", hyperlink_with_id("Documentation", "https://docs.rs", id));
    println!("{}", hyperlink_with_id("(click here)", "https://docs.rs", id));
}
```

## Box Drawing

```rust
use unicode_width::UnicodeWidthStr;

fn draw_box(content: &str, title: Option<&str>) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let content_width = lines.iter()
        .map(|l| UnicodeWidthStr::width(*l))
        .max()
        .unwrap_or(0);
    let title_width = title.map(|t| UnicodeWidthStr::width(t) + 2).unwrap_or(0);
    let width = content_width.max(title_width);

    let top = match title {
        Some(t) => format!("┌─ {} {}┐", t, "─".repeat(width - UnicodeWidthStr::width(t) - 2)),
        None => format!("┌{}┐", "─".repeat(width + 2)),
    };

    let bottom = format!("└{}┘", "─".repeat(width + 2));

    let mut result = vec![top];
    for line in lines {
        let padding = width - UnicodeWidthStr::width(line);
        result.push(format!("│ {}{} │", line, " ".repeat(padding)));
    }
    result.push(bottom);

    result.join("\n")
}

fn main() {
    println!("{}", draw_box("Hello, World!\nThis is a box.", Some("Message")));
}
```

## Table Rendering

```rust
use unicode_width::UnicodeWidthStr;

fn render_table(headers: &[&str], rows: &[Vec<&str>]) -> String {
    let col_count = headers.len();

    // Calculate column widths
    let mut widths: Vec<usize> = headers.iter()
        .map(|h| UnicodeWidthStr::width(*h))
        .collect();

    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_count {
                widths[i] = widths[i].max(UnicodeWidthStr::width(*cell));
            }
        }
    }

    let separator: String = widths.iter()
        .map(|w| "─".repeat(w + 2))
        .collect::<Vec<_>>()
        .join("┼");

    let format_row = |cells: &[&str], bold: bool| -> String {
        let formatted: Vec<String> = cells.iter()
            .enumerate()
            .map(|(i, cell)| {
                let padding = widths.get(i).unwrap_or(&0) - UnicodeWidthStr::width(*cell);
                format!(" {}{} ", cell, " ".repeat(padding))
            })
            .collect();

        let content = formatted.join("│");
        if bold {
            format!("\x1b[1m{}\x1b[0m", content)
        } else {
            content
        }
    };

    let mut lines = vec![
        format_row(headers, true),
        format!("─{}─", separator),
    ];

    for row in rows {
        let cells: Vec<&str> = row.iter().map(|s| *s).collect();
        lines.push(format_row(&cells, false));
    }

    lines.join("\n")
}

fn main() {
    let headers = vec!["Name", "Age", "Role"];
    let rows = vec![
        vec!["Alice", "25", "Engineer"],
        vec!["Bob", "30", "Designer"],
        vec!["Carol", "28", "Manager"],
    ];

    println!("{}", render_table(&headers, &rows));
}
```

## Feature Detection

```rust
use std::env;

#[derive(Debug)]
struct TerminalFeatures {
    true_color: bool,
    colors_256: bool,
    hyperlinks: bool,
    kitty_graphics: bool,
    iterm2_graphics: bool,
}

fn detect_features() -> TerminalFeatures {
    let term = env::var("TERM").unwrap_or_default();
    let colorterm = env::var("COLORTERM").unwrap_or_default();
    let term_program = env::var("TERM_PROGRAM").unwrap_or_default();

    let true_color = colorterm == "truecolor" || colorterm == "24bit";
    let colors_256 = term.contains("256color") || true_color;
    let is_kitty = term == "xterm-kitty" || env::var("KITTY_WINDOW_ID").is_ok();
    let is_iterm = term_program == "iTerm.app";
    let is_wezterm = term_program == "WezTerm" || env::var("WEZTERM_PANE").is_ok();

    TerminalFeatures {
        true_color,
        colors_256,
        hyperlinks: colors_256, // Most modern terminals support OSC 8
        kitty_graphics: is_kitty || is_wezterm,
        iterm2_graphics: is_iterm || is_wezterm,
    }
}

fn main() {
    let features = detect_features();
    println!("{:#?}", features);
}
```
