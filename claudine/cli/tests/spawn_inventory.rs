//! Production process-spawn census and `AGENT_CWD` governance guard.
//!
//! A construction is *governed* only when the shared
//! `contribute_child_environment` helper is applied to **that same command**
//! before every execution of it and before it leaves the constructing
//! function, on every path. Counting helper calls per function is not enough:
//! two commands and two helper calls on one of them must still fail. What the
//! flow analysis cannot prove is reported `UNCONTROLLED` — the guard fails
//! closed.
//!
//! A file that names `Command` through a glob import (`use super::*`) is
//! censused too: intra-crate globs are resolved to the module's backing file so
//! the alias is inherited. An unqualified name is only ever a process command
//! when some resolved import supplies it — `clap::Command::new` spells its own
//! path and stays out.
//!
//! Known limits of the analysis, each deliberately conservative or scoped:
//!
//! - a closure or `async` body is analyzed from the state at its definition
//!   site and its effects do not flow back to the enclosing path;
//! - macro arguments are read as observations (`debug!`, `format!`), not as
//!   hand-offs that could execute a child.
//!
//! Regenerate after an intentional spawn-seam change:
//!
//! ```text
//! CLAUDINE_UPDATE_SPAWN_INVENTORY=1 cargo nextest run -p claudine-cli --test spawn_inventory
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

const BLESS_ENV: &str = "CLAUDINE_UPDATE_SPAWN_INVENTORY";
const REGEN_COMMAND: &str =
    "CLAUDINE_UPDATE_SPAWN_INVENTORY=1 cargo nextest run -p claudine-cli --test spawn_inventory";

/// The one helper allowed to satisfy the `AGENT_CWD` contract.
const HELPER: &str = "contribute_child_environment";

/// The module that defines [`HELPER`], and the file backing it — relative to
/// `claudine/`, the prefix the census strips. Identity, not spelling: a
/// function of the same name in any other module is a different item.
const HELPER_MODULE: &str = "child_environment";
const HELPER_DEFINITION_FILE: &str = "lib/src/child_environment.rs";

/// Path roots that reach [`HELPER_MODULE`]: `crate` inside the `claudine` lib,
/// the crate name from every other crate in the workspace.
const HELPER_ROOTS: [&str; 2] = ["crate", "claudine"];

/// Methods that make a built `Command` execute a child. `exec` is the Unix
/// `CommandExt` seam that never returns.
const EXECUTION_METHODS: [&str; 4] = ["spawn", "status", "output", "exec"];

const UNCONTROLLED: &str = "UNCONTROLLED";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct SpawnSite {
    path: String,
    line: usize,
    function: String,
    command_kind: &'static str,
    governed_by: String,
}

#[derive(Debug, Serialize)]
struct Inventory {
    tool: &'static str,
    scanned_roots: [&'static str; 2],
    regenerate: &'static str,
    sites: Vec<SpawnSite>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandKind {
    Std,
    Tokio,
}

impl CommandKind {
    fn label(self) -> &'static str {
        match self {
            Self::Std => "std",
            Self::Tokio => "tokio",
        }
    }
}

#[derive(Default)]
struct Aliases {
    std: BTreeSet<String>,
    tokio: BTreeSet<String>,
}

impl Aliases {
    fn collect(file: &syn::File) -> Self {
        let mut collector = AliasCollector::default();
        collector.visit_file(file);
        collector.aliases
    }

    fn merge(&mut self, other: Self) {
        self.std.extend(other.std);
        self.tokio.extend(other.tokio);
    }

    /// An *unqualified* `Alias::new` is a process command only when an import —
    /// written in the file or inherited through a glob — brought that alias into
    /// scope. A longer path has to spell the real one, so `clap::Command::new`
    /// stays out of the census even in a file that also has `std::process::Command`
    /// in scope.
    fn kind_for_constructor(&self, path: &syn::Path) -> Option<CommandKind> {
        let segments: Vec<_> = path.segments.iter().map(|segment| segment.ident.to_string()).collect();
        if segments.last().map(String::as_str) != Some("new") || segments.len() < 2 {
            return None;
        }
        if let [alias, _] = segments.as_slice() {
            if self.std.contains(alias) {
                return Some(CommandKind::Std);
            }
            if self.tokio.contains(alias) {
                return Some(CommandKind::Tokio);
            }
            return None;
        }
        if segments.ends_with(&[
            "std".to_string(),
            "process".to_string(),
            "Command".to_string(),
            "new".to_string(),
        ]) {
            return Some(CommandKind::Std);
        }
        if segments.ends_with(&[
            "tokio".to_string(),
            "process".to_string(),
            "Command".to_string(),
            "new".to_string(),
        ]) {
            return Some(CommandKind::Tokio);
        }
        None
    }
}

/// Transitive glob hops to follow. Two are enough for
/// `session.rs → wiring/mod.rs → re-exported child`; the third is slack, and the
/// visited set — not the cap — is what stops the real cycle
/// (`wiring/mod.rs` re-exports `session::*`, which imports `super::*`).
const GLOB_DEPTH: usize = 3;

/// Resolves a glob import to the file backing that module, so a `Command::new`
/// written under `use super::*` is still censused.
///
/// A path that does not resolve to a file under this crate's `src` —
/// `ratatui::widgets::*` — contributes nothing. Following a module's *own*
/// globs while merging deliberately over-approximates Rust's visibility rules:
/// a private `use` re-exported through `pub use child::*` is treated as visible,
/// because missing a real construction is the failure that matters here.
struct GlobResolver {
    src_root: PathBuf,
}

impl GlobResolver {
    fn aliases_for(&self, file_path: &Path, file: &syn::File) -> Aliases {
        let mut aliases = Aliases::collect(file);
        let mut visited = BTreeSet::from([file_path.to_path_buf()]);
        self.merge_globs(file_path, file, &mut aliases, &mut visited, GLOB_DEPTH);
        aliases
    }

    fn merge_globs(
        &self,
        file_path: &Path,
        file: &syn::File,
        aliases: &mut Aliases,
        visited: &mut BTreeSet<PathBuf>,
        depth: usize,
    ) {
        if depth == 0 {
            return;
        }
        for target in glob_targets(file) {
            let Some(resolved) = self.resolve(file_path, &target) else {
                continue;
            };
            if !visited.insert(resolved.clone()) {
                continue;
            }
            let Ok(source) = fs::read_to_string(&resolved) else {
                continue;
            };
            let Ok(parsed) = syn::parse_file(&source) else {
                continue;
            };
            aliases.merge(Aliases::collect(&parsed));
            self.merge_globs(&resolved, &parsed, aliases, visited, depth - 1);
        }
    }

