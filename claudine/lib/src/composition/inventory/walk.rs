//! The pure walk and member checks of Claudine's registration inventory,
//! `claudine/registration/`.
//!
//! This file is compiled twice: by `claudine/lib/build.rs` (through
//! `#[path]`), which refuses the build when the folder is not a valid
//! inventory and generates the embedded include table from [`walk`]'s
//! result, and by the library ([`super`]), whose tests drive the same
//! functions over temporary folders. It therefore depends on `std` and
//! `serde_yaml_ng` only.
//!
//! The checks here are structural: what a member file must be for the
//! inventory to be well formed. Darkmatter's registration loader still
//! judges every member's full envelope and payload when Claudine's
//! generator prepares the inventory, and again when a published bundle is
//! loaded.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// What a member file is, by its own `kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemberKind {
    /// `kind: schema-trigger`: selects and types Markdown documents.
    Trigger,
    /// `kind: yaml-schema`: defines a YAML document kind.
    YamlSchema,
    /// `kind: namespace-schema`: declares one expression global.
    Namespace,
    /// `kind: schema` with `types` and no `$schema`: reusable types.
    Library,
}

impl MemberKind {
    /// The `kind` value a member of this kind declares.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Trigger => "schema-trigger",
            Self::YamlSchema => "yaml-schema",
            Self::Namespace => "namespace-schema",
            Self::Library => "schema",
        }
    }

    /// The variant's name, as the generated include table spells it.
    #[must_use]
    pub const fn variant(self) -> &'static str {
        match self {
            Self::Trigger => "Trigger",
            Self::YamlSchema => "YamlSchema",
            Self::Namespace => "Namespace",
            Self::Library => "Library",
        }
    }
}

/// One member found by [`walk`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkedMember {
    /// The path relative to the inventory root, written with `/`
    /// (`action.yaml`, `nested/types.yaml`).
    pub path: String,
    /// The file that was read.
    pub file: PathBuf,
    /// Its exact text.
    pub text: String,
    #[allow(missing_docs)]
    pub kind: MemberKind,
    /// Its display `name`, when it declares one.
    pub name: Option<String>,
}

/// Why the inventory is not well formed, at one path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryError {
    /// The relative path the problem is at (`""` for the root itself).
    pub path: String,
    #[allow(missing_docs)]
    pub message: String,
}

impl std::fmt::Display for InventoryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.path.is_empty() {
            write!(formatter, "{}", self.message)
        } else {
            write!(formatter, "`{}`: {}", self.path, self.message)
        }
    }
}

/// Every member of the inventory at `root`, sorted by [`WalkedMember::path`]
/// (byte order), and every directory visited (the root first), for build
/// rerun tracking.
///
/// ## Errors
///
/// Every problem found, in path order: an unreadable entry, a symlink or
/// other reparse point, a file that is not `.yaml`, a nonportable path
/// component, two paths that differ only in letter case, a file that is not
/// UTF-8, and an ill-formed member envelope ([`check_member`]). A missing or
/// empty root is refused too.
pub fn walk(root: &Path) -> Result<(Vec<WalkedMember>, Vec<PathBuf>), Vec<InventoryError>> {
    let mut errors = Vec::new();
    let mut members = Vec::new();
    let mut directories = Vec::new();
    match std::fs::symlink_metadata(root) {
        Ok(metadata) if is_link(&metadata) => {
            errors.push(error("", format!("the inventory root `{}` is a symlink or junction", root.display())));
        }
        Ok(metadata) if metadata.is_dir() => visit(root, "", &mut members, &mut directories, &mut errors),
        Ok(_) => errors.push(error("", format!("the inventory root `{}` is not a directory", root.display()))),
        Err(cause) => errors.push(error("", format!("the inventory root `{}` cannot be read: {cause}", root.display()))),
    }
    if errors.is_empty() && members.is_empty() {
        errors.push(error("", format!("the inventory root `{}` holds no member", root.display())));
    }
    let paths: Vec<String> = members.iter().map(|member: &WalkedMember| member.path.clone()).collect();
    errors.extend(case_collisions(&paths));
    if !errors.is_empty() {
        errors.sort_by(|left, right| left.path.cmp(&right.path).then_with(|| left.message.cmp(&right.message)));
        return Err(errors);
    }
    members.sort_by(|left, right| left.path.cmp(&right.path));
    Ok((members, directories))
}

