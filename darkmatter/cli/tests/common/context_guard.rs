//! Shared engine for every package's `context_construction_guard.rs`: three
//! source-scan gates that keep file resolution on one prepared request
//! context.
//!
//! - **Construction.** `FileResolutionContext::new`,
//!   `FileResolutionContext::from_snapshot`, and any `::from_process` call.
//!   Contexts come from the builder; a binary reads the process once.
//! - **Optional context.** `Option<FileResolutionContext>` and
//!   `Option<&FileResolutionContext>` (any spacing, path prefix, lifetime, or
//!   `mut`). This gate takes no allowlist: a context is required.
//! - **Ambient state.** Process reads that would bypass the request snapshot:
//!   `std::env::{current_dir, var, vars, var_os, vars_os, home_dir}`,
//!   `dirs::home_dir`, `home::home_dir`, biscuit-file's `home_dir` and
//!   `capture_env`, the context-free `FileReference` resolvers (`.resolve()`,
//!   `.resolve_from(..)`, `.resolve_target()`), and a `PortablePath` built
//!   without `.with_ctx(..)` in the same expression.
//!
//! A package scans its own `src/` under `source_scan`'s production-scope
//! rule (comments, literals, and `#[cfg(test)]` code blanked). Every site must
//! match an [`Allowance`] exactly: an unlisted site, a moved count, an unused
//! entry, a duplicate entry, or an entry without a reason fails. An ambient
//! read may be allowlisted only when it feeds neither file resolution, `ctx.*`,
//! nor `env.*`.
//!
//! A package other than `darkmatter-cli` that includes this file declares both
//! it and `source_scan.rs` in `[package.metadata.ci.tests] source-inputs`, and
//! spells both with `include_str!` inside the guard test, because CI's
//! test-input index does not count a `#[path]` include as a reference.

// Each including test binary uses a different subset.
#![allow(dead_code)]

// A binary's other source-scan gate may load `source_scan.rs` too; the copies
// are stateless, so loading it twice is harmless.
#[allow(clippy::duplicate_mod)]
#[path = "source_scan.rs"]
mod source_scan;

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use source_scan::{ident_offsets, is_ident, line_at, production_sources};

/// The three gates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Gate {
    Construction,
    OptionalContext,
    AmbientState,
}

impl fmt::Display for Gate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Construction => "construction",
            Self::OptionalContext => "optional-context",
            Self::AmbientState => "ambient-state",
        })
    }
}

/// One allowlisted `(path, identifier)` pair with its exact occurrence count.
/// `path` is `/`-separated and relative to the scanned `src/`; `identifier` is
/// the label the scan reports (see the failure message for a new site).
#[derive(Clone, Copy, Debug)]
pub struct Allowance {
    pub gate: Gate,
    pub path: &'static str,
    pub identifier: &'static str,
    pub count: usize,
    pub reason: &'static str,
}

/// One gate hit in production source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Site {
    pub gate: Gate,
    pub path: String,
    pub identifier: String,
    pub line: usize,
}

const CONSTRUCTORS: &[&str] = &["new", "from_snapshot"];
const ENV_READS: &[&str] = &["current_dir", "var", "vars", "var_os", "vars_os", "home_dir"];
const PORTABLE_CONSTRUCTORS: &[&str] = &["from_path", "from_reference"];
/// Context-free `FileReference` resolvers, as `(method, takes arguments)`.
const AMBIENT_RESOLVERS: &[(&str, bool)] = &[("resolve", false), ("resolve_target", false), ("resolve_from", true)];
pub const PORTABLE_WITHOUT_CTX: &str = "PortablePath without with_ctx";

/// Fails with every problem the three gates find under `src`.
pub fn assert_guarded(package: &str, src: &Path, allowlist: &[Allowance]) {
    let problems = problems(src, allowlist);
    assert!(
        problems.is_empty(),
        "{package} context construction guard ({}):\n  {}",
        src.display(),
        problems.join("\n  ")
    );
}

pub fn problems(src: &Path, allowlist: &[Allowance]) -> Vec<String> {
    check(&scan(&production_sources(src)), allowlist)
}

/// Every gate hit in already-sanitized production sources.
pub fn scan(sources: &BTreeMap<String, String>) -> Vec<Site> {
    let mut sites = Vec::new();
    for (path, text) in sources {
        let bytes = text.as_bytes();
        let mut push = |gate: Gate, identifier: String, offset: usize| {
            sites.push(Site { gate, path: path.clone(), identifier, line: line_at(text, offset) });
        };
        for offset in ident_offsets(text, "FileResolutionContext") {
            if let Some(name) = path_segment_after(bytes, offset + "FileResolutionContext".len())
                && CONSTRUCTORS.contains(&name)
            {
                push(Gate::Construction, format!("FileResolutionContext::{name}"), offset);
            }
        }
        for offset in ident_offsets(text, "from_process") {
            if let Some(qualifier) = qualifier_before(bytes, offset) {
                push(Gate::Construction, format!("{qualifier}::from_process"), offset);
            }
        }
        for offset in ident_offsets(text, "Option") {
            if let Some(identifier) = optional_context(bytes, offset + "Option".len()) {
                push(Gate::OptionalContext, identifier.to_string(), offset);
            }
        }
        scan_ambient(text, &mut push);
    }
    sites
}

