//! The scope guards of the sequence error standard: what a
//! `CompositionError` must carry and how it may render.
//!
//! - **Site.** A variant named `Sequence…` carries a `site` field, or both
//!   `source_path` and `property`, so its task and key reach the author.
//! - **Noun.** No `#[error]` text and no string literal in a `render/`
//!   block hard-codes a construct noun such as "external sequence file";
//!   the noun comes from `ExecutableField::noun`.
//! - **Catch-all.** A variant no renderer names falls through to the
//!   generic "composition failed" block in `render/provider.rs`; only the
//!   variants listed may.
//! - **Own arm.** Every variant the dispatcher routes to
//!   `render/sequence.rs` has its own arm in that file's `parts`, and
//!   `parts` names no variant the dispatcher does not route there.
//!
//! Each guard compares the variants it finds with its entries in
//! [`ALLOWLIST`]: an unlisted variant fails, and so does an entry that no
//! longer matches one. Every `Sequence…` variant now meets the standard, so
//! the only entries left are tagged `outside-scope`; a `Sequence…` entry
//! fails.

use std::collections::BTreeSet;
use std::path::Path;

use serde::Deserialize;
use syn::visit::Visit;

use super::source_scan::area_root;

/// Area-relative path of the guards' exception list.
const ALLOWLIST: &str = "cli/tests/l1/error_guards/sequence-scope-allow.toml";

/// Construct nouns a message must take from the noun table.
const FORBIDDEN_NOUNS: &[&str] = &["external sequence file", "external YAML file", "external task file", "external group file"];

#[derive(Debug, Deserialize)]
struct Allowlist {
    #[serde(default)]
    allow: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
struct Entry {
    guard: String,
    /// The variant, or for a noun in a renderer, its `render/<file>.rs`.
    variant: String,
    tag: String,
    reason: String,
}

fn allowed(guard: &str) -> BTreeSet<String> {
    let text = std::fs::read_to_string(area_root().join(ALLOWLIST)).expect("the scope allowlist is readable");
    let list: Allowlist = toml::from_str(&text).expect("the scope allowlist parses");
    list.allow.into_iter().filter(|entry| entry.guard == guard).map(|entry| entry.variant).collect()
}

/// Each `CompositionError` variant: its name, its named fields, and the
/// text of its `#[error]` attribute.
struct Variant {
    name: String,
    fields: BTreeSet<String>,
    message: String,
}

fn parse(path: &Path) -> syn::File {
    let text = std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    syn::parse_file(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn variants(error_mod: &Path) -> Vec<Variant> {
    let file = parse(error_mod);
    let item = file
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Enum(item) if item.ident == "CompositionError" => Some(item),
            _ => None,
        })
        .expect("`CompositionError` is declared in error/mod.rs");
    item.variants
        .iter()
        .map(|variant| Variant {
            name: variant.ident.to_string(),
            fields: variant.fields.iter().filter_map(|field| field.ident.as_ref().map(ToString::to_string)).collect(),
            message: variant
                .attrs
                .iter()
                .filter(|attr| attr.path().is_ident("error"))
                .filter_map(|attr| attr.meta.require_list().ok().map(|list| list.tokens.to_string()))
                .collect::<Vec<_>>()
                .join(" "),
        })
        .collect()
}

/// Every `CompositionError::X` a pattern names, through any of `owners`
/// (`CompositionError` and the names a `use … as` gives it).
struct PatternVariants<'a> {
    owners: &'a BTreeSet<String>,
    names: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for PatternVariants<'_> {
    fn visit_path(&mut self, path: &'ast syn::Path) {
        let segments: Vec<String> = path.segments.iter().map(|segment| segment.ident.to_string()).collect();
        if let [.., owner, variant] = segments.as_slice()
            && self.owners.contains(owner)
        {
            self.names.insert(variant.clone());
        }
        syn::visit::visit_path(self, path);
    }
}

/// `CompositionError` and every name a `use CompositionError as …` in
/// `file` gives it.
fn owner_names(file: &syn::File) -> BTreeSet<String> {
    #[derive(Default)]
    struct Renames(BTreeSet<String>);
    impl<'ast> Visit<'ast> for Renames {
        fn visit_use_rename(&mut self, rename: &'ast syn::UseRename) {
            if rename.ident == "CompositionError" {
                self.0.insert(rename.rename.to_string());
            }
        }
    }
    let mut renames = Renames::default();
    renames.visit_file(file);
    renames.0.insert("CompositionError".to_string());
    renames.0
}

