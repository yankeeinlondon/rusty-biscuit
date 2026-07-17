//! A `syn`-backed reader for the Claudine area's production Rust sources.
//!
//! Phase 6 of the error-propagation feature replaced a `map_err(|e| …)` grep
//! with this. The reason is not tidiness: the Phase 1 inventory recorded 13
//! grep hits that were *not* lossy at all (`eyre!("unknown server {id}")`,
//! `body.to_string()` one line below an unrelated `map_err`). A text scan
//! cannot tell an error binding from an id, so a grep guard has to choose
//! between missing real defects and drowning in false ones.
//!
//! What this module knows that a grep cannot:
//!
//! - **Provenance.** A binding is only inspected when the syntax tree proves it
//!   is an error: a `map_err`/`or_else`/`unwrap_or_else` closure parameter, or
//!   an `Err(e)` pattern in a `match`/`if let`/`let … else`.
//! - **Retention.** [`retains_typed`] answers the question the whole guard
//!   turns on — does the typed value survive? `Foo { message: e.to_string(),
//!   source: e }` stringifies `e` *and* keeps it, so the cause chain is intact
//!   and nothing is lost. A grep sees only the `to_string()`.
//! - **Enclosing symbol.** Findings and their allowlist entries are keyed to the
//!   function they live in, so an exception cannot silently spread to a
//!   same-looking line in another file.
//!
//! `#[cfg(test)]` items and test directories are skipped structurally, per the
//! spec's requirement that exclusions be path-based rather than substring
//! exceptions.
//!
//! See `claudine/features/2026-07-13-error-propogation/spec.md` §D2 and §D8.

use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{
    Attribute, Expr, ExprClosure, ExprMethodCall, ExprStruct, ImplItem, Item, ItemFn, ItemImpl,
    ItemMod, Macro, Member, Meta, Pat, Stmt,
};

/// The production roots the D8 guard and the D2 parity test scan.
///
/// Matches the spec's in-scope set. `gen/`, `catalog-types/`, and the
/// rendezvous crates are a spec Non-goal — their error systems are not
/// redesigned here — so they are out of scope by *path*, not by exception.
pub const SCAN_ROOTS: &[&str] = &["lib/src", "cli/src", "contract/src"];

/// Method names whose receiver is being flattened rather than carried.
///
/// A binding that only ever appears as one of these receivers has not survived:
/// the typed value is consumed to produce prose.
const STRINGIFY_METHODS: &[&str] = &["to_string", "to_owned", "into_string"];

/// Closure-taking combinators whose parameter binds the error half of a
/// `Result`.
const ERROR_BINDING_COMBINATORS: &[&str] = &["map_err", "or_else", "unwrap_or_else"];

/// Struct/variant field names that carry a failure description in prose.
const ERROR_TEXT_FIELDS: &[&str] = &["reason", "message", "detail", "error", "cause", "msg"];

/// Macros that build a formatted, type-erased report.
const REPORT_MACROS: &[&str] = &["eyre", "bail", "anyhow"];

/// `tracing` emitters, for the log-then-return-another-error shape.
const LOG_MACROS: &[&str] = &["error", "warn", "info", "debug", "trace"];

/// A lossy transport, as D8 enumerates them.
///
/// D8's sixth shape — a concrete `Diagnostic`/`BlockError` implementation absent
/// from discovery — is deliberately **not** a variant here. It is enforced by
/// `registry_lists_every_diagnostic_impl`, which reads [`ScanResult::trait_impls`]
/// against [`discovery_registry`]. Modeling it as a `Finding` would make it
/// allowlistable, and an unregistered diagnostic is never an acceptable
/// exception — it is the motivating incident.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Shape {
    /// `eyre!("context: {e}")` — a typed error becomes a formatted report.
    FormattedReport,
    /// `map_err(|e| … e.to_string() …)` — the classic collapse.
    ToStringCollapse,
    /// `Foo { message: format!("{e}") }` — a prose field populated from a typed
    /// error while the typed value is dropped.
    ErrorTextField,
    /// `format!("context: {e}")` feeding a returned error.
    FormatContext,
    /// A log call consumes the typed error and a *different* error is returned,
    /// so the cause never reaches the caller.
    LogThenReturnOther,
}