    /// `segments` is a glob import's path with the `*` removed.
    fn resolve(&self, file_path: &Path, segments: &[String]) -> Option<PathBuf> {
        let (mut current, rest) = match segments.first().map(String::as_str) {
            Some("crate") => (self.crate_root_file()?, &segments[1..]),
            Some("self") => (file_path.to_path_buf(), &segments[1..]),
            Some("super") => (self.parent_module_file(file_path)?, &segments[1..]),
            // A bare `use foo::*` is an external crate unless a sibling or child
            // module of that name backs it.
            _ => (file_path.to_path_buf(), segments),
        };
        for segment in rest {
            current = child_module_file(&current, segment)?;
        }
        Some(current)
    }

    fn parent_module_file(&self, file_path: &Path) -> Option<PathBuf> {
        let directory = file_path.parent()?;
        let parent_directory = if is_module_root(file_path) {
            directory.parent()?
        } else {
            directory
        };
        if parent_directory == self.src_root {
            return self.crate_root_file();
        }
        let name = parent_directory.file_name()?.to_str()?;
        first_existing([
            parent_directory.join("mod.rs"),
            parent_directory.parent()?.join(format!("{name}.rs")),
        ])
    }

    fn crate_root_file(&self) -> Option<PathBuf> {
        first_existing([self.src_root.join("lib.rs"), self.src_root.join("main.rs")])
    }
}

/// `mod.rs`, `lib.rs`, and `main.rs` are their directory's module; every other
/// file owns a same-named subdirectory.
fn is_module_root(file_path: &Path) -> bool {
    matches!(
        file_path.file_stem().and_then(|stem| stem.to_str()),
        Some("mod" | "lib" | "main")
    )
}

fn child_module_file(parent: &Path, name: &str) -> Option<PathBuf> {
    let directory = if is_module_root(parent) {
        parent.parent()?.to_path_buf()
    } else {
        parent.with_extension("")
    };
    first_existing([
        directory.join(format!("{name}.rs")),
        directory.join(name).join("mod.rs"),
    ])
}

fn first_existing<const N: usize>(candidates: [PathBuf; N]) -> Option<PathBuf> {
    candidates.into_iter().find(|candidate| candidate.is_file())
}

/// Glob-import paths declared at file scope, with the `*` removed. Globs inside
/// an inline `mod` are skipped: their relative paths anchor to that module, not
/// to the file.
fn glob_targets(file: &syn::File) -> Vec<Vec<String>> {
    let mut targets = Vec::new();
    for item in &file.items {
        if let syn::Item::Use(item) = item {
            collect_glob_tree(&item.tree, &mut Vec::new(), &mut targets);
        }
    }
    targets
}

fn collect_glob_tree(tree: &syn::UseTree, prefix: &mut Vec<String>, targets: &mut Vec<Vec<String>>) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_glob_tree(&path.tree, prefix, targets);
            prefix.pop();
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_glob_tree(item, prefix, targets);
            }
        }
        syn::UseTree::Glob(_) => targets.push(prefix.clone()),
        syn::UseTree::Name(_) | syn::UseTree::Rename(_) => {}
    }
}

#[derive(Default)]
struct AliasCollector {
    aliases: Aliases,
}

impl<'ast> Visit<'ast> for AliasCollector {
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if cfg_test(&item.attrs) {
            return;
        }
        visit::visit_item_mod(self, item);
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        collect_use_tree(&item.tree, &mut Vec::new(), &mut self.aliases);
    }
}

fn collect_use_tree(tree: &syn::UseTree, prefix: &mut Vec<String>, aliases: &mut Aliases) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_use_tree(&path.tree, prefix, aliases);
            prefix.pop();
        }
        syn::UseTree::Name(name) => {
            let mut full = prefix.clone();
            full.push(name.ident.to_string());
            record_alias(&full, name.ident.to_string(), aliases);
        }
        syn::UseTree::Rename(rename) => {
            let mut full = prefix.clone();
            full.push(rename.ident.to_string());
            record_alias(&full, rename.rename.to_string(), aliases);
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_use_tree(item, prefix, aliases);
            }
        }
        syn::UseTree::Glob(_) => {}
    }
}

fn record_alias(full: &[String], alias: String, aliases: &mut Aliases) {
    let full: Vec<&str> = full.iter().map(String::as_str).collect();
    match full.as_slice() {
        ["std", "process", "Command"] => {
            aliases.std.insert(alias);
        }
        ["tokio", "process", "Command"] => {
            aliases.tokio.insert(alias);
        }
        _ => {}
    }
}

/// Index into [`FunctionAnalyzer::commands`]; one per construction site.
type CommandId = usize;

struct CommandRecord {
    line: usize,
    kind: CommandKind,
    /// The helper was applied to this command somewhere.
    helped: bool,
    /// This command could execute, or left the function, unhelped.
    violated: bool,
}

/// Path-sensitive facts at one point in a function body.
#[derive(Clone, Default)]
struct FlowState {
    /// The path cannot reach any following statement (`return`, `break`, ...).
    diverged: bool,
    /// Local bindings that may hold a tracked command. A binding holds a *set*
    /// because branches merge.
    bindings: BTreeMap<String, BTreeSet<CommandId>>,
    /// Commands constructed on this path.
    live: BTreeSet<CommandId>,
    /// Commands the helper has been applied to on this path.
    helped: BTreeSet<CommandId>,
}

impl FlowState {
    /// Merge two branches pessimistically: a binding may hold whatever either
    /// branch put in it, and a command survives as helped only when every
    /// branch that *constructed* it also helped it. A branch that never saw the
    /// command abstains rather than voting it unhelped.
    fn join(self, other: Self) -> Self {
        if self.diverged {
            return other;
        }
        if other.diverged {
            return self;
        }
        let mut bindings = self.bindings;
        for (name, ids) in other.bindings {
            bindings.entry(name).or_default().extend(ids);
        }
        let helped = self
            .helped
            .union(&other.helped)
            .copied()
            .filter(|id| {
                (!self.live.contains(id) || self.helped.contains(id))
                    && (!other.live.contains(id) || other.helped.contains(id))
            })
            .collect();
        Self {
            diverged: false,
            bindings,
            live: self.live.union(&other.live).copied().collect(),
            helped,
        }
    }
}

