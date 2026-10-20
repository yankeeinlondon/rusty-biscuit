//! Ownership proofs and the retained inventory of an existing Rust output.
//!
//! The generator only overwrites or deletes what it can prove it wrote: a
//! `.rs` file whose first line is [`GENERATED_FILE_HEADER`], a manifest
//! carrying [`MANIFEST_OWNERSHIP_MARKER`], an export artifact by its name, or
//! a directory whose whole tree is such files. Symbolic links are never
//! followed, and a path that cannot be read is never proven owned.
//!
//! A partial run keeps every module group it does not regenerate. It learns
//! which ones exist from the output itself ([`read_rust_inventory`]) and
//! refuses the output when that evidence is missing, inconsistent, or does
//! not match the current definitions.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::Path;

use indexmap::IndexMap;
use schematic_define::RestApi;

use crate::cargo_gen::MANIFEST_OWNERSHIP_MARKER;
use crate::errors::GeneratorError;
use crate::output::format::GENERATED_FILE_HEADER;
use crate::output::ws_modules::ws_definition_modules;

/// How a file proves the generator wrote it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Proof {
    /// A `.rs` file starting with [`GENERATED_FILE_HEADER`].
    RustSource,
    /// A manifest containing the [`MANIFEST_OWNERSHIP_MARKER`] line.
    Manifest,
    /// An export artifact: a regular file whose name the caller matched to
    /// the artifact naming rule. JSON and YAML carry no header to check.
    Artifact,
}

/// What occupies one path, as far as ownership can be proven.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Occupant {
    Absent,
    /// A regular file carrying its ownership proof.
    OwnedFile,
    /// A directory whose entire tree is generator-owned `.rs` files.
    OwnedDir,
    /// Anything else, with the reason ownership could not be proven.
    Foreign { is_dir: bool, reason: String },
}

/// Inspects `path` without following a symbolic link.
pub(crate) fn occupant(path: &Path, proof: Proof) -> Occupant {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Occupant::Absent,
        Err(err) => return foreign(false, format!("cannot be inspected ({err})")),
    };
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        return foreign(false, "is a symbolic link, which is never followed".to_string());
    }
    if file_type.is_dir() {
        return match owned_tree(path) {
            Ok(()) => Occupant::OwnedDir,
            Err(reason) => foreign(true, reason),
        };
    }
    if !file_type.is_file() {
        return foreign(false, "is not a regular file".to_string());
    }
    match file_is_owned(path, proof) {
        Ok(true) => Occupant::OwnedFile,
        Ok(false) => foreign(false, "was not written by schematic-gen".to_string()),
        Err(reason) => foreign(false, reason),
    }
}

fn foreign(is_dir: bool, reason: String) -> Occupant {
    Occupant::Foreign { is_dir, reason }
}

fn file_is_owned(path: &Path, proof: Proof) -> Result<bool, String> {
    let unreadable = |err: std::io::Error| format!("cannot be read to prove ownership ({err})");
    match proof {
        Proof::Artifact => Ok(true),
        Proof::RustSource => {
            if path.extension().is_none_or(|ext| ext != "rs") {
                return Ok(false);
            }
            let mut start = vec![0; GENERATED_FILE_HEADER.len()];
            let mut file = fs::File::open(path).map_err(unreadable)?;
            match file.read_exact(&mut start) {
                Ok(()) => Ok(start == GENERATED_FILE_HEADER.as_bytes()),
                Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => Ok(false),
                Err(err) => Err(unreadable(err)),
            }
        }
        Proof::Manifest => {
            let bytes = fs::read(path).map_err(unreadable)?;
            Ok(String::from_utf8_lossy(&bytes)
                .lines()
                .any(|line| line == MANIFEST_OWNERSHIP_MARKER))
        }
    }
}

