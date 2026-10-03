//! Shared engine for every package's `path_lookup_guard.rs`: a source guard
//! that keeps filesystem canonicalization and home-directory lookup on
//! biscuit-file's shared helpers.
//!
//! - **Canonicalize.** `std::fs::canonicalize`, `fs::canonicalize` (std or
//!   tokio), `Path::canonicalize` / `PathBuf::canonicalize`, the
//!   zero-argument method `.canonicalize()`, and `dunce::canonicalize`. On
//!   Windows these return verbatim (`\\?\C:\…`) spellings that the
//!   file-reference grammar rejects and that compare unequal to the legacy
//!   spelling another producer wrote, so a result that leaves a private
//!   comparison goes through `biscuit_file::canonicalize_simplified`.
//! - **Home lookup.** `dirs::home_dir`, `std::env::home_dir`, and
//!   `home::home_dir`. These disagree on native Windows (`dirs` ignores
//!   `USERPROFILE`), so a package under this rule reads its home through
//!   `biscuit_file::home_dir`, the one approved process-capture point, which
//!   owns the policy (environment first, relative home is no home).
//!
//! Each form is recognized when written qualified (`std::fs::canonicalize`),
//! through an imported module or module alias (`use std::fs as filesystem;`
//! then `filesystem::canonicalize`), imported bare (`use dunce::canonicalize;`
//! then `canonicalize(..)`), or imported under an alias
//! (`use dirs::home_dir as home;` then `home()`), and as a function reference
//! (`.map(std::fs::canonicalize)`) as well as a call.
//!
//! **Scope.** A package's production source: every directory holding a
//! `[lib]` or `[[bin]]` root its manifest declares (`src/` always), and its
//! build script, under `source_scan`'s production-scope rule (comments,
//! literals, and `#[cfg(test)]` code blanked; test-only module files dropped).
//! Every `#[cfg(..)]` platform branch is scanned. A `[[bin]]` under `tests/`
//! is a test fixture and is not scanned. A missing source root, or a scan that
//! finds no file, fails: an empty scan cannot pass.
//!
//! **Exceptions.** A remaining direct call is an [`Exception`] keyed by rule,
//! repository-relative path, enclosing item (`Type::method`, `outer::inner`),
//! and operation, with its exact occurrence count and a reason. Line numbers
//! appear only in diagnostics, so moving a call keeps its exception. A new
//! call, a changed count, an unused (stale) or duplicate entry, an entry
//! without a reason, and an entry whose [`Kind`] does not fit the site all
//! fail, each reported with `path:line`.
//!
//! **Unresolved candidates.** A `canonicalize` or `home_dir` the scanner
//! cannot attribute (an unknown qualifier such as `descriptor::canonicalize`,
//! a bare call with neither an import nor a local definition) is reported for
//! explicit review rather than ignored; a reviewed one is listed as
//! [`Kind::Reviewed`] with what it actually calls. Definitions (`fn
//! canonicalize`), and calls to a function the same file defines, are not
//! candidates; that is what keeps Darkmatter's style-name normalizer out.
//!
//! **Limits.** This is a source guard, not proof about compiled code. Calls
//! produced by macro expansion (`macro_rules!` bodies invoked elsewhere,
//! procedural macros), files pulled in with `include!`, and a function
//! re-exported under a new name and called from another module
//! (`pub use std::fs::canonicalize as canon;` in one file, `util::canon(..)` in
//! another) are not followed. A method named `canonicalize` that takes
//! arguments is taken to be a non-filesystem method (the permission backends'
//! policy canonicalization): `Path::canonicalize` takes none.
//!
//! A package other than `darkmatter-cli` that includes this file declares it
//! and `source_scan.rs` in `[package.metadata.ci.tests] source-inputs` and
//! spells both with `include_str!` inside its guard test, because CI's
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
use std::path::{Path, PathBuf};

use source_scan::{ident_offsets, is_ident, line_at, matching_brace, production_file, production_sources};

/// What a guarded call is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rule {
    Canonicalize,
    HomeLookup,
}

