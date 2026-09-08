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
//! Both ends of that contract are resolved by *identity* rather than by
//! spelling. Every path — in a `use`, a `type` alias, or an expression — is
//! followed through the scanned crate's own bindings to the item it names: a
//! name is what its module's `use` or `type` bound it to, an inline or
//! file-backed submodule, or, failing those, what a glob import brings in from
//! the module it names. A path constructs a process command when it reaches
//! `std::process::Command` or `tokio::process::Command` that way, through any
//! number of re-exports or aliases and in whichever order they were declared —
//! `process::Command::new`, `p::Command::new`, an alias of an alias, and a
//! name imported from a local module that re-exports the type under another
//! name are all censused, while `clap::Command::new` resolves elsewhere and
//! stays out. A call discharges the obligation when its path reaches the
//! helper item the same way: `crate::child_environment::contribute_child_environment`
//! from either root, a `use` of it, or a local re-export of it. A function
//! that merely shares the name governs nothing, and a locally defined function
//! or module still wins over a glob-inherited import, as it does in Rust.
//!
//! A constructor or the helper held in a local binding — `let construct =
//! Command::new;` — constructs or governs when that binding is called, exactly
//! as the direct call does. Rebinding the name clears it. Across branches the
//! two fail closed in opposite directions: a constructor bound on either
//! branch still constructs, a helper governs only if every branch bound it.
//!
//! A constructor that leaves a function as a *value* — an argument, a struct
//! field, a tuple element, a return value, a closure's result — is a
//! construction this function can no longer describe: the code that receives
//! it decides which children it makes and when. It is recorded at the
//! expression as `UNCONTROLLED`, the same verdict a command handed off unhelped
//! gets, because no caller-side proof exists for it. The helper handed off the
//! same way constructs nothing and needs no record. On the receiving side, a
//! parameter whose type is a callable returning the command — `impl Fn(..) ->
//! Command`, a `fn` pointer, `Box<dyn Fn..>`, `&dyn Fn..`, or a generic bound
//! that way in the generics list or the `where` clause — constructs when it is
//! called, exactly like a bound constructor. A `Command` received by value is
//! the caller's hand-off obligation, not the callee's.
//!
//! A `macro_rules!` defined in a scanned file is expanded at the syntax level:
//! each transcriber is rewritten with placeholders for its metavariables, one
//! copy per repetition, and `$crate` as a definition-crate marker, then
//! censused under the name `macro!` — or `macro!::fn` for a function the
//! transcriber emits. A non-exported macro is checked in every module context
//! in its scanned crate; a macro exported directly or through a
//! production-reachable `cfg_attr` is checked in every module context across
//! both guarded crates. Unknown feature and platform predicates are reachable
//! for this purpose. This conservative superset includes every possible local
//! invocation scope, so invocation-site resolution cannot hide a construction;
//! an impossible context may instead produce a loud false positive. A
//! transcriber that yields neither a block nor items fails the census outright.
//! Invocation arguments are still read as expressions.
//!
//! Glob chains through the crate are followed to any depth; the resolution
//! stack, not a hop limit, is what terminates the real cycle in
//! `cli/src/commands/wrap/exec/wiring/` (`mod.rs` re-exports `session::*`,
//! which imports `super::*`).
//!
//! An item is left out only when its `cfg` predicate is proven false outside
//! test builds. `not(test)` and `any(test, unix)` select production code and
//! stay in the census; `test` and `all(test, unix)` do not.
//!
//! An allowlisted factory may name the caller that governs its product, but
//! only for a command that leaves the factory: a child the factory executes
//! itself is `UNCONTROLLED` regardless.
//!
//! Known limits of the analysis, each deliberately conservative or scoped:
//!
//! - a closure or `async` body is analyzed from the state at its definition
//!   site and its effects do not flow back to the enclosing path;
//! - a macro defined outside the two scanned roots — declarative or procedural
//!   — is not expanded because its generated tokens are not source-authored in
//!   the guarded roots. Its invocation arguments are still seen;
//! - a local `struct` or `enum` that shadows a glob-inherited `Command` name
//!   is still censused. That error is a loud false positive the inventory
//!   check surfaces, never a silent pass, so it is not resolved.
//!
//! Regenerate after an intentional spawn-seam change:
//!
//! ```text
//! CLAUDINE_UPDATE_SPAWN_INVENTORY=1 cargo nextest run -p claudine-cli --test spawn_inventory
//! ```

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use proc_macro2::{Delimiter, Group, Ident, TokenStream, TokenTree};
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
const MACRO_DEFINITION_CRATE: &str = "__macro_definition_crate";

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

/// What the scanned file itself says about a helper name it could resolve.
///
/// A local `fn` and a `use` of the same name collide in the value namespace, so
/// they cannot both be written in one module — but a local `fn` legally shadows
/// a name a *glob* import brought in. Treating any locally defined function as
/// the winner therefore rejects both shadowing forms, and rejects nothing a
/// module could legitimately have resolved to the shared helper. A local `mod`
/// stands in the same relation to a glob-imported module in the type
/// namespace, so the helper's module binding gets the same treatment.
struct BareHelperScope {
    /// Free functions the file declares outside `#[cfg(test)]`.
    local_functions: BTreeSet<String>,
    /// Modules the file declares outside `#[cfg(test)]`, inline or `mod x;`.
    local_modules: BTreeSet<String>,
    /// The file is the one that defines [`HELPER`], where the bare name is it.
    definition_site: bool,
}

impl BareHelperScope {
    fn for_location(path: &str, location: &Location) -> Self {
        let scope = location.scope().expect("a scanner location names a module scope");
        let definition_file = location
            .file
            .as_deref()
            .is_some_and(|file| file.ends_with(HELPER_DEFINITION_FILE));
        Self {
            local_functions: scope.local_functions.clone(),
            local_modules: scope.inline.keys().chain(&scope.declared).cloned().collect(),
            definition_site: (definition_file || location.file.is_none() && path == HELPER_DEFINITION_FILE)
                && location.inline_path.is_empty(),
        }
    }
}

/// One module's own name bindings, as far as resolution needs them. An item
/// declared inside a function body belongs to the module around it.
#[derive(Default)]
struct ModuleScope {
    /// `use` bindings as `(name, target path)`. A rename binds the new name;
    /// `use a::{self}` binds `a` to the path `a`.
    uses: Vec<(String, Vec<String>)>,
    /// `type` aliases as `(name, target path)`.
    type_aliases: Vec<(String, Vec<String>)>,
    /// Glob imports with the `*` removed.
    globs: Vec<Vec<String>>,
    /// Inline `mod x { ... }` bodies outside `#[cfg(test)]`.
    inline: BTreeMap<String, ModuleScope>,
    /// `mod x;` declarations, whose bodies are files found on lookup.
    declared: BTreeSet<String>,
    /// Free functions declared in this module, used for helper shadowing.
    local_functions: BTreeSet<String>,
}

impl ModuleScope {
    fn of_file(file: &syn::File) -> Self {
        let mut collector = ScopeCollector {
            stack: vec![Self::default()],
        };
        collector.visit_file(file);
        collector.stack.pop().expect("the root scope outlives the walk")
    }
}

struct ScopeCollector {
    stack: Vec<ModuleScope>,
}

impl ScopeCollector {
    fn current(&mut self) -> &mut ModuleScope {
        self.stack.last_mut().expect("the root scope outlives the walk")
    }
}

impl<'ast> Visit<'ast> for ScopeCollector {
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if cfg_test(&item.attrs) {
            return;
        }
        if item.content.is_none() {
            self.current().declared.insert(item.ident.to_string());
            return;
        }
        self.stack.push(ModuleScope::default());
        visit::visit_item_mod(self, item);
        let scope = self.stack.pop().expect("the pushed scope is still there");
        self.current().inline.insert(item.ident.to_string(), scope);
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        collect_use_tree(&item.tree, &mut Vec::new(), self.current());
    }

    fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
        if cfg_test(&item.attrs) {
            return;
        }
        if let syn::Type::Path(target) = item.ty.as_ref()
            && target.qself.is_none()
        {
            self.current().type_aliases.push((item.ident.to_string(), path_segments(&target.path)));
        }
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if cfg_test(&item.attrs) {
            return;
        }
        self.current().local_functions.insert(item.sig.ident.to_string());
        visit::visit_item_fn(self, item);
    }
}

fn collect_use_tree(tree: &syn::UseTree, prefix: &mut Vec<String>, scope: &mut ModuleScope) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_use_tree(&path.tree, prefix, scope);
            prefix.pop();
        }
        syn::UseTree::Name(name) => {
            if name.ident == "self" {
                if let Some(last) = prefix.last() {
                    scope.uses.push((last.clone(), prefix.clone()));
                }
            } else {
                let mut full = prefix.clone();
                full.push(name.ident.to_string());
                scope.uses.push((name.ident.to_string(), full));
            }
        }
        syn::UseTree::Rename(rename) => {
            let mut full = prefix.clone();
            if rename.ident != "self" {
                full.push(rename.ident.to_string());
            }
            scope.uses.push((rename.rename.to_string(), full));
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_use_tree(item, prefix, scope);
            }
        }
        syn::UseTree::Glob(_) => scope.globs.push(prefix.clone()),
    }
}

fn path_segments(path: &syn::Path) -> Vec<String> {
    path.segments.iter().map(|segment| segment.ident.to_string()).collect()
}

fn as_strs(segments: &[String]) -> Vec<&str> {
    segments.iter().map(String::as_str).collect()
}

/// What a path names once every binding along it is followed.
#[derive(Clone)]
enum Identity {
    Command(CommandKind),
    ProcessModule(CommandKind),
    CrateRoot(CommandKind),
    Helper,
    HelperModule,
    /// A module of the scanned crate; resolution continues inside it.
    Module(Location),
}

/// A module of the scanned crate: its file's scope tree and the inline `mod`
/// chain down to it.
#[derive(Clone)]
struct Location {
    /// `None` for a source scanned without a crate around it, which is then
    /// its own crate root and has no parent.
    file: Option<PathBuf>,
    root: Rc<ModuleScope>,
    inline_path: Vec<String>,
}

impl Location {
    fn scope(&self) -> Option<&ModuleScope> {
        self.inline_path
            .iter()
            .try_fold(&*self.root, |scope, name| scope.inline.get(name))
    }

    fn child(&self, name: &str) -> Self {
        let mut inline_path = self.inline_path.clone();
        inline_path.push(name.to_string());
        Self {
            file: self.file.clone(),
            root: Rc::clone(&self.root),
            inline_path,
        }
    }

    fn key(&self) -> LocationKey {
        (self.file.clone(), self.inline_path.clone())
    }

    fn module_contexts(&self) -> Vec<Self> {
        fn collect(location: &Location, contexts: &mut Vec<Location>) {
            contexts.push(location.clone());
            let children = location
                .scope()
                .into_iter()
                .flat_map(|scope| scope.inline.keys().cloned())
                .collect::<Vec<_>>();
            for child in children {
                collect(&location.child(&child), contexts);
            }
        }

        let mut contexts = Vec::new();
        collect(self, &mut contexts);
        contexts
    }
}

