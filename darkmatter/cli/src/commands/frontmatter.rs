//! `md get` / `md set` / `md rm` / `md edit` frontmatter subcommand implementations.

use crate::args::Cli;
use crate::io::{load_markdown, resolve_file_path};
use color_eyre::eyre::{Context, Result, eyre};
use std::path::PathBuf;
use tracing::instrument;

/// Get frontmatter properties from a markdown document.
#[instrument(skip_all)]
pub fn run_get(
    input: &PathBuf,
    props: &[String],
    json5: bool,
    yaml: bool,
    toml: bool,
    raw: bool,
    compact: bool,
) -> Result<()> {
    let md = load_markdown(Some(input))?;
    let fm = md.frontmatter();

    let value = if props.len() == 1 {
        // Single property: return the raw value (or empty string if missing)
        fm.as_map()
            .get(&props[0])
            .cloned()
            .unwrap_or(serde_json::Value::String(String::new()))
    } else {
        // Multiple properties: return a dictionary of requested keys
        let mut map = serde_json::Map::new();
        for prop in props {
            let v = fm
                .as_map()
                .get(prop)
                .cloned()
                .unwrap_or(serde_json::Value::String(String::new()));
            map.insert(prop.clone(), v);
        }
        serde_json::Value::Object(map)
    };

    let output = format_value(&value, json5, yaml, toml, raw, compact)?;
    println!("{output}");

    Ok(())
}

/// Set a frontmatter property on a markdown document.
///
/// By default the modified document is written to stdout without changing the
/// source file. With `--save` the file is updated in place and nothing is
/// printed.
#[instrument(skip_all)]
pub fn run_set(input: &PathBuf, prop: &str, raw_value: &str, save: bool) -> Result<()> {
    let is_stdin = input.to_str() == Some("-");

    if save && is_stdin {
        return Err(eyre!(
            "--save requires an input file path (stdin is not supported)"
        ));
    }

    let mut md = load_markdown(Some(input))?;

    let value: serde_json::Value = serde_json::from_str(raw_value)
        .unwrap_or_else(|_| serde_json::Value::String(raw_value.to_string()));

    md.fm_insert(prop, value)
        .map_err(|e| eyre!("Failed to set frontmatter property: {e}"))?;

    if save {
        let resolved = resolve_file_path(input)?;
        std::fs::write(&resolved, md.as_string())
            .wrap_err_with(|| format!("Failed to write to {:?}", resolved))?;
    } else {
        print!("{}", md.as_string());
    }

    Ok(())
}