impl Rule {
    fn helper(self) -> &'static str {
        match self {
            Self::Canonicalize => "biscuit_file::canonicalize_simplified",
            Self::HomeLookup => "biscuit_file::home_dir",
        }
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Canonicalize => "canonicalize",
            Self::HomeLookup => "home-lookup",
        })
    }
}

/// Why an exception is allowed to stay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A direct call whose result stays inside a private comparison (both
    /// operands canonicalized identically; only a bool, a dedupe entry, or a
    /// relative suffix leaves), or the shared helper's own call. The reason
    /// names that invariant.
    Invariant,
    /// A direct call scheduled for migration to the shared helper; the
    /// reason says where its result goes. Not a permanent state.
    Temporary,
    /// An unresolved candidate a reviewer judged not to be the guarded
    /// function; the reason names what it calls.
    Reviewed,
}

/// One allowed `(rule, path, item, operation)` with its exact occurrence
/// count. `path` is repository-relative and `/`-separated; `item` and
/// `operation` are the labels the scan reports (see the failure message for a
/// new site).
#[derive(Clone, Copy, Debug)]
pub struct Exception {
    pub rule: Rule,
    pub kind: Kind,
    pub path: &'static str,
    pub item: &'static str,
    pub operation: &'static str,
    pub count: usize,
    pub reason: &'static str,
}

/// One guarded call or candidate in production source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Site {
    pub rule: Rule,
    pub path: String,
    pub item: String,
    pub operation: String,
    pub line: usize,
    /// `false` for an unresolved candidate.
    pub resolved: bool,
}

/// The spellings one rule recognizes.
struct Target {
    rule: Rule,
    name: &'static str,
    /// Last path segments that make `segment::name` the guarded function.
    modules: &'static [&'static str],
    /// Last path segments that make `segment::name` the approved helper.
    helpers: &'static [&'static str],
    /// Whether a zero-argument `.name()` method call is the guarded function.
    method: bool,
}

const CANONICALIZE: Target = Target {
    rule: Rule::Canonicalize,
    name: "canonicalize",
    modules: &["fs", "dunce", "Path", "PathBuf"],
    helpers: &[],
    method: true,
};

const HOME_LOOKUP: Target = Target {
    rule: Rule::HomeLookup,
    name: "home_dir",
    modules: &["dirs", "env", "home"],
    helpers: &["biscuit_file"],
    method: false,
};

fn target(rule: Rule) -> &'static Target {
    match rule {
        Rule::Canonicalize => &CANONICALIZE,
        Rule::HomeLookup => &HOME_LOOKUP,
    }
}

/// Fails with every problem the guard finds in the package at `manifest_dir`,
/// which `package_dir` names relative to the repository root (`claudine/lib`).
pub fn assert_guarded(package_dir: &str, manifest_dir: &Path, rules: &[Rule], exceptions: &[Exception]) {
    let problems = problems(package_dir, manifest_dir, rules, exceptions);
    assert!(
        problems.is_empty(),
        "{package_dir} path lookup guard ({}):\n  {}",
        manifest_dir.display(),
        problems.join("\n  ")
    );
}

pub fn problems(package_dir: &str, manifest_dir: &Path, rules: &[Rule], exceptions: &[Exception]) -> Vec<String> {
    let (sources, mut problems) = package_sources(package_dir, manifest_dir);
    let sites = scan(&sources, rules);
    problems.extend(check(&sites, exceptions));
    problems
}