impl Shape {
    pub fn as_str(self) -> &'static str {
        match self {
            Shape::FormattedReport => "formatted_report",
            Shape::ToStringCollapse => "to_string_collapse",
            Shape::ErrorTextField => "error_text_field",
            Shape::FormatContext => "format_context",
            Shape::LogThenReturnOther => "log_then_return_other",
        }
    }

    pub fn parse(slug: &str) -> Option<Self> {
        [
            Shape::FormattedReport,
            Shape::ToStringCollapse,
            Shape::ErrorTextField,
            Shape::FormatContext,
            Shape::LogThenReturnOther,
        ]
        .into_iter()
        .find(|shape| shape.as_str() == slug)
    }
}

impl fmt::Display for Shape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A single lossy transport, keyed to the symbol that owns it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    pub shape: Shape,
    /// Area-relative, forward-slashed (`lib/src/harness/resolve.rs`).
    pub file: String,
    /// The enclosing symbol path *within* the file (`HarnessError::detail`).
    pub symbol: String,
    pub line: usize,
    /// The offending construct, for the failure message only. Never matched on.
    pub evidence: String,
}

/// A production `impl Diagnostic for …` / `impl BlockError for …`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitImpl {
    pub trait_name: String,
    pub type_name: String,
    pub file: String,
    /// Every string literal `fn code` can return. Empty when `code` is not
    /// overridden.
    pub codes: BTreeSet<String>,
    /// Every `detail` key the impl writes — `base["k"] = …` index assignments
    /// and `json!({ "k": … })` literals alike.
    pub detail_keys: BTreeSet<String>,
    pub overrides_detail: bool,
}

/// Where a `Box<T>` sits in a type position that puts it on a cause chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BoxSite {
    /// A `#[source]` / `#[from]` field typed `Box<T>` — D-7's original shape.
    SourceField,
    /// A `Result<_, Box<T>>` return type. `?` and `.into()` on one of these
    /// hands `Report::from` a `Box<T>`, which is how D-13 entered.
    ResultError,
}

impl BoxSite {
    pub fn as_str(self) -> &'static str {
        match self {
            BoxSite::SourceField => "source_field",
            BoxSite::ResultError => "result_error",
        }
    }
}

/// A `Box<T>` in a cause-chain type position.
///
/// The scan records every one and leaves *registered-ness* to the guard, so the
/// policy ("a boxed **registered diagnostic** is unreachable") lives with the
/// registry it depends on rather than being hard-coded into the reader.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BoxedError {
    pub site: BoxSite,
    /// The `T` in `Box<T>`.
    pub type_name: String,
    pub file: String,
    /// The enclosing symbol: `CompositionError::AtomicWriteFailed` for a field,
    /// the function path for a return type.
    pub symbol: String,
    pub line: usize,
}

/// Everything one pass over the production sources yields.
#[derive(Debug, Default)]
pub struct ScanResult {
    pub findings: Vec<Finding>,
    pub trait_impls: Vec<TraitImpl>,
    pub boxed_errors: Vec<BoxedError>,
}

/// The `claudine/` package-area root.
pub fn area_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("claudine/cli always has a parent")
        .to_path_buf()
}

/// Scan every production source under [`SCAN_ROOTS`], once per test binary.
///
/// Parsing the whole area takes a few seconds; the guards below all want the
/// same answer, so the result is computed once and shared.
///
/// ## Panics
///
/// Panics when a source root is unreadable or a file does not parse — either
/// means the guard is blind, which must never be silent.
pub fn scan_production_sources() -> &'static ScanResult {
    static SCAN: OnceLock<ScanResult> = OnceLock::new();
    SCAN.get_or_init(run_scan)
}

