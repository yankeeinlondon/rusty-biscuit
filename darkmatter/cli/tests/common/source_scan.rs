//! Byte-preserving Rust source sanitizer and production-scope rule shared by
//! the structural gates that read source text: `spawn_site_guard.rs` here,
//! the library's `semantic_results_never_persist.rs`, and every package's
//! `context_construction_guard.rs` (through `context_guard.rs`).
//!
//! A gate answers "does this file *do* X?" by searching source text, and it
//! must not fire on an X that appears in a doc comment or a string literal —
//! the guard's own explanatory prose names the very APIs it forbids.
//! [`sanitize`] blanks comments and literals while keeping every byte offset
//! and newline in place, so a hit in the sanitized buffer indexes straight
//! back into the original source (line numbers, and the literal text a call
//! site passed).
//!
//! [`production_sources`] applies the production-scope rule on top: every
//! `#[cfg(test)]` / `#[cfg(all(test, ..))]` item is blanked, and every file
//! such an item declares with `mod name;` (honoring `#[path]`) is dropped
//! along with everything below that module.

// Each gate including this file by `#[path]` uses a different subset.
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Whether `byte` may appear inside a Rust identifier.
pub fn is_ident(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}

/// Blanks comments and literals while preserving bytes and newline offsets.
pub fn sanitize(source: &str) -> Vec<u8> {
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
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
            let start = index;
            while index < bytes.len() && !matches!(bytes[index], b'\n' | b'\r') {
                index += 1;
            }
            blank(&mut output, start, index);
        } else if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            let start = index;
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
            blank(&mut output, start, index);
        } else if let Some((content_start, hashes)) = raw_string_open(bytes, index) {
            let start = index;
            index = content_start;
            while index < bytes.len() {
                if bytes[index] == b'"'
                    && bytes[index + 1..].len() >= hashes
                    && bytes[index + 1..]
                        .iter()
                        .take(hashes)
                        .all(|&byte| byte == b'#')
                {
                    index += 1 + hashes;
                    break;
                }
                index += 1;
            }
            blank(&mut output, start, index);
        } else if bytes[index] == b'"' {
            let start = index;
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
            blank(&mut output, start, index);
        } else if bytes[index] == b'\'' {
            if let Some(end) = char_literal_end(bytes, index) {
                blank(&mut output, index, end);
                index = end;
            } else {
                index += 1;
            }
        } else {
            index += 1;
        }
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
        let width = std::str::from_utf8(&bytes[cursor..])
            .ok()?
            .chars()
            .next()?
            .len_utf8();
        cursor += width;
    }
    (bytes.get(cursor) == Some(&b'\'')).then_some(cursor + 1)
}

/// 1-indexed line holding `offset`.
pub fn line_at(source: &str, offset: usize) -> usize {
    source.as_bytes()[..offset.min(source.len())]
        .iter()
        .filter(|&&byte| byte == b'\n')
        .count()
        + 1
}

/// Byte offsets of `ident` on identifier boundaries, so `FileStoreX` is not
/// `FileStore`.
pub fn ident_offsets(text: &str, ident: &str) -> Vec<usize> {
    let bytes = text.as_bytes();
    text.match_indices(ident)
        .map(|(offset, _)| offset)
        .filter(|&offset| {
            let before = offset.checked_sub(1).map(|at| bytes[at]);
            let after = bytes.get(offset + ident.len()).copied();
            !before.is_some_and(is_ident) && !after.is_some_and(is_ident)
        })
        .collect()
}