/// Flow-ordered walk of one function body, tracking each constructed command
/// through the bindings that hold it.
struct FunctionAnalyzer<'a> {
    aliases: &'a Aliases,
    commands: Vec<CommandRecord>,
}

impl<'a> FunctionAnalyzer<'a> {
    fn analyze(aliases: &'a Aliases, block: &syn::Block) -> Vec<CommandRecord> {
        let mut analyzer = Self {
            aliases,
            commands: Vec::new(),
        };
        let mut state = FlowState::default();
        // A command in the body's tail position is this function's return
        // value: it leaves local control exactly like an argument hand-off.
        let returned = analyzer.block(block, &mut state);
        analyzer.require_helped(&returned, &state);
        analyzer.commands
    }

    /// Returns the block's value: the ids its tail expression evaluates to.
    fn block(&mut self, block: &syn::Block, state: &mut FlowState) -> BTreeSet<CommandId> {
        let mut value = BTreeSet::new();
        for (index, stmt) in block.stmts.iter().enumerate() {
            if state.diverged {
                break;
            }
            match stmt {
                syn::Stmt::Local(local) => self.local(local, state),
                // A nested item is its own scope; `FileScanner` reaches it
                // through the syntax-tree traversal instead.
                syn::Stmt::Item(_) => {}
                syn::Stmt::Expr(expr, semicolon) => {
                    let ids = self.expr(expr, state);
                    if semicolon.is_none() && index + 1 == block.stmts.len() {
                        value = ids;
                    }
                }
                syn::Stmt::Macro(stmt) => self.macro_tokens(&stmt.mac, state),
            }
        }
        value
    }

    fn local(&mut self, local: &syn::Local, state: &mut FlowState) {
        let mut ids = BTreeSet::new();
        if let Some(init) = &local.init {
            ids = self.expr(&init.expr, state);
            if let Some((_, diverge)) = &init.diverge {
                let mut unmatched = state.clone();
                self.expr(diverge, &mut unmatched);
            }
        }
        match binding_name(&local.pat) {
            Some(name) => {
                state.bindings.insert(name, ids);
            }
            // Destructured into something this analysis cannot follow.
            None => self.require_helped(&ids, state),
        }
    }

    fn expr(&mut self, expr: &syn::Expr, state: &mut FlowState) -> BTreeSet<CommandId> {
        match expr {
            syn::Expr::Path(path) => path
                .path
                .get_ident()
                .and_then(|ident| state.bindings.get(&ident.to_string()).cloned())
                .unwrap_or_default(),
            syn::Expr::Call(call) => self.call(call, state),
            syn::Expr::MethodCall(call) => self.method_call(call, state),
            // Forms that pass a command through unchanged.
            syn::Expr::Paren(inner) => self.expr(&inner.expr, state),
            syn::Expr::Group(inner) => self.expr(&inner.expr, state),
            syn::Expr::Await(inner) => self.expr(&inner.base, state),
            syn::Expr::Try(inner) => self.expr(&inner.expr, state),
            syn::Expr::Cast(inner) => self.expr(&inner.expr, state),
            syn::Expr::Reference(inner) => self.expr(&inner.expr, state),
            syn::Expr::Unary(inner) => self.expr(&inner.expr, state),
            syn::Expr::Let(binding) => {
                self.expr(&binding.expr, state);
                BTreeSet::new()
            }
            syn::Expr::If(branch) => self.if_expr(branch, state),
            syn::Expr::Match(branch) => self.match_expr(branch, state),
            syn::Expr::Block(block) => self.block(&block.block, state),
            syn::Expr::Unsafe(block) => self.block(&block.block, state),
            syn::Expr::TryBlock(block) => self.block(&block.block, state),
            syn::Expr::Loop(repeat) => self.loop_body(&repeat.body, state),
            syn::Expr::While(repeat) => {
                self.expr(&repeat.cond, state);
                self.loop_body(&repeat.body, state)
            }
            syn::Expr::ForLoop(repeat) => {
                self.expr(&repeat.expr, state);
                self.loop_body(&repeat.body, state)
            }
            syn::Expr::Closure(closure) => {
                self.deferred(&closure.body, state);
                BTreeSet::new()
            }
            syn::Expr::Async(block) => {
                let mut deferred = state.clone();
                self.block(&block.block, &mut deferred);
                BTreeSet::new()
            }
            syn::Expr::Return(exit) => {
                self.exit(exit.expr.as_deref(), state);
                BTreeSet::new()
            }
            syn::Expr::Break(exit) => {
                self.exit(exit.expr.as_deref(), state);
                BTreeSet::new()
            }
            syn::Expr::Continue(_) => {
                state.diverged = true;
                BTreeSet::new()
            }
            syn::Expr::Assign(assign) => {
                let ids = self.expr(&assign.right, state);
                match binding_name_of_expr(&assign.left) {
                    Some(name) => {
                        state.bindings.insert(name, ids);
                    }
                    None => self.require_helped(&ids, state),
                }
                BTreeSet::new()
            }
            syn::Expr::Macro(invocation) => {
                self.macro_tokens(&invocation.mac, state);
                BTreeSet::new()
            }
            other => {
                self.opaque(other, state);
                BTreeSet::new()
            }
        }
    }

    fn call(&mut self, call: &syn::ExprCall, state: &mut FlowState) -> BTreeSet<CommandId> {
        if let syn::Expr::Path(function) = call.func.as_ref() {
            let function = &function.path;
            if let Some(kind) = self.aliases.kind_for_constructor(function) {
                for arg in &call.args {
                    self.argument(arg, state);
                }
                let id = self.commands.len();
                self.commands.push(CommandRecord {
                    line: call.span().start().line,
                    kind,
                    helped: false,
                    violated: false,
                });
                state.live.insert(id);
                return BTreeSet::from([id]);
            }
            if function.segments.last().is_some_and(|segment| segment.ident == HELPER) {
                let mut targets = BTreeSet::new();
                for (index, arg) in call.args.iter().enumerate() {
                    if index == 0 {
                        targets = self.expr(arg, state);
                    } else {
                        self.argument(arg, state);
                    }
                }
                for id in targets {
                    self.commands[id].helped = true;
                    state.helped.insert(id);
                }
                return BTreeSet::new();
            }
        } else {
            self.expr(call.func.as_ref(), state);
        }
        for arg in &call.args {
            self.argument(arg, state);
        }
        BTreeSet::new()
    }

