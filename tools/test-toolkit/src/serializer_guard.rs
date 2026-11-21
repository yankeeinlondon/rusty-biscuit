//! Source guard: production code writes YAML documents through one writer.
//!
//! Every production YAML document goes through `biscuit_file::to_yaml_string`,
//! which owns indentation and the backend's quirks. A direct call to the
//! backend's serializer (`serde_yaml_ng::to_string`, `to_writer`, or a
//! constructed `Serializer`) bypasses that, so each package that depends on a
//! YAML backend runs [`assert_package`] over its own `src/` tree from one of
//! its Level 1 tests, with an allowlist of the sites that must stay direct.
//!
//! ## What counts as a site
//!
//! An expression path (a call or a function value) or a path inside a macro
//! invocation that resolves to a backend serializer. Resolution follows the
//! crate's `use` declarations, so import syntax cannot hide a call:
//!
//! - qualified: `serde_yaml_ng::to_string`, `::serde_yaml::to_writer`;
//! - a namespace alias: `use serde_yaml_ng as yaml;` then `yaml::to_string`;
//! - a leaf alias: `use serde_yaml_ng::to_string as ser;` then `ser(..)`;
//! - grouped imports: `use serde_yaml_ng::{to_string, Serializer};`;
//! - glob imports: `use serde_yaml_ng::*;` then a bare `to_string(..)`, unless
//!   the module defines its own `to_string`;
//! - re-exports across modules of the same crate: `pub use serde_yaml_ng as y;`
//!   in one module, `crate::m::y::to_string` (or a `use` of it) in another;
//! - workspace crates' re-exports of a backend ([`REEXPORTS`]), such as
//!   `biscuit_file::serde_yaml_ng`.
//!
//! A method call (`value.to_string()`) is never a site: it names no path.
//!
//! ## What is not scanned
//!
//! Only modules a production target compiles are read, found by walking `mod`
//! declarations from `src/lib.rs`, `src/main.rs`, and `src/bin/`. An item
//! gated on `test` (`#[cfg(test)]`, `#[cfg(all(test, …))]`) or marked as a
//! test (`#[test]`, `#[tokio::test]`, `#[rstest]`) is skipped with everything
//! inside it, including the files of an out-of-line `mod`. `tests/`,
//! `benches/`, and `examples/` are never reached.
//!
//! The resolution is deliberately conservative: a `use` inside a function body
//! counts for its whole module, and every `cfg` other than `test` is treated as
//! enabled. Either can only add a site, never hide one.
//!
//! Macro expansion is out of reach: a `macro_rules!` body is scanned where it
//! is defined, and a call another crate's macro expands to is not seen.
//!
//! ## Cost
//!
//! Parsing is nearly all of a scan's time, so a file is parsed for sites only
//! when its text spells a backend word and a serializer word outside a method
//! call ([`Vocabulary`]); its ancestors are parsed for the module tree, and any
//! other module only when a resolution enters it. Every governed chain spells
//! those words in the file that holds it, so the filter hides no site.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::{Attribute, Item, Meta, Token, UseTree};

/// The YAML backend crates whose document serializers the guard governs.
pub const BACKENDS: &[&str] = &["serde_yaml", "serde_yaml_ng"];

/// Paths at which workspace crates re-export a backend, and the backend each
/// names. A path through one of these is the backend's own path.
pub const REEXPORTS: &[(&[&str], &str)] = &[(&["biscuit_file", "serde_yaml_ng"], "serde_yaml_ng")];

/// Macros whose tokens are code a generator emits, not calls this crate makes.
const OUTPUT_CODE_MACROS: &[&str] = &["quote", "quote_spanned", "parse_quote"];

/// Bounds alias and glob chains, so a cyclic `use` cannot recurse forever.
const MAX_RESOLUTION_DEPTH: usize = 32;

/// One retained direct serializer call.
///
/// `file` is relative to the package directory (`src/yaml/write.rs`) and
/// `symbol` is the enclosing item path within that file, as [`Site::symbol`]
/// spells it. One entry covers every site in that symbol.
#[derive(Debug, Clone, Copy)]
pub struct Allow {
    pub file: &'static str,
    pub symbol: &'static str,
    /// The contract that the shared writer cannot meet. Must not be empty.
    pub reason: &'static str,
}

/// A direct serializer reference found in production source.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Site {
    /// Package-relative path with `/` separators.
    pub file: String,
    /// 1-based line of the path's first segment.
    pub line: usize,
    /// Enclosing items joined by `::`: inline modules, the `impl` self type,
    /// then functions (`Writer::emit`, `parse::inner`). A site outside any item
    /// body is `(module)`.
    pub symbol: String,
    /// The resolved backend path (`serde_yaml_ng::to_string`).
    pub call: String,
}

/// The outcome of one package scan.
#[derive(Debug, Default)]
pub struct ScanReport {
    /// Every package-relative file the scan parsed.
    pub files: BTreeSet<String>,
    pub sites: Vec<Site>,
}

/// A file that could not be read or parsed, or a `mod` with no file.
#[derive(Debug)]
pub struct GuardError(String);

impl fmt::Display for GuardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for GuardError {}

