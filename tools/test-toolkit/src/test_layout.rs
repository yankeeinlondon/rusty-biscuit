//! Layout gate for a package whose integration tests are consolidated into
//! explicitly declared test binaries (`2026-09-21-consolidated-test-binaries`,
//! acceptance 12).
//!
//! With `autotests = false`, Cargo builds only the `[[test]]` roots the
//! manifest declares. A new top-level `tests/*.rs` file, an undeclared
//! `tests/<x>/main.rs`, or a module file no `mod` reaches is then never
//! compiled, and its tests silently never run. [`layout_violations`] walks the
//! Rust module graph from the declared roots and reports every `tests/` source
//! it does not reach, so one rule covers all three cases.
//!
//! The walk is textual: a `cfg`-gated `mod` still counts as declared (the gate
//! decides where a module compiles, not whether anything compiles it), and a
//! `mod x;` inside a comment or string literal does not.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::Path;

/// Directories under `tests/` that hold data, never Rust modules.
const DATA_DIRECTORIES: &[&str] = &["fixtures", "snapshots"];

/// Every `.rs` file under `crate_root/tests/`, keyed by its `/`-separated path
/// relative to `crate_root` (`tests/l1/main.rs`), with its source.
///
/// `fixtures/` and `snapshots/` directories are skipped at any depth.
pub fn collect_test_sources(crate_root: &Path) -> io::Result<BTreeMap<String, String>> {
    let mut files = BTreeMap::new();
    collect(crate_root, &crate_root.join("tests"), &mut files)?;
    Ok(files)
}

fn collect(crate_root: &Path, directory: &Path, files: &mut BTreeMap<String, String>) -> io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            if !path
                .file_name()
                .is_some_and(|name| DATA_DIRECTORIES.iter().any(|data| name == *data))
            {
                collect(crate_root, &path, files)?;
            }
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let relative = path.strip_prefix(crate_root).unwrap_or(&path);
            let relative = relative
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            files.insert(relative, fs::read_to_string(&path)?);
        }
    }
    Ok(())
}

/// Every layout violation for a crate whose `Cargo.toml` text is `manifest`
/// and whose `tests/` sources are `files` (as [`collect_test_sources`]
/// returns them). Empty means the layout holds.
///
/// Besides unreached files, it requires `autotests = false` and an explicit,
/// existing `path` on every `[[test]]`.
pub fn layout_violations(manifest: &str, files: &BTreeMap<String, String>) -> Vec<String> {
    let mut violations = Vec::new();
    let manifest: toml::Table = match toml::from_str(manifest) {
        Ok(table) => table,
        Err(error) => return vec![format!("Cargo.toml does not parse: {error}")],
    };
    let autotests = manifest
        .get("package")
        .and_then(|package| package.get("autotests"))
        .and_then(toml::Value::as_bool);
    if autotests != Some(false) {
        violations.push(
            "Cargo.toml must set `autotests = false`: every test target is declared explicitly"
                .to_string(),
        );
    }

    let mut pending = Vec::new();
    for target in manifest
        .get("test")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
    {
        let name = target.get("name").and_then(toml::Value::as_str).unwrap_or("<unnamed>");
        match target.get("path").and_then(toml::Value::as_str) {
            Some(path) if files.contains_key(&normalize_relative(path)) => {
                pending.push(normalize_relative(path));
            }
            Some(path) => violations.push(format!("[[test]] {name}: {path} does not exist")),
            None => violations.push(format!("[[test]] {name}: declare its `path` explicitly")),
        }
    }

    let mut reached = BTreeSet::new();
    while let Some(file) = pending.pop() {
        if !reached.insert(file.clone()) {
            continue;
        }
        for declaration in module_declarations(&files[&file]) {
            if let Some(found) = declared_module_files(&file, &declaration)
                .into_iter()
                .find(|candidate| files.contains_key(candidate))
            {
                pending.push(found);
            }
        }
    }

    for file in files.keys().filter(|file| !reached.contains(*file)) {
        let (directory, name) = file.rsplit_once('/').unwrap_or(("", file));
        violations.push(if directory == "tests" {
            format!(
                "{file}: a top-level test file is never compiled; move it into a declared \
                 target's directory and declare it in that target's main.rs"
            )
        } else if name == "main.rs" {
            format!(
                "{file}: an undeclared test crate root; add it to an existing target, or \
                 declare a [[test]] for a new execution contract"
            )
        } else {
            format!("{file}: no declared test target compiles this module; declare it with `mod`")
        });
    }
    violations
}