    fn method_call(&mut self, call: &syn::ExprMethodCall, state: &mut FlowState) -> BTreeSet<CommandId> {
        let receiver = self.expr(&call.receiver, state);
        for arg in &call.args {
            self.argument(arg, state);
        }
        if EXECUTION_METHODS.contains(&call.method.to_string().as_str()) {
            self.require_helped(&receiver, state);
            return BTreeSet::new();
        }
        // A builder method hands the same command back; an unrecognized method
        // is assumed to as well, so tracking survives instead of going quiet.
        receiver
    }

    fn if_expr(&mut self, branch: &syn::ExprIf, state: &mut FlowState) -> BTreeSet<CommandId> {
        self.expr(&branch.cond, state);
        let mut taken = state.clone();
        let mut value = self.block(&branch.then_branch, &mut taken);
        let skipped = match &branch.else_branch {
            Some((_, otherwise)) => {
                let mut skipped = state.clone();
                value.extend(self.expr(otherwise, &mut skipped));
                skipped
            }
            None => state.clone(),
        };
        *state = taken.join(skipped);
        value
    }

    fn match_expr(&mut self, branch: &syn::ExprMatch, state: &mut FlowState) -> BTreeSet<CommandId> {
        self.expr(&branch.expr, state);
        let mut value = BTreeSet::new();
        let mut merged: Option<FlowState> = None;
        for arm in &branch.arms {
            let mut arm_state = state.clone();
            if let Some((_, guard)) = &arm.guard {
                self.expr(guard, &mut arm_state);
            }
            value.extend(self.expr(&arm.body, &mut arm_state));
            merged = Some(match merged {
                Some(previous) => previous.join(arm_state),
                None => arm_state,
            });
        }
        if let Some(merged) = merged {
            *state = merged;
        }
        value
    }

    /// Two passes. The second starts from the merge of the entry state and the
    /// first pass's exit, so a helper applied at the *end* of the body cannot
    /// govern the execution at the top of the next iteration.
    fn loop_body(&mut self, body: &syn::Block, state: &mut FlowState) -> BTreeSet<CommandId> {
        let mut first = state.clone();
        self.block(body, &mut first);
        let mut second = state.clone().join(first);
        second.diverged = false;
        self.block(body, &mut second);
        *state = state.clone().join(second);
        state.diverged = false;
        BTreeSet::new()
    }

    /// A closure or `async` body runs at an unknown point, so its effects do
    /// not flow back into the enclosing path. Violations it contains still
    /// count.
    fn deferred(&mut self, body: &syn::Expr, state: &FlowState) {
        let mut deferred = state.clone();
        self.expr(body, &mut deferred);
    }

    fn exit(&mut self, value: Option<&syn::Expr>, state: &mut FlowState) {
        if let Some(value) = value {
            let ids = self.expr(value, state);
            self.require_helped(&ids, state);
        }
        state.diverged = true;
    }

    /// A command handed to another function leaves this analysis: by value, or
    /// behind `&mut` (the callee can execute it). A shared borrow cannot
    /// execute a command, so it is not a hand-off.
    fn argument(&mut self, arg: &syn::Expr, state: &mut FlowState) {
        if let syn::Expr::Reference(reference) = strip_groups(arg) {
            let ids = self.expr(&reference.expr, state);
            if reference.mutability.is_some() {
                self.require_helped(&ids, state);
            }
            return;
        }
        let ids = self.expr(arg, state);
        self.require_helped(&ids, state);
    }

    /// Expression forms without dedicated handling: evaluate their direct
    /// children in source order and treat every command that reaches one as
    /// leaving local control.
    fn opaque(&mut self, expr: &syn::Expr, state: &mut FlowState) {
        for child in child_expressions(expr) {
            self.argument(child, state);
        }
    }

    /// Macro arguments are read as observations, not hand-offs: the macros in
    /// this tree format, log, or assert on a command rather than execute one.
    fn macro_tokens(&mut self, invocation: &syn::Macro, state: &mut FlowState) {
        use syn::parse::Parser;
        let parser = syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated;
        let Ok(args) = parser.parse2(invocation.tokens.clone()) else {
            return;
        };
        for arg in &args {
            self.expr(arg, state);
        }
    }

    /// The proof obligation: every command reaching a point where it can
    /// execute — its own execution seam, or a hand-off to code this analysis
    /// does not follow — must already carry the helper on this path.
    fn require_helped(&mut self, ids: &BTreeSet<CommandId>, state: &FlowState) {
        for id in ids {
            if !state.helped.contains(id) {
                self.commands[*id].violated = true;
            }
        }
    }
}

fn binding_name(pattern: &syn::Pat) -> Option<String> {
    match pattern {
        syn::Pat::Ident(ident) if ident.subpat.is_none() => Some(ident.ident.to_string()),
        syn::Pat::Type(typed) => binding_name(&typed.pat),
        _ => None,
    }
}

fn binding_name_of_expr(expr: &syn::Expr) -> Option<String> {
    match strip_groups(expr) {
        syn::Expr::Path(path) => path.path.get_ident().map(ToString::to_string),
        _ => None,
    }
}

fn strip_groups(expr: &syn::Expr) -> &syn::Expr {
    match expr {
        syn::Expr::Paren(inner) => strip_groups(&inner.expr),
        syn::Expr::Group(inner) => strip_groups(&inner.expr),
        other => other,
    }
}

/// The expressions one level below `expr`, in syntactic order.
fn child_expressions(expr: &syn::Expr) -> Vec<&syn::Expr> {
    #[derive(Default)]
    struct Children<'ast> {
        found: Vec<&'ast syn::Expr>,
    }

    impl<'ast> Visit<'ast> for Children<'ast> {
        fn visit_expr(&mut self, expr: &'ast syn::Expr) {
            self.found.push(expr);
        }
    }

    let mut children = Children::default();
    visit::visit_expr(&mut children, expr);
    children.found
}

struct FileScanner<'a> {
    path: &'a str,
    aliases: &'a Aliases,
    sites: Vec<SpawnSite>,
}

impl FileScanner<'_> {
    fn scan_function(&mut self, name: String, block: &syn::Block) {
        let indirect = indirect_governor(self.path, &name);
        for command in FunctionAnalyzer::analyze(self.aliases, block) {
            let governed_by = if command.helped && !command.violated {
                HELPER.to_string()
            } else if let Some(governor) = indirect {
                governor.to_string()
            } else {
                UNCONTROLLED.to_string()
            };
            self.sites.push(SpawnSite {
                path: self.path.to_string(),
                line: command.line,
                function: name.clone(),
                command_kind: command.kind.label(),
                governed_by,
            });
        }
    }
}

