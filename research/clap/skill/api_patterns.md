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