/// Remove one or more frontmatter properties from a markdown document.
///
/// Saves the file in place. By default produces no output on success.
/// With `-v`, prints a human-readable summary. With `--json`, outputs
/// structured JSON.
#[instrument(skip_all)]
pub fn run_rm(input: &PathBuf, props: &[String], json: bool, cli: &Cli) -> Result<()> {
    let resolved = resolve_file_path(input)?;
    let mut md = load_markdown(Some(input))?;
    let fm = md.frontmatter_mut().as_map_mut();

    let mut removed = Vec::new();
    let mut not_found = Vec::new();

    for prop in props {
        if fm.shift_remove(prop).is_some() {
            removed.push(prop.clone());
        } else {
            not_found.push(prop.clone());
        }
    }

    if !not_found.is_empty() {
        let missing = not_found.join(", ");
        return Err(eyre!(
            "Property {} not found in frontmatter",
            if not_found.len() == 1 {
                format!("\"{}\"", missing)
            } else {
                format!(
                    "[{}]",
                    not_found
                        .iter()
                        .map(|p| format!("\"{p}\""))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        ));
    }

    std::fs::write(&resolved, md.as_string())
        .wrap_err_with(|| format!("Failed to write to {:?}", resolved))?;

    let remaining: Vec<String> = md.frontmatter().as_map().keys().cloned().collect();

    if json {
        let output = serde_json::json!({
            "removed": removed,
            "remaining": remaining,
            "filename": resolved.to_string_lossy(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if cli.verbose > 0 {
        use biscuit_terminal::components::prose::Prose;
        use biscuit_terminal::components::renderable::TerminalRenderable;
        use biscuit_terminal::terminal::Terminal;
        let props_label = if removed.len() == 1 {
            format!("<b>{}</b> property", removed[0])
        } else {
            format!(
                "<b>{}</b> properties",
                removed
                    .iter()
                    .map(|p| format!("\"{}\"", p))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let remaining_label = remaining.join(", ");
        let terminal = Terminal::default();
        eprintln!(
            "{}",
            Prose::new(format!(
                "- removed the {} from frontmatter (<dim>remaining: <i>{}</i></dim>)",
                props_label, remaining_label
            ))
            .render(&terminal)
        );
    }

    Ok(())
}

/// Format a `serde_json::Value` according to the requested output format.
fn format_value(
    value: &serde_json::Value,
    json5: bool,
    yaml: bool,
    toml: bool,
    raw: bool,
    compact: bool,
) -> Result<String> {
    if raw {
        return Ok(format_raw(value));
    }
    if compact {
        return Ok(serde_json::to_string(value)?);
    }
    if json5 {
        let json_str = serde_json::to_string(value)?;
        let j5 = biscuit_file::Json5::from_str(&json_str)
            .map_err(|e| eyre!("JSON5 conversion failed: {e}"))?;
        Ok(j5.as_json5())
    } else if yaml {
        let json_str = serde_json::to_string(value)?;
        let j5 = biscuit_file::Json5::from_str(&json_str)
            .map_err(|e| eyre!("YAML conversion failed: {e}"))?;
        let mut yaml_str = j5
            .as_yaml()
            .map_err(|e| eyre!("YAML conversion failed: {e}"))?;
        // serde_yaml appends a trailing newline; trim for consistent output
        if yaml_str.ends_with('\n') {
            yaml_str.truncate(yaml_str.len() - 1);
        }
        Ok(yaml_str)
    } else if toml {
        // TOML requires a table at the top level; wrap scalar values
        let toml_value = if value.is_object() {
            value.clone()
        } else {
            serde_json::json!({ "value": value })
        };
        let json_str = serde_json::to_string(&toml_value)?;
        let j5 = biscuit_file::Json5::from_str(&json_str)
            .map_err(|e| eyre!("TOML conversion failed: {e}"))?;
        let mut toml_str = j5
            .as_toml()
            .map_err(|e| eyre!("TOML conversion failed: {e}"))?;
        // Trim trailing newline for consistent output
        if toml_str.ends_with('\n') {
            toml_str.truncate(toml_str.len() - 1);
        }
        Ok(toml_str)
    } else {
        // Default: JSON
        Ok(serde_json::to_string_pretty(value)?)
    }
}

/// Format a JSON value as raw output (no JSON quoting).
///
/// - Strings: printed without quotes
/// - Null: empty string
/// - Booleans/numbers: their natural representation
/// - Arrays: one element per line (each element formatted raw)
/// - Objects: one `key: value` pair per line (values formatted raw)
fn format_raw(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => arr.iter().map(format_raw).collect::<Vec<_>>().join("\n"),
        serde_json::Value::Object(map) => map
            .iter()
            .map(|(k, v)| format!("{k}: {}", format_raw(v)))
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// Open a file in the user's preferred editor, blocking until the editor exits.
///
/// Resolves the file path using biscuit-file's `FileReference` system. Creates the
/// file if it doesn't exist. After the editor exits, validates that the file exists
/// and is non-empty (after trimming whitespace). Prints the fully qualified path on
/// success.
pub fn run_edit(raw_file: &str) -> Result<()> {
    use biscuit_file::FileReference;

    // --- Resolve the file path ---
    let path = match FileReference::new(raw_file) {
        Ok(file_ref) => {
            let resolved = file_ref
                .resolve()
                .wrap_err("Failed to resolve file reference")?;
            match resolved {
                Some(p) => p,
                None => {
                    // FileReference couldn't resolve it — treat raw input as a
                    // relative path (may not exist yet, which is fine).
                    std::env::current_dir()
                        .wrap_err("Failed to get current directory")?
                        .join(raw_file)
                }
            }
        }
        Err(_) => {
            // Not a valid file reference syntax — treat as plain path.
            let p = PathBuf::from(raw_file);
            if p.is_absolute() {
                p
            } else {
                std::env::current_dir()
                    .wrap_err("Failed to get current directory")?
                    .join(raw_file)
            }
        }
    };

    // Ensure parent directory exists
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent)
            .wrap_err_with(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    // Create the file if it doesn't exist
    if !path.exists() {
        std::fs::write(&path, "")
            .wrap_err_with(|| format!("Failed to create file: {}", path.display()))?;
    }

    // --- Launch the editor ---
    let canonical = path
        .canonicalize()
        .wrap_err_with(|| format!("Failed to canonicalize path: {}", path.display()))?;
    darkmatter::editor::launch_editor_on_path(&canonical)?;

    // --- Validate the result ---
    if !canonical.exists() {
        return Err(eyre!(
            "File was deleted during editing: {}",
            canonical.display()
        ));
    }

    let content = std::fs::read_to_string(&canonical)
        .wrap_err_with(|| format!("Failed to read file after editing: {}", canonical.display()))?;

    if content.trim().is_empty() {
        return Err(eyre!(
            "File is empty after editing: {}",
            canonical.display()
        ));
    }

    // Output the fully qualified filename
    println!("{}", canonical.display());

    Ok(())
}