impl<'ast> Visit<'ast> for FileScanner<'_> {
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if !cfg_test(&item.attrs) {
            visit::visit_item_mod(self, item);
        }
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if cfg_test(&item.attrs) {
            return;
        }
        self.scan_function(item.sig.ident.to_string(), &item.block);
        // Recurse so a nested item is analyzed under its own name.
        visit::visit_item_fn(self, item);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        if cfg_test(&item.attrs) {
            return;
        }
        self.scan_function(item.sig.ident.to_string(), &item.block);
        visit::visit_impl_item_fn(self, item);
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        if cfg_test(&item.attrs) {
            return;
        }
        if let Some(default) = &item.default {
            self.scan_function(item.sig.ident.to_string(), default);
        }
        visit::visit_trait_item_fn(self, item);
    }
}

fn cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("cfg"))
        .any(|attr| meta_contains_test(&attr.meta))
}

fn meta_contains_test(meta: &syn::Meta) -> bool {
    match meta {
        syn::Meta::Path(path) => path.is_ident("test"),
        syn::Meta::List(list) => {
            use syn::parse::Parser;
            let parser = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated;
            parser
                .parse2(list.tokens.clone())
                .map(|nested| nested.iter().any(meta_contains_test))
                .unwrap_or(false)
        }
        syn::Meta::NameValue(_) => false,
    }
}

/// Factories whose product is governed by the caller that executes it. Per the
/// spec an allowlist may describe an indirect governed path; it may not exempt
/// a spawned child from `AGENT_CWD`.
fn indirect_governor(path: &str, function: &str) -> Option<&'static str> {
    match (path, function) {
        ("lib/src/composition/lifecycle/executor.rs", "system_shell_command") => {
            Some("caller:SystemShellRunner::run")
        }
        ("lib/src/composition/sequence/task/shell.rs", "system_shell_command") => {
            Some("caller:SystemTaskShell::run")
        }
        _ => None,
    }
}

/// Scan one source with only the imports it writes itself — no filesystem.
fn scan_source(path: &str, source: &str) -> Vec<SpawnSite> {
    let file = parse_source(path, source);
    let aliases = Aliases::collect(&file);
    scan_parsed(path, &file, &aliases)
}

/// Scan one source with the aliases its glob imports inherit from the crate.
fn scan_crate_file(display_path: &str, file_path: &Path, source: &str, resolver: &GlobResolver) -> Vec<SpawnSite> {
    let file = parse_source(display_path, source);
    let aliases = resolver.aliases_for(file_path, &file);
    scan_parsed(display_path, &file, &aliases)
}

fn parse_source(path: &str, source: &str) -> syn::File {
    syn::parse_file(source).unwrap_or_else(|error| panic!("failed to parse {path}: {error}"))
}

fn scan_parsed(path: &str, file: &syn::File, aliases: &Aliases) -> Vec<SpawnSite> {
    let mut scanner = FileScanner {
        path,
        aliases,
        sites: Vec::new(),
    };
    scanner.visit_file(file);
    scanner.sites.sort();
    scanner.sites
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("claudine/cli has a workspace root")
        .to_path_buf()
}

const SCANNED_ROOTS: [&str; 2] = ["claudine/lib/src", "claudine/cli/src"];

fn production_files(src_root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_rs_files(src_root, &mut files);
    files.sort();
    files
}

fn collect_rs_files(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) != Some("tests") {
                collect_rs_files(&path, files);
            }
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs")
            && path.file_name().and_then(|name| name.to_str()) != Some("tests.rs")
        {
            files.push(path);
        }
    }
}

fn generate_inventory() -> Inventory {
    let root = workspace_root();
    let mut sites = Vec::new();
    for relative_root in SCANNED_ROOTS {
        let src_root = root.join(relative_root);
        let resolver = GlobResolver {
            src_root: src_root.clone(),
        };
        for file in production_files(&src_root) {
            let relative = file.strip_prefix(root.join("claudine")).unwrap();
            let relative = relative.to_string_lossy().replace('\\', "/");
            let source = fs::read_to_string(&file).unwrap();
            sites.extend(scan_crate_file(&relative, &file, &source, &resolver));
        }
    }
    sites.sort();
    Inventory {
        tool: "claudine-spawn-inventory",
        scanned_roots: SCANNED_ROOTS,
        regenerate: REGEN_COMMAND,
        sites,
    }
}

#[test]
fn production_spawn_inventory_is_complete_and_governed() {
    let inventory = generate_inventory();
    let uncontrolled: Vec<_> = inventory
        .sites
        .iter()
        .filter(|site| site.governed_by == UNCONTROLLED)
        .collect();
    assert!(
        uncontrolled.is_empty(),
        "production process constructors without AGENT_CWD governance: {uncontrolled:#?}"
    );

    let rendered = format!("{}\n", serde_json::to_string_pretty(&inventory).unwrap());
    let artifact = workspace_root().join("claudine/docs/providers/spawn-seam-inventory.json");
    if std::env::var_os(BLESS_ENV).is_some() {
        fs::write(&artifact, rendered).unwrap();
        return;
    }
    let committed = fs::read_to_string(&artifact).unwrap_or_else(|error| {
        panic!("{} is missing ({error}); regenerate with `{REGEN_COMMAND}`", artifact.display())
    });
    assert_eq!(committed, rendered, "spawn inventory drifted; regenerate with `{REGEN_COMMAND}`");
}

/// Function names whose every construction is governed, and whose every
/// construction is not. A function that mixes both appears in neither set.
fn verdicts(sites: &[SpawnSite]) -> (BTreeSet<&str>, BTreeSet<&str>) {
    let governed = sites
        .iter()
        .filter(|site| site.governed_by != UNCONTROLLED)
        .map(|site| site.function.as_str())
        .collect::<BTreeSet<_>>();
    let uncontrolled = sites
        .iter()
        .filter(|site| site.governed_by == UNCONTROLLED)
        .map(|site| site.function.as_str())
        .collect::<BTreeSet<_>>();
    (
        governed.difference(&uncontrolled).copied().collect(),
        uncontrolled.difference(&governed).copied().collect(),
    )
}

fn tally(sites: &[SpawnSite], function: &str) -> (usize, usize) {
    let in_function = sites.iter().filter(|site| site.function == function);
    let (mut governed, mut uncontrolled) = (0, 0);
    for site in in_function {
        if site.governed_by == UNCONTROLLED {
            uncontrolled += 1;
        } else {
            governed += 1;
        }
    }
    (governed, uncontrolled)
}