fn visit(
    directory: &Path,
    prefix: &str,
    members: &mut Vec<WalkedMember>,
    directories: &mut Vec<PathBuf>,
    errors: &mut Vec<InventoryError>,
) {
    directories.push(directory.to_path_buf());
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(cause) => {
            errors.push(error(prefix.trim_end_matches('/'), format!("the directory cannot be listed: {cause}")));
            return;
        }
    };
    // `read_dir` order is the file system's; collect and sort so errors and
    // members never depend on it.
    let mut listed: Vec<(String, PathBuf)> = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => match entry.file_name().into_string() {
                Ok(name) => listed.push((name, entry.path())),
                Err(raw) => errors.push(error(
                    &format!("{prefix}{}", raw.to_string_lossy()),
                    "the file name is not UTF-8".to_string(),
                )),
            },
            Err(cause) => errors.push(error(prefix.trim_end_matches('/'), format!("an entry cannot be read: {cause}"))),
        }
    }
    listed.sort();
    for (name, path) in listed {
        let relative = format!("{prefix}{name}");
        if let Err(message) = check_portable_component(&name) {
            errors.push(error(&relative, message));
            continue;
        }
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(cause) => {
                errors.push(error(&relative, format!("the entry cannot be read: {cause}")));
                continue;
            }
        };
        if is_link(&metadata) {
            errors.push(error(&relative, "a symlink or junction is not an inventory member".to_string()));
        } else if metadata.is_dir() {
            visit(&path, &format!("{relative}/"), members, directories, errors);
        } else if !metadata.is_file() {
            errors.push(error(&relative, "only regular files and directories belong to the inventory".to_string()));
        } else if !name.ends_with(".yaml") {
            errors.push(error(&relative, "only `.yaml` files belong to the inventory".to_string()));
        } else {
            match std::fs::read(&path) {
                Ok(bytes) => match check_member(&bytes) {
                    Ok((text, kind, label)) => {
                        members.push(WalkedMember { path: relative, file: path, text, kind, name: label });
                    }
                    Err(message) => errors.push(error(&relative, message)),
                },
                Err(cause) => errors.push(error(&relative, format!("the file cannot be read: {cause}"))),
            }
        }
    }
}

fn error(path: &str, message: String) -> InventoryError {
    InventoryError { path: path.to_string(), message }
}

/// Whether `metadata` (from `symlink_metadata`) is a symlink, or on Windows
/// any reparse point (a junction is not reported as a symlink there).
fn is_link(metadata: &std::fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    false
}

/// The Windows device names, which no file or directory may be called on
/// any platform the inventory is published to (with or without an
/// extension).
const RESERVED_STEMS: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8", "com9", "lpt1", "lpt2",
    "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Checks that `name`, one path component, means the same file on macOS,
/// Linux, and Windows: ASCII letters, digits, `.`, `_`, and `-`; not
/// starting with `.` or `-`; not ending with `.`; and not a Windows device
/// name.
///
/// ## Errors
///
/// Why the component is not portable.
pub fn check_portable_component(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("an empty path component is not portable".to_string());
    }
    if let Some(bad) = name.chars().find(|c| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))) {
        return Err(format!("`{name}` is not a portable name: `{bad}` is outside ASCII letters, digits, `.`, `_`, and `-`"));
    }
    if name.starts_with('.') || name.starts_with('-') {
        return Err(format!("`{name}` is not a portable name: it starts with `{}`", &name[..1]));
    }
    if name.ends_with('.') {
        return Err(format!("`{name}` is not a portable name: Windows drops a trailing `.`"));
    }
    let stem = name.split('.').next().unwrap_or(name).to_ascii_lowercase();
    if RESERVED_STEMS.contains(&stem.as_str()) {
        return Err(format!("`{name}` is not a portable name: `{stem}` is a Windows device name"));
    }
    Ok(())
}