/// Scan one file's text exactly as [`scan_production_sources`] scans each
/// production file, attributing findings to `relative`.
///
/// Exposed so a guard arm with no live production instance can still be
/// anchored against a synthetic fixture rather than asserting nothing.
/// `CompositionError::AtomicWriteFailed` was the last `source_field` box in the
/// tree; retiring it left that arm with no production site to prove it works.
///
/// ## Panics
///
/// Panics if `text` is not parseable Rust.
pub fn scan_text(relative: &str, text: &str) -> ScanResult {
    let ast = syn::parse_file(text).expect("fixture text parses as Rust");
    let mut result = ScanResult::default();
    let mut scan = FileScan {
        file: relative.to_string(),
        file_ast: &ast,
        stack: Vec::new(),
        result: &mut result,
    };
    scan.visit_file(&ast);
    result.findings.sort();
    result.boxed_errors.sort();
    result
}

fn run_scan() -> ScanResult {
    let root = area_root();
    let mut result = ScanResult::default();

    for source_root in SCAN_ROOTS {
        let dir = root.join(source_root);
        let mut files = Vec::new();
        collect_rust_files(&dir, &mut files);
        files.sort();

        for file in files {
            let relative = relative_to(&root, &file);
            if is_generated(&relative) {
                continue;
            }
            let text = fs::read_to_string(&file)
                .unwrap_or_else(|error| panic!("failed to read {relative}: {error}"));
            if text.contains("@generated") {
                continue;
            }
            let ast = syn::parse_file(&text)
                .unwrap_or_else(|error| panic!("failed to parse {relative}: {error}"));

            let mut scan = FileScan {
                file: relative,
                file_ast: &ast,
                stack: Vec::new(),
                result: &mut result,
            };
            scan.visit_file(&ast);
        }
    }

    result.findings.sort();
    result.boxed_errors.sort();
    result
}

/// Every `downcast_ref::<T>()` the discovery seam performs, in source order.
///
/// The registry is a hand-authored downcast list because stable Rust cannot
/// upcast `&dyn Error` to `&dyn Diagnostic`. Reading it from source is what lets
/// the parity test compare the list against reality instead of trusting it.
pub fn discovery_registry() -> BTreeSet<String> {
    let path = area_root().join("lib/src/diagnostics/discovery.rs");
    let text = fs::read_to_string(&path).expect("discovery.rs is readable");
    let ast = syn::parse_file(&text).expect("discovery.rs parses");

    let mut registered = BTreeSet::new();
    for item in &ast.items {
        let Item::Fn(function) = item else { continue };
        if function.sig.ident != "as_diagnostic" {
            continue;
        }
        let mut collector = DowncastCollector {
            types: &mut registered,
        };
        collector.visit_item_fn(function);
    }
    registered
}

struct DowncastCollector<'a> {
    types: &'a mut BTreeSet<String>,
}

impl<'ast> Visit<'ast> for DowncastCollector<'_> {
    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        if node.method == "downcast_ref"
            && let Some(turbofish) = &node.turbofish
            && let Some(syn::GenericArgument::Type(ty)) = turbofish.args.first()
            && let Some(name) = type_name(ty)
        {
            self.types.insert(name);
        }
        syn::visit::visit_expr_method_call(self, node);
    }
}

// ---------------------------------------------------------------------------
// File walk
// ---------------------------------------------------------------------------

fn collect_rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => panic!("failed to scan {}: {error}", dir.display()),
    };
    for entry in entries {
        let entry = entry.expect("readable directory entry");
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            // Structural exclusions: build output and co-located test trees.
            if name == "target" || name == "tests" || name == "snapshots" {
                continue;
            }
            collect_rust_files(&path, out);
        } else if name.ends_with(".rs") && name != "tests.rs" {
            out.push(path);
        }
    }
}

fn relative_to(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .expect("scanned file lives under the area root")
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// `provider/<slug>/data.rs` is emitted by `claudine-gen`; a shape change there
/// belongs in `gen/src/emit.rs`, never in the emitted file.
fn is_generated(relative: &str) -> bool {
    relative.ends_with("/data.rs") && relative.contains("/provider/")
}

fn is_cfg_test(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        let Meta::List(list) = &attr.meta else {
            return false;
        };
        list.path.is_ident("cfg") && list.tokens.to_string().contains("test")
    })
}

// ---------------------------------------------------------------------------
// Per-file scan
// ---------------------------------------------------------------------------