fn scan_ambient(text: &str, push: &mut impl FnMut(Gate, String, usize)) {
    let bytes = text.as_bytes();
    for offset in ident_offsets(text, "env") {
        let after = skip_ws(bytes, offset + "env".len());
        if !bytes[after..].starts_with(b"::") {
            continue;
        }
        let next = skip_ws(bytes, after + 2);
        if bytes.get(next) == Some(&b'{') {
            let close = source_scan::matching_brace(bytes, next);
            let group = &text[next..=close];
            for name in ENV_READS {
                for inner in ident_offsets(group, name) {
                    push(Gate::AmbientState, format!("std::env::{name}"), next + inner);
                }
            }
        } else if let Some(name) = ident_at(bytes, next)
            && ENV_READS.contains(&name)
        {
            push(Gate::AmbientState, format!("std::env::{name}"), offset);
        }
    }
    for function in ["home_dir", "capture_env"] {
        for offset in ident_offsets(text, function) {
            if is_method_or_definition(bytes, offset) {
                continue;
            }
            match qualifier_before(bytes, offset) {
                Some("env") => {}
                Some(qualifier) => push(Gate::AmbientState, format!("{qualifier}::{function}"), offset),
                None if bytes.get(skip_ws(bytes, offset + function.len())) == Some(&b'(') => {
                    push(Gate::AmbientState, format!("{function}()"), offset)
                }
                None => {}
            }
        }
    }
    for &(method, takes_arguments) in AMBIENT_RESOLVERS {
        for offset in ident_offsets(text, method) {
            if previous_non_ws(bytes, offset) != Some(b'.') {
                continue;
            }
            let open = skip_ws(bytes, offset + method.len());
            if bytes.get(open) != Some(&b'(') {
                continue;
            }
            let empty = bytes.get(skip_ws(bytes, open + 1)) == Some(&b')');
            if empty != takes_arguments {
                let label = if takes_arguments { format!(".{method}(..)") } else { format!(".{method}()") };
                push(Gate::AmbientState, label, offset);
            }
        }
    }
    for offset in ident_offsets(text, "PortablePath") {
        let Some(name) = path_segment_after(bytes, offset + "PortablePath".len()) else { continue };
        if PORTABLE_CONSTRUCTORS.contains(&name) {
            let end = expression_end(bytes, offset);
            if ident_offsets(&text[offset..end], "with_ctx").is_empty() {
                push(Gate::AmbientState, PORTABLE_WITHOUT_CTX.to_string(), offset);
            }
        }
    }
}

/// Compares the scanned sites with the allowlist.
pub fn check(sites: &[Site], allowlist: &[Allowance]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut allowed: BTreeMap<(Gate, &str, &str), &Allowance> = BTreeMap::new();
    for entry in allowlist {
        if entry.gate == Gate::OptionalContext {
            problems.push(format!(
                "allowlist entry for {} `{}` in {}: the optional-context gate takes no allowlist",
                entry.gate, entry.identifier, entry.path
            ));
        }
        if entry.reason.trim().is_empty() || entry.count == 0 {
            problems.push(format!(
                "allowlist entry for {} `{}` in {} needs a reason and a non-zero count",
                entry.gate, entry.identifier, entry.path
            ));
        }
        if allowed.insert((entry.gate, entry.path, entry.identifier), entry).is_some() {
            problems.push(format!("duplicate allowlist entry: {} `{}` in {}", entry.gate, entry.identifier, entry.path));
        }
    }

    let mut found: BTreeMap<(Gate, &str, &str), Vec<usize>> = BTreeMap::new();
    for site in sites {
        found.entry((site.gate, &site.path, &site.identifier)).or_default().push(site.line);
    }
    for (&(gate, path, identifier), lines) in &found {
        match allowed.get(&(gate, path, identifier)) {
            None => problems.push(format!(
                "unlisted {gate} site `{identifier}` in {path} at lines {lines:?}; remove it, or (if the gate allows) add \
                 Allowance {{ gate: Gate::{gate:?}, path: \"{path}\", identifier: \"{identifier}\", count: {}, reason: \"..\" }}",
                lines.len()
            )),
            Some(entry) if entry.count != lines.len() => problems.push(format!(
                "moved count for {gate} `{identifier}` in {path}: allowlisted {}, found {} at lines {lines:?}",
                entry.count,
                lines.len()
            )),
            Some(_) => {}
        }
    }
    for (key, entry) in &allowed {
        if !found.contains_key(key) {
            problems.push(format!(
                "unused allowlist entry: {} `{}` in {} no longer occurs",
                entry.gate, entry.identifier, entry.path
            ));
        }
    }
    problems
}