/// Sanitized production source per repository-relative path, plus a problem
/// for every missing root or an empty scan.
pub fn package_sources(package_dir: &str, manifest_dir: &Path) -> (BTreeMap<String, String>, Vec<String>) {
    let mut problems = Vec::new();
    let expected: PathBuf = package_dir.split('/').collect();
    if !manifest_dir.ends_with(&expected) {
        problems.push(format!("{} is not the package directory `{package_dir}`", manifest_dir.display()));
    }
    let manifest_path = manifest_dir.join("Cargo.toml");
    let manifest = match std::fs::read_to_string(&manifest_path) {
        Ok(text) => text,
        Err(error) => {
            problems.push(format!("required manifest {} is unreadable: {error}", manifest_path.display()));
            return (BTreeMap::new(), problems);
        }
    };

    let mut directories = vec!["src".to_string()];
    for root in declared_target_roots(&manifest) {
        if root.starts_with("tests/") {
            continue;
        }
        if !manifest_dir.join(&root).is_file() {
            problems.push(format!("declared source root {package_dir}/{root} is missing"));
            continue;
        }
        let directory = root.rsplit_once('/').map_or(".", |(directory, _)| directory).to_string();
        if !directories.iter().any(|known| directory == *known || directory.starts_with(&format!("{known}/"))) {
            directories.push(directory);
        }
    }

    let mut sources = BTreeMap::new();
    for directory in &directories {
        let path = manifest_dir.join(directory);
        if !path.is_dir() {
            problems.push(format!("required source root {package_dir}/{directory} is missing"));
            continue;
        }
        for (key, text) in production_sources(&path) {
            sources.insert(format!("{package_dir}/{directory}/{key}"), text);
        }
    }
    let build_script = build_script(&manifest);
    if let Some(script) = &build_script {
        let path = manifest_dir.join(script);
        if path.is_file() {
            sources.insert(format!("{package_dir}/{script}"), production_file(&path));
        } else if script != "build.rs" {
            problems.push(format!("declared build script {package_dir}/{script} is missing"));
        }
    }
    if sources.is_empty() {
        problems.push(format!("scanned no production source under {package_dir}; an empty scan cannot pass"));
    }
    (sources, problems)
}

/// `path` values of the manifest's `[lib]` and `[[bin]]` tables.
fn declared_target_roots(manifest: &str) -> Vec<String> {
    let mut roots = Vec::new();
    let mut in_target = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            in_target = line == "[lib]" || line == "[[bin]]";
        } else if in_target && let Some(value) = string_value(line, "path") {
            roots.push(value);
        }
    }
    roots
}

/// The build script: `package.build` when set to a path, else `build.rs`.
fn build_script(manifest: &str) -> Option<String> {
    let mut in_package = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            in_package = line == "[package]";
        } else if in_package && line.starts_with("build") {
            if line.ends_with("false") {
                return None;
            }
            if let Some(value) = string_value(line, "build") {
                return Some(value);
            }
        }
    }
    Some("build.rs".to_string())
}

fn string_value(line: &str, key: &str) -> Option<String> {
    let rest = line.strip_prefix(key)?.trim_start().strip_prefix('=')?.trim();
    Some(rest.strip_prefix('"')?.split('"').next()?.to_string())
}

/// Every site of `rules` in already-sanitized production sources.
pub fn scan(sources: &BTreeMap<String, String>, rules: &[Rule]) -> Vec<Site> {
    let mut sites = Vec::new();
    for (path, text) in sources {
        let items = item_spans(text.as_bytes());
        let uses = use_statements(text);
        for &rule in rules {
            scan_target(path, text, &items, &uses, target(rule), &mut sites);
        }
    }
    sites
}

/// What a file's `use` statements bring in for one target.
#[derive(Default)]
struct Imports {
    /// The bare name refers to the guarded function.
    bare_guarded: Option<String>,
    /// The bare name refers to the approved helper.
    bare_helper: bool,
    /// The bare name was imported from a module the scanner cannot place.
    bare_unknown: Option<String>,
    /// `(alias, imported path)` for `use module::name as alias;`.
    aliases: Vec<(String, String)>,
    /// Names bound to a guarded module (`use std::fs as filesystem;`).
    module_aliases: Vec<String>,
}

