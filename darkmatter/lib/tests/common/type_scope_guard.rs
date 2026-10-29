//! Shared engine for the type-scope ownership guards
//! (`2026-10-16-type-narrowing`, acceptance 8): the library's
//! `type_scope_ownership_guard.rs` and DMLS's `type_scope_ownership_guard.rs`.
//!
//! Static presence and type decisions have one owner, the shared
//! `TypeScope`. The guard reads a package's production source under
//! `source_scan`'s production-scope rule (comments, literals, and
//! `#[cfg(test)]` code blanked) and rejects:
//!
//! - **Unreviewed scope construction.** Every `unguarded(` call (a
//!   `TypeScope` with no guard facts) is counted per file and enclosing item
//!   (`Type::method` or `function`). Each live site must match an
//!   [`Unguarded`] entry with its exact count and a reason; an unlisted site,
//!   a changed count, and a stale or reasonless entry all fail.
//! - **Retired classifiers.** An identifier from [`Policy::retired`]
//!   anywhere in production source.
//! - **A second guard algebra.** A definition (`fn`, `struct`, `enum`,
//!   `const`, `static`, `type`, `trait`) of an [`Owned`] name outside its
//!   owner files, and a quoted presence-predicate name (`"is_null"`,
//!   `"fileExists"`, …: a predicate-fact table) outside
//!   [`Policy::predicate_literal_owners`].
//! - **A static null decision at a named decision site.** Each
//!   [`DecisionSite`] must still exist, must read the scope through its
//!   `requires` tokens, must not contain its `forbids` tokens, and, where it
//!   has a [`NullRead`], each `Value::Null` read must consult the scope's
//!   guard before the `MissingValue` it can produce.
//!
//! Runtime checks are not decisions and are not matched: a hole filled with
//! `null` (`fill.rs`), the `is_null` / `file_exists` handlers, the
//! evaluator's missing-value observation, and warning-policy absence
//! handling read values, not static facts.
//!
//! **Limits.** A source guard, not proof about compiled code: a macro body
//! expanded elsewhere, and an `unguarded` reached through a renamed import of
//! a function pointer, are not followed.

use std::collections::BTreeMap;
use std::ops::Range;
use std::path::Path;

use super::source_scan::{ident_offsets, is_ident, line_at, matching_brace, production_sources, sanitize};

/// A reviewed `TypeScope::unguarded` construction site.
pub struct Unguarded {
    /// Path relative to the scanned `src` directory.
    pub path: &'static str,
    /// `Type::method` or `function`, as [`enclosing_item`] names it.
    pub item: &'static str,
    pub count: usize,
    pub reason: &'static str,
}

/// A name only its owner files may define.
pub struct Owned {
    pub name: &'static str,
    /// Paths relative to `src`; empty when the package may not define it at all.
    pub owners: &'static [&'static str],
    pub why: &'static str,
}

/// Exactly `count` `Value::Null` reads, each followed by `consult` before the
/// next `MissingValue` construction.
pub struct NullRead {
    pub count: usize,
    pub consult: &'static str,
}

/// A named static decision function and what it must and must not read.
pub struct DecisionSite {
    pub path: &'static str,
    pub item: &'static str,
    pub requires: &'static [&'static str],
    pub forbids: &'static [&'static str],
    pub null_read: Option<NullRead>,
}

pub struct Policy<'a> {
    pub unguarded: &'a [Unguarded],
    pub retired: &'a [&'static str],
    pub owned: &'a [Owned],
    /// Path prefixes (relative to `src`) that may spell a presence-predicate
    /// name as a string literal.
    pub predicate_literal_owners: &'a [&'static str],
    pub decision_sites: &'a [DecisionSite],
}

/// One file: its production text (blanked) and its original text.
pub struct Source {
    pub production: String,
    pub original: String,
}

/// The production sources under `src`, keyed relative to it.
pub fn read_sources(src: &Path) -> BTreeMap<String, Source> {
    production_sources(src)
        .into_iter()
        .map(|(key, production)| {
            let original = std::fs::read_to_string(src.join(&key)).unwrap_or_else(|error| panic!("read {key}: {error}"));
            (key, Source { production, original })
        })
        .collect()
}

/// In-memory sources for the self-tests, sanitized like real ones (they
/// carry no `#[cfg(test)]` items).
pub fn snippets(files: &[(&str, &str)]) -> BTreeMap<String, Source> {
    files
        .iter()
        .map(|(path, text)| {
            let production = String::from_utf8(sanitize(text)).expect("sanitizing keeps UTF-8");
            ((*path).to_string(), Source { production, original: (*text).to_string() })
        })
        .collect()
}

/// Canonical presence-predicate names, compared with underscores removed and
/// case folded, as the registry resolves aliases.
const PRESENCE_PREDICATES: &[&str] = &["isstring", "isnumber", "isarray", "isobject", "isnull", "isempty", "fileexists"];