/// `Ok` when every entry under `dir` is a generator-owned `.rs` file or an
/// owned subdirectory. An empty directory is owned vacuously.
fn owned_tree(dir: &Path) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|err| format!("cannot be listed ({err})"))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("cannot be listed ({err})"))?;
        let path = entry.path();
        match occupant(&path, Proof::RustSource) {
            Occupant::OwnedFile | Occupant::OwnedDir | Occupant::Absent => {}
            Occupant::Foreign { reason, .. } => {
                return Err(format!("contains {} which {reason}", path.display()));
            }
        }
    }
    Ok(())
}

/// The module groups an existing Rust output keeps through a partial run.
#[derive(Debug, Default)]
pub(crate) struct RustInventory {
    /// REST module groups declared by the existing output and not selected,
    /// in the order its `lib.rs` declares them.
    pub retained: Vec<String>,
    /// The runtime ABI the existing `lib.rs` records.
    pub abi: AbiMarker,
}

/// The name of the generated `lib.rs` constant recording the
/// `schematic-runtime` ABI the crate was generated for.
pub(crate) const ABI_MARKER: &str = "SCHEMATIC_RUNTIME_ABI";

/// What an existing `lib.rs` records about the runtime ABI.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum AbiMarker {
    /// No marker: the output predates `schematic-runtime`, or has no
    /// `lib.rs` to read.
    #[default]
    Missing,
    /// `pub const SCHEMATIC_RUNTIME_ABI: u32 = <value>;`
    Declared(u32),
    /// A marker of any other form, with what is wrong with it.
    Malformed(String),
}

/// Remedy shown when a partial run cannot trust the existing output.
pub(crate) const INVENTORY_REMEDY: &str = "Run full generation (`generate --api all`) or \
     generate into an isolated output directory.";

/// Establishes which REST module groups `root` holds, before a partial run
/// writes anything.
///
/// `groups` are the current definitions grouped by module, and `selected` the
/// groups this run regenerates. An absent or empty root holds nothing. Any
/// other root must be fully explained: a generator-owned `lib.rs` and
/// `prelude.rs` that parse, every declared module present in exactly one
/// owned layout, every owned module declared, every declared module known to
/// the current definitions, and the prelude exports of every retained group
/// equal to what the current definitions would export. Unowned content that
/// is not a declared module stays outside the inventory and is preserved.
///
/// ## Errors
///
/// [`GeneratorError::UnsafeOutput`] listing every problem found.
pub(crate) fn read_rust_inventory(
    root: &Path,
    groups: &IndexMap<String, Vec<&RestApi>>,
    selected: &BTreeSet<String>,
) -> Result<RustInventory, GeneratorError> {
    let refuse = |problems: Vec<String>| GeneratorError::UnsafeOutput {
        root: root.display().to_string(),
        problems,
        remedy: INVENTORY_REMEDY.to_string(),
    };
    let entries = match list_entries(root) {
        Ok(None) => return Ok(RustInventory::default()),
        Ok(Some(entries)) if entries.is_empty() => return Ok(RustInventory::default()),
        Ok(Some(entries)) => entries,
        Err(problem) => return Err(refuse(vec![problem])),
    };

    let mut problems = Vec::new();
    let lib = read_owned_source(root, "lib.rs", &mut problems);
    let prelude = read_owned_source(root, "prelude.rs", &mut problems);
    let (Some(lib), Some(prelude)) = (lib, prelude) else {
        return Err(refuse(problems));
    };
    let declared = declared_modules(&lib);
    let exports = prelude_exports(&prelude);
    let abi = abi_marker(&lib);

    let ws_modules: BTreeSet<&str> = ws_definition_modules()
        .iter()
        .map(|ws| ws.module)
        .chain(["ws_shared"])
        .collect();

    let mut owned_modules = BTreeSet::new();
    for name in &entries {
        let path = root.join(name);
        let module = if is_module_dir(&path) {
            Some(name.as_str())
        } else if occupant(&path, Proof::RustSource) == Occupant::OwnedFile {
            name.strip_suffix(".rs")
        } else {
            None
        };
        if let Some(module) = module
            && !["lib", "prelude", "shared"].contains(&module)
        {
            owned_modules.insert(module.to_string());
        }
    }
    for module in &owned_modules {
        if !declared.contains(module) && !selected.contains(module) {
            problems.push(format!(
                "generated module `{module}` is not declared in lib.rs"
            ));
        }
    }

    let mut retained = Vec::new();
    for module in &declared {
        let known_rest = groups.contains_key(module);
        if !known_rest && !ws_modules.contains(module.as_str()) {
            problems.push(format!(
                "lib.rs declares module `{module}`, which no current definition generates"
            ));
            continue;
        }
        if !known_rest || selected.contains(module) {
            continue;
        }
        let file = occupant(&root.join(format!("{module}.rs")), Proof::RustSource);
        match (&file, is_module_dir(&root.join(module))) {
            (Occupant::OwnedFile, true) => problems.push(format!(
                "module `{module}` exists both as {module}.rs and as {module}/"
            )),
            (Occupant::OwnedFile, false) | (_, true) => {}
            (Occupant::Foreign { reason, .. }, false) => {
                problems.push(format!("module `{module}`: {module}.rs {reason}"));
            }
            _ => problems.push(format!(
                "lib.rs declares module `{module}`, but neither {module}.rs nor {module}/mod.rs \
                 is a generated file"
            )),
        }
        let expected = expected_exports(&groups[module]);
        let found = exports.get(module).cloned().unwrap_or_default();
        if found != expected {
            problems.push(format!(
                "prelude.rs exports {found:?} from `{module}`, but the current definitions \
                 export {expected:?}"
            ));
        }
        retained.push(module.clone());
    }
    for module in exports.keys() {
        if !declared.contains(module) {
            problems.push(format!(
                "prelude.rs re-exports from `{module}`, which lib.rs does not declare"
            ));
        }
    }

    if problems.is_empty() {
        Ok(RustInventory { retained, abi })
    } else {
        Err(refuse(problems))
    }
}