type LocationKey = (Option<PathBuf>, Vec<String>);

#[derive(Clone)]
struct ExpansionContext<'a> {
    resolver: &'a ModuleResolver,
    location: Location,
}

/// One binding step on the resolution stack: where it was taken, the name
/// being looked up, and the path being followed for it.
type BindingKey = (LocationKey, String, Vec<String>);

/// A path and where it was written.
type ResolutionKey = (LocationKey, Vec<String>);

/// The state of one top-level resolution.
#[derive(Default)]
struct Resolution {
    /// `use`, `type`, and glob-path steps in progress. A step is popped on
    /// return: the same binding may legitimately serve twice on one resolution
    /// (`pub use crate::api::inner::Cmd` written inside `api/mod.rs`), and only
    /// re-entering a step still in progress is a cycle.
    active: BTreeSet<BindingKey>,
    /// Modules already searched for a name through a glob. A glob keeps the
    /// name, so a second visit can find nothing the first did not — and without
    /// this a parent that globs six children, each importing `super::*`, is
    /// re-entered once per ordering of the siblings.
    globbed: BTreeSet<(LocationKey, String)>,
}

/// Resolves paths to the items they name by following the scanned crate's
/// bindings. A name is what its module's `use` or `type` bound it to, an
/// inline or file-backed submodule, or — failing those — what a glob import
/// brings in from the module it names. A re-export chain of any length
/// resolves in either declaration order; a binding that leads back to itself
/// resolves to nothing.
///
/// Visibility is deliberately ignored, toward censusing more: a private `use`
/// reached through `pub use child::*` counts. A path that leaves the crate —
/// `ratatui::widgets::*`, `clap::Command` — resolves to nothing.
struct ModuleResolver {
    /// The crate's `src`; `None` when the scanned source stands alone.
    src_root: Option<PathBuf>,
    /// Scope trees per file; `None` records a file that failed to parse.
    files: RefCell<BTreeMap<PathBuf, Option<Rc<ModuleScope>>>>,
    child_files: RefCell<BTreeMap<(PathBuf, String), Option<PathBuf>>>,
    /// Top-level resolutions start from an empty [`Resolution`], so their
    /// results depend on nothing but the path and where it was written.
    resolved: RefCell<BTreeMap<ResolutionKey, Option<Identity>>>,
}

impl ModuleResolver {
    fn new(src_root: Option<PathBuf>) -> Self {
        Self {
            src_root,
            files: RefCell::new(BTreeMap::new()),
            child_files: RefCell::new(BTreeMap::new()),
            resolved: RefCell::new(BTreeMap::new()),
        }
    }

    /// The location of a file the caller already parsed, registered so a path
    /// leading back to it agrees with the source in hand.
    fn location_of(&self, file: Option<&Path>, parsed: &syn::File) -> Location {
        let root = Rc::new(ModuleScope::of_file(parsed));
        if let Some(file) = file {
            self.files
                .borrow_mut()
                .insert(file.to_path_buf(), Some(Rc::clone(&root)));
        }
        Location {
            file: file.map(Path::to_path_buf),
            root,
            inline_path: Vec::new(),
        }
    }

    fn file_location(&self, file: PathBuf) -> Option<Location> {
        let cached = self.files.borrow().get(&file).cloned();
        let root = match cached {
            Some(root) => root,
            None => {
                let root = fs::read_to_string(&file)
                    .ok()
                    .and_then(|source| syn::parse_file(&source).ok())
                    .map(|parsed| Rc::new(ModuleScope::of_file(&parsed)));
                self.files.borrow_mut().insert(file.clone(), root.clone());
                root
            }
        }?;
        Some(Location {
            file: Some(file),
            root,
            inline_path: Vec::new(),
        })
    }

    fn resolve(&self, at: &Location, segments: &[&str]) -> Option<Identity> {
        let key = (at.key(), segments.iter().map(ToString::to_string).collect());
        if let Some(found) = self.resolved.borrow().get(&key) {
            return found.clone();
        }
        let found = self.resolve_within(at, segments, &mut Resolution::default());
        self.resolved.borrow_mut().insert(key, found.clone());
        found
    }

    fn resolve_within(&self, at: &Location, segments: &[&str], resolution: &mut Resolution) -> Option<Identity> {
        let (first, rest) = segments.split_first()?;
        let head = match *first {
            "std" => Identity::CrateRoot(CommandKind::Std),
            "tokio" => Identity::CrateRoot(CommandKind::Tokio),
            "self" => Identity::Module(at.clone()),
            "super" => Identity::Module(self.parent(at)?),
            // The helper's module is named by its spelling from either root,
            // ahead of any module a scanned crate might place at that path.
            root if HELPER_ROOTS.contains(&root) && rest.first() == Some(&HELPER_MODULE) => {
                return self.descend(Identity::HelperModule, &rest[1..], resolution);
            }
            "crate" => Identity::Module(self.crate_root(at)?),
            name => self.lookup(at, name, resolution)?,
        };
        self.descend(head, rest, resolution)
    }

    fn descend(&self, head: Identity, rest: &[&str], resolution: &mut Resolution) -> Option<Identity> {
        if rest.is_empty() {
            return Some(head);
        }
        match head {
            Identity::CrateRoot(kind) => match rest {
                ["process"] => Some(Identity::ProcessModule(kind)),
                ["process", "Command"] => Some(Identity::Command(kind)),
                _ => None,
            },
            Identity::ProcessModule(kind) => (rest == ["Command"]).then_some(Identity::Command(kind)),
            Identity::HelperModule => (rest == [HELPER]).then_some(Identity::Helper),
            Identity::Module(location) => self.resolve_within(&location, rest, resolution),
            Identity::Command(_) | Identity::Helper => None,
        }
    }

    fn lookup(&self, at: &Location, name: &str, resolution: &mut Resolution) -> Option<Identity> {
        let scope = at.scope()?;
        if scope.inline.contains_key(name) {
            return Some(Identity::Module(at.child(name)));
        }
        for (bound, target) in &scope.uses {
            if bound != name {
                continue;
            }
            let found = self.follow(at, name, target, resolution);
            if found.is_some() {
                return found;
            }
        }
        // A `type` alias shadows a glob-inherited name; one that does not name
        // the command names nothing this census tracks.
        if let Some((_, target)) = scope.type_aliases.iter().find(|(bound, _)| bound == name) {
            return match self.follow(at, name, target, resolution) {
                Some(Identity::Command(kind)) => Some(Identity::Command(kind)),
                _ => None,
            };
        }
        if scope.declared.contains(name)
            && let Some(file) = self.child_file(at, name)
        {
            return self.file_location(file).map(Identity::Module);
        }
        for glob in &scope.globs {
            // Resolving the glob's own path can fall back to this same glob
            // (`use ratatui::widgets::*` looks `ratatui` up first), so the step
            // goes on the stack like a `use` step.
            let key = (at.key(), name.to_string(), glob.clone());
            if !resolution.active.insert(key.clone()) {
                continue;
            }
            let mut found = None;
            if let Some(Identity::Module(target)) = self.resolve_within(at, &as_strs(glob), resolution)
                && resolution.globbed.insert((target.key(), name.to_string()))
            {
                found = self.lookup(&target, name, resolution);
            }
            resolution.active.remove(&key);
            if found.is_some() {
                return found;
            }
        }
        None
    }

    /// Resolves the path a `use` or `type` bound `name` to, unless that step is
    /// already in progress — which is what a binding cycle looks like.
    fn follow(&self, at: &Location, name: &str, target: &[String], resolution: &mut Resolution) -> Option<Identity> {
        let key = (at.key(), name.to_string(), target.to_vec());
        if !resolution.active.insert(key.clone()) {
            return None;
        }
        let found = self.resolve_within(at, &as_strs(target), resolution);
        resolution.active.remove(&key);
        found
    }

    fn parent(&self, at: &Location) -> Option<Location> {
        if !at.inline_path.is_empty() {
            let mut parent = at.clone();
            parent.inline_path.pop();
            return Some(parent);
        }
        let file = at.file.as_deref()?;
        self.file_location(self.parent_module_file(file)?)
    }

    fn crate_root(&self, at: &Location) -> Option<Location> {
        match self.src_root {
            None => Some(Location {
                file: at.file.clone(),
                root: Rc::clone(&at.root),
                inline_path: Vec::new(),
            }),
            Some(_) => self.file_location(self.crate_root_file()?),
        }
    }

    /// `mod x;` inside an inline module maps to a nested directory this census
    /// does not model, so only a file-level module has file-backed children.
    fn child_file(&self, at: &Location, name: &str) -> Option<PathBuf> {
        if !at.inline_path.is_empty() {
            return None;
        }
        let file = at.file.as_ref()?;
        let key = (file.clone(), name.to_string());
        if let Some(cached) = self.child_files.borrow().get(&key) {
            return cached.clone();
        }
        let found = child_module_file(file, name);
        self.child_files.borrow_mut().insert(key, found.clone());
        found
    }

    fn parent_module_file(&self, file_path: &Path) -> Option<PathBuf> {
        let src_root = self.src_root.as_deref()?;
        let directory = file_path.parent()?;
        let parent_directory = if is_module_root(file_path) {
            directory.parent()?
        } else {
            directory
        };
        if parent_directory == src_root {
            return self.crate_root_file();
        }
        let name = parent_directory.file_name()?.to_str()?;
        first_existing([
            parent_directory.join("mod.rs"),
            parent_directory.parent()?.join(format!("{name}.rs")),
        ])
    }

    fn crate_root_file(&self) -> Option<PathBuf> {
        let src_root = self.src_root.as_deref()?;
        first_existing([src_root.join("lib.rs"), src_root.join("main.rs")])
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

/// A function item a call path or a local binding can name.
#[derive(Clone, Copy)]
enum FunctionItem {
    Constructor(CommandKind),
    Helper,
}

/// The names one function body is read against: the module it sits in, and
/// the file's shadowing rules for the helper.
struct NameScope<'a> {
    resolver: &'a ModuleResolver,
    location: Location,
    macro_definition: Option<&'a ExpansionContext<'a>>,
    helper_scope: &'a BareHelperScope,
}

impl NameScope<'_> {
    fn resolve(&self, segments: &[&str]) -> Option<Identity> {
        if let [MACRO_DEFINITION_CRATE, rest @ ..] = segments {
            let definition = self.macro_definition?;
            let definition_path = std::iter::once("crate")
                .chain(rest.iter().copied())
                .collect::<Vec<_>>();
            definition
                .resolver
                .resolve(&definition.location, &definition_path)
        } else {
            self.resolver.resolve(&self.location, segments)
        }
    }

    fn function_item(&self, path: &syn::Path) -> Option<FunctionItem> {
        if let Some(kind) = self.constructor_kind(path) {
            return Some(FunctionItem::Constructor(kind));
        }
        self.is_helper(path).then_some(FunctionItem::Helper)
    }