/// Scans the production modules under a package's `src` directory.
///
/// File keys in the report are relative to `src_dir`'s parent, the package
/// directory.
///
/// ## Errors
///
/// Returns [`GuardError`] when a reached file cannot be read or parsed, when a
/// non-test `mod` declaration has no file, or when `src_dir` holds no crate
/// root.
pub fn scan_package(src_dir: &Path) -> Result<ScanReport, GuardError> {
    let package = src_dir
        .parent()
        .ok_or_else(|| GuardError(format!("{} has no parent directory", src_dir.display())))?;
    let mut files = BTreeMap::new();
    collect_sources(package, src_dir, &mut files)?;
    let sources = Sources {
        files,
        package: Some(package.to_path_buf()),
    };
    scan_roots(&sources)
}

/// Scans in-memory sources keyed by package-relative path (`src/lib.rs`,
/// `src/a.rs`), with the same crate roots [`scan_package`] discovers.
///
/// ## Errors
///
/// As [`scan_package`].
pub fn scan_sources(files: &[(&str, &str)]) -> Result<ScanReport, GuardError> {
    let files = files
        .iter()
        .map(|(path, text)| ((*path).to_string(), (*text).to_string()))
        .collect();
    scan_roots(&Sources {
        files,
        package: None,
    })
}

fn collect_sources(
    package: &Path,
    dir: &Path,
    out: &mut BTreeMap<String, String>,
) -> Result<(), GuardError> {
    let entries = std::fs::read_dir(dir)
        .map_err(|error| GuardError(format!("cannot list {}: {error}", dir.display())))?;
    for entry in entries {
        let path = entry
            .map_err(|error| GuardError(format!("cannot list {}: {error}", dir.display())))?
            .path();
        if path.is_dir() {
            collect_sources(package, &path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            let text = std::fs::read_to_string(&path)
                .map_err(|error| GuardError(format!("cannot read {}: {error}", path.display())))?;
            let relative = path.strip_prefix(package).unwrap_or(&path);
            let key = relative
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            out.insert(key, text);
        }
    }
    Ok(())
}

fn is_crate_root(path: &str) -> bool {
    if path == "src/lib.rs" || path == "src/main.rs" {
        return true;
    }
    match path.strip_prefix("src/bin/") {
        Some(rest) => {
            (!rest.contains('/') && rest.ends_with(".rs"))
                || rest
                    .split_once('/')
                    .is_some_and(|(_, file)| file == "main.rs")
        }
        None => false,
    }
}

/// Reconciles a scan against a package's allowlist.
///
/// Returns one message per site no entry covers, per entry that covers no
/// site (a stale entry fails so the list cannot outlive its sites), per entry
/// with an empty reason, and per duplicated entry.
#[must_use]
pub fn violations(report: &ScanReport, allowed: &[Allow]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut seen = HashSet::new();
    for entry in allowed {
        if !seen.insert((entry.file, entry.symbol)) {
            problems.push(format!(
                "duplicate allowlist entry {}::{}; keep one entry per file and symbol",
                entry.file, entry.symbol
            ));
        }
        if entry.reason.trim().is_empty() {
            problems.push(format!(
                "allowlist entry {}::{} has no reason; name the contract that keeps the call direct",
                entry.file, entry.symbol
            ));
        }
    }
    for site in &report.sites {
        let covered = allowed
            .iter()
            .any(|entry| entry.file == site.file && entry.symbol == site.symbol);
        if !covered {
            problems.push(format!(
                "{}:{} in `{}`: direct YAML serializer `{}`. Route the document through \
                 `biscuit_file::to_yaml_string(value, indentation)`, or, if its output is bound \
                 by a contract the shared writer cannot meet, add \
                 `Allow {{ file: \"{}\", symbol: \"{}\", reason: \"<the contract>\" }}` to this \
                 package's serializer guard.",
                site.file, site.line, site.symbol, site.call, site.file, site.symbol
            ));
        }
    }
    for entry in allowed {
        let live = report
            .sites
            .iter()
            .any(|site| entry.file == site.file && entry.symbol == site.symbol);
        if !live {
            problems.push(format!(
                "stale allowlist entry {}::{}: no direct YAML serializer remains there; \
                 remove the entry from this package's serializer guard",
                entry.file, entry.symbol
            ));
        }
    }
    problems
}

/// Scans `src_dir` and panics with every [`violations`] message.
///
/// Also panics when the scan parsed no file, so a guard pointed at the wrong
/// directory cannot pass.
///
/// ## Panics
///
/// On any scan error, violation, or an empty scan.
pub fn assert_package(src_dir: &Path, allowed: &[Allow]) {
    let report = scan_package(src_dir).unwrap_or_else(|error| {
        panic!(
            "serializer guard could not scan {}: {error}",
            src_dir.display()
        )
    });
    assert!(
        !report.files.is_empty(),
        "serializer guard parsed no source under {}",
        src_dir.display()
    );
    let problems = violations(&report, allowed);
    assert!(
        problems.is_empty(),
        "serializer guard found {} problem(s):\n{}",
        problems.len(),
        problems.join("\n")
    );
}

// ---------------------------------------------------------------------------
// Module graph and scopes
// ---------------------------------------------------------------------------

type ModPath = Vec<String>;

#[derive(Debug, Clone)]
struct RawPath {
    absolute: bool,
    segs: Vec<String>,
}

#[derive(Debug, Default)]
struct Scope {
    aliases: HashMap<String, Vec<RawPath>>,
    globs: Vec<RawPath>,
    children: HashSet<String>,
    items: HashSet<String>,
}

/// An out-of-line child module found in a parsed file.
struct ChildModule {
    file: String,
    module: ModPath,
    mod_dir: String,
}

/// Every `.rs` file under the package's `src`, by package-relative path.
struct Sources {
    files: BTreeMap<String, String>,
    /// Where a `#[path]` module outside `src` is read from; `None` in memory.
    package: Option<PathBuf>,
}

impl Sources {
    fn read(&self, relative: &str) -> Option<String> {
        if let Some(text) = self.files.get(relative) {
            return Some(text.clone());
        }
        let package = self.package.as_ref()?;
        std::fs::read_to_string(package.join(relative)).ok()
    }

    fn exists(&self, relative: &str) -> bool {
        self.files.contains_key(relative)
            || self
                .package
                .as_ref()
                .is_some_and(|package| package.join(relative).is_file())
    }

    fn roots(&self) -> Vec<String> {
        self.files
            .keys()
            .filter(|path| is_crate_root(path))
            .cloned()
            .collect()
    }
}

/// Parsed files shared by every crate root of one package.
struct Parser<'s> {
    sources: &'s Sources,
    cache: RefCell<HashMap<String, Rc<syn::File>>>,
}