/// Every violation of `policy` in `sources`, each with `path:line`.
pub fn violations(package: &str, sources: &BTreeMap<String, Source>, policy: &Policy<'_>) -> Vec<String> {
    let mut found = Vec::new();
    assert!(!sources.is_empty(), "{package}: the scan found no source file");

    // Unreviewed scope construction.
    let mut live: BTreeMap<(String, String), Vec<usize>> = BTreeMap::new();
    for (path, source) in sources {
        for offset in ident_offsets(&source.production, "unguarded") {
            if !is_call(&source.production, offset) || is_definition(&source.production, offset) {
                continue;
            }
            let item = enclosing_item(&source.production, offset);
            live.entry((path.clone(), item)).or_default().push(line_at(&source.production, offset));
        }
    }
    for entry in policy.unguarded {
        if entry.reason.trim().is_empty() {
            found.push(format!("{package}/{}: `{}` allowlist entry has no reason", entry.path, entry.item));
        }
        let key = (entry.path.to_string(), entry.item.to_string());
        match live.get(&key) {
            None => found.push(format!(
                "{package}/{}: stale allowlist entry: no `TypeScope::unguarded` in `{}`",
                entry.path, entry.item
            )),
            Some(lines) if lines.len() != entry.count => found.push(format!(
                "{package}/{}:{}: `{}` constructs {} unguarded scope(s); the allowlist says {}",
                entry.path,
                lines[0],
                entry.item,
                lines.len(),
                entry.count
            )),
            Some(_) => {}
        }
    }
    for ((path, item), lines) in &live {
        if !policy.unguarded.iter().any(|entry| entry.path == path && entry.item == item) {
            for line in lines {
                found.push(format!(
                    "{package}/{path}:{line}: unreviewed `TypeScope::unguarded` in `{item}`; a site with a guard must \
                     narrow it, and a site that has none needs an allowlist entry with a reason"
                ));
            }
        }
    }

    for (path, source) in sources {
        let text = &source.production;
        for retired in policy.retired {
            for offset in ident_offsets(text, retired) {
                found.push(format!(
                    "{package}/{path}:{}: `{retired}` is a retired static presence classifier; read the shared TypeScope",
                    line_at(text, offset)
                ));
            }
        }
        for owned in policy.owned {
            if owned.owners.contains(&path.as_str()) {
                continue;
            }
            for offset in ident_offsets(text, owned.name) {
                if is_definition(text, offset) {
                    found.push(format!(
                        "{package}/{path}:{}: defines `{}` outside its owner: {}",
                        line_at(text, offset),
                        owned.name,
                        owned.why
                    ));
                }
            }
        }
        if !policy.predicate_literal_owners.iter().any(|owner| path.starts_with(owner)) {
            for (offset, literal) in predicate_literals(source) {
                found.push(format!(
                    "{package}/{path}:{}: presence-predicate name {literal} outside the registry and the shared fact \
                     table; a predicate's facts come from `TypeScope::assume`",
                    line_at(&source.original, offset)
                ));
            }
        }
    }

    for site in policy.decision_sites {
        let Some(source) = sources.get(site.path) else {
            found.push(format!("{package}/{}: decision site file is gone; update the guard", site.path));
            continue;
        };
        let text = &source.production;
        let Some(body) = item_body(text, site.item) else {
            found.push(format!("{package}/{}: decision site `{}` is gone; update the guard", site.path, site.item));
            continue;
        };
        let line = line_at(text, body.start);
        let body_text = &text[body];
        for token in site.requires {
            if !body_text.contains(token) {
                found.push(format!(
                    "{package}/{}:{line}: `{}` no longer reads `{token}`: its decision must come from the shared TypeScope",
                    site.path, site.item
                ));
            }
        }
        for token in site.forbids {
            if body_text.contains(token) {
                found.push(format!(
                    "{package}/{}:{line}: `{}` reads `{token}`: a static null decision here bypasses the shared TypeScope",
                    site.path, site.item
                ));
            }
        }
        if let Some(null_read) = &site.null_read {
            let reads: Vec<usize> = body_text.match_indices("Value::Null").map(|(at, _)| at).collect();
            if reads.len() != null_read.count {
                found.push(format!(
                    "{package}/{}:{line}: `{}` has {} `Value::Null` read(s), expected {}",
                    site.path,
                    site.item,
                    reads.len(),
                    null_read.count
                ));
            }
            for read in reads {
                let rest = &body_text[read..];
                let decision = rest.find("MissingValue").unwrap_or(rest.len());
                if !rest[..decision].contains(null_read.consult) {
                    found.push(format!(
                        "{package}/{}:{line}: a `Value::Null` read in `{}` decides without consulting `{}`",
                        site.path, site.item, null_read.consult
                    ));
                }
            }
        }
    }
    found
}

/// Panics with every violation.
pub fn assert_owned(package: &str, sources: &BTreeMap<String, Source>, policy: &Policy<'_>) {
    let found = violations(package, sources, policy);
    assert!(found.is_empty(), "type-scope ownership violations:\n{}", found.join("\n"));
}

