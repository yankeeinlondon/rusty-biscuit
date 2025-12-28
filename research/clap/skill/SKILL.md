## --- FILE: SKILL.md ---

## name: clap
description: Expert guidance for building command-line interfaces (CLI) in Rust using the `clap` crate. Provides patterns for argument parsing, subcommands, validation, and integration with the Rust ecosystem.

# Clap: Command Line Argument Parser for Rust

`clap` is the standard library for building robust, type-safe command-line interfaces in Rust. It automates help generation, versioning, and argument validation.

## Core Implementation (Derive API)

The **Derive API** is the recommended way to use `clap`. It uses attributes on structs and enums to define your CLI.

````rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "myapp", version, about = "A cool CLI tool")]
struct Cli {
    /// Optional config file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<String>,

    /// Turn debugging on
    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    debug: bool,

    /// Positional input files
    #[arg(required = true)]
    inputs: Vec<String>,
}

fn main() {
    let cli = Cli::parse();
    // Access via cli.config, cli.debug, etc.
}
````

## When to Use This Skill

* You are designing a new CLI tool or adding arguments to an existing one.
* You need to implement complex subcommands (e.g., `git commit` style).
* You want to integrate environment variables as fallbacks for CLI flags.
* You need to validate user input before your application logic runs.
* You are deciding between `clap` and lighter alternatives like `argh` or `pico-args`.

## Key Patterns & Advanced Usage

* **[Subcommands & Enums](api_patterns.md)**: How to structure multi-command tools.
* **[Validation & Integration](validation_integration.md)**: Custom types, `anyhow` error handling, and `serde` integration.
* **[Gotchas & Solutions](gotchas.md)**: Common pitfalls like boolean defaults and `Option` usage.
* **[Choosing an Alternative](alternatives.md)**: Comparison with `bpaf`, `argh`, and others for specific constraints.

## Quick Reference: Argument Attributes

* `short, long`: Enables `-f` and `--flag`.
* `default_value = "t"`: Sets a default string value.
* `env = "VAR"`: Reads from environment variable if flag is missing.
* `conflicts_with = "other"`: Ensures two flags aren't used together.
* `required = true`: Forces the user to provide the argument.

--- FILE: api_patterns.md ---

# Clap API Patterns

## Subcommands (Git-style)

Use an `enum` with the `Subcommand` derive to handle different modes of operation.

````rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Adds files to the index
    Add { 
        name: String 
    },
    /// Removes files
    Remove { 
        #[arg(short)]
        force: bool,
        id: usize 
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Add { name } => println!("Adding {name}"),
        Commands::Remove { force, id } => println!("Removing {id}, force: {force}"),
    }
}
````

## Enum Value Parsing

If an argument only accepts specific strings, use `ValueEnum`.

````rust
#[derive(clap::ValueEnum, Clone)]
enum Mode {
    Fast,
    Slow,
}

#[derive(Parser)]
struct Cli {
    #[arg(value_enum)]
    mode: Mode,
}
````

## The Builder API (Dynamic CLI)

Use this if your CLI structure is only known at runtime or you want to avoid macros.

````rust
use clap::{Command, Arg, ArgAction};

let matches = Command::new("myapp")
    .arg(Arg::new("debug")
        .short('d')
        .action(ArgAction::SetTrue))
    .get_matches();
````

--- FILE: validation_integration.md ---

# Validation and Ecosystem Integration

## Custom Validation

Use `value_parser` to validate input during the parsing phase.

````rust
fn validate_rs_file(s: &str) -> Result<String, String> {
    if s.ends_with(".rs") {
        Ok(s.to_string())
    } else {
        Err("Only .rs files allowed".into())
    }
}

#[derive(clap::Parser)]
struct Cli {
    #[arg(value_parser = validate_rs_file)]
    file: String,
}
````

## Integration with `anyhow`

For clean error reporting after parsing is complete:

````rust
use anyhow::{Context, Result};
use clap::Parser;

fn main() -> Result<()> {
    let args = Cli::parse();
    let content = std::fs::read_to_string(&args.path)
        .with_context(|| format!("Failed to read {:?}", args.path))?;
    Ok(())
}
````

## Integration with `serde` (Config Overlays)

A common pattern is using CLI flags to override a TOML/JSON config file.

````rust
#[derive(serde::Deserialize)]
struct Config {
    api_url: String,
}

#[derive(clap::Parser)]
struct Cli {
    #[arg(long)]
    api_url: Option<String>,
}

fn main() {
    let args = Cli::parse();
    let mut config: Config = toml::from_str(std::fs::read_to_string("config.toml").unwrap()).unwrap();
    
    if let Some(url) = args.api_url {
        config.api_url = url;
    }
}
````

## Verbosity Management

Use `clap-verbosity-flag` to quickly add `-v`, `-vv`, and `-q`.

````rust
#[derive(clap::Parser)]
struct Cli {
    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
}
````

--- FILE: gotchas.md ---

# Common Clap Gotchas

### 1. Boolean Flags defaulting to `true`

By default, `bool` is `false` if not present. To make a flag that disables a feature:

````rust
/// Turn off logging
#[arg(long, action = clap::ArgAction::SetFalse, default_value_t = true)]
logging: bool,
````

### 2. `Option<T>` vs Defaults

* Use `Option<T>` if you need to know if the user *specifically* provided the argument.
* Use `T` with `#[arg(default_value = "...")]` if you just need a value to work with.
* **Pitfall**: `Option<String>` with a default value attribute will still be `Some` (the default) if the flag is missing.

### 3. Argument Visibility in Subcommands

If you have a global flag (like `--verbose`) and subcommands, the flag must be defined in the *top-level* struct. To share logic, use `#[command(flatten)]`.

### 4. `Vec<T>` Requirements

A `Vec<String>` is optional by default (accepts 0 or more). To require at least one:

````rust
#[arg(required = true)]
files: Vec<String>,
````

--- FILE: alternatives.md ---

# When to use Clap vs. Alternatives

### Use `clap` when:

* You need a professional CLI with standard GNU/POSIX behavior.
* You want automatic, high-quality `--help` and shell completions.
* You have complex nested subcommands or complex validation rules.

### Consider alternatives when:

|Library|Use Case|Key Benefit|
|:------|:-------|:----------|
|**bpaf**|High performance / complex logic|Faster compile times, very flexible combinators.|
|**argh**|Simple, small binaries|Tiny footprint, Google-style opinionated CLI.|
|**pico-args**|Zero dependencies|Smallest possible binary size; manual parsing.|
|**lexopt**|Absolute control|Minimalist, no-macro approach for POSIX compliance.|

### Binary Size & Compile Time

`clap` is feature-rich but adds to the binary size and compilation time (due to proc-macros). For embedded systems or extremely small "one-off" tools, `argh` or `pico-args` are often preferred.