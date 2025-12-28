---
name: clap
description: Expert knowledge for building command-line interfaces in Rust using the clap crate. Use when creating CLI tools, parsing arguments, defining subcommands, or implementing shell completions. Covers Derive API, Builder API, custom validation, and ecosystem crates.
hash: 2ef29fc6150d2b15
---

# clap

The standard crate for building command-line interfaces in Rust. Provides automatic help generation, shell completions, and robust argument parsing with two API styles.

## Core Principles

- Start with the **Derive API** for most applications - minimal boilerplate, type-safe
- Use the **Builder API** when you need dynamic argument construction at runtime
- Mix both APIs in the same project when beneficial
- Doc comments (`///`) become help text automatically
- Use `#[arg(short, long)]` to create both `-n` and `--name` flags
- Add `clap = { version = "4", features = ["derive"] }` for Derive API
- Return `Result` from main with clap handling for clean error messages
- Use `#[command(subcommand)]` for git-style nested commands
- Leverage `clap_complete` for shell completion scripts
- Environment variables can serve as argument defaults with the `env` feature

## API Comparison

| Aspect | Derive API | Builder API |
|--------|------------|-------------|
| Approach | Declarative with structs/attributes | Imperative with method chaining |
| Ease of Use | High - minimal boilerplate | Moderate - more verbose |
| Flexibility | Covers 90% of use cases | Maximum - dynamic construction |
| Compile Time | Slightly higher (proc macros) | Lower |
| Best For | Most apps, quick prototyping | Dynamic interfaces, complex logic |

## Quick Reference

### Derive API (Recommended Start)

```rust
use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    name: String,

    /// Number of times to greet
    #[arg(short, long, default_value_t = 1)]
    count: u8,
}

fn main() {
    let args = Args::parse();
    for _ in 0..args.count {
        println!("Hello, {}!", args.name);
    }
}
```

### Builder API

```rust
use clap::{Arg, Command};

fn main() {
    let matches = Command::new("demo")
        .version("1.0")
        .about("Simple program to greet a person")
        .arg(Arg::new("name")
            .short('n')
            .long("name")
            .required(true))
        .arg(Arg::new("count")
            .short('c')
            .long("count")
            .default_value("1"))
        .get_matches();

    let name = matches.get_one::<String>("name").expect("required");
    let count: u8 = matches.get_one::<String>("count")
        .unwrap().parse().expect("count should be a number");

    for _ in 0..count {
        println!("Hello, {}!", name);
    }
}
```

## Argument Types

### Positional Arguments

```rust
#[derive(Parser)]
struct Args {
    /// Input file (required positional)
    input: String,

    /// Output file (optional positional)
    output: Option<String>,

    /// Multiple files
    files: Vec<String>,
}
```

### Options and Flags

```rust
#[derive(Parser)]
struct Args {
    /// Named option: -n NAME or --name NAME
    #[arg(short, long)]
    name: String,

    /// Boolean flag: -v or --verbose
    #[arg(short, long)]
    verbose: bool,

    /// Count occurrences: -v -v -v => 3
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbosity: u8,
}
```

### Subcommands

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "git")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Clone a repository
    Clone {
        remote: String,
        #[arg(short, long)]
        directory: Option<String>,
    },
    /// Show differences
    Diff {
        commit1: String,
        commit2: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Clone { remote, directory } => { /* ... */ }
        Commands::Diff { commit1, commit2 } => { /* ... */ }
    }
}
```

## Common Patterns

### Custom Validation

```rust
#[arg(value_parser = parse_port)]
port: u16,

fn parse_port(s: &str) -> Result<u16, String> {
    let port: u16 = s.parse()
        .map_err(|_| format!("'{}' is not a valid port", s))?;
    if port == 0 {
        return Err("Port must be 1-65535".to_string());
    }
    Ok(port)
}
```

### Environment Variable Fallback

```rust
// Requires: clap = { version = "4", features = ["derive", "env"] }
#[arg(short, long, env = "DATABASE_URL")]
database: String,
```

### Enum Value Selection

```rust
use clap::ValueEnum;

#[derive(Clone, ValueEnum)]
enum Format {
    Json,
    Yaml,
    Toml,
}

#[derive(Parser)]
struct Args {
    #[arg(short, long, value_enum)]
    format: Format,
}
```

## Shell Completions

Generate completion scripts with `clap_complete`:

```rust
// In build.rs
use clap_complete::generate_to;
use clap_complete::shells::{Bash, Fish, Zsh};
use std::env;

include!("src/cli.rs");

fn main() {
    let mut cmd = Cli::command();
    let out_dir = env::var_os("OUT_DIR").unwrap();

    generate_to(Bash, &mut cmd, "myapp", &out_dir).unwrap();
    generate_to(Fish, &mut cmd, "myapp", &out_dir).unwrap();
    generate_to(Zsh, &mut cmd, "myapp", &out_dir).unwrap();
}
```

## Ecosystem Crates

| Crate | Purpose |
|-------|---------|
| `clap_complete` | Shell completion script generation |
| `clap_mangen` | Man page generation |
| `clap-verbosity-flag` | Standardized `-v/-q` verbosity handling |
| `clap-cargo` | Cargo-style argument patterns |
| `wild` | Windows wildcard expansion like Linux |
| `argfile` | Load arguments from files (@file) |

## Cargo.toml

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }

# Optional features
# clap = { version = "4", features = ["derive", "env", "wrap_help"] }

[build-dependencies]
clap_complete = "4"  # For shell completions
```

## Resources

- [clap Documentation](https://docs.rs/clap)
- [clap GitHub](https://github.com/clap-rs/clap)
- [clap Derive Tutorial](https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html)
- [clap Builder Tutorial](https://docs.rs/clap/latest/clap/_tutorial/index.html)