struct FileScan<'a> {
    file: String,
    /// The whole file, so [`collect_trait_impl`] can follow a `code()` body into
    /// the file-level helper it delegates to.
    file_ast: &'a syn::File,
    stack: Vec<String>,
    result: &'a mut ScanResult,
}

impl FileScan<'_> {
    fn symbol(&self) -> String {
        if self.stack.is_empty() {
            "<file>".to_string()
        } else {
            self.stack.join("::")
        }
    }

    fn record(&mut self, shape: Shape, line: usize, evidence: String) {
        let finding = Finding {
            shape,
            file: self.file.clone(),
            symbol: self.symbol(),
            line,
            evidence,
        };
        self.result.findings.push(finding);
    }

    /// Inspect an error-bound body for every lossy shape.
    ///
    /// A body that retains the typed value is skipped wholesale: the cause chain
    /// survives, so stringifying the same error for a message alongside it is
    /// not a transport defect.
    fn scan_error_body(&mut self, body: &Expr, binding: &str) {
        if retains_typed(body, binding) {
            return;
        }
        let mut body_scan = BodyScan {
            binding,
            hits: Vec::new(),
        };
        body_scan.visit_expr(body);
        body_scan.scan_log_then_return(body);

        let hits = std::mem::take(&mut body_scan.hits);
        for (shape, line, evidence) in hits {
            self.record(shape, line, evidence);
        }
    }
}

impl FileScan<'_> {
    /// Record a `Box<T>` return type on a function signature.
    fn scan_result_error(&mut self, sig: &syn::Signature) {
        let syn::ReturnType::Type(_, ty) = &sig.output else {
            return;
        };
        let Some(error_ty) = result_error_type(ty) else {
            return;
        };
        let Some(inner) = boxed_inner_name(error_ty) else {
            return;
        };
        let symbol = self.symbol();
        let line = line_of(error_ty.span());
        self.result.boxed_errors.push(BoxedError {
            site: BoxSite::ResultError,
            type_name: inner,
            file: self.file.clone(),
            symbol,
            line,
        });
    }

    /// Record every `#[source]` / `#[from]` field typed `Box<T>` on a variant or
    /// struct.
    fn scan_source_fields(&mut self, owner: &str, fields: &syn::Fields) {
        for field in fields {
            if !has_source_or_from(&field.attrs) {
                continue;
            }
            let Some(inner) = boxed_inner_name(&field.ty) else {
                continue;
            };
            self.result.boxed_errors.push(BoxedError {
                site: BoxSite::SourceField,
                type_name: inner,
                file: self.file.clone(),
                symbol: owner.to_string(),
                line: line_of(field.ty.span()),
            });
        }
    }
}

