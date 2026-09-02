//! Production process-spawn census and `AGENT_CWD` governance guard.
//!
//! Regenerate after an intentional spawn-seam change:
//!
//! ```text
//! CLAUDINE_UPDATE_SPAWN_INVENTORY=1 cargo nextest run -p claudine-cli --test spawn_inventory
//! ```

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

const BLESS_ENV: &str = "CLAUDINE_UPDATE_SPAWN_INVENTORY";
const REGEN_COMMAND: &str =
    "CLAUDINE_UPDATE_SPAWN_INVENTORY=1 cargo nextest run -p claudine-cli --test spawn_inventory";

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

    fn kind_for_constructor(&self, path: &syn::Path) -> Option<CommandKind> {
        let segments: Vec<_> = path.segments.iter().map(|segment| segment.ident.to_string()).collect();
        if segments.last().map(String::as_str) != Some("new") || segments.len() < 2 {
            return None;
        }
        let command = &segments[segments.len() - 2];
        if self.std.contains(command)
            || segments.ends_with(&[
                "std".to_string(),
                "process".to_string(),
                "Command".to_string(),
                "new".to_string(),
            ])
        {
            return Some(CommandKind::Std);
        }
        if self.tokio.contains(command)
            || segments.ends_with(&[
                "tokio".to_string(),
                "process".to_string(),
                "Command".to_string(),
                "new".to_string(),
            ])
        {
            return Some(CommandKind::Tokio);
        }
        None
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

struct FunctionScanner<'a> {
    aliases: &'a Aliases,
    constructors: Vec<(usize, CommandKind)>,
    contributions: Vec<usize>,
}

impl<'a> FunctionScanner<'a> {
    fn new(aliases: &'a Aliases) -> Self {
        Self {
            aliases,
            constructors: Vec::new(),
            contributions: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for FunctionScanner<'_> {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(function) = call.func.as_ref() {
            if let Some(kind) = self.aliases.kind_for_constructor(&function.path) {
                self.constructors.push((call.span().start().line, kind));
            }
            if function.path.segments.last().is_some_and(|segment| {
                segment.ident == "contribute_child_environment"
            }) {
                self.contributions.push(call.span().start().line);
            }
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if !cfg_test(&item.attrs) {
            visit::visit_item_mod(self, item);
        }
    }
}

struct FileScanner<'a> {
    path: &'a str,
    aliases: &'a Aliases,
    sites: Vec<SpawnSite>,
}

impl FileScanner<'_> {
    fn scan_function(&mut self, name: String, block: &syn::Block) {
        let mut scanner = FunctionScanner::new(self.aliases);
        scanner.visit_block(block);
        if scanner.constructors.is_empty() {
            return;
        }

        let shared_after_branches = scanner.contributions.len() == 1
            && scanner
                .constructors
                .iter()
                .all(|(line, _)| *line < scanner.contributions[0]);
        let direct = scanner.contributions.len() >= scanner.constructors.len()
            || shared_after_branches;
        let indirect = indirect_governor(self.path, &name);
        for (line, kind) in scanner.constructors {
            let governed_by = if direct {
                "contribute_child_environment".to_string()
            } else if let Some(governor) = indirect {
                governor.to_string()
            } else {
                "UNCONTROLLED".to_string()
            };
            self.sites.push(SpawnSite {
                path: self.path.to_string(),
                line,
                function: name.clone(),
                command_kind: kind.label(),
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
        if !cfg_test(&item.attrs) {
            self.scan_function(item.sig.ident.to_string(), &item.block);
        }
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        if !cfg_test(&item.attrs) {
            self.scan_function(item.sig.ident.to_string(), &item.block);
        }
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

fn scan_source(path: &str, source: &str) -> Vec<SpawnSite> {
    let file = syn::parse_file(source).unwrap_or_else(|error| panic!("failed to parse {path}: {error}"));
    let aliases = Aliases::collect(&file);
    let mut scanner = FileScanner {
        path,
        aliases: &aliases,
        sites: Vec::new(),
    };
    scanner.visit_file(&file);
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

fn production_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for relative in ["claudine/lib/src", "claudine/cli/src"] {
        collect_rs_files(&root.join(relative), &mut files);
    }
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
    for file in production_files(&root) {
        let relative = file.strip_prefix(root.join("claudine")).unwrap();
        let relative = relative.to_string_lossy().replace('\\', "/");
        let source = fs::read_to_string(&file).unwrap();
        sites.extend(scan_source(&relative, &source));
    }
    sites.sort();
    Inventory {
        tool: "claudine-spawn-inventory",
        scanned_roots: ["claudine/lib/src", "claudine/cli/src"],
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
        .filter(|site| site.governed_by == "UNCONTROLLED")
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

#[test]
fn scanner_covers_governed_and_ungoverned_construction_forms() {
    let source = r#"
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
    let governed: BTreeSet<_> = sites
        .iter()
        .filter(|site| site.governed_by != "UNCONTROLLED")
        .map(|site| site.function.as_str())
        .collect();
    let uncontrolled: BTreeSet<_> = sites
        .iter()
        .filter(|site| site.governed_by == "UNCONTROLLED")
        .map(|site| site.function.as_str())
        .collect();
    assert_eq!(
        governed,
        BTreeSet::from(["governed_helper", "governed_platform", "governed_qualified", "governed_std", "governed_tokio"])
    );
    assert_eq!(
        uncontrolled,
        BTreeSet::from(["ungoverned_helper", "ungoverned_platform", "ungoverned_qualified", "ungoverned_std", "ungoverned_tokio"])
    );
}