impl Parser<'_> {
    fn parse(&self, relative: &str) -> Result<Rc<syn::File>, GuardError> {
        if let Some(ast) = self.cache.borrow().get(relative) {
            return Ok(Rc::clone(ast));
        }
        let source = self
            .sources
            .read(relative)
            .ok_or_else(|| GuardError(format!("cannot read {relative}")))?;
        let ast = syn::parse_file(&source).map_err(|error| {
            let line = error.span().start().line;
            GuardError(format!("cannot parse {relative}:{line}: {error}"))
        })?;
        let ast = Rc::new(ast);
        self.cache
            .borrow_mut()
            .insert(relative.to_string(), Rc::clone(&ast));
        Ok(ast)
    }
}

/// The final segments of a governed path. Every governed path spells one of
/// these, or an alias of one.
const LEAVES: &[&str] = &["to_string", "to_writer", "Serializer"];

/// The words a file must spell to hold a site.
struct Vocabulary {
    /// Backend names and the `use` aliases that lead to one.
    backend: BTreeSet<String>,
    /// [`LEAVES`] and the `use` aliases of one.
    leaves: HashSet<String>,
}

/// The files that can hold a site, and the words that prove it.
///
/// Parsing is the guard's whole cost, and most files never name a backend.
/// A resolution chain that ends at a backend starts at a `use` or path that
/// spells the backend's name, each `use` hop introduces a name the next hop
/// spells, and a glob hop passes names through unchanged. So a file can hold
/// a site only if it spells a word of [`Vocabulary::backend`] and, outside a
/// method call, one of [`Vocabulary::leaves`]. The aliases are read from the
/// `use` declarations of files that spell a backend word, to a fixpoint; a
/// glob of a backend path makes the bare leaves backend words.
fn relevant_files(sources: &Sources) -> (BTreeSet<String>, Vocabulary) {
    let mut words = Vocabulary {
        backend: BACKENDS.iter().map(|b| (*b).to_string()).collect(),
        leaves: LEAVES.iter().map(|l| (*l).to_string()).collect(),
    };
    for (_, backend) in REEXPORTS {
        words.backend.insert((*backend).to_string());
    }
    let mut tainting = BTreeSet::new();
    loop {
        let before = (words.backend.len(), words.leaves.len());
        for (path, text) in &sources.files {
            if !tainting.contains(path)
                && words
                    .backend
                    .iter()
                    .any(|word| contains_word(text, word, false))
            {
                tainting.insert(path.clone());
            }
        }
        for path in &tainting {
            taint_from(&sources.files[path], &mut words);
        }
        if (words.backend.len(), words.leaves.len()) == before {
            break;
        }
    }
    let relevant = tainting
        .into_iter()
        .filter(|path| {
            words
                .leaves
                .iter()
                .any(|word| contains_word(&sources.files[path], word, true))
        })
        .collect();
    (relevant, words)
}

/// Adds the aliases `text`'s `use` declarations introduce toward a backend.
///
/// Each declaration is cut out at the next `;` and parsed alone; text that is
/// not one (prose, a string) fails to parse and is skipped.
fn taint_from(text: &str, words: &mut Vocabulary) {
    let mut scopes = HashMap::new();
    let mut collector = ScopeCollector::detached(&mut scopes);
    for (at, _) in text.match_indices("use") {
        if !is_word_at(text, at, 3) {
            continue;
        }
        let Some(end) = text[at..].find(';') else {
            continue;
        };
        let snippet = &text[at..=at + end];
        if !words
            .backend
            .iter()
            .any(|word| contains_word(snippet, word, false))
        {
            continue;
        }
        if let Ok(item) = syn::parse_str::<syn::ItemUse>(snippet) {
            collector.visit_item_use(&item);
        }
    }
    let mut glob = false;
    for scope in scopes.values() {
        for (name, targets) in &scope.aliases {
            for target in targets {
                let Some(last) = target.segs.last() else {
                    continue;
                };
                if !target.segs.iter().any(|seg| words.backend.contains(seg)) {
                    continue;
                }
                if words.leaves.contains(last) {
                    words.leaves.insert(name.clone());
                    words.backend.insert(name.clone());
                } else if words.backend.contains(last) || last == "ser" {
                    words.backend.insert(name.clone());
                }
            }
        }
        glob |= scope
            .globs
            .iter()
            .any(|g| g.segs.iter().any(|seg| words.backend.contains(seg)));
    }
    if glob {
        words
            .backend
            .extend(LEAVES.iter().map(|leaf| (*leaf).to_string()));
    }
}