fn imports(text: &str, uses: &[(usize, usize)], target: &Target) -> Imports {
    let mut imports = Imports::default();
    for &(start, end) in uses {
        let statement = &text[start..end];
        for module in target.modules {
            for offset in ident_offsets(statement, module) {
                if let Some(alias) = alias_after(statement.as_bytes(), offset + module.len()) {
                    imports.module_aliases.push(alias.to_string());
                }
                let after = skip_ws(statement.as_bytes(), offset + module.len());
                if statement.as_bytes()[after..].starts_with(b"::") {
                    let star = skip_ws(statement.as_bytes(), after + 2);
                    if statement.as_bytes().get(star) == Some(&b'*') {
                        imports.bare_guarded = Some(format!("{module}::*"));
                    }
                }
            }
        }
        for offset in ident_offsets(statement, target.name) {
            let source = use_source(&statement.as_bytes()[..offset]).unwrap_or_default();
            let alias = alias_after(statement.as_bytes(), offset + target.name.len());
            let guarded = target.modules.contains(&source.as_str());
            let helper = target.helpers.contains(&source.as_str());
            match (alias, guarded, helper) {
                (Some(alias), true, _) => imports.aliases.push((alias.to_string(), format!("{source}::{}", target.name))),
                (Some(_), false, _) => {}
                (None, true, _) => imports.bare_guarded = Some(format!("{source}::{}", target.name)),
                (None, false, true) => imports.bare_helper = true,
                (None, false, false) => imports.bare_unknown = Some(format!("{source}::{}", target.name)),
            }
        }
    }
    imports
}

fn scan_target(
    path: &str,
    text: &str,
    items: &[ItemSpan],
    uses: &[(usize, usize)],
    target: &Target,
    sites: &mut Vec<Site>,
) {
    let bytes = text.as_bytes();
    let imports = imports(text, uses, target);
    let in_use = |offset: usize| uses.iter().any(|&(start, end)| (start..end).contains(&offset));
    let defines_locally = ident_offsets(text, target.name).into_iter().any(|offset| is_definition(bytes, offset));
    let mut push = |operation: String, offset: usize, resolved: bool| {
        sites.push(Site {
            rule: target.rule,
            path: path.to_string(),
            item: enclosing_item(items, offset),
            operation,
            line: line_at(text, offset),
            resolved,
        });
    };

    for offset in ident_offsets(text, target.name) {
        if in_use(offset) || is_definition(bytes, offset) {
            continue;
        }
        let open = skip_ws(bytes, offset + target.name.len());
        let called = bytes.get(open) == Some(&b'(');
        if previous_non_ws(bytes, offset) == Some(b'.') {
            let no_arguments = called && bytes.get(skip_ws(bytes, open + 1)) == Some(&b')');
            if target.method && no_arguments {
                push(format!(".{}()", target.name), offset, true);
            }
            continue;
        }
        if let Some(qualifier) = qualifier_path(bytes, offset) {
            let last = qualifier.rsplit("::").next().unwrap_or_default();
            let operation = format!("{qualifier}::{}", target.name);
            if target.helpers.contains(&last) {
                continue;
            }
            let guarded = target.modules.contains(&last) || imports.module_aliases.iter().any(|alias| alias == last);
            push(operation, offset, guarded);
            continue;
        }
        if let Some(source) = &imports.bare_guarded {
            push(format!("{} (imported {source})", target.name), offset, true);
        } else if !called || imports.bare_helper || defines_locally {
            // A variable, field, or argument that shares the name, or a call
            // to the helper or to this file's own function.
        } else if let Some(source) = &imports.bare_unknown {
            push(format!("{} (imported {source})", target.name), offset, false);
        } else {
            push(target.name.to_string(), offset, false);
        }
    }

    for (alias, source) in &imports.aliases {
        for offset in ident_offsets(text, alias) {
            if in_use(offset)
                || is_definition(bytes, offset)
                || previous_non_ws(bytes, offset) == Some(b'.')
                || qualifier_path(bytes, offset).is_some()
            {
                continue;
            }
            push(format!("{alias} (alias of {source})"), offset, true);
        }
    }
}