impl<'ast> Visit<'ast> for FileScan<'_> {
    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        if is_cfg_test(&node.attrs) {
            return;
        }
        for variant in &node.variants {
            let owner = format!("{}::{}", node.ident, variant.ident);
            self.scan_source_fields(&owner, &variant.fields);
        }
        syn::visit::visit_item_enum(self, node);
    }

    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        if is_cfg_test(&node.attrs) {
            return;
        }
        let owner = node.ident.to_string();
        self.scan_source_fields(&owner, &node.fields);
        syn::visit::visit_item_struct(self, node);
    }

    fn visit_item_mod(&mut self, node: &'ast ItemMod) {
        if is_cfg_test(&node.attrs) {
            return;
        }
        self.stack.push(node.ident.to_string());
        syn::visit::visit_item_mod(self, node);
        self.stack.pop();
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        if is_cfg_test(&node.attrs) {
            return;
        }
        self.stack.push(node.sig.ident.to_string());
        self.scan_result_error(&node.sig);
        syn::visit::visit_item_fn(self, node);
        self.stack.pop();
    }

    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        if is_cfg_test(&node.attrs) {
            return;
        }
        let self_name = type_name(&node.self_ty).unwrap_or_else(|| "<impl>".to_string());
        let trait_name = node
            .trait_
            .as_ref()
            .and_then(|(_, path, _)| path.segments.last())
            .map(|segment| segment.ident.to_string());

        if let Some(trait_name) = &trait_name
            && (trait_name == "Diagnostic" || trait_name == "BlockError")
        {
            self.result.trait_impls.push(collect_trait_impl(
                trait_name,
                &self_name,
                &self.file,
                node,
                self.file_ast,
            ));
        }

        self.stack.push(self_name);
        syn::visit::visit_item_impl(self, node);
        self.stack.pop();
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        if is_cfg_test(&node.attrs) {
            return;
        }
        self.stack.push(node.sig.ident.to_string());
        self.scan_result_error(&node.sig);
        syn::visit::visit_impl_item_fn(self, node);
        self.stack.pop();
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        if ERROR_BINDING_COMBINATORS.contains(&node.method.to_string().as_str())
            && node.args.len() == 1
            && let Expr::Closure(closure) = &node.args[0]
            && let Some(binding) = closure_error_binding(closure)
        {
            self.scan_error_body(&closure.body, &binding);
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        for arm in &node.arms {
            if let Some(binding) = err_pattern_binding(&arm.pat) {
                self.scan_error_body(&arm.body, &binding);
            }
        }
        syn::visit::visit_expr_match(self, node);
    }

    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        if let Expr::Let(let_expr) = &*node.cond
            && let Some(binding) = err_pattern_binding(&let_expr.pat)
        {
            let block = Expr::Block(syn::ExprBlock {
                attrs: Vec::new(),
                label: None,
                block: node.then_branch.clone(),
            });
            self.scan_error_body(&block, &binding);
        }
        syn::visit::visit_expr_if(self, node);
    }

    fn visit_local(&mut self, node: &'ast syn::Local) {
        if let Some(binding) = err_pattern_binding(&node.pat)
            && let Some(init) = &node.init
            && let Some((_, diverge)) = &init.diverge
        {
            self.scan_error_body(diverge, &binding);
        }
        syn::visit::visit_local(self, node);
    }
}

fn collect_trait_impl(
    trait_name: &str,
    type_name: &str,
    file: &str,
    node: &ItemImpl,
    file_ast: &syn::File,
) -> TraitImpl {
    let mut codes = BTreeSet::new();
    let mut detail_keys = BTreeSet::new();
    let mut overrides_detail = false;

    for item in &node.items {
        let ImplItem::Fn(function) = item else {
            continue;
        };
        if function.sig.ident == "code" {
            let mut collector = StrLitCollector { lits: &mut codes };
            collector.visit_block(&function.block);
            // `CompositionError::code` hands its `ComposeFailed` arm to a
            // file-level `compose_failed_code(md)`. The codes it returns are
            // just as much part of the contract as the ones written inline, so
            // a scan that stopped at the `fn code` block would let a whole
            // family of codes escape both the catalog and the corpus checks.
            for helper in called_helpers(&function.block) {
                if let Some(body) = file_level_fn(file_ast, &helper) {
                    let mut collector = StrLitCollector { lits: &mut codes };
                    collector.visit_block(body);
                }
            }
        } else if function.sig.ident == "detail" {
            overrides_detail = true;
            let mut collector = DetailKeyCollector {
                keys: &mut detail_keys,
            };
            collector.visit_block(&function.block);
        }
    }

    TraitImpl {
        trait_name: trait_name.to_string(),
        type_name: type_name.to_string(),
        file: file.to_string(),
        codes,
        detail_keys,
        overrides_detail,
    }
}

/// Every plain function name called in a block (`foo(..)`, not `x.foo(..)`).
fn called_helpers(block: &syn::Block) -> BTreeSet<String> {
    struct Calls<'a> {
        names: &'a mut BTreeSet<String>,
    }
    impl<'ast> Visit<'ast> for Calls<'_> {
        fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
            if let Expr::Path(path) = &*node.func
                && let Some(ident) = path.path.get_ident()
            {
                self.names.insert(ident.to_string());
            }
            syn::visit::visit_expr_call(self, node);
        }
    }
    let mut names = BTreeSet::new();
    Calls { names: &mut names }.visit_block(block);
    names
}

fn file_level_fn<'a>(file: &'a syn::File, name: &str) -> Option<&'a syn::Block> {
    file.items.iter().find_map(|item| match item {
        Item::Fn(function) if function.sig.ident == name => Some(&*function.block),
        _ => None,
    })
}