/// One error per path whose spelling another path matches when letter case
/// is ignored, at any depth (`Types/a.yaml` and `types/a.yaml` collide on
/// `types`). `paths` are relative, written with `/`.
#[must_use]
pub fn case_collisions(paths: &[String]) -> Vec<InventoryError> {
    let mut by_folded: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for path in paths {
        let mut prefix = String::new();
        for component in path.split('/') {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(component);
            let spellings = by_folded.entry(prefix.to_ascii_lowercase()).or_default();
            if !spellings.contains(&prefix) {
                spellings.push(prefix.clone());
            }
        }
    }
    let mut errors = Vec::new();
    for spellings in by_folded.values().filter(|spellings| spellings.len() > 1) {
        for spelling in spellings {
            let others: Vec<String> =
                spellings.iter().filter(|other| *other != spelling).map(|other| format!("`{other}`")).collect();
            errors.push(error(
                spelling,
                format!("collides with {} on a case-insensitive file system", others.join(", ")),
            ));
        }
    }
    errors.sort_by(|left, right| left.path.cmp(&right.path));
    errors
}

/// Checks one member's bytes: UTF-8 text holding one YAML mapping whose
/// `kind` is an allowed member kind with the fields that kind needs.
/// Returns the text, the kind, and the optional display `name`.
///
/// | `kind` | Requires |
/// |---|---|
/// | `schema-trigger` | `match` |
/// | `yaml-schema` | a `$schema` mapping |
/// | `namespace-schema` | a string `namespace` and a `$schema` mapping |
/// | `schema` | a non-empty `types` mapping and no `$schema` |
///
/// A `name`, when present, is a kebab-case identifier; a `description`,
/// when present, is a string.
///
/// ## Errors
///
/// Why the bytes are not a well-formed member.
pub fn check_member(bytes: &[u8]) -> Result<(String, MemberKind, Option<String>), String> {
    let text = std::str::from_utf8(bytes).map_err(|cause| format!("the file is not UTF-8 text: {cause}"))?;
    let value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).map_err(|cause| format!("the file is not one YAML document: {cause}"))?;
    let serde_yaml_ng::Value::Mapping(mapping) = &value else {
        return Err("the file is not a YAML mapping".to_string());
    };
    let field = |key: &str| mapping.get(serde_yaml_ng::Value::String(key.to_string()));
    let kind = match field("kind") {
        Some(serde_yaml_ng::Value::String(kind)) => kind.as_str(),
        Some(_) => return Err("its `kind` is not a string".to_string()),
        None => return Err(format!("it declares no `kind`; a member is {ALLOWED}")),
    };
    let is_mapping = |key: &str| matches!(field(key), Some(serde_yaml_ng::Value::Mapping(_)));
    let kind = match kind {
        "schema-trigger" => {
            if field("match").is_none() {
                return Err("a `schema-trigger` declares `match`".to_string());
            }
            MemberKind::Trigger
        }
        "yaml-schema" => {
            if !is_mapping("$schema") {
                return Err("a `yaml-schema` declares a `$schema` mapping".to_string());
            }
            MemberKind::YamlSchema
        }
        "namespace-schema" => {
            if !matches!(field("namespace"), Some(serde_yaml_ng::Value::String(_))) {
                return Err("a `namespace-schema` declares a string `namespace`".to_string());
            }
            if !is_mapping("$schema") {
                return Err("a `namespace-schema` declares a `$schema` mapping".to_string());
            }
            MemberKind::Namespace
        }
        "schema" => {
            if field("$schema").is_some() {
                return Err("a `kind: schema` member is a types-only library and may not declare `$schema`".to_string());
            }
            match field("types") {
                Some(serde_yaml_ng::Value::Mapping(types)) if !types.is_empty() => MemberKind::Library,
                _ => return Err("a `kind: schema` member declares a non-empty `types` mapping".to_string()),
            }
        }
        other => return Err(format!("its `kind` is `{other}`; a member is {ALLOWED}")),
    };
    if field("description").is_some_and(|description| !matches!(description, serde_yaml_ng::Value::String(_))) {
        return Err("its `description` is not a string".to_string());
    }
    let name = match field("name") {
        None => None,
        Some(serde_yaml_ng::Value::String(name)) if is_kebab_case(name) => Some(name.clone()),
        Some(serde_yaml_ng::Value::String(name)) => {
            return Err(format!("its `name` `{name}` is not a kebab-case identifier"));
        }
        Some(_) => return Err("its `name` is not a string".to_string()),
    };
    Ok((text.to_string(), kind, name))
}

const ALLOWED: &str = "a `schema-trigger`, `yaml-schema`, `namespace-schema`, or types-only `schema`";

/// Lowercase letters, digits, and inner single dashes, starting with a
/// letter (`inline-compose-schema`).
fn is_kebab_case(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_lowercase())
        && !name.ends_with('-')
        && !name.contains("--")
        && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}