/// Whether `path` is a real directory (not a link) holding a generated
/// `mod.rs`. Other content inside does not change that; it only keeps the
/// directory from being deleted as a whole.
fn is_module_dir(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir())
        && occupant(&path.join("mod.rs"), Proof::RustSource) == Occupant::OwnedFile
}

/// Lists the names directly under `root`; `None` when it does not exist.
pub(crate) fn list_entries(root: &Path) -> Result<Option<Vec<String>>, String> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("{} cannot be listed ({err})", root.display())),
    };
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| format!("{} cannot be listed ({err})", root.display()))?;
        match entry.file_name().into_string() {
            Ok(name) => names.push(name),
            Err(name) => {
                return Err(format!(
                    "{} holds an entry whose name is not UTF-8: {name:?}",
                    root.display()
                ));
            }
        }
    }
    names.sort();
    Ok(Some(names))
}

fn read_owned_source(root: &Path, name: &str, problems: &mut Vec<String>) -> Option<syn::File> {
    let path = root.join(name);
    match occupant(&path, Proof::RustSource) {
        Occupant::OwnedFile => {}
        Occupant::Absent => {
            problems.push(format!("{name} is missing, so the existing modules are unknown"));
            return None;
        }
        Occupant::OwnedDir => {
            problems.push(format!("{name} is a directory"));
            return None;
        }
        Occupant::Foreign { reason, .. } => {
            problems.push(format!("{name} {reason}"));
            return None;
        }
    }
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) => {
            problems.push(format!("{name} cannot be read ({err})"));
            return None;
        }
    };
    match syn::parse_file(&text) {
        Ok(file) => Some(file),
        Err(err) => {
            problems.push(format!("{name} does not parse as Rust ({err})"));
            None
        }
    }
}

/// `pub mod x;` declarations of a `lib.rs`, without `shared` and `prelude`.
fn declared_modules(lib: &syn::File) -> BTreeSet<String> {
    lib.items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Mod(module) if module.content.is_none() => Some(module.ident.to_string()),
            _ => None,
        })
        .filter(|name| name != "shared" && name != "prelude")
        .collect()
}