#[test]
fn scanner_covers_governed_and_ungoverned_construction_forms() {
    let source = r#"
use claudine::child_environment::contribute_child_environment;
use std::process::Command;
use tokio::process::Command as TokioCommand;

fn governed_std() { let mut c = Command::new("x"); contribute_child_environment(&mut c); c.spawn(); }
fn ungoverned_std() { Command::new("x").spawn(); }
fn governed_tokio() { let mut c = TokioCommand::new("x"); contribute_child_environment(&mut c); c.output(); }
fn ungoverned_tokio() { TokioCommand::new("x").output(); }
fn governed_qualified() { let mut c = std::process::Command::new("x"); contribute_child_environment(&mut c); c.status(); }
fn ungoverned_qualified() { tokio::process::Command::new("x").status(); }
fn governed_helper() -> Command { let mut c = Command::new("x"); contribute_child_environment(&mut c); c }
fn ungoverned_helper() -> Command { Command::new("x") }
#[cfg(windows)] fn governed_platform() { let mut c = Command::new("cmd"); contribute_child_environment(&mut c); c.output(); }
#[cfg(unix)] fn ungoverned_platform() { Command::new("sh").output(); }
#[cfg(test)] mod tests { fn ignored() { std::process::Command::new("test-only").status(); } }
"#;
    let sites = scan_source("fixture.rs", source);
    let (governed, uncontrolled) = verdicts(&sites);
    assert_eq!(
        governed,
        BTreeSet::from(["governed_helper", "governed_platform", "governed_qualified", "governed_std", "governed_tokio"])
    );
    assert_eq!(
        uncontrolled,
        BTreeSet::from(["ungoverned_helper", "ungoverned_platform", "ungoverned_qualified", "ungoverned_std", "ungoverned_tokio"])
    );
}

/// The counterexamples the per-function helper-call count accepted: the helper
/// must reach the command that executes, and reach it first.
#[test]
fn scanner_requires_the_helper_on_the_executing_command() {
    let source = r#"
use claudine::child_environment::contribute_child_environment;
use std::process::Command;

fn duplicate_helper_on_one_command() {
    let mut safe = Command::new("safe");
    let mut leaked = Command::new("leaked");
    contribute_child_environment(&mut safe);
    contribute_child_environment(&mut safe);
    safe.spawn();
    leaked.output();
}

fn helper_after_execution() {
    let mut c = Command::new("x");
    c.spawn();
    contribute_child_environment(&mut c);
}

fn helper_after_execution_chain() {
    let mut c = Command::new("x");
    let status = c.status();
    contribute_child_environment(&mut c);
}

fn wrong_receiver() {
    let mut other = std::process::Command::new("other");
    let mut c = Command::new("x");
    contribute_child_environment(&mut other);
    contribute_child_environment(&mut other);
    other.spawn();
    c.spawn();
}

fn helped_after_handoff() {
    let mut c = Command::new("x");
    run_it(&mut c);
    contribute_child_environment(&mut c);
}
"#;
    let sites = scan_source("fixture.rs", source);
    assert_eq!(tally(&sites, "duplicate_helper_on_one_command"), (1, 1));
    assert_eq!(tally(&sites, "wrong_receiver"), (1, 1));
    assert_eq!(tally(&sites, "helper_after_execution"), (0, 1));
    assert_eq!(tally(&sites, "helper_after_execution_chain"), (0, 1));
    assert_eq!(tally(&sites, "helped_after_handoff"), (0, 1));
}

/// Governance must hold on every reachable path, and only on every reachable
/// path — a helper on each arm is enough.
#[test]
fn scanner_requires_the_helper_on_every_branch() {
    let source = r#"
use claudine::child_environment::contribute_child_environment;
use std::process::Command;

fn one_arm_only(flag: bool) {
    let mut c = Command::new("x");
    if flag {
        contribute_child_environment(&mut c);
    }
    c.spawn();
}

fn both_arms(flag: bool) {
    let mut c = Command::new("x");
    if flag {
        contribute_child_environment(&mut c);
        c.spawn();
    } else {
        contribute_child_environment(&mut c);
        c.status();
    }
}

fn one_match_arm(choice: u8) {
    let mut c = Command::new("x");
    match choice {
        0 => { contribute_child_environment(&mut c); }
        _ => {}
    }
    c.output();
}

fn every_match_arm_constructs(choice: u8) {
    let mut c = match choice {
        0 => Command::new("first"),
        _ => Command::new("second"),
    };
    contribute_child_environment(&mut c);
    c.spawn();
}

fn one_match_arm_constructs(choice: u8) {
    let mut c = match choice {
        0 => { let mut first = Command::new("first"); contribute_child_environment(&mut first); first }
        _ => Command::new("second"),
    };
    c.spawn();
}

fn early_return_before_execution(flag: bool) {
    let mut c = Command::new("x");
    if contribute_child_environment(&mut c).is_err() {
        return;
    }
    c.output();
}
"#;
    let sites = scan_source("fixture.rs", source);
    assert_eq!(tally(&sites, "one_arm_only"), (0, 1));
    assert_eq!(tally(&sites, "both_arms"), (1, 0));
    assert_eq!(tally(&sites, "one_match_arm"), (0, 1));
    assert_eq!(tally(&sites, "every_match_arm_constructs"), (2, 0));
    assert_eq!(tally(&sites, "one_match_arm_constructs"), (1, 1));
    assert_eq!(tally(&sites, "early_return_before_execution"), (1, 0));
}

/// A loop executes its body repeatedly: the helper has to precede the first
/// execution, not merely appear somewhere in the body.
#[test]
fn scanner_reads_loops_as_repeating() {
    let source = r#"
use claudine::child_environment::contribute_child_environment;
use std::process::Command;

fn helped_before_the_loop(runs: u32) {
    let mut c = Command::new("x");
    contribute_child_environment(&mut c);
    for _ in 0..runs {
        c.spawn();
    }
}

fn helped_inside_the_loop(runs: u32) {
    let mut c = Command::new("x");
    for _ in 0..runs {
        contribute_child_environment(&mut c);
        c.spawn();
    }
}

fn helped_after_executing(runs: u32) {
    let mut c = Command::new("x");
    for _ in 0..runs {
        c.spawn();
        contribute_child_environment(&mut c);
    }
}
"#;
    let sites = scan_source("fixture.rs", source);
    assert_eq!(tally(&sites, "helped_before_the_loop"), (1, 0));
    assert_eq!(tally(&sites, "helped_inside_the_loop"), (1, 0));
    assert_eq!(tally(&sites, "helped_after_executing"), (0, 1));
}

