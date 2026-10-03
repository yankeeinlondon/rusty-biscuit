//! Structural identity of a position in Rust source, for the site guards that
//! pin an exact set of call sites (`exit_site_guard.rs`,
//! `run_harness_loop_call_sites.rs`).
//!
//! A line number changes whenever an unrelated line is inserted above it, and a
//! per-file count accepts a site that moved to a different function. The
//! identity here is neither: it is the enclosing function's path within the
//! file plus the chain of branches between that function and the site, so it
//! survives reformatting and unrelated edits but changes when a site moves to
//! another function or out of the branch that justified it.
//!
//! The file is parsed with `syn`, so the caller's own detector decides *what*
//! is a site; this module only says *where* it is. A site inside a macro
//! invocation's tokens is attributed to the function and branches around the
//! macro, since `syn` does not parse macro bodies.

use proc_macro2::LineColumn;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

/// Where a site sits, structurally.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SiteContext {
    /// Enclosing function as a `::` path within the file: inline modules,
    /// `impl` self types (`<Type as Trait>` for trait impls), traits, and
    /// enclosing functions, ending with the function itself. [`MODULE_SCOPE`]
    /// when the site is outside every function.
    pub(crate) function: String,
    /// The branches between that function and the site, outermost first and
    /// joined by ` > `: `if <cond>`, `else of <cond>`, `arm <pattern>`, and
    /// `let-else <pattern>`, each spelled from the source with whitespace
    /// collapsed. Without a branch, [`TAIL`] when the site is inside the
    /// function body's tail expression and [`BODY`] otherwise.
    pub(crate) position: String,
}

/// [`SiteContext::function`] for a site outside every function (a `use`, a
/// `const`, a `static` initializer).
pub(crate) const MODULE_SCOPE: &str = "<module>";
/// [`SiteContext::position`] for a site in the function body's tail expression.
pub(crate) const TAIL: &str = "tail";
/// [`SiteContext::position`] for a site in any other statement of the body.
pub(crate) const BODY: &str = "body";

/// The structural context of each byte offset in `offsets`, in order.
///
/// ## Panics
///
/// Panics when `source` does not parse as a Rust file.
pub(crate) fn site_contexts(source: &str, offsets: &[usize]) -> Vec<SiteContext> {
    let file = syn::parse_file(source).unwrap_or_else(|error| {
        let start = error.span().start();
        panic!("source does not parse at {}:{}: {error}", start.line, start.column + 1)
    });
    let mut collector = Collector {
        source,
        line_starts: line_starts(source),
        path: Vec::new(),
        functions: Vec::new(),
        branches: Vec::new(),
    };
    collector.visit_file(&file);
    offsets
        .iter()
        .map(|&offset| collector.context_of(offset))
        .collect()
}

struct Function {
    range: (usize, usize),
    name: String,
    tail: Option<(usize, usize)>,
}

struct Branch {
    range: (usize, usize),
    label: String,
}

struct Collector<'a> {
    source: &'a str,
    line_starts: Vec<usize>,
    path: Vec<String>,
    functions: Vec<Function>,
    branches: Vec<Branch>,
}

impl Collector<'_> {
    fn offset(&self, at: LineColumn) -> usize {
        let line_start = self.line_starts[at.line - 1];
        // `LineColumn::column` counts characters, not bytes.
        self.source[line_start..]
            .char_indices()
            .nth(at.column)
            .map_or(self.source.len(), |(byte, _)| line_start + byte)
    }

    fn range(&self, node: &impl Spanned) -> (usize, usize) {
        let span = node.span();
        (self.offset(span.start()), self.offset(span.end()))
    }

    fn text(&self, node: &impl Spanned) -> String {
        let (start, end) = self.range(node);
        self.source[start..end].split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn function(&mut self, node: &impl Spanned, name: &syn::Ident, block: &syn::Block) {
        let tail = match block.stmts.last() {
            Some(syn::Stmt::Expr(expr, None)) => Some(self.range(expr)),
            _ => None,
        };
        let mut segments = self.path.clone();
        segments.push(name.to_string());
        self.functions.push(Function {
            range: self.range(node),
            name: segments.join("::"),
            tail,
        });
    }

    fn branch(&mut self, node: &impl Spanned, label: String) {
        let range = self.range(node);
        self.branches.push(Branch { range, label });
    }

    fn scoped(&mut self, segment: String, visit: impl FnOnce(&mut Self)) {
        self.path.push(segment);
        visit(self);
        self.path.pop();
    }

    fn context_of(&self, offset: usize) -> SiteContext {
        let contains = |(start, end): (usize, usize)| start <= offset && offset < end;
        // The innermost function is the one that starts last among those
        // containing the site: function ranges nest, they never overlap.
        let Some(function) = self
            .functions
            .iter()
            .filter(|function| contains(function.range))
            .max_by_key(|function| function.range.0)
        else {
            return SiteContext {
                function: MODULE_SCOPE.to_string(),
                position: BODY.to_string(),
            };
        };
        let mut branches: Vec<&Branch> = self
            .branches
            .iter()
            .filter(|branch| contains(branch.range) && branch.range.0 >= function.range.0)
            .collect();
        branches.sort_by_key(|branch| branch.range.0);
        let position = if branches.is_empty() {
            if function.tail.is_some_and(contains) { TAIL } else { BODY }.to_string()
        } else {
            branches
                .iter()
                .map(|branch| branch.label.as_str())
                .collect::<Vec<_>>()
                .join(" > ")
        };
        SiteContext {
            function: function.name.clone(),
            position,
        }
    }
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        self.scoped(node.ident.to_string(), |this| visit::visit_item_mod(this, node));
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        let self_ty = self.text(&*node.self_ty);
        let segment = match &node.trait_ {
            Some((_, trait_path, _)) => format!("<{self_ty} as {}>", self.text(trait_path)),
            None => self_ty,
        };
        self.scoped(segment, |this| visit::visit_item_impl(this, node));
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        self.scoped(node.ident.to_string(), |this| visit::visit_item_trait(this, node));
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.function(node, &node.sig.ident, &node.block);
        self.scoped(node.sig.ident.to_string(), |this| visit::visit_item_fn(this, node));
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.function(node, &node.sig.ident, &node.block);
        self.scoped(node.sig.ident.to_string(), |this| visit::visit_impl_item_fn(this, node));
    }

    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        if let Some(block) = &node.default {
            self.function(node, &node.sig.ident, block);
        }
        self.scoped(node.sig.ident.to_string(), |this| visit::visit_trait_item_fn(this, node));
    }

    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        let condition = self.text(&*node.cond);
        self.branch(&node.then_branch, format!("if {condition}"));
        if let Some((_, otherwise)) = &node.else_branch {
            self.branch(&**otherwise, format!("else of {condition}"));
        }
        visit::visit_expr_if(self, node);
    }

    fn visit_arm(&mut self, node: &'ast syn::Arm) {
        let mut label = format!("arm {}", self.text(&node.pat));
        if let Some((_, guard)) = &node.guard {
            label.push_str(&format!(" if {}", self.text(&**guard)));
        }
        self.branch(&*node.body, label);
        visit::visit_arm(self, node);
    }

    fn visit_local(&mut self, node: &'ast syn::Local) {
        if let Some(diverge) = node.init.as_ref().and_then(|init| init.diverge.as_ref()) {
            let label = format!("let-else {}", self.text(&node.pat));
            self.branch(&*diverge.1, label);
        }
        visit::visit_local(self, node);
    }
}

fn line_starts(source: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(source.match_indices('\n').map(|(at, _)| at + 1))
        .collect()
}