fn next_non_space(text: &str, from: usize) -> Option<u8> {
    text.as_bytes()[from..].iter().copied().find(|byte| !byte.is_ascii_whitespace())
}

fn previous_word(text: &str, before: usize) -> &str {
    let head = text[..before].trim_end();
    let start = head.rfind(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_')).map_or(0, |at| at + 1);
    &head[start..]
}

fn is_call(text: &str, offset: usize) -> bool {
    let after = offset + "unguarded".len();
    next_non_space(text, after) == Some(b'(')
}

fn is_definition(text: &str, offset: usize) -> bool {
    matches!(previous_word(text, offset), "fn" | "struct" | "enum" | "const" | "static" | "type" | "trait")
}

/// `{ … }` ranges of every `fn` with a body, with its name.
fn functions(text: &str) -> Vec<(String, Range<usize>)> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    for offset in ident_offsets(text, "fn") {
        let name_start = offset + 2 + text[offset + 2..].len() - text[offset + 2..].trim_start().len();
        let name_end = name_start + bytes[name_start..].iter().take_while(|&&byte| is_ident(byte)).count();
        if name_end == name_start {
            continue;
        }
        let Some(stop) = text[name_end..].find(['{', ';']).map(|at| name_end + at) else {
            continue;
        };
        if bytes[stop] == b'{' {
            found.push((text[name_start..name_end].to_string(), stop..matching_brace(bytes, stop) + 1));
        }
    }
    found
}

/// `{ … }` ranges of every `impl` block, with its self type's last segment.
fn impls(text: &str) -> Vec<(String, Range<usize>)> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    for offset in ident_offsets(text, "impl") {
        // An item-level `impl`, not `-> impl Trait` or `x: impl Trait`.
        let before = text[..offset].trim_end();
        if !(before.is_empty() || before.ends_with(['}', ';', '{', ']']) || before.ends_with("unsafe")) {
            continue;
        }
        let Some(open) = text[offset..].find(['{', ';']).map(|at| offset + at) else {
            continue;
        };
        if bytes[open] != b'{' {
            continue;
        }
        let mut header = text[offset + 4..open].trim();
        if header.starts_with('<') {
            let mut depth = 0usize;
            let end = header
                .char_indices()
                .find(|&(_, ch)| {
                    match ch {
                        '<' => depth += 1,
                        '>' => depth -= 1,
                        _ => {}
                    }
                    depth == 0
                })
                .map_or(header.len(), |(at, _)| at + 1);
            header = header[end..].trim_start();
        }
        if let Some(at) = header.find(" for ") {
            header = header[at + 5..].trim_start();
        }
        let path = header.split(|ch: char| ch == '<' || ch.is_whitespace()).next().unwrap_or_default();
        let name = path.rsplit("::").next().unwrap_or(path).to_string();
        found.push((name, open..matching_brace(bytes, open) + 1));
    }
    found
}

/// `Type::method` for a method, `function` for a free function (the
/// innermost one), or `<module>` outside every function.
pub fn enclosing_item(text: &str, offset: usize) -> String {
    let innermost = |ranges: Vec<(String, Range<usize>)>| {
        ranges.into_iter().filter(|(_, range)| range.contains(&offset)).max_by_key(|(_, range)| range.start)
    };
    let Some((function, range)) = innermost(functions(text)) else {
        return "<module>".to_string();
    };
    match innermost(impls(text)) {
        Some((owner, block)) if block.start < range.start => format!("{owner}::{function}"),
        _ => function,
    }
}

/// The body of the item [`enclosing_item`] names `item`.
fn item_body(text: &str, item: &str) -> Option<Range<usize>> {
    functions(text)
        .into_iter()
        .find(|(_, range)| enclosing_item(text, range.start + 1) == item)
        .map(|(_, range)| range)
}

/// Quoted presence-predicate names in production code: a literal whose line
/// still holds code after blanking (so not a comment or a test item) and that
/// no `//` precedes on its line.
fn predicate_literals(source: &Source) -> Vec<(usize, String)> {
    let original = &source.original;
    let mut found = Vec::new();
    let mut cursor = 0;
    while let Some(open) = original[cursor..].find('"').map(|at| cursor + at) {
        let Some(close) = original[open + 1..].find('"').map(|at| open + 1 + at) else {
            break;
        };
        cursor = close + 1;
        let inner = &original[open + 1..close];
        if inner.is_empty() || !inner.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_') {
            cursor = open + 1;
            continue;
        }
        let folded = inner.replace('_', "").to_ascii_lowercase();
        if !PRESENCE_PREDICATES.contains(&folded.as_str()) {
            continue;
        }
        let line_start = original[..open].rfind('\n').map_or(0, |at| at + 1);
        let line_end = original[open..].find('\n').map_or(original.len(), |at| open + at);
        let production_line = source.production.get(line_start..line_end).unwrap_or_default();
        if production_line.trim().is_empty() || original[line_start..open].contains("//") {
            continue;
        }
        found.push((open, format!("\"{inner}\"")));
    }
    found
}
