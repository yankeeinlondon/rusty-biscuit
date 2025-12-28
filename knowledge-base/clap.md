---
name: clap
description: Comprehensive guide to building command-line interfaces in Rust with clap
created: 2025-12-08
hash: ddac8722da9cc9fc
tags:
  - rust
  - cli
  - command-line
  - argument-parsing
  - clap
---

# Clap: Command Line Argument Parser for Rust

Clap (Command Line Argument Parser) is the de facto standard for building command-line interfaces in Rust. It provides a polished CLI experience out of the box with minimal boilerplate while remaining flexible enough to accommodate everything from simple scripts to complex multi-command applications. Clap automatically generates help messages, error messages with suggestions, and shell completions, allowing developers to focus on application logic rather than parsing infrastructure.

## Table of Contents

- [Clap: Command Line Argument Parser for Rust](#clap-command-line-argument-parser-for-rust)
  - [Table of Contents](#table-of-contents)
  - [Introduction](#introduction)
  - [API Modes](#api-modes)
    - [Derive API](#derive-api)
    - [Builder API](#builder-api)
    - [Hybrid Approach](#hybrid-approach)
    - [API Comparison](#api-comparison)
  - [Argument Types](#argument-types)
    - [Positional Arguments](#positional-arguments)
    - [Options](#options)
    - [Flags](#flags)
    - [Subcommands](#subcommands)
    - [Argument Actions](#argument-actions)
  - [Code Examples](#code-examples)
    - [Simple CLI with Derive API](#simple-cli-with-derive-api)
    - [CLI with Subcommands](#cli-with-subcommands)
    - [Builder API Example](#builder-api-example)
    - [Advanced CLI with Custom Validation](#advanced-cli-with-custom-validation)
  - [Advanced Features](#advanced-features)
    - [Custom Validation and Parsing](#custom-validation-and-parsing)
    - [Environment Variables](#environment-variables)
    - [Shell Completions](#shell-completions)
    - [Layered Configuration](#layered-configuration)
    - [Custom Help Messages](#custom-help-messages)
  - [Ecosystem and Related Crates](#ecosystem-and-related-crates)
  - [Choosing an API](#choosing-an-api)
  - [Quick Reference](#quick-reference)
    - [Common Derive Attributes](#common-derive-attributes)
    - [Common Types and Their Behavior](#common-types-and-their-behavior)
  - [Resources](#resources)

## Introduction

Clap is designed with several key aspirations that shape its development:

- **Polished User Experience** - Common argument behaviors, typo suggestions, colored output, and automatic help generation create a professional CLI feel
- **Reasonable Performance** - Balanced parse performance while maintaining flexibility for diverse use cases
- **Resilient Maintainership** - Controlled breaking changes (approximately every 6-9 months) with clear migration paths
- **Broad Compatibility** - Supports the last two minor Rust releases (currently MSRV 1.74)

To get started, add clap to your `Cargo.toml`:

```toml
# For Derive API (recommended for most cases)
clap = { version = "4", features = ["derive"] }

# For Builder API only
clap = "4"
```

## API Modes

Clap offers three primary approaches for defining command-line interfaces, each suited to different requirements and preferences.

### Derive API

The Derive API uses Rust's procedural macros to generate CLI parsing code from struct definitions. This approach is highly concise and idiomatic for Rust developers.

**Advantages:**

- Minimal boilerplate with clean, declarative syntax
- Type safety leverages Rust's type system for argument validation
- Doc comments automatically become help messages
- Compile-time detection of configuration issues
- Unified argument definition and storage

**Limitations:**

- Less runtime flexibility for dynamic CLI structures
- Complex configurations may require attribute workarounds
- Slightly higher compile times due to procedural macro expansion

### Builder API

The Builder API provides a programmatic, method-chaining approach to constructing command-line interfaces. This offers maximum flexibility and explicit control.

**Advantages:**

- Maximum flexibility for dynamic configuration based on runtime conditions
- All settings are explicitly visible in the code
- Handles intricate argument relationships elegantly
- Easier debugging without macro magic
- Lower compile times when not using other procedural macros

**Limitations:**

- More verbose than the Derive API
- Requires manually connecting argument definitions to storage variables
- Some configuration issues may only surface at runtime

### Hybrid Approach

The Hybrid Approach combines both APIs, using the Derive API for subcommands and the Builder API for dynamic aspects of the main command structure.

**Advantages:**

- Best of both worlds for mixed requirements
- Targeted flexibility where needed
- Gradual migration path between APIs

**Limitations:**

- Mixing APIs may reduce code uniformity
- Requires understanding both approaches

### API Comparison

| Aspect | Derive API | Builder API | Hybrid |
|--------|------------|-------------|--------|
| **Code Verbosity** | Low | High | Medium |
| **Flexibility** | Limited | Extensive | Balanced |
| **Type Safety** | High | Medium | High |
| **Compile Time** | Higher (macros) | Lower | Medium |
| **Runtime Configuration** | Difficult | Easy | Possible |
| **Best For** | Most applications, quick prototyping | Dynamic interfaces, complex logic | Medium to complex CLIs |

## Argument Types

Clap supports various argument types that can be configured through either API.

### Positional Arguments

Arguments identified by their position rather than a flag:

```rust
#[derive(Parser)]
struct Args {
    /// Input file to process (required positional)
    input: String,

    /// Additional files (multiple positional values)
    files: Vec<String>,
}
```

### Options

Named arguments that take values:

```rust
#[derive(Parser)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    name: String,  // -n NAME or --name NAME

    /// Output file (optional)
    #[arg(short, long)]
    output: Option<String>,
}
```

### Flags

Boolean options that don't take values:

```rust
#[derive(Parser)]
struct Args {
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,  // -v or --verbose
}
```

### Subcommands

Nested commands that create their own argument structure:

```rust
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Add { name: String },
    Remove { name: String },
}
```

### Argument Actions

`ArgAction` defines how arguments behave when encountered:

| Action | Description | Example Use |
|--------|-------------|-------------|
| **Set** | Store a single value (default) | Most arguments |
| **Append** | Collect multiple values into a vector | `--file a.txt --file b.txt` |
| **SetTrue/SetFalse** | Toggle boolean flags | `--verbose` / `--no-color` |
| **Count** | Increment a counter | `-v` / `-vv` / `-vvv` for verbosity |
| **Help/Version** | Display help or version | Built-in `--help` / `--version` |

```rust
#[derive(Parser)]
struct Args {
    /// Verbosity level (can be specified multiple times)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}
```

## Code Examples

### Simple CLI with Derive API

This example demonstrates a basic greeting application:

```rust
use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
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
        println!("Hello {}!", args.name);
    }
}
```

**Usage:**

```bash
$ demo --help
Simple program to greet a person

Usage: demo [OPTIONS] --name <NAME>

Options:
  -n, --name <NAME>    Name of the person to greet
  -c, --count <COUNT>  Number of times to greet [default: 1]
  -h, --help           Print help
  -V, --version        Print version

$ demo --name World --count 3
Hello World!
Hello World!
Hello World!
```

### CLI with Subcommands

This example shows a git-like CLI with multiple subcommands:

```rust
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "git")]
#[command(about = "A fictional versioning CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Clones repos
    Clone {
        /// The remote to clone
        remote: String,
        /// The local directory to clone into
        #[arg(short, long)]
        directory: Option<PathBuf>,
    },
    /// Compare two commits
    Diff {
        /// First commit to compare
        commit1: String,
        /// Second commit to compare
        commit2: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Clone { remote, directory } => {
            println!("Cloning {} into {:?}", remote, directory);
        }
        Commands::Diff { commit1, commit2 } => {
            println!("Comparing {} with {}", commit1, commit2);
        }
    }
}
```

### Builder API Example

The same greeting application using the Builder API:

```rust
use clap::{Arg, Command};

fn main() {
    let matches = Command::new("demo")
        .author("Author Name")
        .version("1.0")
        .about("Simple program to greet a person")
        .arg(
            Arg::new("name")
                .short('n')
                .long("name")
                .value_name("NAME")
                .help("Name of the person to greet")
                .required(true),
        )
        .arg(
            Arg::new("count")
                .short('c')
                .long("count")
                .value_name("COUNT")
                .help("Number of times to greet")
                .default_value("1"),
        )
        .get_matches();

    let name = matches.get_one::<String>("name").expect("required");
    let count: u8 = matches
        .get_one::<String>("count")
        .unwrap()
        .parse()
        .expect("count should be a number");

    for _ in 0..count {
        println!("Hello, {}!", name);
    }
}
```

### Advanced CLI with Custom Validation

This example demonstrates custom validation and multiple argument types:

```rust
use clap::{Parser, ValueEnum};
use std::num::ParseIntError;

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// Input file to process
    #[arg(short, long)]
    input: String,

    /// Output format
    #[arg(short, long, value_enum)]
    format: OutputFormat,

    /// Verbosity level (use multiple times for more output)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Timeout in seconds
    #[arg(short, long, value_parser = parse_timeout)]
    timeout: Option<u64>,
}

#[derive(Clone, Debug, ValueEnum)]
enum OutputFormat {
    Json,
    Yaml,
    Toml,
}

fn parse_timeout(s: &str) -> Result<u64, ParseIntError> {
    s.parse::<u64>()
}

fn main() {
    let args = Args::parse();
    println!("Input: {}", args.input);
    println!("Format: {:?}", args.format);
    println!("Verbosity: {}", args.verbose);
    println!("Timeout: {:?}", args.timeout);
}
```

**Usage:**

```bash
$ advanced --input data.txt --format json -vvv --timeout 30
Input: data.txt
Format: Json
Verbosity: 3
Timeout: Some(30)
```

## Advanced Features

### Custom Validation and Parsing

Clap allows custom value parsers for complex validation scenarios:

```rust
#[arg(value_parser = parse_port)]
port: u16,

fn parse_port(s: &str) -> Result<u16, String> {
    let port: u16 = s.parse()
        .map_err(|_| format!("'{}' is not a valid port number", s))?;
    if port == 0 {
        return Err("Port must be between 1 and 65535".to_string());
    }
    Ok(port)
}
```

### Environment Variables

With the `env` feature enabled, clap can read arguments from environment variables:

```toml
clap = { version = "4", features = ["derive", "env"] }
```

```rust
#[arg(short, long, env = "DATABASE_URL")]
database: String,
```

This allows `DATABASE_URL=postgres://... myapp` or `myapp --database postgres://...`.

### Shell Completions

The `clap_complete` crate generates shell completion scripts:

```rust
// In build.rs
use clap_complete::generate_to;
use clap_complete::shells::{Bash, Fish, Zsh, PowerShell};
use std::env;
use std::path::PathBuf;

include!("src/cli.rs");

fn main() {
    let mut cmd = Cli::command();
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    generate_to(Bash, &mut cmd, "myapp", &out_dir).unwrap();
    generate_to(Fish, &mut cmd, "myapp", &out_dir).unwrap();
    generate_to(Zsh, &mut cmd, "myapp", &out_dir).unwrap();
    generate_to(PowerShell, &mut cmd, "myapp", &out_dir).unwrap();
}
```

### Layered Configuration

For complex applications, layer command-line arguments on top of configuration files and environment variables:

```rust
use serde::Deserialize;
use std::fs;

#[derive(Deserialize, Default)]
struct FileConfig {
    port: Option<u16>,
    host: Option<String>,
}

#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    port: Option<u16>,

    #[arg(short, long)]
    host: Option<String>,

    #[arg(short, long, default_value = "config.toml")]
    config: String,
}

fn main() {
    let args = Args::parse();

    // Load file config
    let file_config: FileConfig = fs::read_to_string(&args.config)
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default();

    // CLI args override file config
    let port = args.port.or(file_config.port).unwrap_or(8080);
    let host = args.host.or(file_config.host).unwrap_or("localhost".into());

    println!("Server at {}:{}", host, port);
}
```

### Custom Help Messages

Customize help messages using doc comments and attributes:

```rust
/// A brief description shown in command listings
///
/// A much longer explanation that will appear when
/// the user runs --help. This can span multiple lines
/// and include detailed usage information.
#[derive(Parser)]
#[command(
    author = "Your Name",
    version,
    about,
    long_about = None,
    after_help = "Examples:\n  myapp --input file.txt\n  myapp -v -v"
)]
struct Cli {
    /// Help for this specific argument
    #[arg(short, long, help = "Short help", long_help = "Extended help text")]
    field: String,
}
```

## Ecosystem and Related Crates

The clap ecosystem includes several complementary crates:

| Crate | Purpose |
|-------|---------|
| **clap_complete** | Generate shell completion scripts (bash, zsh, fish, PowerShell) |
| **clap_mangen** | Generate man page source (roff format) |
| **clap-verbosity-flag** | Standardized `-v`, `-vv`, `-vvv` verbosity handling |
| **clap-cargo** | Integration with Cargo commands |
| **wild** | Support wildcards (`*`) on Windows like Linux |
| **argfile** | Load additional arguments from a file (`@args.txt`) |

## Choosing an API

**Start with the Derive API** if:

- You're new to clap
- Building a typical CLI application
- You prefer declarative, clean code
- You want minimal boilerplate

**Use the Builder API** if:

- You need to generate arguments dynamically at runtime
- You're loading valid choices from external sources (files, databases)
- Compile time is a major concern
- You need very fine-grained control

**Use the Hybrid Approach** if:

- Most of your CLI is static but some parts need runtime flexibility
- You're migrating between APIs incrementally

You can mix both APIs within the same project. For instance, use the Derive API for the main structure and Builder API for a particularly complex or dynamic subcommand.

## Quick Reference

### Common Derive Attributes

```rust
// Command-level
#[command(name = "myapp")]
#[command(version, author, about)]
#[command(subcommand)]

// Argument-level
#[arg(short, long)]           // Enable -x and --xxx
#[arg(short = 'x')]           // Custom short flag
#[arg(default_value = "val")] // Default value
#[arg(default_value_t = 42)]  // Typed default
#[arg(required = true)]       // Make required
#[arg(value_enum)]            // Use enum variants as values
#[arg(action = Count)]        // Count occurrences
#[arg(env = "VAR")]           // Read from environment
#[arg(hide = true)]           // Hide from help
```

### Common Types and Their Behavior

| Rust Type | Clap Behavior |
|-----------|---------------|
| `String` | Required argument |
| `Option<String>` | Optional argument |
| `Vec<String>` | Multiple values allowed |
| `bool` | Flag (no value) |
| `u8` with `Count` | Counts occurrences |
| Custom enum with `ValueEnum` | Restricted choices |

## Resources

- [Official Documentation](https://docs.rs/clap/latest/clap/)
- [Clap GitHub Repository](https://github.com/clap-rs/clap)
- [Clap Derive Tutorial](https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html)
- [Clap Cookbook](https://docs.rs/clap/latest/clap/_cookbook/index.html)
- [crates.io: clap](https://crates.io/crates/clap)
- [crates.io: clap_complete](https://crates.io/crates/clap_complete)