struct StrLitCollector<'a> {
    lits: &'a mut BTreeSet<String>,
}

impl<'ast> Visit<'ast> for StrLitCollector<'_> {
    fn visit_lit_str(&mut self, node: &'ast syn::LitStr) {
        self.lits.insert(node.value());
    }
}

/// Collects every key a `detail()` projection writes.
///
/// Two shapes reach the same place: `base["reference"] = …` index assignment
/// (used where the projection seeds from `null_detail_for`) and a
/// `json!({ "command": … })` object literal.
struct DetailKeyCollector<'a> {
    keys: &'a mut BTreeSet<String>,
}

impl<'ast> Visit<'ast> for DetailKeyCollector<'_> {
    fn visit_expr_assign(&mut self, node: &'ast syn::ExprAssign) {
        if let Expr::Index(index) = &*node.left
            && let Expr::Lit(lit) = &*index.index
            && let syn::Lit::Str(key) = &lit.lit
        {
            self.keys.insert(key.value());
        }
        syn::visit::visit_expr_assign(self, node);
    }

    fn visit_macro(&mut self, node: &'ast Macro) {
        if node
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "json")
        {
            for key in json_object_keys(&node.tokens) {
                self.keys.insert(key);
            }
        }
        syn::visit::visit_macro(self, node);
    }
}

/// The direct keys of the outermost `{ … }` group in a `json!` token stream.
///
/// Only top-level keys are `detail` fields; a nested object's keys belong to
/// that value, not to the catalog's field set.
fn json_object_keys(tokens: &TokenStream) -> Vec<String> {
    let Some(TokenTree::Group(group)) = tokens.clone().into_iter().next() else {
        return Vec::new();
    };
    if group.delimiter() != Delimiter::Brace {
        return Vec::new();
    }

    let mut keys = Vec::new();
    let trees: Vec<TokenTree> = group.stream().into_iter().collect();
    let mut depth_ok = true;
    for (index, tree) in trees.iter().enumerate() {
        match tree {
            TokenTree::Punct(punct) if punct.as_char() == ',' => depth_ok = true,
            TokenTree::Literal(literal) => {
                if !depth_ok {
                    continue;
                }
                let is_key = matches!(
                    trees.get(index + 1),
                    Some(TokenTree::Punct(next)) if next.as_char() == ':'
                );
                if is_key && let Ok(lit) = syn::parse_str::<syn::LitStr>(&literal.to_string()) {
                    keys.push(lit.value());
                }
                depth_ok = false;
            }
            _ => depth_ok = false,
        }
    }
    keys
}

// ---------------------------------------------------------------------------
// Error-binding analysis
// ---------------------------------------------------------------------------

fn closure_error_binding(closure: &ExprClosure) -> Option<String> {
    if closure.inputs.len() != 1 {
        return None;
    }
    pat_ident(&closure.inputs[0])
}

fn pat_ident(pat: &Pat) -> Option<String> {
    match pat {
        Pat::Ident(ident) if ident.subpat.is_none() => Some(ident.ident.to_string()),
        Pat::Type(typed) => pat_ident(&typed.pat),
        _ => None,
    }
}

/// The binding in an `Err(e)` pattern, if the pattern is exactly that.
fn err_pattern_binding(pat: &Pat) -> Option<String> {
    let Pat::TupleStruct(tuple) = pat else {
        return None;
    };
    if tuple.path.segments.last().is_none_or(|s| s.ident != "Err") || tuple.elems.len() != 1 {
        return None;
    }
    pat_ident(&tuple.elems[0])
}

/// Whether a field carries `#[source]` or `#[from]` — the two attributes that
/// publish it to `Error::source()`.
fn has_source_or_from(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        matches!(&attr.meta, Meta::Path(path) if path.is_ident("source") || path.is_ident("from"))
    })
}

/// The `T` in `Box<T>`, when `ty` is exactly that.
fn boxed_inner_name(ty: &syn::Type) -> Option<String> {
    let syn::Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    if segment.ident != "Box" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    let syn::GenericArgument::Type(inner) = args.args.first()? else {
        return None;
    };
    type_name(inner)
}