/// Whether `text` holds `word` as a whole identifier; with `path_only`, an
/// occurrence right after `.` (a method call or field) does not count.
fn contains_word(text: &str, word: &str, path_only: bool) -> bool {
    text.match_indices(word).any(|(at, _)| {
        is_word_at(text, at, word.len()) && !(path_only && text[..at].trim_end().ends_with('.'))
    })
}

fn is_word_at(text: &str, at: usize, len: usize) -> bool {
    let is_ident = |c: char| c == '_' || c.is_alphanumeric();
    let before = text[..at].chars().next_back();
    let after = text[at + len..].chars().next();
    !before.is_some_and(is_ident) && !after.is_some_and(is_ident)
}

/// Files the module walk must parse: the relevant ones, every file that may
/// lead to one by directory layout, and every file declaring a `#[path]`.
fn needed_files(sources: &Sources, relevant: &BTreeSet<String>) -> HashSet<String> {
    let mut needed: HashSet<String> = relevant.iter().cloned().collect();
    needed.extend(sources.roots());
    needed.extend(
        sources
            .files
            .iter()
            .filter(|(_, text)| text.contains("#[path"))
            .map(|(path, _)| path.clone()),
    );
    let seeds: Vec<String> = needed.iter().cloned().collect();
    for file in seeds {
        let mut dir = parent_dir(&file);
        while !dir.is_empty() {
            for candidate in [format!("{dir}.rs"), format!("{dir}/mod.rs")] {
                if sources.files.contains_key(&candidate) {
                    needed.insert(candidate);
                }
            }
            dir = parent_dir(&dir);
        }
    }
    needed
}

fn scan_roots(sources: &Sources) -> Result<ScanReport, GuardError> {
    let roots = sources.roots();
    if roots.is_empty() {
        return Err(GuardError(
            "no crate root (src/lib.rs, src/main.rs, src/bin/) found".into(),
        ));
    }
    let parser = Parser {
        sources,
        cache: RefCell::new(HashMap::new()),
    };
    let (relevant, words) = relevant_files(sources);
    let needed = needed_files(sources, &relevant);
    let mut report = ScanReport::default();
    let mut sites = BTreeSet::new();
    // Each root is its own crate: `crate::` and its scopes are per root.
    for root in &roots {
        let krate = Crate {
            parser: &parser,
            scopes: RefCell::new(HashMap::new()),
            pending: RefCell::new(HashMap::new()),
            error: RefCell::new(None),
        };
        let files = krate.walk(root, &needed)?;
        for (relative, module) in files {
            report.files.insert(relative.clone());
            if !relevant.contains(&relative) {
                continue;
            }
            let ast = parser.parse(&relative)?;
            let mut finder = SiteFinder {
                resolver: &krate,
                leaves: &words.leaves,
                file: &relative,
                module,
                symbol: Vec::new(),
                sites: &mut sites,
            };
            finder.visit_file(&ast);
        }
        if let Some(error) = krate.error.borrow_mut().take() {
            return Err(error);
        }
    }
    report.sites = sites.into_iter().collect();
    Ok(report)
}

/// One crate's module scopes. A module the walk did not need is parsed only
/// when a resolution enters it.
struct Crate<'p> {
    parser: &'p Parser<'p>,
    scopes: RefCell<HashMap<ModPath, Scope>>,
    pending: RefCell<HashMap<ModPath, ChildModule>>,
    error: RefCell<Option<GuardError>>,
}

impl Crate<'_> {
    /// Walks the module tree from `root`, parsing the `needed` files, and
    /// returns each parsed file with its module path.
    fn walk(
        &self,
        root: &str,
        needed: &HashSet<String>,
    ) -> Result<Vec<(String, ModPath)>, GuardError> {
        let mut files = Vec::new();
        let mut queue = vec![ChildModule {
            file: root.to_string(),
            module: Vec::new(),
            mod_dir: parent_dir(root),
        }];
        let mut visited = HashSet::new();
        while let Some(child) = queue.pop() {
            if !visited.insert(child.file.clone()) {
                continue;
            }
            let outside = !self.parser.sources.files.contains_key(&child.file);
            if !needed.contains(&child.file) && !outside {
                self.pending
                    .borrow_mut()
                    .insert(child.module.clone(), child);
                continue;
            }
            queue.extend(self.load(&child)?);
            files.push((child.file, child.module));
        }
        files.sort();
        Ok(files)
    }

    fn load(&self, child: &ChildModule) -> Result<Vec<ChildModule>, GuardError> {
        let ast = self.parser.parse(&child.file)?;
        let mut scopes = self.scopes.borrow_mut();
        let mut collector = ScopeCollector {
            scopes: &mut scopes,
            module: child.module.clone(),
            file_dir: parent_dir(&child.file),
            mod_dir: child.mod_dir.clone(),
            inline_depth: 0,
            body_depth: 0,
            children: Vec::new(),
            missing: Vec::new(),
            sources: Some(self.parser.sources),
        };
        scopes_entry(collector.scopes, &child.module);
        collector.visit_file(&ast);
        if let Some(name) = collector.missing.first() {
            return Err(GuardError(format!(
                "{}: `mod {name};` has no file",
                child.file
            )));
        }
        Ok(collector.children)
    }

    /// Makes `module`'s scope available, parsing its file on first use.
    fn ensure(&self, module: &[String]) {
        if self.scopes.borrow().contains_key(module) {
            return;
        }
        if !module.is_empty() {
            self.ensure(&module[..module.len() - 1]);
        }
        let pending = self.pending.borrow_mut().remove(module);
        if let Some(child) = pending {
            match self.load(&child) {
                Ok(children) => {
                    let mut pending = self.pending.borrow_mut();
                    for grandchild in children {
                        pending.insert(grandchild.module.clone(), grandchild);
                    }
                }
                Err(error) => {
                    self.error.borrow_mut().get_or_insert(error);
                }
            }
        }
    }

    fn with_scope<R>(&self, module: &[String], f: impl FnOnce(&Scope) -> R) -> Option<R> {
        self.ensure(module);
        self.scopes.borrow().get(module).map(f)
    }
}