    fn constructor_kind(&self, path: &syn::Path) -> Option<CommandKind> {
        let segments = path_segments(path);
        let segments = as_strs(&segments);
        let [type_path @ .., "new"] = segments.as_slice() else {
            return None;
        };
        match self.resolve(type_path)? {
            Identity::Command(kind) => Some(kind),
            _ => None,
        }
    }

    /// Resolves a call path against the *item* [`HELPER`] names, not against
    /// its terminal segment, after the file's own shadowing rules: a local
    /// function or module wins over an inherited binding of the same name.
    fn is_helper(&self, path: &syn::Path) -> bool {
        let segments = path_segments(path);
        let segments = as_strs(&segments);
        match segments.as_slice() {
            [name] if self.helper_scope.definition_site && *name == HELPER => return true,
            [name] if self.helper_scope.local_functions.contains(*name) => return false,
            [module, _] if self.helper_scope.local_modules.contains(*module) => return false,
            _ => {}
        }
        matches!(
            self.resolve(&segments),
            Some(Identity::Helper)
        )
    }

    /// The command kind a type path names, through any alias.
    fn command_type(&self, ty: &syn::Type) -> Option<CommandKind> {
        let syn::Type::Path(path) = ty else {
            return None;
        };
        if path.qself.is_some() {
            return None;
        }
        let segments = path_segments(&path.path);
        match self.resolve(&as_strs(&segments))? {
            Identity::Command(kind) => Some(kind),
            _ => None,
        }
    }

    /// The command kind a parameter of type `ty` constructs when called: a
    /// callable — `impl Fn(..) -> T`, `fn(..) -> T`, `Box<dyn Fn..(..) -> T>`,
    /// `&dyn Fn..(..) -> T`, or a generic parameter bound that way — whose
    /// return type names the command.
    fn constructor_parameter(&self, ty: &syn::Type, generics: &syn::Generics) -> Option<CommandKind> {
        match ty {
            syn::Type::Reference(reference) => self.constructor_parameter(&reference.elem, generics),
            syn::Type::Paren(inner) => self.constructor_parameter(&inner.elem, generics),
            syn::Type::BareFn(pointer) => self.callable_output(&pointer.output),
            syn::Type::ImplTrait(bounds) => self.callable_bound(bounds.bounds.iter()),
            syn::Type::TraitObject(bounds) => self.callable_bound(bounds.bounds.iter()),
            syn::Type::Path(path) if path.qself.is_none() => {
                if let Some(ident) = path.path.get_ident() {
                    return self.callable_bound(generic_bounds(generics, ident));
                }
                let last = path.path.segments.last()?;
                if last.ident != "Box" {
                    return None;
                }
                let syn::PathArguments::AngleBracketed(arguments) = &last.arguments else {
                    return None;
                };
                match arguments.args.first()? {
                    syn::GenericArgument::Type(inner) => self.constructor_parameter(inner, generics),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn callable_bound<'b>(&self, bounds: impl Iterator<Item = &'b syn::TypeParamBound>) -> Option<CommandKind> {
        bounds.filter_map(|bound| self.callable_trait(bound)).next()
    }

    fn callable_trait(&self, bound: &syn::TypeParamBound) -> Option<CommandKind> {
        let syn::TypeParamBound::Trait(bound) = bound else {
            return None;
        };
        let last = bound.path.segments.last()?;
        if !matches!(last.ident.to_string().as_str(), "Fn" | "FnMut" | "FnOnce") {
            return None;
        }
        let syn::PathArguments::Parenthesized(arguments) = &last.arguments else {
            return None;
        };
        self.callable_output(&arguments.output)
    }

    fn callable_output(&self, output: &syn::ReturnType) -> Option<CommandKind> {
        match output {
            syn::ReturnType::Type(_, ty) => self.command_type(ty),
            syn::ReturnType::Default => None,
        }
    }
}

/// Every bound written on generic parameter `name`, in the generics list and
/// the `where` clause.
fn generic_bounds<'g>(generics: &'g syn::Generics, name: &'g syn::Ident) -> impl Iterator<Item = &'g syn::TypeParamBound> {
    let inline = generics
        .type_params()
        .filter(move |param| &param.ident == name)
        .flat_map(|param| param.bounds.iter());
    let predicates = generics
        .where_clause
        .iter()
        .flat_map(|clause| clause.predicates.iter())
        .filter_map(move |predicate| match predicate {
            syn::WherePredicate::Type(bounded) => match &bounded.bounded_ty {
                syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident(name) => Some(&bounded.bounds),
                _ => None,
            },
            _ => None,
        })
        .flat_map(|bounds| bounds.iter());
    inline.chain(predicates)
}

/// Index into [`FunctionAnalyzer::commands`]; one per construction site.
type CommandId = usize;

struct CommandRecord {
    line: usize,
    kind: CommandKind,
    /// The helper was applied to this command somewhere.
    helped: bool,
    /// This command reached one of its own execution seams unhelped. No caller
    /// can govern a child this function already started.
    executed_unhelped: bool,
    /// This command left the function unhelped. Only this is a hand-off an
    /// allowlisted governor may describe.
    escaped_unhelped: bool,
}

/// Why a command has to carry the helper at the point it reaches.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Obligation {
    /// It reached `spawn`/`status`/`output`/`exec` on itself.
    Execution,
    /// It left local control: returned, in tail position, or handed to code
    /// this analysis does not follow.
    Escape,
}

/// Path-sensitive facts at one point in a function body.
#[derive(Clone, Default)]
struct FlowState {
    /// The path cannot reach any following statement (`return`, `break`, ...).
    diverged: bool,
    /// Local bindings that may hold a tracked command. A binding holds a *set*
    /// because branches merge.
    bindings: BTreeMap<String, BTreeSet<CommandId>>,
    /// Local bindings holding a constructor function item
    /// (`let construct = Command::new;`).
    constructors: BTreeMap<String, CommandKind>,
    /// Local bindings holding the helper function item.
    helpers: BTreeSet<String>,
    /// Commands constructed on this path.
    live: BTreeSet<CommandId>,
    /// Commands the helper has been applied to on this path.
    helped: BTreeSet<CommandId>,
}

impl FlowState {
    /// Rebinding a name drops whatever it held before, so a constructor or
    /// helper alias does not outlive the value that replaced it.
    fn bind(&mut self, name: String, ids: BTreeSet<CommandId>, item: Option<FunctionItem>) {
        self.constructors.remove(&name);
        self.helpers.remove(&name);
        match item {
            Some(FunctionItem::Constructor(kind)) => {
                self.constructors.insert(name.clone(), kind);
            }
            Some(FunctionItem::Helper) => {
                self.helpers.insert(name.clone());
            }
            None => {}
        }
        self.bindings.insert(name, ids);
    }

    /// Merge two branches pessimistically: a binding may hold whatever either
    /// branch put in it, and a command survives as helped only when every
    /// branch that *constructed* it also helped it. A branch that never saw the
    /// command abstains rather than voting it unhelped. The two function-item
    /// maps fail closed in opposite directions for the same reason: a
    /// constructor bound on either branch still constructs, while a helper
    /// governs only if every branch bound it.
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
        let mut constructors = self.constructors;
        constructors.extend(other.constructors);
        let helpers = self.helpers.intersection(&other.helpers).cloned().collect();
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
            constructors,
            helpers,
            live: self.live.union(&other.live).copied().collect(),
            helped,
        }
    }
}

/// Flow-ordered walk of one function body, tracking each constructed command
/// through the bindings that hold it.
struct FunctionAnalyzer<'a> {
    scope: &'a NameScope<'a>,
    commands: Vec<CommandRecord>,
}