/// The `E` in a `Result<_, E>` return type.
fn result_error_type(ty: &syn::Type) -> Option<&syn::Type> {
    let syn::Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    if segment.ident != "Result" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    // `Result<T, E>`; a one-argument `Result<T>` is an aliased error type this
    // scan cannot resolve and deliberately does not guess at.
    if args.args.len() != 2 {
        return None;
    }
    match args.args.last()? {
        syn::GenericArgument::Type(error) => Some(error),
        _ => None,
    }
}

fn type_name(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(path) => path.path.segments.last().map(|s| s.ident.to_string()),
        syn::Type::Reference(reference) => type_name(&reference.elem),
        _ => None,
    }
}

fn expr_is_binding(expr: &Expr, binding: &str) -> bool {
    matches!(expr, Expr::Path(path) if path.path.is_ident(binding))
}

/// Does the typed value survive this body?
///
/// True when the binding appears as a bare expression anywhere that is not a
/// stringifying receiver — moved into a `#[source]` field, handed to `From`,
/// boxed, or passed to `wrap_err`. In every such case the cause chain is intact
/// and the body is not a transport defect, however much prose it also builds.
///
/// Macro token streams are deliberately *not* searched: `eyre!("{e}")` mentions
/// `e` only inside a string literal, which is the collapse this guard exists to
/// catch, not a retention.
fn retains_typed(body: &Expr, binding: &str) -> bool {
    let mut probe = RetentionProbe {
        binding,
        retained: false,
    };
    probe.visit_expr(body);
    probe.retained
}

struct RetentionProbe<'a> {
    binding: &'a str,
    retained: bool,
}

impl<'ast> Visit<'ast> for RetentionProbe<'_> {
    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let stringifies = STRINGIFY_METHODS.contains(&node.method.to_string().as_str())
            && expr_is_binding(&node.receiver, self.binding);
        if !stringifies {
            self.visit_expr(&node.receiver);
        }
        for arg in &node.args {
            self.visit_expr(arg);
        }
    }

    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        if node.path.is_ident(self.binding) {
            self.retained = true;
        }
    }
}

/// Finds the lossy shapes inside a body already known to drop its typed value.
struct BodyScan<'a> {
    binding: &'a str,
    hits: Vec<(Shape, usize, String)>,
}

impl BodyScan<'_> {
    fn hit(&mut self, shape: Shape, line: usize, evidence: String) {
        self.hits.push((shape, line, evidence));
    }

    /// The log-then-return-another-error shape.
    ///
    /// Only a statement block can exhibit it: the typed error is consumed by a
    /// `tracing` call and a *different* error is returned, so the caller sees a
    /// failure whose cause exists only in the log.
    fn scan_log_then_return(&mut self, body: &Expr) {
        let Expr::Block(block) = body else { return };
        let mut logged_at: Option<usize> = None;

        for stmt in &block.block.stmts {
            if let Some(line) = self.log_line(stmt) {
                logged_at = Some(line);
                continue;
            }
            if let Some(line) = logged_at
                && returns_unrelated_error(stmt, self.binding)
            {
                self.hit(
                    Shape::LogThenReturnOther,
                    line,
                    format!("`{}` is logged, then an unrelated error is returned", self.binding),
                );
                return;
            }
        }
    }

    fn log_line(&self, stmt: &Stmt) -> Option<usize> {
        let expr = match stmt {
            Stmt::Expr(expr, _) => expr,
            Stmt::Macro(mac) => {
                return macro_is(&mac.mac, LOG_MACROS)
                    .then(|| line_of(mac.mac.span()))
                    .filter(|_| macro_mentions(&mac.mac.tokens, self.binding));
            }
            _ => return None,
        };
        let Expr::Macro(mac) = expr else { return None };
        (macro_is(&mac.mac, LOG_MACROS) && macro_mentions(&mac.mac.tokens, self.binding))
            .then(|| line_of(mac.mac.span()))
    }
}