/// Compares the scanned sites with the exceptions.
pub fn check(sites: &[Site], exceptions: &[Exception]) -> Vec<String> {
    type Key<'a> = (Rule, &'a str, &'a str, &'a str);
    let mut problems = Vec::new();
    let mut allowed: BTreeMap<Key<'_>, &Exception> = BTreeMap::new();
    for entry in exceptions {
        if entry.reason.trim().is_empty() || entry.count == 0 {
            problems.push(format!(
                "exception for {} `{}` in {} ({}) needs a reason and a non-zero count",
                entry.rule, entry.operation, entry.item, entry.path
            ));
        }
        if allowed.insert((entry.rule, entry.path, entry.item, entry.operation), entry).is_some() {
            problems.push(format!(
                "duplicate exception: {} `{}` in {} ({})",
                entry.rule, entry.operation, entry.item, entry.path
            ));
        }
    }

    let mut found: BTreeMap<Key<'_>, (bool, Vec<usize>)> = BTreeMap::new();
    for site in sites {
        found
            .entry((site.rule, &site.path, &site.item, &site.operation))
            .or_insert_with(|| (site.resolved, Vec::new()))
            .1
            .push(site.line);
    }
    for (&(rule, path, item, operation), (resolved, lines)) in &found {
        let locations = lines.iter().map(|line| format!("{path}:{line}")).collect::<Vec<_>>().join(", ");
        let suggested_kind = if *resolved { "Kind::Invariant" } else { "Kind::Reviewed" };
        let suggestion = format!(
            "Exception {{ rule: Rule::{rule:?}, kind: {suggested_kind}, path: \"{path}\", item: \"{item}\", \
             operation: \"{operation}\", count: {}, reason: \"..\" }}",
            lines.len()
        );
        match allowed.get(&(rule, path, item, operation)) {
            None if *resolved => problems.push(format!(
                "new {rule} call `{operation}` in {item} at {locations}; use {}, or, when the result stays inside a \
                 private comparison, add {suggestion}",
                rule.helper()
            )),
            None => problems.push(format!(
                "unresolved {rule} candidate `{operation}` in {item} at {locations}: the scanner cannot tell what it \
                 calls; review it and add {suggestion} naming what it calls"
            )),
            Some(entry) => {
                if entry.count != lines.len() {
                    problems.push(format!(
                        "changed count for {rule} `{operation}` in {item}: excepted {}, found {} at {locations}",
                        entry.count,
                        lines.len()
                    ));
                }
                if (entry.kind == Kind::Reviewed) == *resolved {
                    problems.push(format!(
                        "exception for {rule} `{operation}` in {item} at {locations} is {:?}, but the site is {}",
                        entry.kind,
                        if *resolved { "a resolved call (use Invariant or Temporary)" } else { "unresolved (use Reviewed)" }
                    ));
                }
            }
        }
    }
    for (key, entry) in &allowed {
        if !found.contains_key(key) {
            problems.push(format!(
                "stale exception: {} `{}` in {} ({}) no longer occurs",
                entry.rule, entry.operation, entry.item, entry.path
            ));
        }
    }
    problems
}

/// A named item with a brace body: `fn`, `impl`, `mod`, or `trait`.
struct ItemSpan {
    start: usize,
    end: usize,
    name: String,
}

fn item_spans(bytes: &[u8]) -> Vec<ItemSpan> {
    let text = std::str::from_utf8(bytes).expect("sanitized source is UTF-8");
    let mut spans = Vec::new();
    for keyword in ["fn", "mod", "trait"] {
        for offset in ident_offsets(text, keyword) {
            let name_at = skip_ws(bytes, offset + keyword.len());
            let Some(name) = ident_at(bytes, name_at) else { continue };
            if let Some(open) = body_open(bytes, name_at + name.len()) {
                spans.push(ItemSpan { start: offset, end: matching_brace(bytes, open), name: name.to_string() });
            }
        }
    }
    for offset in ident_offsets(text, "impl") {
        let item_position = match previous_non_ws(bytes, offset) {
            None | Some(b';' | b'}' | b'{' | b']') => true,
            Some(_) => bytes[..offset].trim_ascii_end().ends_with(b"unsafe"),
        };
        if !item_position {
            continue;
        }
        if let Some(open) = body_open(bytes, offset + "impl".len()) {
            let name = impl_type_name(&text[offset + "impl".len()..open]);
            spans.push(ItemSpan { start: offset, end: matching_brace(bytes, open), name });
        }
    }
    spans.sort_by_key(|span| span.start);
    spans
}

/// The `{` opening the body of the item whose header continues at `from`, or
/// `None` for a body-less declaration (`fn f();`, `mod m;`).
fn body_open(bytes: &[u8], from: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, &byte) in bytes.iter().enumerate().skip(from) {
        match byte {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth = depth.saturating_sub(1),
            b'{' if depth == 0 => return Some(offset),
            b';' if depth == 0 => return None,
            _ => {}
        }
    }
    None
}