/// One out-of-line `mod name;` declaration and its `#[path]` value, if any.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ModuleDeclaration {
    pub(crate) name: String,
    pub(crate) path: Option<String>,
}

/// Every out-of-line `mod name;` in `source`, whatever its `cfg` or visibility.
pub(crate) fn module_declarations(source: &str) -> Vec<ModuleDeclaration> {
    let code = sanitize(source);
    let mut declarations = Vec::new();
    let mut index = 0;
    while index < code.len() {
        if index > 0 && is_ident(code[index - 1]) {
            index += 1;
            continue;
        }
        let mut cursor = index;
        let mut path = None;
        while code.get(cursor) == Some(&b'#') && code.get(cursor + 1) == Some(&b'[') {
            let Some(close) = matching_bracket(&code, cursor + 1) else {
                break;
            };
            // The attribute's string value is blanked in `code`, so it is read
            // back from `source` at the same offsets.
            path = path.or_else(|| path_attribute(&source[cursor..=close]));
            cursor = skip_whitespace(&code, close + 1);
        }
        cursor = skip_visibility(&code, cursor);
        if code.get(cursor..cursor + 3) == Some(b"mod")
            && code.get(cursor + 3).is_some_and(u8::is_ascii_whitespace)
        {
            let name_start = skip_whitespace(&code, cursor + 3);
            let mut name_end = name_start;
            while code.get(name_end).is_some_and(|&byte| is_ident(byte)) {
                name_end += 1;
            }
            let after = skip_whitespace(&code, name_end);
            if name_end > name_start && code.get(after) == Some(&b';') {
                declarations.push(ModuleDeclaration {
                    name: source[name_start..name_end].to_string(),
                    path,
                });
                index = after + 1;
                continue;
            }
        }
        index = cursor.max(index + 1);
    }
    declarations
}

/// Where rustc looks for `declaration` made in `file` (crate-relative paths).
///
/// A `#[path]` is relative to the declaring file's directory. A plain `mod x;`
/// resolves beside a `main.rs`/`mod.rs`, and under `<stem>/` for any other
/// file.
pub(crate) fn declared_module_files(file: &str, declaration: &ModuleDeclaration) -> Vec<String> {
    let (directory, name) = file.rsplit_once('/').unwrap_or(("", file));
    if let Some(path) = &declaration.path {
        return vec![normalize_relative(&format!("{directory}/{path}"))];
    }
    let base = if name == "main.rs" || name == "mod.rs" {
        directory.to_string()
    } else {
        format!("{directory}/{}", name.trim_end_matches(".rs"))
    };
    vec![
        normalize_relative(&format!("{base}/{}.rs", declaration.name)),
        normalize_relative(&format!("{base}/{}/mod.rs", declaration.name)),
    ]
}

/// `a/b/../c` → `a/c`, on `/`-separated relative paths.
fn normalize_relative(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            part => parts.push(part),
        }
    }
    parts.join("/")
}

fn path_attribute(attribute: &str) -> Option<String> {
    let inner = attribute.strip_prefix("#[")?.trim_start();
    let value = inner.strip_prefix("path")?.trim_start().strip_prefix('=')?;
    let value = value.trim_start().strip_prefix('"')?;
    Some(value[..value.find('"')?].to_string())
}

fn is_ident(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}

fn skip_whitespace(bytes: &[u8], mut index: usize) -> usize {
    while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
        index += 1;
    }
    index
}

fn matching_bracket(bytes: &[u8], open: usize) -> Option<usize> {
    matching(bytes, open, b'[', b']')
}