fn scopes_entry<'a>(scopes: &'a mut HashMap<ModPath, Scope>, module: &ModPath) -> &'a mut Scope {
    scopes.entry(module.clone()).or_default()
}

fn parent_dir(relative: &str) -> String {
    relative
        .rsplit_once('/')
        .map_or_else(String::new, |(dir, _)| dir.to_string())
}

fn join(dir: &str, tail: &str) -> String {
    let mut parts: Vec<&str> = if dir.is_empty() {
        Vec::new()
    } else {
        dir.split('/').collect()
    };
    for part in tail.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

struct ScopeCollector<'a> {
    scopes: &'a mut HashMap<ModPath, Scope>,
    module: ModPath,
    file_dir: String,
    /// Where the current module's out-of-line children live.
    mod_dir: String,
    inline_depth: usize,
    /// Nonzero inside a function or other item body, whose items are not
    /// module items.
    body_depth: usize,
    children: Vec<ChildModule>,
    missing: Vec<String>,
    /// `None` collects one file's names without following its modules.
    sources: Option<&'a Sources>,
}

impl<'a> ScopeCollector<'a> {
    fn detached(scopes: &'a mut HashMap<ModPath, Scope>) -> Self {
        ScopeCollector {
            scopes,
            module: Vec::new(),
            file_dir: String::new(),
            mod_dir: String::new(),
            inline_depth: 0,
            body_depth: 0,
            children: Vec::new(),
            missing: Vec::new(),
            sources: None,
        }
    }

    fn scope(&mut self) -> &mut Scope {
        scopes_entry(self.scopes, &self.module)
    }

    fn record_item(&mut self, ident: &syn::Ident) {
        if self.body_depth == 0 {
            let name = unraw(ident);
            self.scope().items.insert(name);
        }
    }

    fn record_use(&mut self, tree: &UseTree, prefix: &mut Vec<String>, absolute: bool) {
        match tree {
            UseTree::Path(path) => {
                prefix.push(unraw(&path.ident));
                self.record_use(&path.tree, prefix, absolute);
                prefix.pop();
            }
            UseTree::Name(name) => {
                let ident = unraw(&name.ident);
                if ident == "self" {
                    if let Some(last) = prefix.last().cloned() {
                        self.add_alias(last, prefix.clone(), absolute);
                    }
                } else {
                    let mut segs = prefix.clone();
                    segs.push(ident.clone());
                    self.add_alias(ident, segs, absolute);
                }
            }
            UseTree::Rename(rename) => {
                let alias = unraw(&rename.rename);
                if alias == "_" {
                    return;
                }
                let ident = unraw(&rename.ident);
                let segs = if ident == "self" {
                    prefix.clone()
                } else {
                    let mut segs = prefix.clone();
                    segs.push(ident);
                    segs
                };
                self.add_alias(alias, segs, absolute);
            }
            UseTree::Glob(_) => {
                let glob = RawPath {
                    absolute,
                    segs: prefix.clone(),
                };
                self.scope().globs.push(glob);
            }
            UseTree::Group(group) => {
                for tree in &group.items {
                    self.record_use(tree, prefix, absolute);
                }
            }
        }
    }

    fn add_alias(&mut self, name: String, segs: Vec<String>, absolute: bool) {
        if segs.is_empty() {
            return;
        }
        self.scope()
            .aliases
            .entry(name)
            .or_default()
            .push(RawPath { absolute, segs });
    }

    fn out_of_line_mod(&mut self, item: &syn::ItemMod) {
        let name = unraw(&item.ident);
        let explicit = item.attrs.iter().find_map(path_attr);
        let candidates: Vec<(String, String)> = match explicit {
            Some(path) => {
                let base = if self.inline_depth == 0 {
                    &self.file_dir
                } else {
                    &self.mod_dir
                };
                let file = join(base, &path);
                let dir = parent_dir(&file);
                vec![(file, dir)]
            }
            None => {
                let dir = join(&self.mod_dir, &name);
                vec![
                    (format!("{dir}.rs"), dir.clone()),
                    (format!("{dir}/mod.rs"), dir),
                ]
            }
        };
        let Some(sources) = self.sources else {
            return;
        };
        let mut module = self.module.clone();
        module.push(name.clone());
        for (file, mod_dir) in candidates {
            if sources.exists(&file) {
                self.children.push(ChildModule {
                    file,
                    module,
                    mod_dir,
                });
                return;
            }
        }
        self.missing.push(name);
    }
}