/// The arms of the first `match` in `function`'s body, in `file`: a
/// statement `match`, or the value of a `let`.
fn first_match_arms<'f>(file: &'f syn::File, function: &str) -> &'f [syn::Arm] {
    struct Finder<'a, 'f> {
        function: &'a str,
        arms: Option<&'f [syn::Arm]>,
    }
    impl<'f> Visit<'f> for Finder<'_, 'f> {
        fn visit_item_impl(&mut self, item: &'f syn::ItemImpl) {
            // Only `CompositionError`'s own renderer.
            if matches!(&*item.self_ty, syn::Type::Path(path) if path.path.is_ident("CompositionError")) {
                syn::visit::visit_item_impl(self, item);
            }
        }
        fn visit_impl_item_fn(&mut self, item: &'f syn::ImplItemFn) {
            if item.sig.ident == self.function && self.arms.is_none() {
                self.arms = first_match(&item.block);
            }
        }
        fn visit_item_fn(&mut self, item: &'f syn::ItemFn) {
            if item.sig.ident == self.function && self.arms.is_none() {
                self.arms = first_match(&item.block);
            }
        }
    }
    fn first_match(block: &syn::Block) -> Option<&[syn::Arm]> {
        block.stmts.iter().find_map(|stmt| match stmt {
            syn::Stmt::Expr(syn::Expr::Match(found), _) => Some(found.arms.as_slice()),
            syn::Stmt::Local(syn::Local { init: Some(init), .. }) => match &*init.expr {
                syn::Expr::Match(found) => Some(found.arms.as_slice()),
                _ => None,
            },
            _ => None,
        })
    }
    let mut finder = Finder { function, arms: None };
    finder.visit_file(file);
    finder.arms.unwrap_or_else(|| panic!("no `match` found in `{function}`"))
}

/// The variants the patterns of `arms` name.
fn arm_variants<'a>(file: &syn::File, arms: impl IntoIterator<Item = &'a syn::Arm>) -> BTreeSet<String> {
    let owners = owner_names(file);
    let mut names = PatternVariants { owners: &owners, names: BTreeSet::new() };
    for arm in arms {
        names.visit_pat(&arm.pat);
    }
    names.names
}

/// The variants named by the arms of the first `match` in `function`'s
/// body, in `file`.
fn named_arms(file: &syn::File, function: &str) -> BTreeSet<String> {
    arm_variants(file, first_match_arms(file, function))
}

/// The variants named by the arms of `function`'s first `match` whose body
/// calls the path `callee` (for example `["sequence", "status_block"]`).
fn arms_routed_to(file: &syn::File, function: &str, callee: &[&str]) -> BTreeSet<String> {
    struct Calls<'a> {
        callee: &'a [&'a str],
        found: bool,
    }
    impl<'ast> Visit<'ast> for Calls<'_> {
        fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
            if let syn::Expr::Path(path) = &*call.func {
                let segments: Vec<String> = path.path.segments.iter().map(|segment| segment.ident.to_string()).collect();
                self.found |= segments.ends_with(&self.callee.iter().map(ToString::to_string).collect::<Vec<_>>());
            }
            syn::visit::visit_expr_call(self, call);
        }
    }
    let routed = first_match_arms(file, function).iter().filter(|arm| {
        let mut calls = Calls { callee, found: false };
        calls.visit_expr(&arm.body);
        calls.found
    });
    arm_variants(file, routed)
}

/// The guard's findings against its allowlist: unlisted findings, then
/// stale entries.
fn compare(guard: &str, found: &BTreeSet<String>) -> Result<(), String> {
    let allowed = allowed(guard);
    let unlisted: Vec<_> = found.difference(&allowed).collect();
    let stale: Vec<_> = allowed.difference(found).collect();
    if unlisted.is_empty() && stale.is_empty() {
        return Ok(());
    }
    Err(format!("{guard} guard: unlisted {unlisted:?}; stale entries in {ALLOWLIST}: {stale:?}"))
}

/// A `Sequence…` variant names its task and key.
#[test]
fn every_sequence_variant_carries_its_site() {
    let variants = variants(&area_root().join("lib/src/composition/error/mod.rs"));
    let without_site: BTreeSet<String> = variants
        .iter()
        .filter(|variant| variant.name.starts_with("Sequence"))
        .filter(|variant| {
            !(variant.fields.contains("site")
                || (variant.fields.contains("source_path") && variant.fields.contains("property")))
        })
        .map(|variant| variant.name.clone())
        .collect();
    if let Err(problem) = compare("site", &without_site) {
        panic!("{problem}");
    }
}

/// No message hard-codes a construct noun.
#[test]
fn no_message_hard_codes_a_construct_noun() {
    let root = area_root();
    let mut found: BTreeSet<String> = variants(&root.join("lib/src/composition/error/mod.rs"))
        .into_iter()
        .filter(|variant| FORBIDDEN_NOUNS.iter().any(|noun| variant.message.contains(noun)))
        .map(|variant| variant.name)
        .collect();
    let render = root.join("lib/src/composition/error/render");
    for entry in std::fs::read_dir(&render).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|extension| extension == "rs") {
            let text = std::fs::read_to_string(&path).unwrap();
            if FORBIDDEN_NOUNS.iter().any(|noun| text.contains(noun)) {
                found.insert(format!("render/{}", path.file_name().unwrap().to_string_lossy()));
            }
        }
    }
    if let Err(problem) = compare("noun", &found) {
        panic!("{problem}");
    }
}