fn matching(bytes: &[u8], open: usize, opening: u8, closing: u8) -> Option<usize> {
    let mut depth = 0usize;
    for (index, &byte) in bytes.iter().enumerate().skip(open) {
        if byte == opening {
            depth += 1;
        } else if byte == closing {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn skip_visibility(bytes: &[u8], cursor: usize) -> usize {
    if bytes.get(cursor..cursor + 3) != Some(b"pub") || bytes.get(cursor + 3).is_some_and(|&byte| is_ident(byte)) {
        return cursor;
    }
    let mut cursor = skip_whitespace(bytes, cursor + 3);
    if bytes.get(cursor) == Some(&b'(')
        && let Some(close) = matching(bytes, cursor, b'(', b')')
    {
        cursor = skip_whitespace(bytes, close + 1);
    }
    cursor
}

/// `source` with comments and string/char literals blanked to spaces; every
/// byte offset and newline is preserved, so offsets index back into `source`.
fn sanitize(source: &str) -> Vec<u8> {
    let bytes = source.as_bytes();
    let mut output = bytes.to_vec();
    let blank = |output: &mut [u8], from: usize, to: usize| {
        for byte in output.iter_mut().take(to).skip(from) {
            if !matches!(*byte, b'\n' | b'\r') {
                *byte = b' ';
            }
        }
    };

    let mut index = 0;
    while index < bytes.len() {
        let start = index;
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
            while index < bytes.len() && !matches!(bytes[index], b'\n' | b'\r') {
                index += 1;
            }
        } else if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            let mut depth = 1usize;
            index += 2;
            while index < bytes.len() && depth > 0 {
                if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
                    depth += 1;
                    index += 2;
                } else if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                    depth -= 1;
                    index += 2;
                } else {
                    index += 1;
                }
            }
        } else if let Some((content_start, hashes)) = raw_string_open(bytes, index) {
            index = content_start;
            while index < bytes.len() {
                if bytes[index] == b'"'
                    && bytes[index + 1..].len() >= hashes
                    && bytes[index + 1..].iter().take(hashes).all(|&byte| byte == b'#')
                {
                    index += 1 + hashes;
                    break;
                }
                index += 1;
            }
        } else if bytes[index] == b'"' {
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    index = (index + 2).min(bytes.len());
                } else if bytes[index] == b'"' {
                    index += 1;
                    break;
                } else {
                    index += 1;
                }
            }
        } else if bytes[index] == b'\''
            && let Some(end) = char_literal_end(bytes, index)
        {
            index = end;
        } else {
            index += 1;
            continue;
        }
        blank(&mut output, start, index);
    }
    output
}

fn raw_string_open(bytes: &[u8], index: usize) -> Option<(usize, usize)> {
    if index > 0 && is_ident(bytes[index - 1]) {
        return None;
    }
    let mut cursor = index;
    if bytes.get(cursor) == Some(&b'b') {
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b'r') {
        return None;
    }
    cursor += 1;
    let mut hashes = 0;
    while bytes.get(cursor) == Some(&b'#') {
        hashes += 1;
        cursor += 1;
    }
    (bytes.get(cursor) == Some(&b'"')).then_some((cursor + 1, hashes))
}

/// End of the char literal opening at `index`, or `None` for a lifetime.
fn char_literal_end(bytes: &[u8], index: usize) -> Option<usize> {
    let mut cursor = index + 1;
    if bytes.get(cursor) == Some(&b'\\') {
        cursor += 1;
        if bytes.get(cursor) == Some(&b'u') && bytes.get(cursor + 1) == Some(&b'{') {
            cursor += 2;
            while cursor < bytes.len() && bytes[cursor] != b'}' {
                cursor += 1;
            }
        }
        cursor += 1;
    } else {
        let width = std::str::from_utf8(&bytes[cursor..]).ok()?.chars().next()?.len_utf8();
        cursor += width;
    }
    (bytes.get(cursor) == Some(&b'\'')).then_some(cursor + 1)
}

#[cfg(test)]
mod tests;