impl<'a> FunctionAnalyzer<'a> {
    fn analyze(scope: &'a NameScope<'a>, signature: Option<&syn::Signature>, block: &syn::Block) -> Vec<CommandRecord> {
        let mut analyzer = Self {
            scope,
            commands: Vec::new(),
        };
        let mut state = FlowState::default();
        if let Some(signature) = signature {
            analyzer.seed_parameters(signature, &mut state);
        }
        // A command in the body's tail position is this function's return
        // value: it leaves local control exactly like an argument hand-off.
        let returned = analyzer.block(block, &mut state);
        analyzer.require_helped(&returned, &state, Obligation::Escape);
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

    /// A parameter typed as a callable returning the command is a constructor
    /// this function was handed; calling it constructs.
    fn seed_parameters(&self, signature: &syn::Signature, state: &mut FlowState) {
        for input in &signature.inputs {
            let syn::FnArg::Typed(typed) = input else {
                continue;
            };
            let Some(name) = binding_name(&typed.pat) else {
                continue;
            };
            if let Some(kind) = self.scope.constructor_parameter(&typed.ty, &signature.generics) {
                state.constructors.insert(name, kind);
            }
        }
    }

    fn local(&mut self, local: &syn::Local, state: &mut FlowState) {
        let name = binding_name(&local.pat);
        let mut ids = BTreeSet::new();
        let mut item = None;
        if let Some(init) = &local.init {
            // A function item bound to a plain name is followed as a binding;
            // evaluated as an expression it would count as leaving the function.
            item = name.as_ref().and_then(|_| self.function_item_value(&init.expr, state));
            if item.is_none() {
                ids = self.expr(&init.expr, state);
            }
            if let Some((_, diverge)) = &init.diverge {
                let mut unmatched = state.clone();
                self.expr(diverge, &mut unmatched);
            }
        }
        match name {
            Some(name) => state.bind(name, ids, item),
            // Destructured into something this analysis cannot follow.
            None => self.require_helped(&ids, state, Obligation::Escape),
        }
    }

    fn expr(&mut self, expr: &syn::Expr, state: &mut FlowState) -> BTreeSet<CommandId> {
        match expr {
            syn::Expr::Path(path) => {
                // Reached only when the path is a value, not a callee or a
                // plain-name initializer: the constructor leaves this function
                // and whatever it builds is out of reach.
                if path.qself.is_none()
                    && let Some(FunctionItem::Constructor(kind)) = self.function_item(&path.path, state)
                {
                    self.commands.push(CommandRecord {
                        line: path.span().start().line,
                        kind,
                        helped: false,
                        executed_unhelped: false,
                        escaped_unhelped: true,
                    });
                    return BTreeSet::new();
                }
                path.path
                    .get_ident()
                    .and_then(|ident| state.bindings.get(&ident.to_string()).cloned())
                    .unwrap_or_default()
            }
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
                match binding_name_of_expr(&assign.left) {
                    Some(name) => {
                        let item = self.function_item_value(&assign.right, state);
                        let ids = if item.is_some() {
                            BTreeSet::new()
                        } else {
                            self.expr(&assign.right, state)
                        };
                        state.bind(name, ids, item);
                    }
                    None => {
                        let ids = self.expr(&assign.right, state);
                        self.require_helped(&ids, state, Obligation::Escape);
                    }
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
        if let syn::Expr::Path(function) = strip_groups(call.func.as_ref()) {
            match self.function_item(&function.path, state) {
                Some(FunctionItem::Constructor(kind)) => {
                    for arg in &call.args {
                        self.argument(arg, state);
                    }
                    let id = self.commands.len();
                    self.commands.push(CommandRecord {
                        line: call.span().start().line,
                        kind,
                        helped: false,
                        executed_unhelped: false,
                        escaped_unhelped: false,
                    });
                    state.live.insert(id);
                    return BTreeSet::from([id]);
                }
                Some(FunctionItem::Helper) => {
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
                None => {}
            }
        } else {
            self.expr(call.func.as_ref(), state);
        }
        for arg in &call.args {
            self.argument(arg, state);
        }
        BTreeSet::new()
    }

    /// The function item a path names on this path. A local binding shadows
    /// an import, so the flow state is consulted before the module's names.
    fn function_item(&self, path: &syn::Path, state: &FlowState) -> Option<FunctionItem> {
        if let Some(name) = path.get_ident().map(ToString::to_string) {
            if let Some(kind) = state.constructors.get(&name) {
                return Some(FunctionItem::Constructor(*kind));
            }
            if state.helpers.contains(&name) {
                return Some(FunctionItem::Helper);
            }
        }
        self.scope.function_item(path)
    }

    /// A bare path used as a value names a function item without calling it.
    fn function_item_value(&self, value: &syn::Expr, state: &FlowState) -> Option<FunctionItem> {
        match strip_groups(value) {
            syn::Expr::Path(path) if path.qself.is_none() => self.function_item(&path.path, state),
            _ => None,
        }
    }

    fn method_call(&mut self, call: &syn::ExprMethodCall, state: &mut FlowState) -> BTreeSet<CommandId> {
        let receiver = self.expr(&call.receiver, state);
        for arg in &call.args {
            self.argument(arg, state);
        }
        if EXECUTION_METHODS.contains(&call.method.to_string().as_str()) {
            self.require_helped(&receiver, state, Obligation::Execution);
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
            self.require_helped(&ids, state, Obligation::Escape);
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
                self.require_helped(&ids, state, Obligation::Escape);
            }
            return;
        }
        let ids = self.expr(arg, state);
        self.require_helped(&ids, state, Obligation::Escape);
    }

    /// Expression forms without dedicated handling: evaluate their direct
    /// children in source order and treat every command that reaches one as
    /// leaving local control.
    fn opaque(&mut self, expr: &syn::Expr, state: &mut FlowState) {
        for child in child_expressions(expr) {
            self.argument(child, state);
        }
    }

    /// Invocation arguments are read as expressions: a command among them is
    /// observed (`debug!`, `format!`), a constructor value among them is the
    /// hand-off record it is anywhere else. What the macro's own body does is
    /// censused at its definition, when that definition is in a scanned file.
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
    /// does not follow — must already carry the helper on this path. The two
    /// causes are recorded apart because only an escape can be discharged
    /// elsewhere; see [`indirect_governor`].
    fn require_helped(
        &mut self,
        ids: &BTreeSet<CommandId>,
        state: &FlowState,
        obligation: Obligation,
    ) {
        for id in ids {
            if state.helped.contains(id) {
                continue;
            }
            match obligation {
                Obligation::Execution => self.commands[*id].executed_unhelped = true,
                Obligation::Escape => self.commands[*id].escaped_unhelped = true,
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
    resolver: &'a ModuleResolver,
    /// The module being walked; inline `mod` items push onto it.
    location: Location,
    /// Modules in the definition crate whose bindings any local macro may
    /// inherit.
    local_expansion_contexts: &'a [ExpansionContext<'a>],
    /// Modules in either guarded crate whose bindings an exported macro may
    /// inherit.
    exported_expansion_contexts: &'a [ExpansionContext<'a>],
    /// The defining crate and module while scanning a macro transcriber.
    macro_definition: Option<ExpansionContext<'a>>,
    /// `macro!::` while walking a macro's expanded transcriber, so a function
    /// it emits is reported under the macro that emits it.
    function_prefix: String,
    sites: Vec<SpawnSite>,
}

impl FileScanner<'_> {
    fn scan_function(&mut self, name: String, signature: Option<&syn::Signature>, block: &syn::Block) {
        let name = format!("{}{name}", self.function_prefix);
        let indirect = indirect_governor(self.path, &name);
        let helper_scope = BareHelperScope::for_location(self.path, &self.location);
        let scope = NameScope {
            resolver: self.resolver,
            location: self.location.clone(),
            macro_definition: self.macro_definition.as_ref(),
            helper_scope: &helper_scope,
        };
        for command in FunctionAnalyzer::analyze(&scope, signature, block) {
            let proven = command.helped && !command.executed_unhelped && !command.escaped_unhelped;
            let governed_by = if proven {
                HELPER.to_string()
            } else if let Some(governor) = indirect.filter(|_| !command.executed_unhelped) {
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

    /// Censuses every transcriber of a `macro_rules!` definition against every
    /// module context where it can be invoked. Exported macros include both
    /// guarded crates; `$crate` continues to use the definition context.
    fn scan_macro_rules(&mut self, name: &str, definition: &syn::Macro, exported: bool) {
        let path = self.path;
        let unanalyzable = move |what: &str| -> ! {
            panic!(
                "{path}: macro_rules! {name} {what}; its transcriber could not be analyzed and must be simplified or given a syntactically analyzable shape (a block, statements, or items)"
            )
        };
        let Some(transcribers) = macro_rules_transcribers(definition.tokens.clone()) else {
            unanalyzable("does not split into `(matcher) => {transcriber}` rules")
        };
        let prefix = format!("{}{name}!::", self.function_prefix);
        let contexts = if exported {
            self.exported_expansion_contexts.to_vec()
        } else {
            self.local_expansion_contexts.to_vec()
        };
        for transcriber in transcribers {
            let body = rewrite_transcriber(transcriber);
            let braced = TokenStream::from(TokenTree::Group(Group::new(Delimiter::Brace, body.clone())));
            let block = syn::parse2::<syn::Block>(braced).ok();
            let file = block
                .is_none()
                .then(|| syn::parse2::<syn::File>(body))
                .transpose()
                .unwrap_or_else(|_| unanalyzable("expands to neither a block nor items"));
            let definition = ExpansionContext {
                resolver: self.resolver,
                location: self.location.clone(),
            };
            for context in &contexts {
                self.resolver = context.resolver;
                self.location = context.location.clone();
                self.macro_definition = Some(definition.clone());
                if let Some(block) = &block {
                    self.scan_function(format!("{name}!"), None, block);
                    let outer = std::mem::replace(&mut self.function_prefix, prefix.clone());
                    visit::visit_block(self, block);
                    self.function_prefix = outer;
                } else if let Some(file) = &file {
                    let outer = std::mem::replace(&mut self.function_prefix, prefix.clone());
                    self.visit_file(file);
                    self.function_prefix = outer;
                }
            }
            self.resolver = definition.resolver;
            self.location = definition.location;
            self.macro_definition = None;
        }
    }
}

/// The transcriber token streams of a `macro_rules!` body, one per rule.
fn macro_rules_transcribers(tokens: TokenStream) -> Option<Vec<TokenStream>> {
    let mut rules = Vec::new();
    let mut tokens = tokens.into_iter().peekable();
    while let Some(matcher) = tokens.next() {
        let TokenTree::Group(_) = matcher else {
            return None;
        };
        match (tokens.next(), tokens.next()) {
            (Some(TokenTree::Punct(eq)), Some(TokenTree::Punct(gt))) if eq.as_char() == '=' && gt.as_char() == '>' => {}
            _ => return None,
        }
        let Some(TokenTree::Group(transcriber)) = tokens.next() else {
            return None;
        };
        rules.push(transcriber.stream());
        if matches!(tokens.peek(), Some(TokenTree::Punct(semicolon)) if semicolon.as_char() == ';') {
            tokens.next();
        }
    }
    Some(rules)
}

/// A transcriber as analyzable Rust: `$name` becomes the placeholder
/// `__meta_name`, `$crate` becomes a definition-context marker, and a repetition
/// `$( ... ) sep? op` is emitted once without its separator and operator.
/// Spans are kept, so a record inside the expansion carries the definition's
/// own line.
fn rewrite_transcriber(tokens: TokenStream) -> TokenStream {
    let mut rewritten = Vec::new();
    let mut tokens = tokens.into_iter().peekable();
    while let Some(token) = tokens.next() {
        match token {
            TokenTree::Punct(dollar) if dollar.as_char() == '$' => match tokens.peek() {
                Some(TokenTree::Ident(name)) => {
                    let placeholder = if name == "crate" {
                        Ident::new(MACRO_DEFINITION_CRATE, name.span())
                    } else {
                        Ident::new(&format!("__meta_{name}"), name.span())
                    };
                    rewritten.push(TokenTree::Ident(placeholder));
                    tokens.next();
                }
                Some(TokenTree::Group(repetition)) if repetition.delimiter() == Delimiter::Parenthesis => {
                    rewritten.extend(rewrite_transcriber(repetition.stream()));
                    tokens.next();
                    match tokens.next() {
                        Some(TokenTree::Punct(op)) if matches!(op.as_char(), '*' | '+' | '?') => {}
                        // A separator precedes the operator.
                        Some(_) => {
                            tokens.next();
                        }
                        None => {}
                    }
                }
                _ => rewritten.push(TokenTree::Punct(dollar)),
            },
            TokenTree::Group(group) => {
                let mut inner = Group::new(group.delimiter(), rewrite_transcriber(group.stream()));
                inner.set_span(group.span());
                rewritten.push(TokenTree::Group(inner));
            }
            other => rewritten.push(other),
        }
    }
    rewritten.into_iter().collect()
}

impl<'ast> Visit<'ast> for FileScanner<'_> {
    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        if cfg_test(&item.attrs) {
            return;
        }
        if let Some(name) = &item.ident
            && item.mac.path.is_ident("macro_rules")
        {
            let exported = production_macro_export(&item.attrs);
            self.scan_macro_rules(&name.to_string(), &item.mac, exported);
        }
        visit::visit_item_macro(self, item);
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if cfg_test(&item.attrs) {
            return;
        }
        if item.content.is_none() {
            return;
        }
        self.location.inline_path.push(item.ident.to_string());
        visit::visit_item_mod(self, item);
        self.location.inline_path.pop();
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if cfg_test(&item.attrs) {
            return;
        }
        self.scan_function(item.sig.ident.to_string(), Some(&item.sig), &item.block);
        // Recurse so a nested item is analyzed under its own name.
        visit::visit_item_fn(self, item);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        if cfg_test(&item.attrs) {
            return;
        }
        self.scan_function(item.sig.ident.to_string(), Some(&item.sig), &item.block);
        visit::visit_impl_item_fn(self, item);
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        if cfg_test(&item.attrs) {
            return;
        }
        if let Some(default) = &item.default {
            self.scan_function(item.sig.ident.to_string(), Some(&item.sig), default);
        }
        visit::visit_trait_item_fn(self, item);
    }
}

/// An item is test-only when a `cfg` predicate on it is proven false in a
/// production build — not when the predicate merely mentions `test`.
fn cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("cfg"))
        .any(|attr| {
            attr.parse_args::<syn::Meta>()
                .is_ok_and(|predicate| production_truth(&predicate) == CfgTruth::False)
        })
}

fn production_macro_export(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| meta_applies_macro_export(&attr.meta))
}

fn meta_applies_macro_export(meta: &syn::Meta) -> bool {
    if meta.path().is_ident("macro_export") {
        return true;
    }
    let syn::Meta::List(list) = meta else {
        return false;
    };
    if !list.path.is_ident("cfg_attr") {
        return false;
    }
    use syn::parse::Parser;
    let parser = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated;
    let Ok(arguments) = parser.parse2(list.tokens.clone()) else {
        return false;
    };
    let mut arguments = arguments.iter();
    let Some(predicate) = arguments.next() else {
        return false;
    };
    production_truth(predicate) != CfgTruth::False && arguments.any(meta_applies_macro_export)
}

/// A `cfg` predicate's value in a production build. Only `test` is known;
/// every other leaf — `unix`, `feature = "..."` — is unknown, and unknown is
/// kept in the census.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CfgTruth {
    True,
    False,
    Unknown,
}