/// Reads the runtime ABI marker of a generated `lib.rs`.
///
/// Only one top-level `const SCHEMATIC_RUNTIME_ABI: u32` whose value is a
/// plain integer literal counts as a declaration; any other form, or a second
/// declaration, is reported as malformed rather than guessed at.
fn abi_marker(lib: &syn::File) -> AbiMarker {
    let declarations: Vec<&syn::ItemConst> = lib
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Const(constant) if constant.ident == ABI_MARKER => Some(constant),
            _ => None,
        })
        .collect();
    let constant = match declarations.as_slice() {
        [] => return AbiMarker::Missing,
        [constant] => constant,
        many => return AbiMarker::Malformed(format!("is declared {} times", many.len())),
    };
    let is_u32 = matches!(&*constant.ty, syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident("u32"));
    if !is_u32 {
        return AbiMarker::Malformed("is not a `u32`".to_string());
    }
    match &*constant.expr {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(value),
            ..
        }) if matches!(value.suffix(), "" | "u32") => match value.base10_parse::<u32>() {
            Ok(value) => AbiMarker::Declared(value),
            Err(_) => AbiMarker::Malformed("is not a `u32` value".to_string()),
        },
        _ => AbiMarker::Malformed("is not an integer literal".to_string()),
    }
}

/// Names each `pub use crate::<module>::…` of a `prelude.rs` imports, sorted,
/// keyed by module. `shared` and the WebSocket helper re-exports are included
/// under their own modules.
fn prelude_exports(prelude: &syn::File) -> BTreeMap<String, Vec<String>> {
    let mut exports: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for item in &prelude.items {
        let syn::Item::Use(item_use) = item else {
            continue;
        };
        let syn::UseTree::Path(krate) = &item_use.tree else {
            continue;
        };
        if krate.ident != "crate" {
            continue;
        }
        let syn::UseTree::Path(module) = krate.tree.as_ref() else {
            continue;
        };
        let names = exports.entry(module.ident.to_string()).or_default();
        collect_use_names(&module.tree, names);
        names.sort();
    }
    exports.remove("shared");
    for ws in ws_definition_modules() {
        exports.remove(ws.module);
    }
    exports
}

fn collect_use_names(tree: &syn::UseTree, names: &mut Vec<String>) {
    match tree {
        syn::UseTree::Name(name) => names.push(name.ident.to_string()),
        syn::UseTree::Rename(rename) => names.push(rename.ident.to_string()),
        syn::UseTree::Glob(_) => names.push("*".to_string()),
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_use_names(item, names);
            }
        }
        syn::UseTree::Path(path) => names.push(format!("{}::…", path.ident)),
    }
}

/// What the prelude exports from one module group under current definitions.
fn expected_exports(group: &[&RestApi]) -> Vec<String> {
    let mut names: Vec<String> = group
        .iter()
        .map(|api| api.name.clone())
        .collect();
    names.sort();
    names
}

