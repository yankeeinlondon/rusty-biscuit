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