fn production_truth(predicate: &syn::Meta) -> CfgTruth {
    let syn::Meta::List(list) = predicate else {
        return if predicate.path().is_ident("test") {
            CfgTruth::False
        } else {
            CfgTruth::Unknown
        };
    };
    use syn::parse::Parser;
    let parser = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated;
    let Ok(operands) = parser.parse2(list.tokens.clone()) else {
        return CfgTruth::Unknown;
    };
    let operands: Vec<CfgTruth> = operands.iter().map(production_truth).collect();
    if list.path.is_ident("not") {
        match operands.as_slice() {
            [CfgTruth::True] => CfgTruth::False,
            [CfgTruth::False] => CfgTruth::True,
            _ => CfgTruth::Unknown,
        }
    } else if list.path.is_ident("all") {
        if operands.contains(&CfgTruth::False) {
            CfgTruth::False
        } else if operands.iter().all(|operand| *operand == CfgTruth::True) {
            CfgTruth::True
        } else {
            CfgTruth::Unknown
        }
    } else if list.path.is_ident("any") {
        if operands.contains(&CfgTruth::True) {
            CfgTruth::True
        } else if operands.iter().all(|operand| *operand == CfgTruth::False) {
            CfgTruth::False
        } else {
            CfgTruth::Unknown
        }
    } else {
        CfgTruth::Unknown
    }
}

/// Factories whose product is governed by the caller that executes it.
///
/// This describes a hand-off and nothing else: it applies only to a command
/// that leaves the factory without reaching an execution seam inside it. A
/// child the factory starts itself is one the named caller never sees, so it
/// stays `UNCONTROLLED`. Per the spec an allowlist may describe an indirect
/// governed path; it may not exempt a spawned child from `AGENT_CWD`.
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

/// Scan one source with only the bindings it writes itself — no filesystem.
fn scan_source(path: &str, source: &str) -> Vec<SpawnSite> {
    let file = parse_source(path, source);
    let resolver = ModuleResolver::new(None);
    let location = resolver.location_of(None, &file);
    let contexts = location
        .module_contexts()
        .into_iter()
        .map(|location| ExpansionContext {
            resolver: &resolver,
            location,
        })
        .collect::<Vec<_>>();
    scan_parsed(path, &file, &resolver, location, &contexts, &contexts)
}

/// Scan one source with the bindings its imports reach through the crate.
fn scan_crate_file(
    display_path: &str,
    file_path: &Path,
    source: &str,
    resolver: &ModuleResolver,
) -> Vec<SpawnSite> {
    let file = parse_source(display_path, source);
    let location = resolver.location_of(Some(file_path), &file);
    let contexts = location
        .module_contexts()
        .into_iter()
        .map(|location| ExpansionContext { resolver, location })
        .collect::<Vec<_>>();
    scan_parsed(display_path, &file, resolver, location, &contexts, &contexts)
}