impl<'ast> Visit<'ast> for ScopeCollector<'_> {
    fn visit_item(&mut self, item: &'ast Item) {
        if is_test_gated(item_attrs(item)) {
            return;
        }
        visit::visit_item(self, item);
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        let name = unraw(&item.ident);
        if self.body_depth == 0 {
            self.scope().children.insert(name.clone());
        }
        match &item.content {
            None => self.out_of_line_mod(item),
            Some((_, items)) => {
                // A module inside a function body is still a module; its path
                // is approximated as a child of the enclosing module.
                let saved_dir = self.mod_dir.clone();
                let saved_body = self.body_depth;
                self.mod_dir = join(&self.mod_dir, &name);
                self.module.push(name);
                self.inline_depth += 1;
                self.body_depth = 0;
                scopes_entry(self.scopes, &self.module);
                for item in items {
                    self.visit_item(item);
                }
                self.body_depth = saved_body;
                self.inline_depth -= 1;
                self.module.pop();
                self.mod_dir = saved_dir;
            }
        }
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        self.record_use(&item.tree, &mut Vec::new(), item.leading_colon.is_some());
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.record_item(&item.sig.ident);
        self.body_depth += 1;
        visit::visit_item_fn(self, item);
        self.body_depth -= 1;
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        self.body_depth += 1;
        visit::visit_item_impl(self, item);
        self.body_depth -= 1;
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        self.record_item(&item.ident);
        self.body_depth += 1;
        visit::visit_item_trait(self, item);
        self.body_depth -= 1;
    }

    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        self.record_item(&item.ident);
        self.body_depth += 1;
        visit::visit_item_const(self, item);
        self.body_depth -= 1;
    }

    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        self.record_item(&item.ident);
        self.body_depth += 1;
        visit::visit_item_static(self, item);
        self.body_depth -= 1;
    }

    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        self.record_item(&item.ident);
    }

    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        self.record_item(&item.ident);
    }

    fn visit_item_union(&mut self, item: &'ast syn::ItemUnion) {
        self.record_item(&item.ident);
    }

    fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
        self.record_item(&item.ident);
    }
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

impl Crate<'_> {
    /// Every path `segs`, written in `module`, may name. Crate-internal
    /// results start with `crate`.
    fn resolve(&self, module: &[String], path: &RawPath, depth: usize, out: &mut Vec<Vec<String>>) {
        let mut seen = HashSet::new();
        self.resolve_path(module, path.absolute, &path.segs, depth, &mut seen, out);
    }

    fn resolve_path(
        &self,
        module: &[String],
        absolute: bool,
        segs: &[String],
        depth: usize,
        seen: &mut HashSet<(ModPath, String)>,
        out: &mut Vec<Vec<String>>,
    ) {
        if absolute || depth > MAX_RESOLUTION_DEPTH || segs.is_empty() {
            out.push(segs.to_vec());
            return;
        }
        match segs[0].as_str() {
            "crate" => self.resolve_in(&[], &segs[1..], depth, seen, out),
            "Self" => out.push(segs.to_vec()),
            _ => self.resolve_in(module, segs, depth, seen, out),
        }
    }

    fn resolve_in(
        &self,
        module: &[String],
        segs: &[String],
        depth: usize,
        seen: &mut HashSet<(ModPath, String)>,
        out: &mut Vec<Vec<String>>,
    ) {
        if depth > MAX_RESOLUTION_DEPTH {
            out.push(segs.to_vec());
            return;
        }
        let Some(name) = segs.first() else {
            let mut path = vec!["crate".to_string()];
            path.extend_from_slice(module);
            out.push(path);
            return;
        };
        match name.as_str() {
            "self" => return self.resolve_in(module, &segs[1..], depth + 1, seen, out),
            "super" => {
                let parent = &module[..module.len().saturating_sub(1)];
                return self.resolve_in(parent, &segs[1..], depth + 1, seen, out);
            }
            _ => {}
        }
        // Copied out so the recursion below may parse another module's file.
        let Some((targets, is_child, is_item, globs)) = self.with_scope(module, |scope| {
            (
                scope.aliases.get(name).cloned().unwrap_or_default(),
                scope.children.contains(name),
                scope.items.contains(name),
                scope.globs.clone(),
            )
        }) else {
            out.push(segs.to_vec());
            return;
        };
        let mut found = !targets.is_empty() || is_child || is_item;
        for target in &targets {
            let mut full = target.segs.clone();
            full.extend_from_slice(&segs[1..]);
            // `use serde_yaml_ng;` names the extern crate, not itself.
            let absolute = target.absolute || target.segs.first() == Some(name);
            self.resolve_path(module, absolute, &full, depth + 1, seen, out);
        }
        if is_child {
            let mut child = module.to_vec();
            child.push(name.clone());
            self.resolve_in(&child, &segs[1..], depth + 1, seen, out);
        }
        if is_item {
            let mut path = vec!["crate".to_string()];
            path.extend_from_slice(module);
            path.extend_from_slice(segs);
            out.push(path);
        }
        if !found && seen.insert((module.to_vec(), name.clone())) {
            for glob in &globs {
                let mut bases = Vec::new();
                self.resolve_path(
                    module,
                    glob.absolute,
                    &glob.segs,
                    depth + 1,
                    seen,
                    &mut bases,
                );
                for base in bases {
                    if base.first().map(String::as_str) == Some("crate") {
                        if self.defines(&base[1..], name) {
                            found = true;
                            self.resolve_in(&base[1..], segs, depth + 1, seen, out);
                        }
                    } else {
                        // An external glob's names are unknown; offer the
                        // candidate and let the matcher decide.
                        let mut full = base;
                        full.extend_from_slice(segs);
                        out.push(full);
                    }
                }
            }
        }
        if !found {
            out.push(segs.to_vec());
        }
    }

    fn defines(&self, module: &[String], name: &str) -> bool {
        self.with_scope(module, |scope| {
            scope.aliases.contains_key(name)
                || scope.children.contains(name)
                || scope.items.contains(name)
                || !scope.globs.is_empty()
        })
        .unwrap_or(false)
    }
}