fn skip_ws(bytes: &[u8], mut index: usize) -> usize {
    while bytes.get(index).is_some_and(|byte| byte.is_ascii_whitespace()) {
        index += 1;
    }
    index
}

fn previous_non_ws(bytes: &[u8], offset: usize) -> Option<u8> {
    bytes[..offset].iter().rev().copied().find(|byte| !byte.is_ascii_whitespace())
}

/// The identifier starting at `index`, if one does.
fn ident_at(bytes: &[u8], index: usize) -> Option<&str> {
    let length = bytes.get(index..)?.iter().take_while(|&&byte| is_ident(byte)).count();
    (length > 0).then(|| std::str::from_utf8(&bytes[index..index + length]).expect("ASCII identifier"))
}

/// The identifier after `::` that follows `end`, as in `Type::name`.
fn path_segment_after(bytes: &[u8], end: usize) -> Option<&str> {
    let separator = skip_ws(bytes, end);
    bytes[separator..].starts_with(b"::").then(|| ident_at(bytes, skip_ws(bytes, separator + 2)))?
}

/// The identifier before the `::` that precedes `offset`, as in `qualifier::name`.
fn qualifier_before(bytes: &[u8], offset: usize) -> Option<&str> {
    let mut cursor = offset;
    while cursor > 0 && bytes[cursor - 1].is_ascii_whitespace() {
        cursor -= 1;
    }
    if cursor < 2 || &bytes[cursor - 2..cursor] != b"::" {
        return None;
    }
    cursor -= 2;
    while cursor > 0 && bytes[cursor - 1].is_ascii_whitespace() {
        cursor -= 1;
    }
    let end = cursor;
    while cursor > 0 && is_ident(bytes[cursor - 1]) {
        cursor -= 1;
    }
    (cursor < end).then(|| std::str::from_utf8(&bytes[cursor..end]).expect("ASCII identifier"))
}

/// Whether the identifier at `offset` is a method call or a `fn` definition.
fn is_method_or_definition(bytes: &[u8], offset: usize) -> bool {
    if previous_non_ws(bytes, offset) == Some(b'.') {
        return true;
    }
    let trimmed = bytes[..offset].trim_ascii_end();
    trimmed.ends_with(b"fn") && trimmed.len().checked_sub(3).is_none_or(|at| !is_ident(trimmed[at]))
}

/// The gate label when `Option` at `after - "Option".len()` wraps a context.
fn optional_context(bytes: &[u8], after: usize) -> Option<&'static str> {
    let mut cursor = skip_ws(bytes, after);
    if bytes.get(cursor) != Some(&b'<') {
        return None;
    }
    cursor = skip_ws(bytes, cursor + 1);
    let borrowed = bytes.get(cursor) == Some(&b'&');
    if borrowed {
        cursor = skip_ws(bytes, cursor + 1);
        if bytes.get(cursor) == Some(&b'\'') {
            cursor = skip_ws(bytes, cursor + 1 + ident_at(bytes, cursor + 1)?.len());
        }
        if ident_at(bytes, cursor) == Some("mut") {
            cursor = skip_ws(bytes, cursor + 3);
        }
    }
    if bytes[cursor..].starts_with(b"::") {
        cursor = skip_ws(bytes, cursor + 2);
    }
    let mut last = ident_at(bytes, cursor)?;
    cursor = skip_ws(bytes, cursor + last.len());
    while bytes[cursor..].starts_with(b"::") {
        cursor = skip_ws(bytes, cursor + 2);
        last = ident_at(bytes, cursor)?;
        cursor = skip_ws(bytes, cursor + last.len());
    }
    (last == "FileResolutionContext" && bytes.get(cursor) == Some(&b'>')).then_some(if borrowed {
        "Option<&FileResolutionContext>"
    } else {
        "Option<FileResolutionContext>"
    })
}

/// End of the expression holding `start`: the first `;` or `,` at its own
/// nesting depth, or the bracket that closes the enclosing one.
fn expression_end(bytes: &[u8], start: usize) -> usize {
    let mut depth = 0usize;
    for (offset, &byte) in bytes.iter().enumerate().skip(start) {
        match byte {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' if depth == 0 => return offset,
            b')' | b']' | b'}' => depth -= 1,
            b';' | b',' if depth == 0 => return offset,
            _ => {}
        }
    }
    bytes.len()
}