/// The self type's last path segment in an `impl` header
/// (`<T> Trait for a::Type<T> where ..` names `Type`).
fn impl_type_name(header: &str) -> String {
    let mut header = header.trim_start();
    if header.starts_with('<') {
        let mut depth = 0usize;
        for (offset, character) in header.char_indices() {
            match character {
                '<' => depth += 1,
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        header = &header[offset + 1..];
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    let for_at = ident_offsets(header, "for").into_iter().next();
    let self_type = for_at.map_or(header, |offset| &header[offset + 3..]);
    let self_type = ident_offsets(self_type, "where").first().map_or(self_type, |&offset| &self_type[..offset]);
    let path = self_type.split('<').next().unwrap_or_default();
    path.rsplit("::").next().unwrap_or_default().trim().trim_start_matches(['&', ' ']).trim_start_matches("dyn ").to_string()
}

/// The `::`-joined names of the items enclosing `offset`, outermost first.
fn enclosing_item(items: &[ItemSpan], offset: usize) -> String {
    let names = items
        .iter()
        .filter(|item| item.start <= offset && offset <= item.end)
        .map(|item| item.name.as_str())
        .collect::<Vec<_>>();
    if names.is_empty() { "(module)".to_string() } else { names.join("::") }
}

/// `[start, end)` of every `use` statement, through its `;`.
fn use_statements(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    ident_offsets(text, "use")
        .into_iter()
        .filter(|&offset| bytes.get(skip_ws(bytes, offset + 3)) != Some(&b'<'))
        .filter_map(|offset| {
            let end = bytes[offset..].iter().position(|&byte| byte == b';')?;
            Some((offset, offset + end + 1))
        })
        .collect()
}

/// The last `segment::` before the end of `prefix` (the module a name in a
/// `use` statement is imported from, groups included:
/// `use std::{fs::{self, canonicalize}}` gives `fs`).
fn use_source(prefix: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(prefix).ok()?;
    let separator = text.rfind("::")?;
    let before = text[..separator].trim_end();
    let start = before.rfind(|character: char| !(character == '_' || character.is_ascii_alphanumeric())).map_or(0, |at| at + 1);
    let segment = &before[start..];
    (!segment.is_empty()).then(|| segment.to_string())
}

/// The alias in `.. as alias` directly after `end`.
fn alias_after(bytes: &[u8], end: usize) -> Option<&str> {
    let keyword = skip_ws(bytes, end);
    if ident_at(bytes, keyword) != Some("as") {
        return None;
    }
    ident_at(bytes, skip_ws(bytes, keyword + 2)).filter(|alias| *alias != "_")
}

/// The `::`-joined path before the `::` that precedes `offset`
/// (`std::fs` for `std::fs::canonicalize`), or `<..>` for a qualified-self
/// path whose qualifier is not an identifier.
fn qualifier_path(bytes: &[u8], offset: usize) -> Option<String> {
    let mut segments = Vec::new();
    let mut cursor = offset;
    loop {
        let before = bytes[..cursor].trim_ascii_end();
        if !before.ends_with(b"::") {
            break;
        }
        let segment_end = before.len() - 2;
        let trimmed = bytes[..segment_end].trim_ascii_end();
        let mut start = trimmed.len();
        while start > 0 && is_ident(trimmed[start - 1]) {
            start -= 1;
        }
        if start == trimmed.len() {
            segments.push("<..>".to_string());
            break;
        }
        segments.push(String::from_utf8_lossy(&trimmed[start..]).into_owned());
        cursor = start;
    }
    (!segments.is_empty()).then(|| {
        segments.reverse();
        segments.join("::")
    })
}

fn is_definition(bytes: &[u8], offset: usize) -> bool {
    let trimmed = bytes[..offset].trim_ascii_end();
    trimmed.ends_with(b"fn") && trimmed.len().checked_sub(3).is_none_or(|at| !is_ident(trimmed[at]))
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