/// The backend serializer a resolved path names, spelled from the backend.
fn classify(path: &[String]) -> Option<String> {
    let mut segs: Vec<&str> = path.iter().map(String::as_str).collect();
    for (prefix, backend) in REEXPORTS {
        if segs.starts_with(prefix) {
            let rest = segs.split_off(prefix.len());
            segs = std::iter::once(*backend).chain(rest).collect();
            break;
        }
    }
    let (backend, mut rest) = segs.split_first()?;
    if !BACKENDS.contains(backend) {
        return None;
    }
    if rest.first() == Some(&"ser") {
        rest = &rest[1..];
    }
    let governed = matches!(rest, ["to_string" | "to_writer"] | ["Serializer", ..]);
    governed.then(|| format!("{backend}::{}", rest.join("::")))
}

// ---------------------------------------------------------------------------
// Site finding
// ---------------------------------------------------------------------------

struct SiteFinder<'a> {
    resolver: &'a Crate<'a>,
    /// A path that spells none of these cannot be governed, so it is not
    /// resolved (resolution may parse other modules).
    leaves: &'a HashSet<String>,
    file: &'a str,
    module: ModPath,
    symbol: Vec<String>,
    sites: &'a mut BTreeSet<Site>,
}

impl SiteFinder<'_> {
    fn check(&mut self, absolute: bool, segs: Vec<String>, span: Span) {
        if !segs.iter().any(|seg| self.leaves.contains(seg)) {
            return;
        }
        let mut candidates = Vec::new();
        self.resolver.resolve(
            &self.module,
            &RawPath { absolute, segs },
            0,
            &mut candidates,
        );
        if let Some(call) = candidates.iter().find_map(|path| classify(path)) {
            let symbol = if self.symbol.is_empty() {
                "(module)".to_string()
            } else {
                self.symbol.join("::")
            };
            self.sites.insert(Site {
                file: self.file.to_string(),
                line: span.start().line,
                symbol,
                call,
            });
        }
    }

    fn scan_tokens(&mut self, tokens: TokenStream) {
        let flat = flatten(tokens);
        let mut index = 0;
        while index < flat.len() {
            let start = index;
            let absolute = is_path_sep(&flat, index);
            let first = if absolute { index + 2 } else { index };
            let Some(Flat::Ident(ident, span)) = flat.get(first) else {
                index += 1;
                continue;
            };
            let preceded_by_dot =
                start > 0 && matches!(flat.get(start - 1), Some(Flat::Punct('.', _)));
            let preceded_by_fn = start > 0
                && matches!(flat.get(start - 1), Some(Flat::Ident(word, _)) if word == "fn");
            let mut segs = vec![ident.clone()];
            let span = *span;
            let mut cursor = first + 1;
            while is_path_sep(&flat, cursor) {
                match flat.get(cursor + 2) {
                    Some(Flat::Ident(next, _)) => {
                        segs.push(next.clone());
                        cursor += 3;
                    }
                    _ => break,
                }
            }
            if !preceded_by_dot && !preceded_by_fn {
                self.check(absolute, segs, span);
            }
            index = cursor.max(start + 1);
        }
    }

    fn with_symbol(&mut self, name: String, f: impl FnOnce(&mut Self)) {
        self.symbol.push(name);
        f(self);
        self.symbol.pop();
    }
}

impl<'ast> Visit<'ast> for SiteFinder<'_> {
    fn visit_item(&mut self, item: &'ast Item) {
        if is_test_gated(item_attrs(item)) {
            return;
        }
        visit::visit_item(self, item);
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        // An out-of-line module's file is visited on its own.
        if let Some((_, items)) = &item.content {
            let name = unraw(&item.ident);
            self.module.push(name.clone());
            self.with_symbol(name, |this| {
                for item in items {
                    this.visit_item(item);
                }
            });
            self.module.pop();
        }
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.with_symbol(unraw(&item.sig.ident), |this| {
            visit::visit_item_fn(this, item)
        });
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        self.with_symbol(type_name(&item.self_ty), |this| {
            visit::visit_item_impl(this, item)
        });
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        self.with_symbol(unraw(&item.ident), |this| {
            visit::visit_item_trait(this, item)
        });
    }

    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        self.with_symbol(unraw(&item.ident), |this| {
            visit::visit_item_const(this, item)
        });
    }

    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        self.with_symbol(unraw(&item.ident), |this| {
            visit::visit_item_static(this, item)
        });
    }

    fn visit_impl_item(&mut self, item: &'ast syn::ImplItem) {
        let attrs = match item {
            syn::ImplItem::Const(i) => &i.attrs,
            syn::ImplItem::Fn(i) => &i.attrs,
            syn::ImplItem::Type(i) => &i.attrs,
            syn::ImplItem::Macro(i) => &i.attrs,
            _ => return visit::visit_impl_item(self, item),
        };
        if !is_test_gated(attrs) {
            visit::visit_impl_item(self, item);
        }
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        self.with_symbol(unraw(&item.sig.ident), |this| {
            visit::visit_impl_item_fn(this, item)
        });
    }

    fn visit_trait_item(&mut self, item: &'ast syn::TraitItem) {
        let attrs = match item {
            syn::TraitItem::Const(i) => &i.attrs,
            syn::TraitItem::Fn(i) => &i.attrs,
            syn::TraitItem::Type(i) => &i.attrs,
            syn::TraitItem::Macro(i) => &i.attrs,
            _ => return visit::visit_trait_item(self, item),
        };
        if !is_test_gated(attrs) {
            visit::visit_trait_item(self, item);
        }
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        self.with_symbol(unraw(&item.sig.ident), |this| {
            visit::visit_trait_item_fn(this, item)
        });
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        if !is_test_gated(&local.attrs) {
            visit::visit_local(self, local);
        }
    }

    fn visit_expr_path(&mut self, expr: &'ast syn::ExprPath) {
        if expr.qself.is_none() {
            let segs = expr.path.segments.iter().map(|s| unraw(&s.ident)).collect();
            let span = expr
                .path
                .segments
                .first()
                .map_or_else(Span::call_site, |s| s.ident.span());
            self.check(expr.path.leading_colon.is_some(), segs, span);
        }
        visit::visit_expr_path(self, expr);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        let name = mac
            .path
            .segments
            .last()
            .map(|s| unraw(&s.ident))
            .unwrap_or_default();
        if !OUTPUT_CODE_MACROS.contains(&name.as_str()) {
            self.scan_tokens(mac.tokens.clone());
        }
    }
}