impl<'ast> Visit<'ast> for BodyScan<'_> {
    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        if STRINGIFY_METHODS.contains(&node.method.to_string().as_str())
            && expr_is_binding(&node.receiver, self.binding)
        {
            self.hit(
                Shape::ToStringCollapse,
                line_of(node.method.span()),
                format!("{}.{}()", self.binding, node.method),
            );
            return;
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_struct(&mut self, node: &'ast ExprStruct) {
        // A prose field populated from the binding is the most specific reading
        // of the site, so it claims the field and the generic macro/`to_string`
        // shapes inside it are not reported a second time.
        for field in &node.fields {
            let claimed = matches!(&field.member, Member::Named(name)
                if ERROR_TEXT_FIELDS.contains(&name.to_string().as_str()))
                && expr_mentions(&field.expr, self.binding);
            if claimed {
                let Member::Named(name) = &field.member else {
                    unreachable!("claimed implies a named member")
                };
                self.hit(
                    Shape::ErrorTextField,
                    line_of(field.span()),
                    format!("`{name}` is built from `{}`", self.binding),
                );
            } else {
                self.visit_expr(&field.expr);
            }
        }
        if let Some(rest) = &node.rest {
            self.visit_expr(rest);
        }
    }

    fn visit_macro(&mut self, node: &'ast Macro) {
        if !macro_mentions(&node.tokens, self.binding) {
            return;
        }
        let line = line_of(node.span());
        if macro_is(node, REPORT_MACROS) {
            let name = macro_name(node);
            self.hit(
                Shape::FormattedReport,
                line,
                format!("{name}! interpolates `{}`", self.binding),
            );
        } else if macro_is(node, &["format"]) {
            self.hit(
                Shape::FormatContext,
                line,
                format!("format! interpolates `{}`", self.binding),
            );
        }
    }
}

fn macro_name(mac: &Macro) -> String {
    mac.path
        .segments
        .last()
        .map(|s| s.ident.to_string())
        .unwrap_or_default()
}

fn macro_is(mac: &Macro, names: &[&str]) -> bool {
    names.contains(&macro_name(mac).as_str())
}

/// Does a macro's token stream reference `binding`?
///
/// Covers both the captured form (`"{e}"`) and the positional form
/// (`"{}", e`), because the collapse is identical either way.
fn macro_mentions(tokens: &TokenStream, binding: &str) -> bool {
    for tree in tokens.clone() {
        match tree {
            TokenTree::Ident(ident) if ident == binding => return true,
            TokenTree::Group(group) => {
                if macro_mentions(&group.stream(), binding) {
                    return true;
                }
            }
            TokenTree::Literal(literal) => {
                let text = literal.to_string();
                if text.contains(&format!("{{{binding}}}")) || text.contains(&format!("{{{binding}:"))
                {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// Does the expression reference `binding` at all — as a path or inside a macro?
fn expr_mentions(expr: &Expr, binding: &str) -> bool {
    struct Probe<'a> {
        binding: &'a str,
        found: bool,
    }
    impl<'ast> Visit<'ast> for Probe<'_> {
        fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
            if node.path.is_ident(self.binding) {
                self.found = true;
            }
        }
        fn visit_macro(&mut self, node: &'ast Macro) {
            if macro_mentions(&node.tokens, self.binding) {
                self.found = true;
            }
        }
    }
    let mut probe = Probe {
        binding,
        found: false,
    };
    probe.visit_expr(expr);
    probe.found
}

fn returns_unrelated_error(stmt: &Stmt, binding: &str) -> bool {
    let expr = match stmt {
        Stmt::Expr(expr, _) => expr,
        _ => return false,
    };
    let candidate = match expr {
        Expr::Return(ret) => match &ret.expr {
            Some(inner) => inner.as_ref(),
            None => return false,
        },
        other => other,
    };
    let Expr::Call(call) = candidate else {
        return false;
    };
    let is_err = matches!(&*call.func, Expr::Path(path)
        if path.path.segments.last().is_some_and(|s| s.ident == "Err"));
    is_err && !call.args.iter().any(|arg| expr_mentions(arg, binding))
}

fn line_of(span: proc_macro2::Span) -> usize {
    span.start().line
}