/// A command moved into a closure or `async` body executes there, at a time
/// this walk cannot order against the enclosing path.
#[test]
fn scanner_follows_a_command_into_a_deferred_body() {
    let source = r#"
use claudine::child_environment::contribute_child_environment;
use std::process::Command;

fn spawned_from_a_closure() {
    let mut c = Command::new("x");
    std::thread::spawn(move || {
        c.spawn();
    });
}

fn helped_before_the_closure() {
    let mut c = Command::new("x");
    contribute_child_environment(&mut c);
    std::thread::spawn(move || {
        c.spawn();
    });
}

fn awaited_without_the_helper() {
    let job = async {
        let mut c = Command::new("x");
        c.output().await;
    };
}
"#;
    let sites = scan_source("fixture.rs", source);
    assert_eq!(tally(&sites, "spawned_from_a_closure"), (0, 1));
    assert_eq!(tally(&sites, "helped_before_the_closure"), (1, 0));
    assert_eq!(tally(&sites, "awaited_without_the_helper"), (0, 1));
}

/// The shape of `cli/src/commands/wrap/exec/wiring/`: a provider spawn seam
/// written under `use super::*`, in a module its parent re-exports back with
/// `pub(crate) use session::*` — the cycle the visited set has to survive.
#[test]
fn a_glob_import_inherits_the_command_alias_from_the_crate() {
    let crate_dir = tempfile::tempdir().expect("temp crate");
    let src = crate_dir.path().join("src");
    fs::create_dir_all(src.join("wiring")).expect("temp crate layout");
    fs::write(src.join("lib.rs"), "mod wiring;\n").expect("crate root");
    fs::write(
        src.join("wiring/mod.rs"),
        "use claudine::child_environment::contribute_child_environment;\nuse std::process::{ChildStdin, Command, Stdio};\nmod session;\npub(crate) use session::*;\n",
    )
    .expect("parent module");

    let session_path = src.join("wiring/session.rs");
    let session = r#"
use super::*;

fn run_session() {
    let mut command = Command::new("provider");
    command.stdout(Stdio::piped());
    contribute_child_environment(&mut command).unwrap();
    command.spawn();
}

fn leaks_a_session() {
    Command::new("provider").spawn();
}
"#;
    fs::write(&session_path, session).expect("session module");

    let resolver = GlobResolver { src_root: src };
    let sites = scan_crate_file("wiring/session.rs", &session_path, session, &resolver);
    assert_eq!(tally(&sites, "run_session"), (1, 0));
    assert_eq!(tally(&sites, "leaks_a_session"), (0, 1));
    assert!(
        scan_source("wiring/session.rs", session).is_empty(),
        "without the crate context the alias is unknown, which is what made this seam invisible"
    );
}

/// A glob that leaves the crate resolves to no file, so it cannot make an
/// unqualified `Command::new` mean `std::process::Command`.
#[test]
fn a_non_crate_glob_supplies_no_command_alias() {
    let crate_dir = tempfile::tempdir().expect("temp crate");
    let src = crate_dir.path().join("src");
    fs::create_dir_all(&src).expect("temp crate layout");
    fs::write(src.join("lib.rs"), "mod widgets;\n").expect("crate root");

    let widgets_path = src.join("widgets.rs");
    let widgets = r#"
use ratatui::widgets::*;
use tracing_subscriber::prelude::*;

fn draw() {
    Command::new("x").spawn();
}
"#;
    fs::write(&widgets_path, widgets).expect("widgets module");

    let resolver = GlobResolver { src_root: src };
    assert!(scan_crate_file("widgets.rs", &widgets_path, widgets, &resolver).is_empty());
}

/// `clap::Command` shares the name and spells its own path; a file that has
/// both in scope must census only the process one.
#[test]
fn a_qualified_clap_command_is_not_a_process_command() {
    let source = r#"
use claudine::child_environment::contribute_child_environment;
use std::process::Command;

fn build_cli() {
    let muted = std::mem::replace(sub, clap::Command::new("__placeholder__"));
}

fn spawn_child() {
    let mut c = Command::new("x");
    contribute_child_environment(&mut c);
    c.spawn();
}
"#;
    let sites = scan_source("fixture.rs", source);
    assert_eq!(
        sites.iter().map(|site| site.function.as_str()).collect::<Vec<_>>(),
        vec!["spawn_child"]
    );
    assert_eq!(tally(&sites, "spawn_child"), (1, 0));
}

/// `use std::process;` and `use std::process as p;` are ordinary ways to reach
/// the constructor. A scanner that knows only the imported type and the fully
/// spelled path omits these children from the census altogether — the worst
/// outcome for a guard whose job is to enumerate them.
#[test]
fn a_module_or_crate_alias_still_names_the_process_command() {
    let source = r#"
use claudine::child_environment::contribute_child_environment;
use std as rust;
use std::process;
use std::process as sync;
use tokio::process as async_process;

fn governed_module_import() {
    let mut c = process::Command::new("x");
    contribute_child_environment(&mut c);
    c.spawn();
}

fn ungoverned_module_import() { process::Command::new("x").spawn(); }

fn governed_module_alias() {
    let mut c = sync::Command::new("x");
    contribute_child_environment(&mut c);
    c.status();
}

fn ungoverned_module_alias() { sync::Command::new("x").status(); }

fn governed_crate_alias() {
    let mut c = rust::process::Command::new("x");
    contribute_child_environment(&mut c);
    c.output();
}

fn ungoverned_crate_alias() { rust::process::Command::new("x").output(); }

fn governed_tokio_module_alias() {
    let mut c = async_process::Command::new("x");
    contribute_child_environment(&mut c);
    c.spawn();
}

fn ungoverned_tokio_module_alias() { async_process::Command::new("x").spawn(); }
"#;
    let sites = scan_source("fixture.rs", source);
    assert_eq!(tally(&sites, "governed_module_import"), (1, 0));
    assert_eq!(tally(&sites, "ungoverned_module_import"), (0, 1));
    assert_eq!(tally(&sites, "governed_module_alias"), (1, 0));
    assert_eq!(tally(&sites, "ungoverned_module_alias"), (0, 1));
    assert_eq!(tally(&sites, "governed_crate_alias"), (1, 0));
    assert_eq!(tally(&sites, "ungoverned_crate_alias"), (0, 1));
    assert_eq!(tally(&sites, "governed_tokio_module_alias"), (1, 0));
    assert_eq!(tally(&sites, "ungoverned_tokio_module_alias"), (0, 1));
    assert_eq!(
        sites
            .iter()
            .filter(|site| site.function.ends_with("tokio_module_alias"))
            .map(|site| site.command_kind)
            .collect::<Vec<_>>(),
        vec!["tokio", "tokio"]
    );
    assert_eq!(sites.len(), 8);
}