/// Production source per `/`-separated path relative to `src`, with comments,
/// literals, and test-only code blanked (byte offsets preserved).
pub fn production_sources(src: &Path) -> BTreeMap<String, String> {
    let mut files = Vec::new();
    let mut pending = vec![src.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).unwrap_or_else(|error| panic!("read {}: {error}", dir.display())) {
            let path = entry.expect("directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }

    let mut blanked = BTreeMap::new();
    let mut test_modules = BTreeSet::new();
    for path in files {
        let text = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        let mut bytes = sanitize(&text);
        for declared in blank_test_items(&text, &mut bytes) {
            test_modules.insert(test_module_path(&path, &declared));
        }
        blanked.insert(path, String::from_utf8(bytes).expect("blanking keeps UTF-8"));
    }

    blanked
        .into_iter()
        .filter(|(path, _)| !test_modules.iter().any(|module| is_within_test_module(path, module)))
        .map(|(path, text)| (relative_key(src, &path), text))
        .collect()
}

/// A `mod name;` declaration found under a test-only annotation.
struct DeclaredModule {
    name: String,
    path_attribute: Option<String>,
}

/// Blanks every `#[cfg(test)]` / `#[cfg(all(test, ..))]` item or statement in
/// `sanitized`, returning the out-of-line modules those annotations declare.
/// `original` supplies `#[path]` values, which `sanitized` has blanked.
fn blank_test_items(original: &str, sanitized: &mut [u8]) -> Vec<DeclaredModule> {
    const MARKERS: [&[u8]; 2] = [b"#[cfg(test)]", b"#[cfg(all(test"];
    let mut declared = Vec::new();
    let mut index = 0;
    while index < sanitized.len() {
        if !MARKERS.iter().any(|marker| sanitized[index..].starts_with(marker)) {
            index += 1;
            continue;
        }
        let start = index;
        let mut cursor = index;
        let end = loop {
            match sanitized.get(cursor) {
                None => break sanitized.len(),
                Some(b';') => {
                    let item = &original[start..=cursor];
                    if let Some(name) = out_of_line_module(&original[..=cursor]) {
                        declared.push(DeclaredModule { name, path_attribute: path_attribute(item) });
                    }
                    break cursor + 1;
                }
                Some(b'{') => break matching_brace(sanitized, cursor) + 1,
                Some(_) => cursor += 1,
            }
        };
        for byte in &mut sanitized[start..end] {
            if !matches!(*byte, b'\n' | b'\r') {
                *byte = b' ';
            }
        }
        index = end;
    }
    declared
}

/// Offset of the `}` closing the `{` at `open`; braces inside comments and
/// literals are already blanked.
pub fn matching_brace(bytes: &[u8], open: usize) -> usize {
    let mut depth = 0usize;
    for (offset, byte) in bytes.iter().enumerate().skip(open) {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return offset;
                }
            }
            _ => {}
        }
    }
    bytes.len() - 1
}

/// The module name when the text ends in `mod <name>;`.
fn out_of_line_module(through_semicolon: &str) -> Option<String> {
    let before = through_semicolon.strip_suffix(';')?.trim_end();
    let name_start = before.rfind(|character: char| !(character == '_' || character.is_ascii_alphanumeric()))? + 1;
    let name = &before[name_start..];
    before[..name_start].trim_end().ends_with("mod").then(|| name.to_string())
}

fn path_attribute(item: &str) -> Option<String> {
    let start = item.find("#[path")?;
    let open = start + item[start..].find('"')? + 1;
    let close = open + item[open..].find('"')?;
    Some(item[open..close].to_string())
}

/// The file (or directory) a test-only `mod` declaration in `declaring` names.
fn test_module_path(declaring: &Path, declared: &DeclaredModule) -> PathBuf {
    let directory = declaring.parent().expect("a file has a parent");
    if let Some(path) = &declared.path_attribute {
        return directory.join(path);
    }
    let owns_directory = declaring
        .file_name()
        .is_some_and(|name| name == "mod.rs" || name == "lib.rs" || name == "main.rs");
    let module_directory = if owns_directory {
        directory.to_path_buf()
    } else {
        directory.join(declaring.file_stem().expect("a file has a stem"))
    };
    module_directory.join(&declared.name)
}

/// Whether `path` is the test module `module` (`module.rs`, a `#[path]`
/// target) or lies below its directory.
fn is_within_test_module(path: &Path, module: &Path) -> bool {
    path == module || path == module.with_extension("rs") || path.starts_with(module)
}

fn relative_key(src: &Path, path: &Path) -> String {
    path.strip_prefix(src)
        .expect("under src")
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