/// Only listed variants render as the generic "composition failed" block.
#[test]
fn only_listed_variants_reach_the_catch_all_block() {
    let root = area_root();
    let all: BTreeSet<String> = variants(&root.join("lib/src/composition/error/mod.rs")).into_iter().map(|variant| variant.name).collect();
    let dispatched = named_arms(&parse(&root.join("lib/src/composition/error/render/mod.rs")), "status_block");
    let provider = named_arms(&parse(&root.join("lib/src/composition/error/render/provider.rs")), "status_block");
    let generic: BTreeSet<String> =
        all.iter().filter(|name| !dispatched.contains(*name) && !provider.contains(*name)).cloned().collect();
    if let Err(problem) = compare("catch_all", &generic) {
        panic!("{problem}");
    }
}

/// Every variant routed to the sequence renderer has its own arm there, and
/// that renderer names nothing it is not routed: a variant added to the
/// route without a headline and hint fails, and so does a stale arm.
#[test]
fn every_routed_sequence_variant_has_its_own_arm() {
    let root = area_root();
    let render = root.join("lib/src/composition/error/render");
    let routed = arms_routed_to(&parse(&render.join("mod.rs")), "status_block", &["sequence", "status_block"]);
    let own = named_arms(&parse(&render.join("sequence.rs")), "parts");
    let missing: Vec<_> = routed.difference(&own).collect();
    let stale: Vec<_> = own.difference(&routed).collect();
    assert!(
        missing.is_empty() && stale.is_empty(),
        "render/sequence.rs `parts`: no arm for {missing:?}; arms the dispatcher does not route: {stale:?}"
    );
    assert!(!routed.is_empty(), "the dispatcher routes sequence variants to render/sequence.rs");
}

/// Each entry names a known guard and an `outside-scope` tag (no
/// `Sequence…` variant or renderer may be excepted any more), and each gives
/// a reason.
#[test]
fn every_scope_entry_names_its_guard_tag_and_reason() {
    let text = std::fs::read_to_string(area_root().join(ALLOWLIST)).unwrap();
    let list: Allowlist = toml::from_str(&text).unwrap();
    let mut seen = BTreeSet::new();
    for entry in &list.allow {
        assert!(["site", "noun", "catch_all"].contains(&entry.guard.as_str()), "unknown guard: {entry:?}");
        assert!(
            !entry.variant.starts_with("Sequence") && !entry.variant.starts_with("render/"),
            "a sequence variant or renderer meets the standard and takes no exception: {entry:?}"
        );
        assert_eq!(entry.tag, "outside-scope", "{entry:?}");
        assert!(entry.reason.split_whitespace().count() >= 5, "a reason says why: {entry:?}");
        assert!(seen.insert((entry.guard.clone(), entry.variant.clone())), "duplicate entry: {entry:?}");
    }
}

/// The guards are not blind: each finds what it is for in a sample.
#[test]
fn the_scope_guards_see_their_subjects() {
    let text = r#"
        pub enum CompositionError {
            #[error("failed to load external sequence file: {path}")]
            SequenceNoSite { path: String },
            SequenceSited { site: String },
            Other,
        }
        impl BlockError for CompositionError {
            fn status_block(&self) -> u8 {
                match self {
                    CompositionError::SequenceSited { .. } | CompositionError::Other => 1,
                    CompositionError::SequenceNoSite { .. } => sequence::status_block(self),
                    _ => 2,
                }
            }
        }
        fn parts(err: &CompositionError) -> u8 {
            use CompositionError as E;
            let found = match err {
                E::SequenceNoSite { .. } => 1,
                _ => 2,
            };
            found
        }
    "#;
    let dir = std::env::temp_dir().join(format!("scope-guard-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("mod.rs");
    std::fs::write(&path, text).unwrap();
    let found = variants(&path);
    let no_site: Vec<_> = found.iter().filter(|variant| variant.name.starts_with("Sequence") && !variant.fields.contains("site")).map(|variant| variant.name.as_str()).collect();
    assert_eq!(no_site, ["SequenceNoSite"]);
    assert!(found[0].message.contains("external sequence file"));
    let file = parse(&path);
    assert_eq!(
        named_arms(&file, "status_block"),
        BTreeSet::from(["SequenceSited".to_string(), "Other".to_string(), "SequenceNoSite".to_string()])
    );
    assert_eq!(arms_routed_to(&file, "status_block", &["sequence", "status_block"]), BTreeSet::from(["SequenceNoSite".to_string()]));
    assert_eq!(named_arms(&file, "parts"), BTreeSet::from(["SequenceNoSite".to_string()]), "an aliased `let` match is read");
    std::fs::remove_dir_all(&dir).unwrap();
}