fn scan_crate_file_with_contexts(
    display_path: &str,
    file_path: &Path,
    source: &str,
    resolver: &ModuleResolver,
    local_expansion_contexts: &[ExpansionContext<'_>],
    exported_expansion_contexts: &[ExpansionContext<'_>],
) -> Vec<SpawnSite> {
    let file = parse_source(display_path, source);
    let location = resolver.location_of(Some(file_path), &file);
    scan_parsed(
        display_path,
        &file,
        resolver,
        location,
        local_expansion_contexts,
        exported_expansion_contexts,
    )
}

fn parse_source(path: &str, source: &str) -> syn::File {
    syn::parse_file(source).unwrap_or_else(|error| panic!("failed to parse {path}: {error}"))
}

fn scan_parsed(
    path: &str,
    file: &syn::File,
    resolver: &ModuleResolver,
    location: Location,
    local_expansion_contexts: &[ExpansionContext<'_>],
    exported_expansion_contexts: &[ExpansionContext<'_>],
) -> Vec<SpawnSite> {
    let mut scanner = FileScanner {
        path,
        resolver,
        location,
        local_expansion_contexts,
        exported_expansion_contexts,
        macro_definition: None,
        function_prefix: String::new(),
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
    struct ScannedCrate {
        resolver: ModuleResolver,
        files: Vec<PathBuf>,
        contexts: Vec<Location>,
    }

    let root = workspace_root();
    let mut sites = Vec::new();
    let mut crates = SCANNED_ROOTS.map(|relative_root| {
        let src_root = root.join(relative_root);
        let files = production_files(&src_root);
        ScannedCrate {
            resolver: ModuleResolver::new(Some(src_root.clone())),
            files,
            contexts: Vec::new(),
        }
    });
    for scanned_crate in &mut crates {
        for file in &scanned_crate.files {
            let source = fs::read_to_string(file).unwrap();
            let parsed = parse_source(&file.display().to_string(), &source);
            scanned_crate
                .contexts
                .extend(scanned_crate.resolver.location_of(Some(file), &parsed).module_contexts());
        }
    }
    let exported_contexts = crates
        .iter()
        .flat_map(|scanned_crate| {
            scanned_crate
                .contexts
                .iter()
                .cloned()
                .map(|location| ExpansionContext {
                    resolver: &scanned_crate.resolver,
                    location,
                })
        })
        .collect::<Vec<_>>();
    for scanned_crate in &crates {
        let local_contexts = scanned_crate
            .contexts
            .iter()
            .cloned()
            .map(|location| ExpansionContext {
                resolver: &scanned_crate.resolver,
                location,
            })
            .collect::<Vec<_>>();
        for file in &scanned_crate.files {
            let relative = file.strip_prefix(root.join("claudine")).unwrap();
            let relative = relative.to_string_lossy().replace('\\', "/");
            let source = fs::read_to_string(file).unwrap();
            sites.extend(scan_crate_file_with_contexts(
                &relative,
                file,
                &source,
                &scanned_crate.resolver,
                &local_contexts,
                &exported_contexts,
            ));
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
/// Both bindings travel that hop: the constructor's and the helper's.
#[test]
fn a_glob_import_inherits_the_command_and_helper_bindings() {
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

    let resolver = ModuleResolver::new(Some(src));
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

    let resolver = ModuleResolver::new(Some(src));
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
    fs::write(src.join("lib.rs"), "mod prelude;\nmod shadow;\n").expect("crate root");
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

    let resolver = ModuleResolver::new(Some(src));
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

/// A `cfg` predicate excludes an item only when it is *proven* test-only.
/// `not(test)` is the production-only spelling, and `any(test, unix)` is live
/// production code on Unix; a scanner that skips any predicate mentioning
/// `test` omits both children from the census.
#[test]
fn a_cfg_predicate_excludes_only_items_proven_test_only() {
    let source = r#"
use claudine::child_environment::contribute_child_environment;
use std::process::Command;

#[cfg(not(test))]
fn governed_production_only() { let mut c = Command::new("x"); contribute_child_environment(&mut c); c.status(); }
#[cfg(not(test))]
fn ungoverned_production_only() { Command::new("x").status(); }
#[cfg(any(test, unix))]
fn ungoverned_on_unix() { Command::new("x").spawn(); }
#[cfg(all(not(test), feature = "daemon"))]
fn ungoverned_behind_feature() { Command::new("x").output(); }
#[cfg(not(test))]
mod production_only { fn ungoverned_in_module() { std::process::Command::new("x").spawn(); } }

#[cfg(all(test, unix))]
fn test_only_on_unix() { Command::new("x").spawn(); }
#[cfg(not(not(test)))]
fn doubly_negated_test_only() { Command::new("x").spawn(); }
#[cfg(test)]
fn plain_test_only() { Command::new("x").spawn(); }
#[cfg(test)]
mod tests { fn ignored() { std::process::Command::new("x").spawn(); } }
"#;
    let sites = scan_source("fixture.rs", source);
    let (governed, uncontrolled) = verdicts(&sites);
    assert_eq!(governed, BTreeSet::from(["governed_production_only"]));
    assert_eq!(
        uncontrolled,
        BTreeSet::from([
            "ungoverned_behind_feature",
            "ungoverned_in_module",
            "ungoverned_on_unix",
            "ungoverned_production_only",
        ])
    );
    assert_eq!(sites.len(), 5);
}

/// `type Cmd = std::process::Command;` is an aliased construction in AC6's
/// sense. Every spelling the alias target can take — the full path, an
/// imported `process` module, an imported `Command`, or another alias declared
/// later in the file — must reach the census, while `clap::Command` behind an
/// alias still must not.
#[test]
fn a_type_alias_still_names_the_process_command() {
    let source = r#"
use claudine::child_environment::contribute_child_environment;
use std::process;
use std::process::Command;

type Chained = ProcessCommand;
type ProcessCommand = std::process::Command;
type AsyncCommand = tokio::process::Command;
type ViaModule = process::Command;
type ViaImport = Command;
type CliCommand = clap::Command;

fn governed_std_alias() { let mut c = ProcessCommand::new("x"); contribute_child_environment(&mut c); c.status(); }
fn ungoverned_std_alias() { ProcessCommand::new("x").status(); }
fn governed_tokio_alias() { let mut c = AsyncCommand::new("x"); contribute_child_environment(&mut c); c.output(); }
fn ungoverned_tokio_alias() { AsyncCommand::new("x").output(); }
fn ungoverned_module_alias() { ViaModule::new("x").spawn(); }
fn ungoverned_import_alias() { ViaImport::new("x").spawn(); }
fn ungoverned_chained_alias() { Chained::new("x").spawn(); }
fn builds_a_cli() { CliCommand::new("claudine"); }
"#;
    let sites = scan_source("fixture.rs", source);
    assert_eq!(tally(&sites, "governed_std_alias"), (1, 0));
    assert_eq!(tally(&sites, "ungoverned_std_alias"), (0, 1));
    assert_eq!(tally(&sites, "governed_tokio_alias"), (1, 0));
    assert_eq!(tally(&sites, "ungoverned_tokio_alias"), (0, 1));
    assert_eq!(tally(&sites, "ungoverned_module_alias"), (0, 1));
    assert_eq!(tally(&sites, "ungoverned_import_alias"), (0, 1));
    assert_eq!(tally(&sites, "ungoverned_chained_alias"), (0, 1));
    assert_eq!(
        sites
            .iter()
            .filter(|site| site.function.ends_with("tokio_alias"))
            .map(|site| site.command_kind)
            .collect::<Vec<_>>(),
        vec!["tokio", "tokio"]
    );
    assert_eq!(sites.len(), 7);
}

/// A locally declared `mod` wins over a module a glob import brought in, just
/// as a local `fn` does, so a same-named local module cannot lend its function
/// the shared helper's identity. Both the inline and the `mod x;` declaration
/// form shadow.
#[test]
fn a_local_module_shadows_a_glob_inherited_helper_module() {
    let crate_dir = tempfile::tempdir().expect("temp crate");
    let src = crate_dir.path().join("src");
    fs::create_dir_all(&src).expect("temp crate layout");
    fs::write(src.join("lib.rs"), "mod prelude;\nmod inline;\nmod declared;\n").expect("crate root");
    fs::write(
        src.join("prelude.rs"),
        "pub use claudine::child_environment;\npub use std::process::Command;\n",
    )
    .expect("prelude module");

    let inline_path = src.join("inline.rs");
    let inline = r#"
use crate::prelude::*;

mod child_environment {
    pub fn contribute_child_environment<T>(_: &mut T) {}
}

fn shadowed_by_inline_module() {
    let mut c = Command::new("x");
    child_environment::contribute_child_environment(&mut c);
    c.spawn();
}

fn qualified_anyway() {
    let mut c = Command::new("x");
    crate::child_environment::contribute_child_environment(&mut c);
    c.spawn();
}
"#;
    fs::write(&inline_path, inline).expect("inline module");

    let declared_path = src.join("declared.rs");
    let declared = r#"
use crate::prelude::*;

mod child_environment;

fn shadowed_by_declaration() {
    let mut c = Command::new("x");
    child_environment::contribute_child_environment(&mut c);
    c.spawn();
}
"#;
    fs::write(&declared_path, declared).expect("declared module");

    let resolver = ModuleResolver::new(Some(src));
    let sites = scan_crate_file("inline.rs", &inline_path, inline, &resolver);
    assert_eq!(tally(&sites, "shadowed_by_inline_module"), (0, 1));
    assert_eq!(tally(&sites, "qualified_anyway"), (1, 0));
    let sites = scan_crate_file("declared.rs", &declared_path, declared, &resolver);
    assert_eq!(tally(&sites, "shadowed_by_declaration"), (0, 1));
}

/// A name imported from a local module that re-exports the type — under the
/// same name or another, through `self::`, or across three hops declared in
/// the order that needs the last one resolved first — is
/// `std::process::Command` all the same, and a re-exported helper is the
/// helper. `clap::Command` behind the same shape still resolves elsewhere,
/// and a local function re-exported the same way governs nothing.
#[test]
fn a_named_re_export_still_names_the_process_command() {
    let source = r#"
use claudine::child_environment::contribute_child_environment;
use process_api::ProcessCommand;
use process_api::ProcessCommand as Renamed;
use self::async_api::AsyncCommand as Async;
use hop_one::Outer;
use env_api::govern;
use env_api::govern as apply_environment;
use fake_env::contribute_environment as fake;
use cli_api::CliCommand;

mod process_api { pub use std::process::Command as ProcessCommand; }
mod async_api { pub use tokio::process::Command as AsyncCommand; }
mod hop_one { pub use super::hop_two::Middle as Outer; }
mod hop_two { pub use crate::hop_three::Inner as Middle; }
mod hop_three { pub use std::process::Command as Inner; }
mod env_api { pub use crate::child_environment::contribute_child_environment as govern; }
mod fake_env { pub fn contribute_environment<T>(_: &mut T) {} }
mod cli_api { pub use clap::Command as CliCommand; }

fn governed_same_name_re_export() { let mut c = ProcessCommand::new("x"); contribute_child_environment(&mut c); c.status(); }
fn ungoverned_same_name_re_export() { ProcessCommand::new("x").status(); }
fn governed_renamed_re_export() { let mut c = Renamed::new("x"); contribute_child_environment(&mut c); c.status(); }
fn ungoverned_renamed_re_export() { Renamed::new("x").status(); }
fn governed_self_qualified_re_export() { let mut c = Async::new("x"); contribute_child_environment(&mut c); c.output(); }
fn ungoverned_self_qualified_re_export() { Async::new("x").output(); }
fn governed_three_hop_re_export() { let mut c = Outer::new("x"); contribute_child_environment(&mut c); c.spawn(); }
fn ungoverned_three_hop_re_export() { Outer::new("x").spawn(); }
fn governed_by_re_exported_helper() { let mut c = Renamed::new("x"); govern(&mut c); c.spawn(); }
fn governed_by_renamed_re_exported_helper() { let mut c = Renamed::new("x"); apply_environment(&mut c); c.spawn(); }
fn ungoverned_by_a_re_exported_look_alike() { let mut c = Renamed::new("x"); fake(&mut c); c.spawn(); }
fn builds_a_cli() { CliCommand::new("claudine"); }
"#;
    let sites = scan_source("fixture.rs", source);
    assert_eq!(tally(&sites, "governed_same_name_re_export"), (1, 0));
    assert_eq!(tally(&sites, "ungoverned_same_name_re_export"), (0, 1));
    assert_eq!(tally(&sites, "governed_renamed_re_export"), (1, 0));
    assert_eq!(tally(&sites, "ungoverned_renamed_re_export"), (0, 1));
    assert_eq!(tally(&sites, "governed_self_qualified_re_export"), (1, 0));
    assert_eq!(tally(&sites, "ungoverned_self_qualified_re_export"), (0, 1));
    assert_eq!(tally(&sites, "governed_three_hop_re_export"), (1, 0));
    assert_eq!(tally(&sites, "ungoverned_three_hop_re_export"), (0, 1));
    assert_eq!(tally(&sites, "governed_by_re_exported_helper"), (1, 0));
    assert_eq!(tally(&sites, "governed_by_renamed_re_exported_helper"), (1, 0));
    assert_eq!(tally(&sites, "ungoverned_by_a_re_exported_look_alike"), (0, 1));
    assert_eq!(
        sites
            .iter()
            .filter(|site| site.function.ends_with("self_qualified_re_export"))
            .map(|site| site.command_kind)
            .collect::<Vec<_>>(),
        vec!["tokio", "tokio"]
    );
    assert_eq!(sites.len(), 11);
}

/// The same resolution through the crate's files: `mod process_api;` backed
/// by `process_api.rs`, a helper re-exported from `env_api.rs`, and a chain
/// that crosses three files through `self::` and `crate::`. Without the crate
/// around it the source binds none of these names.
#[test]
fn a_cross_file_re_export_resolves_through_the_crate() {
    let crate_dir = tempfile::tempdir().expect("temp crate");
    let src = crate_dir.path().join("src");
    fs::create_dir_all(src.join("api")).expect("temp crate layout");
    fs::write(src.join("process_api.rs"), "pub use std::process::Command as ProcessCommand;\n")
        .expect("process_api module");
    fs::write(
        src.join("env_api.rs"),
        "pub use claudine::child_environment::contribute_child_environment as govern;\n",
    )
    .expect("env_api module");
    fs::write(src.join("api/mod.rs"), "mod inner;\npub use self::inner::Cmd;\n").expect("api module");
    fs::write(src.join("api/inner.rs"), "pub use crate::process_api::ProcessCommand as Cmd;\n")
        .expect("api::inner module");

    let lib_path = src.join("lib.rs");
    let lib = r#"
use claudine::child_environment::contribute_child_environment;
mod process_api;
mod env_api;
mod api;
use process_api::ProcessCommand;
use env_api::govern;
use api::Cmd;

fn governed_cross_file() { let mut c = ProcessCommand::new("x"); contribute_child_environment(&mut c); c.status(); }
fn ungoverned_cross_file() { ProcessCommand::new("x").status(); }
fn governed_by_cross_file_helper() { let mut c = ProcessCommand::new("x"); govern(&mut c); c.spawn(); }
fn ungoverned_three_file_chain() { Cmd::new("x").spawn(); }
"#;
    fs::write(&lib_path, lib).expect("crate root");

    let resolver = ModuleResolver::new(Some(src));
    let sites = scan_crate_file("lib.rs", &lib_path, lib, &resolver);
    assert_eq!(tally(&sites, "governed_cross_file"), (1, 0));
    assert_eq!(tally(&sites, "ungoverned_cross_file"), (0, 1));
    assert_eq!(tally(&sites, "governed_by_cross_file_helper"), (1, 0));
    assert_eq!(tally(&sites, "ungoverned_three_file_chain"), (0, 1));
    assert_eq!(sites.len(), 4);
    assert!(scan_source("lib.rs", lib).is_empty());
}

/// `let construct = Command::new;` is an aliased construction in AC6's sense:
/// calling the binding constructs the command, in every spelling the
/// constructor path can take, and a binding holding the helper governs. A
/// name rebound before the call is whatever it was rebound to; a constructor
/// bound on one branch still constructs, a helper bound on one branch does
/// not govern; a binding of an unrelated function or of `clap::Command::new`
/// contributes nothing.
#[test]
fn a_constructor_held_in_a_binding_constructs_when_called() {
    let source = r#"
use claudine::child_environment::contribute_child_environment;
use std::process::Command;
use tokio::process::Command as TokioCommand;

fn governed_imported_item() { let construct = Command::new; let mut c = construct("x"); contribute_child_environment(&mut c); c.status(); }
fn ungoverned_imported_item() { let construct = Command::new; construct("x").status(); }
fn ungoverned_qualified_item() { let construct = std::process::Command::new; let mut c = construct("x"); c.status(); }
fn ungoverned_tokio_item() { let construct = TokioCommand::new; construct("x").output(); }
fn ungoverned_item_of_item() { let first = Command::new; let second = first; second("x").spawn(); }
fn governed_by_helper_item() { let govern = contribute_child_environment; let mut c = Command::new("x"); govern(&mut c); c.spawn(); }
fn rebound_before_use() { let construct = Command::new; let construct = unrelated; construct("x").status(); }
fn reassigned_before_use() { let mut construct = Command::new; construct = unrelated; construct("x").status(); }
fn constructed_on_one_branch(flag: bool) { let construct = Command::new; if flag { construct("x").status(); } }
fn bound_on_one_branch(flag: bool) {
    let construct;
    if flag { construct = Command::new; } else { construct = unrelated; }
    construct("x").status();
}
fn helper_bound_on_one_branch(flag: bool) {
    let govern;
    if flag { govern = contribute_child_environment; } else { govern = unrelated; }
    let mut c = Command::new("x");
    govern(&mut c);
    c.spawn();
}
fn aliases_an_unrelated_function() { let construct = unrelated; construct("x").status(); }
fn aliases_a_clap_constructor() { let build = clap::Command::new; build("claudine"); }
"#;
    let sites = scan_source("fixture.rs", source);
    assert_eq!(tally(&sites, "governed_imported_item"), (1, 0));
    assert_eq!(tally(&sites, "ungoverned_imported_item"), (0, 1));
    assert_eq!(tally(&sites, "ungoverned_qualified_item"), (0, 1));
    assert_eq!(tally(&sites, "ungoverned_tokio_item"), (0, 1));
    assert_eq!(tally(&sites, "ungoverned_item_of_item"), (0, 1));
    assert_eq!(tally(&sites, "governed_by_helper_item"), (1, 0));
    assert_eq!(tally(&sites, "rebound_before_use"), (0, 0));
    assert_eq!(tally(&sites, "reassigned_before_use"), (0, 0));
    assert_eq!(tally(&sites, "constructed_on_one_branch"), (0, 1));
    assert_eq!(tally(&sites, "bound_on_one_branch"), (0, 1));
    assert_eq!(tally(&sites, "helper_bound_on_one_branch"), (0, 1));
    assert_eq!(tally(&sites, "aliases_an_unrelated_function"), (0, 0));
    assert_eq!(tally(&sites, "aliases_a_clap_constructor"), (0, 0));
    assert_eq!(
        sites
            .iter()
            .filter(|site| site.function == "ungoverned_tokio_item")
            .map(|site| site.command_kind)
            .collect::<Vec<_>>(),
        vec!["tokio"]
    );
    assert_eq!(sites.len(), 9);
}

/// A glob chain is followed to any depth: five hops of `pub use crate::hop::*`
/// still carry both the constructor's and the helper's binding to the file
/// that writes `use crate::hop1::*`.
#[test]
fn a_glob_chain_of_any_depth_still_names_the_process_command() {
    let crate_dir = tempfile::tempdir().expect("temp crate");
    let src = crate_dir.path().join("src");
    fs::create_dir_all(&src).expect("temp crate layout");
    fs::write(src.join("lib.rs"), "mod deep;\nmod hop1;\nmod hop2;\nmod hop3;\nmod hop4;\nmod hop5;\n")
        .expect("crate root");
    for hop in 1..5 {
        fs::write(src.join(format!("hop{hop}.rs")), format!("pub use crate::hop{}::*;\n", hop + 1))
            .expect("hop module");
    }
    fs::write(
        src.join("hop5.rs"),
        "pub use std::process::Command;\npub use claudine::child_environment::contribute_child_environment;\n",
    )
    .expect("last hop module");

    let deep_path = src.join("deep.rs");
    let deep = r#"
use crate::hop1::*;

fn governed_deep() { let mut c = Command::new("x"); contribute_child_environment(&mut c); c.spawn(); }
fn ungoverned_deep() { Command::new("x").spawn(); }
"#;
    fs::write(&deep_path, deep).expect("deep module");

    let resolver = ModuleResolver::new(Some(src));
    let sites = scan_crate_file("deep.rs", &deep_path, deep, &resolver);
    assert_eq!(tally(&sites, "governed_deep"), (1, 0));
    assert_eq!(tally(&sites, "ungoverned_deep"), (0, 1));
    assert_eq!(sites.len(), 2);
}

/// A constructor that leaves a function as a value is a construction the
/// function can no longer describe, so it is `UNCONTROLLED` at the expression
/// — as an argument (Review 6's `invoke(Command::new::<&'static str>)`
/// included), a bound name passed on, an enum or struct payload, a tuple or
/// array element, a return value, or a closure's result. A plain-name binding
/// stays a binding, a parenthesized callee still calls, the helper and any
/// unrelated function passed the same way contribute nothing, and a closure
/// that constructs is censused on what it does with the command.
#[test]
fn a_constructor_leaving_the_function_fails_closed() {
    let source = r#"
use claudine::child_environment::contribute_child_environment;
use std::process::Command;
use tokio::process::Command as TokioCommand;

fn invoke<F>(construct: F) where F: Fn(&'static str) -> Command { construct("true").status().unwrap(); }
fn launch() { invoke(Command::new::<&'static str>); }
fn handed_plain() { invoke(Command::new); }
fn handed_qualified() { invoke(std::process::Command::new); }
fn handed_bound() { let c = Command::new; invoke(c); }
fn handed_in_option() { let _ = Some(Command::new); }
fn handed_in_struct() { let _ = Factory { construct: Command::new }; }
fn handed_in_tuple() { let _ = (Command::new, 1); }
fn handed_in_array() { let _ = [Command::new]; }
fn handed_to_method() { registry.register(Command::new); }
fn returned_as_tail() -> fn(&str) -> Command { Command::new }
fn returned_explicitly() -> fn(&str) -> Command { return Command::new; }
fn returned_from_closure() { let _ = || Command::new; }
fn handed_tokio() { invoke(TokioCommand::new); }
fn handed_unrelated() { invoke(unrelated); }
fn handed_clap() { invoke(clap::Command::new); }
fn handed_helper() { apply(contribute_child_environment); }
fn closure_constructs_unhelped() { invoke(|s| Command::new(s)); }
fn closure_constructs_helped() { invoke(|s| { let mut c = Command::new(s); contribute_child_environment(&mut c); c }); }
fn parenthesized_call() { (Command::new)("x").status(); }
fn bound_stays_a_binding() { let construct = Command::new; let mut c = construct("x"); contribute_child_environment(&mut c); c.status(); }
"#;
    let sites = scan_source("fixture.rs", source);
    assert_eq!(tally(&sites, "invoke"), (0, 1));
    assert_eq!(tally(&sites, "launch"), (0, 1));
    assert_eq!(tally(&sites, "handed_plain"), (0, 1));
    assert_eq!(tally(&sites, "handed_qualified"), (0, 1));
    assert_eq!(tally(&sites, "handed_bound"), (0, 1));
    assert_eq!(tally(&sites, "handed_in_option"), (0, 1));
    assert_eq!(tally(&sites, "handed_in_struct"), (0, 1));
    assert_eq!(tally(&sites, "handed_in_tuple"), (0, 1));
    assert_eq!(tally(&sites, "handed_in_array"), (0, 1));
    assert_eq!(tally(&sites, "handed_to_method"), (0, 1));
    assert_eq!(tally(&sites, "returned_as_tail"), (0, 1));
    assert_eq!(tally(&sites, "returned_explicitly"), (0, 1));
    assert_eq!(tally(&sites, "returned_from_closure"), (0, 1));
    assert_eq!(tally(&sites, "handed_tokio"), (0, 1));
    assert_eq!(tally(&sites, "handed_unrelated"), (0, 0));
    assert_eq!(tally(&sites, "handed_clap"), (0, 0));
    assert_eq!(tally(&sites, "handed_helper"), (0, 0));
    assert_eq!(tally(&sites, "closure_constructs_unhelped"), (0, 1));
    assert_eq!(tally(&sites, "closure_constructs_helped"), (1, 0));
    assert_eq!(tally(&sites, "parenthesized_call"), (0, 1));
    assert_eq!(tally(&sites, "bound_stays_a_binding"), (1, 0));
    assert_eq!(
        sites
            .iter()
            .filter(|site| site.function == "handed_tokio")
            .map(|site| site.command_kind)
            .collect::<Vec<_>>(),
        vec!["tokio"]
    );
    assert_eq!(sites.len(), 18);
}

/// A parameter typed as a callable returning the command constructs when
/// called, in every callable shape and through a `type` alias, a qualified
/// path, or tokio's command; governance is then the ordinary obligation. A
/// callable returning anything else, an unbounded generic, and a `Command`
/// received by value contribute nothing — the last is the caller's hand-off.
/// Passing such a parameter on is the hand-off record of the previous fixture.
#[test]
fn a_constructor_typed_parameter_constructs_when_called() {
    let source = r#"
use claudine::child_environment::contribute_child_environment;
use std::process::Command;
use tokio::process::Command as TokioCommand;
type Cmd = Command;

fn governed_impl_fn(construct: impl Fn(&str) -> Command) { let mut c = construct("x"); contribute_child_environment(&mut c); c.status(); }
fn ungoverned_impl_fn(construct: impl Fn(&str) -> Command) { construct("x").status(); }
fn governed_fn_pointer(construct: fn(&str) -> Command) { let mut c = construct("x"); contribute_child_environment(&mut c); c.spawn(); }
fn ungoverned_fn_pointer(construct: fn(&str) -> Command) { construct("x").spawn(); }
fn governed_boxed(construct: Box<dyn Fn(&str) -> Command>) { let mut c = construct("x"); contribute_child_environment(&mut c); c.output(); }
fn ungoverned_boxed(mut construct: Box<dyn FnMut(&str) -> Command>) { construct("x").output(); }
fn governed_dyn_ref(construct: &dyn Fn(&str) -> Command) { let mut c = construct("x"); contribute_child_environment(&mut c); c.status(); }
fn ungoverned_dyn_ref(construct: &dyn Fn(&str) -> Command) { construct("x").status(); }
fn governed_generic<F: Fn(&str) -> Command>(construct: F) { let mut c = construct("x"); contribute_child_environment(&mut c); c.status(); }
fn ungoverned_generic<F: FnMut(&str) -> Command>(mut construct: F) { construct("x").status(); }
fn governed_where<F>(construct: F) where F: Fn(&str) -> Command { let mut c = construct("x"); contribute_child_environment(&mut c); c.status(); }
fn ungoverned_where<F>(construct: F) where F: FnOnce(&str) -> Command { construct("x").status(); }
fn ungoverned_alias(construct: impl Fn(&str) -> Cmd) { construct("x").status(); }
fn ungoverned_qualified(construct: impl Fn(&str) -> std::process::Command) { construct("x").status(); }
fn ungoverned_tokio(construct: impl Fn(&str) -> TokioCommand) { construct("x").output(); }
fn parameter_handed_on(construct: impl Fn(&str) -> Command) { other(construct); }
fn clap_parameter(build: impl Fn(&str) -> clap::Command) { build("x"); }
fn unrelated_parameter(make: impl Fn(&str) -> String) { make("x"); }
fn unbounded_generic<F>(construct: F) { construct("x"); }
fn command_by_value(mut c: Command) { c.status(); }
"#;
    let sites = scan_source("fixture.rs", source);
    assert_eq!(tally(&sites, "governed_impl_fn"), (1, 0));
    assert_eq!(tally(&sites, "ungoverned_impl_fn"), (0, 1));
    assert_eq!(tally(&sites, "governed_fn_pointer"), (1, 0));
    assert_eq!(tally(&sites, "ungoverned_fn_pointer"), (0, 1));
    assert_eq!(tally(&sites, "governed_boxed"), (1, 0));
    assert_eq!(tally(&sites, "ungoverned_boxed"), (0, 1));
    assert_eq!(tally(&sites, "governed_dyn_ref"), (1, 0));
    assert_eq!(tally(&sites, "ungoverned_dyn_ref"), (0, 1));
    assert_eq!(tally(&sites, "governed_generic"), (1, 0));
    assert_eq!(tally(&sites, "ungoverned_generic"), (0, 1));
    assert_eq!(tally(&sites, "governed_where"), (1, 0));
    assert_eq!(tally(&sites, "ungoverned_where"), (0, 1));
    assert_eq!(tally(&sites, "ungoverned_alias"), (0, 1));
    assert_eq!(tally(&sites, "ungoverned_qualified"), (0, 1));
    assert_eq!(tally(&sites, "ungoverned_tokio"), (0, 1));
    assert_eq!(tally(&sites, "parameter_handed_on"), (0, 1));
    assert_eq!(tally(&sites, "clap_parameter"), (0, 0));
    assert_eq!(tally(&sites, "unrelated_parameter"), (0, 0));
    assert_eq!(tally(&sites, "unbounded_generic"), (0, 0));
    assert_eq!(tally(&sites, "command_by_value"), (0, 0));
    assert_eq!(
        sites
            .iter()
            .filter(|site| site.function == "ungoverned_tokio")
            .map(|site| site.command_kind)
            .collect::<Vec<_>>(),
        vec!["tokio"]
    );
    assert_eq!(sites.len(), 16);
}

/// A `macro_rules!` transcriber is censused where the macro is defined, under
/// `macro!` — zero-argument, with metavariables, with a repetition, inside a
/// function body — and a function the transcriber emits under `macro!::fn`.
/// `$crate` reaches the helper's module; a `clap::Command` transcriber and a
/// `#[cfg(test)]` definition contribute nothing.
#[test]
fn a_local_macro_rules_transcriber_is_censused() {
    let source = r#"
use claudine::child_environment::contribute_child_environment;
use std::process::Command;

macro_rules! ungoverned_zero_arg { () => { std::process::Command::new("true").status() }; }
macro_rules! governed_zero_arg { () => {{ let mut c = Command::new("true"); contribute_child_environment(&mut c); c.status() }}; }
macro_rules! ungoverned_metavariable { ($p:expr) => { Command::new($p).status() }; }
macro_rules! governed_metavariable { ($p:expr) => {{ let mut c = Command::new($p); contribute_child_environment(&mut c); c.status() }}; }
macro_rules! ungoverned_repetition {
    ($($p:expr),* $(,)?) => {{ let mut c = Command::new("x"); c.args([$($p),*]); $( c.env("K", $p); )* c.status() }};
}
macro_rules! ungoverned_items { ($t:ident) => { impl $t { fn launch(&self) { Command::new("x").status(); } } }; }
macro_rules! governed_by_crate_path { () => {{ let mut c = Command::new("x"); $crate::child_environment::contribute_child_environment(&mut c); c.spawn() }}; }
macro_rules! clap_only { () => { clap::Command::new("x") }; }
#[cfg(test)] macro_rules! test_only { () => { Command::new("x").status() }; }
fn inside_a_function() { macro_rules! ungoverned_inner { () => { Command::new("x").output() }; } ungoverned_inner!(); }
"#;
    let sites = scan_source("fixture.rs", source);
    assert_eq!(tally(&sites, "ungoverned_zero_arg!"), (0, 1));
    assert_eq!(tally(&sites, "governed_zero_arg!"), (1, 0));
    assert_eq!(tally(&sites, "ungoverned_metavariable!"), (0, 1));
    assert_eq!(tally(&sites, "governed_metavariable!"), (1, 0));
    assert_eq!(tally(&sites, "ungoverned_repetition!"), (0, 1));
    assert_eq!(tally(&sites, "ungoverned_items!::launch"), (0, 1));
    assert_eq!(tally(&sites, "governed_by_crate_path!"), (1, 0));
    assert_eq!(tally(&sites, "clap_only!"), (0, 0));
    assert_eq!(tally(&sites, "test_only!"), (0, 0));
    assert_eq!(tally(&sites, "ungoverned_inner!"), (0, 1));
    assert_eq!(tally(&sites, "inside_a_function"), (0, 0));
    assert_eq!(sites.len(), 8);
}

/// Literal names in a declarative macro use the invocation module's bindings.
/// The scanner considers every module context, so the executable expansion is
/// found even though the definition module does not import `Command`.
#[test]
fn a_local_macro_rules_transcriber_uses_invocation_scope() {
    let source = r#"
macro_rules! launch {
    () => { Command::new("true").status().unwrap() };
}

mod call_site {
    use std::process::Command;

    pub fn run() {
        launch!();
    }
}

fn main() {
    call_site::run();
}
"#;
    let sites = scan_source("fixture.rs", source);
    assert_eq!(tally(&sites, "launch!"), (0, 1));
    assert_eq!(sites.len(), 1);
}

/// A conditionally exported transcriber can inherit an ordinary item name from
/// a module in the other guarded crate. Keeping the definition and invocation
/// resolvers separate is what makes that cross-crate expansion visible.
#[test]
fn a_conditionally_exported_macro_rules_transcriber_uses_a_cross_crate_invocation_scope() {
    let definition_source = r#"
#[cfg_attr(not(test), macro_export)]
macro_rules! launch {
    () => { Command::new("true").status().unwrap() };
}
"#;
    let invocation_source = r#"
use std::process::Command;

fn run() {
    dependency::launch!();
}
"#;
    let definition = parse_source("definition/src/lib.rs", definition_source);
    let invocation = parse_source("invocation/src/main.rs", invocation_source);
    let definition_resolver = ModuleResolver::new(None);
    let invocation_resolver = ModuleResolver::new(None);
    let definition_location = definition_resolver.location_of(None, &definition);
    let invocation_location = invocation_resolver.location_of(None, &invocation);
    let definition_contexts = definition_location
        .module_contexts()
        .into_iter()
        .map(|location| ExpansionContext {
            resolver: &definition_resolver,
            location,
        })
        .collect::<Vec<_>>();

    let before = scan_parsed(
        "definition/src/lib.rs",
        &definition,
        &definition_resolver,
        definition_location.clone(),
        &definition_contexts,
        &definition_contexts,
    );
    assert_eq!(tally(&before, "launch!"), (0, 0));

    let mut all_contexts = definition_contexts.clone();
    all_contexts.extend(
        invocation_location
            .module_contexts()
            .into_iter()
            .map(|location| ExpansionContext {
                resolver: &invocation_resolver,
                location,
            }),
    );
    let after = scan_parsed(
        "definition/src/lib.rs",
        &definition,
        &definition_resolver,
        definition_location,
        &definition_contexts,
        &all_contexts,
    );
    assert_eq!(tally(&after, "launch!"), (0, 1));
    assert_eq!(after.len(), 1);
}

#[test]
fn unknown_cfg_attr_predicates_keep_macro_exports_in_the_production_census() {
    for attribute in [
        "#[cfg_attr(feature = \"optional-export\", macro_export)]",
        "#[cfg_attr(unix, macro_export)]",
        "#[cfg_attr(unix, cfg_attr(feature = \"optional-export\", macro_export))]",
    ] {
        let source = format!("{attribute} macro_rules! launch {{ () => {{}} }}");
        let file = syn::parse_file(&source).unwrap();
        let syn::Item::Macro(item) = &file.items[0] else {
            panic!("fixture must parse as a macro_rules! item");
        };
        assert!(production_macro_export(&item.attrs), "{attribute}");
    }

    for attribute in [
        "#[cfg_attr(test, macro_export)]",
        "#[cfg_attr(all(test, unix), macro_export)]",
    ] {
        let source = format!("{attribute} macro_rules! launch {{ () => {{}} }}");
        let file = syn::parse_file(&source).unwrap();
        let syn::Item::Macro(item) = &file.items[0] else {
            panic!("fixture must parse as a macro_rules! item");
        };
        assert!(!production_macro_export(&item.attrs), "{attribute}");
    }
}

#[test]
fn an_exported_macro_rules_transcriber_keeps_dollar_crate_in_the_definition_crate() {
    let definition_source = r#"
pub use std::process::Command as DefinitionCommand;

#[macro_export]
macro_rules! launch {
    () => { $crate::DefinitionCommand::new("true").status().unwrap() };
}
"#;
    let invocation_source = r#"
type DefinitionCommand = String;
"#;
    let definition = parse_source("definition/src/lib.rs", definition_source);
    let invocation = parse_source("invocation/src/main.rs", invocation_source);
    let definition_resolver = ModuleResolver::new(None);
    let invocation_resolver = ModuleResolver::new(None);
    let definition_location = definition_resolver.location_of(None, &definition);
    let invocation_location = invocation_resolver.location_of(None, &invocation);
    let definition_contexts = definition_location
        .module_contexts()
        .into_iter()
        .map(|location| ExpansionContext {
            resolver: &definition_resolver,
            location,
        })
        .collect::<Vec<_>>();
    let mut all_contexts = definition_contexts.clone();
    all_contexts.extend(
        invocation_location
            .module_contexts()
            .into_iter()
            .map(|location| ExpansionContext {
                resolver: &invocation_resolver,
                location,
            }),
    );

    let sites = scan_parsed(
        "definition/src/lib.rs",
        &definition,
        &definition_resolver,
        definition_location,
        &definition_contexts,
        &all_contexts,
    );
    assert_eq!(tally(&sites, "launch!"), (0, 2));
    assert_eq!(sites.len(), 2);
}

/// Externally generated tokens are outside the source-authored census. Both a
/// declarative invocation and procedural attribute therefore contribute no
/// site by themselves, while executable invocation arguments remain visible.
#[test]
fn external_macro_expansions_observe_the_source_inventory_boundary() {
    let source = r#"
use std::process::Command;

#[external_macros::launch]
fn procedural() {}

fn declarative() {
    external_macros::launch!();
}

fn executable_argument() {
    external_macros::identity!(Command::new("true").status());
}
"#;
    let sites = scan_source("fixture.rs", source);
    assert_eq!(tally(&sites, "procedural"), (0, 0));
    assert_eq!(tally(&sites, "declarative"), (0, 0));
    assert_eq!(tally(&sites, "executable_argument"), (0, 1));
    assert_eq!(sites.len(), 1);
}

/// A transcriber that is neither a block nor items cannot be censused, and a
/// construction it might hide must not pass silently: the scan fails, naming
/// the file and the macro.
#[test]
#[should_panic(expected = "fixture.rs: macro_rules! unanalyzable expands to neither a block nor items; its transcriber could not be analyzed")]
fn an_unanalyzable_macro_transcriber_fails_the_census() {
    let source = r#"
use std::process::Command;

macro_rules! unanalyzable { () => { fn }; }
"#;
    scan_source("fixture.rs", source);
}