/// The precision the alias sets exist to protect: `clap::Command` shares the
/// name and is not a process. Neither its own path nor an import of the type
/// may enter the census, in any of the spellings production uses.
#[test]
fn a_clap_command_never_enters_the_census() {
    let source = r#"
use clap::Command;
use std::process;

fn build_cli() {
    let mut app = Command::new("claudine");
    app.subcommand(clap::Command::new("wrap"));
    app
}

fn partition(sub: &mut Command) {
    let muted = std::mem::replace(sub, clap::Command::new("__placeholder__"));
}
"#;
    assert!(scan_source("fixture.rs", source).is_empty());
}

/// The obligation is discharged by one specific item, not by any function that
/// happens to share its name.
#[test]
fn only_the_shared_helper_discharges_the_obligation() {
    let source = r#"
use claudine::child_environment::contribute_child_environment as contribute;
use crate::child_environment;
use std::process::Command;

fn a_local_function_does_not_govern() {
    let mut c = Command::new("x");
    contribute_child_environment(&mut c);
    c.spawn();
}

fn an_unrelated_module_does_not_govern() {
    let mut c = Command::new("x");
    helpers::contribute_child_environment(&mut c);
    c.spawn();
}

fn another_crate_does_not_govern() {
    let mut c = Command::new("x");
    elsewhere::child_environment::contribute_child_environment(&mut c);
    c.spawn();
}

fn the_crate_qualified_path_governs() {
    let mut c = Command::new("x");
    crate::child_environment::contribute_child_environment(&mut c);
    c.spawn();
}

fn the_claudine_qualified_path_governs() {
    let mut c = Command::new("x");
    claudine::child_environment::contribute_child_environment(&mut c);
    c.spawn();
}

fn an_imported_module_governs() {
    let mut c = Command::new("x");
    child_environment::contribute_child_environment(&mut c);
    c.spawn();
}

fn a_renamed_import_governs() {
    let mut c = Command::new("x");
    contribute(&mut c);
    c.spawn();
}

fn contribute_child_environment(target: &mut Command) {}
"#;
    let sites = scan_source("fixture.rs", source);
    let (governed, uncontrolled) = verdicts(&sites);
    assert_eq!(
        governed,
        BTreeSet::from([
            "an_imported_module_governs",
            "a_renamed_import_governs",
            "the_claudine_qualified_path_governs",
            "the_crate_qualified_path_governs",
        ])
    );
    assert_eq!(
        uncontrolled,
        BTreeSet::from([
            "a_local_function_does_not_govern",
            "an_unrelated_module_does_not_govern",
            "another_crate_does_not_govern",
        ])
    );
    assert_eq!(sites.len(), 7);
}

/// A bare name is the helper only through an import, and a locally defined
/// function of that name wins over one inherited through a glob.
#[test]
fn a_local_function_shadows_a_glob_inherited_helper_import() {
    let crate_dir = tempfile::tempdir().expect("temp crate");
    let src = crate_dir.path().join("src");
    fs::create_dir_all(&src).expect("temp crate layout");
    fs::write(src.join("lib.rs"), "mod shadow;\n").expect("crate root");
    fs::write(
        src.join("prelude.rs"),
        "pub use claudine::child_environment::contribute_child_environment;\npub use std::process::Command;\n",
    )
    .expect("prelude module");

    let shadow_path = src.join("shadow.rs");
    let shadow = r#"
use crate::prelude::*;

fn shadowed() {
    let mut c = Command::new("x");
    contribute_child_environment(&mut c);
    c.spawn();
}

fn qualified_anyway() {
    let mut c = Command::new("x");
    claudine::child_environment::contribute_child_environment(&mut c);
    c.spawn();
}

fn contribute_child_environment(target: &mut Command) {}
"#;
    fs::write(&shadow_path, shadow).expect("shadow module");

    let resolver = GlobResolver { src_root: src };
    let sites = scan_crate_file("shadow.rs", &shadow_path, shadow, &resolver);
    assert_eq!(tally(&sites, "shadowed"), (0, 1));
    assert_eq!(tally(&sites, "qualified_anyway"), (1, 0));
}

fn governors(sites: &[SpawnSite]) -> Vec<&str> {
    sites.iter().map(|site| site.governed_by.as_str()).collect()
}

/// The allowlist describes a factory whose caller applies the helper; it stays
/// scoped to the two `system_shell_command` factories.
#[test]
fn indirect_governor_applies_only_to_the_allowlisted_factories() {
    let source = r#"
fn system_shell_command() -> std::process::Command {
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c");
    cmd
}
"#;
    let allowlisted = scan_source("lib/src/composition/sequence/task/shell.rs", source);
    assert_eq!(governors(&allowlisted), vec!["caller:SystemTaskShell::run"]);
    let elsewhere = scan_source("lib/src/other.rs", source);
    assert_eq!(governors(&elsewhere), vec![UNCONTROLLED]);
}

/// An allowlisted factory describes a hand-off to a governing caller, so it may
/// only cover the escape. A child executed inside the factory is never seen by
/// that caller and stays `UNCONTROLLED`.
#[test]
fn the_allowlist_never_covers_an_execution_inside_the_factory() {
    let handed_off = r#"
fn system_shell_command() -> std::process::Command {
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c");
    cmd
}
"#;
    let executed = r#"
fn system_shell_command() -> std::process::Command {
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c");
    cmd.status();
    cmd
}
"#;
    let executor = "lib/src/composition/lifecycle/executor.rs";
    let shell = "lib/src/composition/sequence/task/shell.rs";
    assert_eq!(
        governors(&scan_source(executor, handed_off)),
        vec!["caller:SystemShellRunner::run"]
    );
    assert_eq!(governors(&scan_source(executor, executed)), vec![UNCONTROLLED]);
    assert_eq!(governors(&scan_source(shell, executed)), vec![UNCONTROLLED]);
}