/// Refuses a partial run that would change the shared runtime retained
/// modules were generated against.
///
/// With no retained module there is nothing to keep compatible. Otherwise
/// the existing `lib.rs` must record this generator's
/// [`RUNTIME_ABI`](crate::RUNTIME_ABI) (a different, missing, or malformed
/// marker requires full generation), and each of `planned`, which pairs a
/// generated shared file (`shared.rs` and the WebSocket `ws_shared.rs`) with
/// its new content, must be byte-for-byte unchanged: retained modules call
/// the request helpers in `shared.rs`, and the WebSocket modules are still
/// generated against `ws_shared.rs`. This is the single place the generator
/// decides runtime compatibility.
///
/// ## Errors
///
/// [`GeneratorError::RequiresFullGeneration`] naming what changes and the
/// retained modules.
pub(crate) fn shared_runtime_compatibility(
    root: &Path,
    planned: &[(&str, &str)],
    retained: &[String],
    abi: &AbiMarker,
) -> Result<(), GeneratorError> {
    if retained.is_empty() {
        return Ok(());
    }
    let expected = crate::RUNTIME_ABI;
    let mut changed = Vec::new();
    match abi {
        AbiMarker::Declared(found) if *found == expected => {}
        AbiMarker::Declared(found) => changed.push(format!(
            "lib.rs records schematic-runtime ABI {found}, this generator emits ABI {expected}"
        )),
        AbiMarker::Missing => changed.push(format!(
            "lib.rs records no schematic-runtime ABI, this generator emits ABI {expected}"
        )),
        AbiMarker::Malformed(reason) => {
            changed.push(format!("lib.rs's {ABI_MARKER} {reason}"));
        }
    }
    changed.extend(
        planned
            .iter()
            .filter(|(name, content)| {
                fs::read(root.join(name)).ok().as_deref() != Some(content.as_bytes())
            })
            .map(|(name, _)| name.to_string()),
    );
    if changed.is_empty() {
        Ok(())
    } else {
        Err(GeneratorError::RequiresFullGeneration {
            root: root.display().to_string(),
            changed,
            retained: retained.to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned(text: &str) -> String {
        format!("{GENERATED_FILE_HEADER}\n\n{text}")
    }

    #[test]
    fn ownership_proofs() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path();
        fs::write(dir.join("owned.rs"), owned("pub fn a() {}")).unwrap();
        fs::write(dir.join("hand.rs"), "pub fn a() {}").unwrap();
        fs::write(dir.join("short.rs"), "//").unwrap();
        fs::write(dir.join("notes.txt"), owned("")).unwrap();
        fs::write(dir.join("Cargo.toml"), format!("[package]\n{MANIFEST_OWNERSHIP_MARKER}\n")).unwrap();
        fs::write(dir.join("Other.toml"), "[package]\n").unwrap();
        fs::create_dir_all(dir.join("gen/inner")).unwrap();
        fs::write(dir.join("gen/inner/x.rs"), owned("")).unwrap();
        fs::create_dir_all(dir.join("mixed")).unwrap();
        fs::write(dir.join("mixed/x.rs"), owned("")).unwrap();
        fs::write(dir.join("mixed/keep.rs"), "fn keep() {}").unwrap();

        assert_eq!(occupant(&dir.join("owned.rs"), Proof::RustSource), Occupant::OwnedFile);
        assert!(matches!(occupant(&dir.join("hand.rs"), Proof::RustSource), Occupant::Foreign { .. }));
        assert!(matches!(occupant(&dir.join("short.rs"), Proof::RustSource), Occupant::Foreign { .. }));
        assert!(matches!(occupant(&dir.join("notes.txt"), Proof::RustSource), Occupant::Foreign { .. }));
        assert_eq!(occupant(&dir.join("Cargo.toml"), Proof::Manifest), Occupant::OwnedFile);
        assert!(matches!(occupant(&dir.join("Other.toml"), Proof::Manifest), Occupant::Foreign { .. }));
        assert_eq!(occupant(&dir.join("gen"), Proof::RustSource), Occupant::OwnedDir);
        assert!(matches!(
            occupant(&dir.join("mixed"), Proof::RustSource),
            Occupant::Foreign { is_dir: true, .. }
        ));
        assert_eq!(occupant(&dir.join("missing.rs"), Proof::RustSource), Occupant::Absent);
        assert_eq!(occupant(&dir.join("any.json"), Proof::Artifact), Occupant::Absent);
        fs::write(dir.join("any.json"), "{}").unwrap();
        assert_eq!(occupant(&dir.join("any.json"), Proof::Artifact), Occupant::OwnedFile);
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_links_are_never_followed() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path();
        fs::write(dir.join("target.rs"), owned("")).unwrap();
        std::os::unix::fs::symlink(dir.join("target.rs"), dir.join("link.rs")).unwrap();
        fs::create_dir(dir.join("tree")).unwrap();
        std::os::unix::fs::symlink(dir.join("target.rs"), dir.join("tree/link.rs")).unwrap();
        assert!(matches!(occupant(&dir.join("link.rs"), Proof::RustSource), Occupant::Foreign { .. }));
        assert!(matches!(occupant(&dir.join("tree"), Proof::RustSource), Occupant::Foreign { .. }));
    }

    #[test]
    fn compatibility_passes_identical_runtime_and_refuses_a_change() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::write(root.join("shared.rs"), "same").unwrap();
        fs::write(root.join("ws_shared.rs"), "ws").unwrap();
        let retained = vec!["beta".to_string()];

        let current = AbiMarker::Declared(crate::RUNTIME_ABI);

        shared_runtime_compatibility(root, &[("shared.rs", "same"), ("ws_shared.rs", "ws")], &retained, &current)
            .expect("identical runtime is compatible");
        shared_runtime_compatibility(root, &[("shared.rs", "new")], &[], &AbiMarker::Missing)
            .expect("nothing retained, nothing to keep compatible");

        let err = shared_runtime_compatibility(
            root,
            &[("shared.rs", "new"), ("ws_shared.rs", "ws")],
            &retained,
            &current,
        )
        .unwrap_err();
        let GeneratorError::RequiresFullGeneration { changed, retained, .. } = &err else {
            panic!("expected RequiresFullGeneration, got {err:?}");
        };
        assert_eq!(changed, &["shared.rs"]);
        assert_eq!(retained, &["beta"]);
        assert!(err.to_string().contains("generate --api all"), "{err}");

        fs::remove_file(root.join("ws_shared.rs")).unwrap();
        let err = shared_runtime_compatibility(root, &[("ws_shared.rs", "ws")], &["beta".into()], &current)
            .unwrap_err();
        assert!(matches!(err, GeneratorError::RequiresFullGeneration { .. }));
    }

    #[test]
    fn an_abi_marker_other_than_the_generators_requires_full_generation() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::write(root.join("shared.rs"), "same").unwrap();
        let planned = [("shared.rs", "same")];
        let retained = vec!["beta".to_string()];

        for marker in [
            AbiMarker::Declared(crate::RUNTIME_ABI + 1),
            AbiMarker::Missing,
            AbiMarker::Malformed("is not an integer literal".to_string()),
        ] {
            let err = shared_runtime_compatibility(root, &planned, &retained, &marker).unwrap_err();
            let GeneratorError::RequiresFullGeneration { changed, .. } = &err else {
                panic!("{marker:?}: expected RequiresFullGeneration, got {err:?}");
            };
            assert_eq!(changed.len(), 1, "{marker:?}: {changed:?}");
            assert!(changed[0].starts_with("lib.rs"), "{marker:?}: {changed:?}");
        }
        shared_runtime_compatibility(root, &planned, &[], &AbiMarker::Missing)
            .expect("with nothing retained, the marker does not matter");
    }

    #[test]
    fn the_abi_marker_is_read_only_from_its_exact_form() {
        let read = |source: &str| abi_marker(&syn::parse_file(source).unwrap());
        assert_eq!(read("pub const SCHEMATIC_RUNTIME_ABI: u32 = 7;"), AbiMarker::Declared(7));
        assert_eq!(read("pub const SCHEMATIC_RUNTIME_ABI: u32 = 7u32;"), AbiMarker::Declared(7));
        assert_eq!(read("pub mod shared;"), AbiMarker::Missing);
        assert_eq!(read("pub const OTHER: u32 = 1;"), AbiMarker::Missing);
        for (source, reason) in [
            ("pub const SCHEMATIC_RUNTIME_ABI: u32 = 1 + 1;", "is not an integer literal"),
            ("pub const SCHEMATIC_RUNTIME_ABI: u32 = \"1\";", "is not an integer literal"),
            ("pub const SCHEMATIC_RUNTIME_ABI: u64 = 1;", "is not a `u32`"),
            ("pub const SCHEMATIC_RUNTIME_ABI: u32 = 1u8;", "is not an integer literal"),
            ("pub const SCHEMATIC_RUNTIME_ABI: u32 = 99999999999;", "is not a `u32` value"),
            (
                "pub const SCHEMATIC_RUNTIME_ABI: u32 = 1;\nconst SCHEMATIC_RUNTIME_ABI: u32 = 1;",
                "is declared 2 times",
            ),
        ] {
            assert_eq!(read(source), AbiMarker::Malformed(reason.to_string()), "{source}");
        }
    }
}