enum Flat {
    Ident(String, Span),
    Punct(char, proc_macro2::Spacing),
    Other,
}

fn flatten(tokens: TokenStream) -> Vec<Flat> {
    let mut out = Vec::new();
    for token in tokens {
        match token {
            TokenTree::Ident(ident) => {
                let text = ident.to_string();
                let text = text.strip_prefix("r#").map_or(text.clone(), str::to_string);
                out.push(Flat::Ident(text, ident.span()));
            }
            TokenTree::Punct(punct) => out.push(Flat::Punct(punct.as_char(), punct.spacing())),
            TokenTree::Group(group) => {
                // Delimiters break paths, so a group boundary is a separator.
                out.push(Flat::Other);
                if group.delimiter() != Delimiter::None || !group.stream().is_empty() {
                    out.extend(flatten(group.stream()));
                }
                out.push(Flat::Other);
            }
            TokenTree::Literal(_) => out.push(Flat::Other),
        }
    }
    out
}

fn is_path_sep(flat: &[Flat], index: usize) -> bool {
    matches!(
        (flat.get(index), flat.get(index + 1)),
        (
            Some(Flat::Punct(':', proc_macro2::Spacing::Joint)),
            Some(Flat::Punct(':', _))
        )
    )
}

// ---------------------------------------------------------------------------
// Attributes and names
// ---------------------------------------------------------------------------

fn item_attrs(item: &Item) -> &[Attribute] {
    match item {
        Item::Const(i) => &i.attrs,
        Item::Enum(i) => &i.attrs,
        Item::ExternCrate(i) => &i.attrs,
        Item::Fn(i) => &i.attrs,
        Item::ForeignMod(i) => &i.attrs,
        Item::Impl(i) => &i.attrs,
        Item::Macro(i) => &i.attrs,
        Item::Mod(i) => &i.attrs,
        Item::Static(i) => &i.attrs,
        Item::Struct(i) => &i.attrs,
        Item::Trait(i) => &i.attrs,
        Item::TraitAlias(i) => &i.attrs,
        Item::Type(i) => &i.attrs,
        Item::Union(i) => &i.attrs,
        Item::Use(i) => &i.attrs,
        _ => &[],
    }
}

/// Whether the item only exists in test builds or is itself a test.
fn is_test_gated(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        let last = attr
            .path()
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        if attr.path().is_ident("cfg") {
            return attr
                .parse_args::<Meta>()
                .is_ok_and(|meta| requires_test(&meta));
        }
        matches!(last.as_str(), "test" | "rstest")
    })
}

fn requires_test(meta: &Meta) -> bool {
    match meta {
        Meta::Path(path) => path.is_ident("test"),
        Meta::NameValue(_) => false,
        Meta::List(list) => {
            let Ok(nested) = list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
            else {
                return false;
            };
            if list.path.is_ident("all") {
                nested.iter().any(requires_test)
            } else if list.path.is_ident("any") {
                !nested.is_empty() && nested.iter().all(requires_test)
            } else {
                false
            }
        }
    }
}

fn path_attr(attr: &Attribute) -> Option<String> {
    if !attr.path().is_ident("path") {
        return None;
    }
    match &attr.meta {
        Meta::NameValue(nv) => match &nv.value {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) => Some(s.value()),
            _ => None,
        },
        _ => None,
    }
}

fn type_name(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(path) => path
            .path
            .segments
            .last()
            .map_or_else(|| "_".to_string(), |s| unraw(&s.ident)),
        syn::Type::Reference(reference) => type_name(&reference.elem),
        syn::Type::Paren(paren) => type_name(&paren.elem),
        syn::Type::Group(group) => type_name(&group.elem),
        _ => "_".to_string(),
    }
}

fn unraw(ident: &syn::Ident) -> String {
    let text = ident.to_string();
    text.strip_prefix("r#").map_or(text.clone(), str::to_string)
}

#[cfg(test)]
mod tests;
