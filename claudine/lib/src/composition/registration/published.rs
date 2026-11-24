//! The files a publish writes, compiled into the binary, and the manifest
//! listing them.

use std::borrow::Cow;
use std::sync::OnceLock;

use crate::composition::inventory::{self, MemberKind};

use super::{HOST_ID, MANIFEST_FILE, TEMP_PREFIX};

/// What a published file is. [`Self::as_str`] is the spelling the
/// registration listing uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FileKind {
    /// The `darkmatter-host` manifest.
    Manifest,
    /// `kind: schema-trigger`.
    Trigger,
    /// `kind: schema` with only `types`.
    Library,
    /// `kind: yaml-schema`.
    YamlSchema,
    /// `kind: namespace-schema`.
    Namespace,
    /// `kind: expression-schema`: a host expression catalog.
    ExpressionSchema,
    /// The generated lifecycle artifact.
    Lifecycle,
}

impl FileKind {
    /// `manifest`, `schema-trigger`, `schema`, `yaml-schema`,
    /// `namespace-schema`, `expression-schema`, or `lifecycle`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Manifest => "manifest",
            Self::Trigger => MemberKind::Trigger.as_str(),
            Self::Library => MemberKind::Library.as_str(),
            Self::YamlSchema => MemberKind::YamlSchema.as_str(),
            Self::Namespace => MemberKind::Namespace.as_str(),
            Self::ExpressionSchema => MemberKind::ExpressionSchema.as_str(),
            Self::Lifecycle => "lifecycle",
        }
    }
}

impl From<MemberKind> for FileKind {
    fn from(kind: MemberKind) -> Self {
        match kind {
            MemberKind::Trigger => Self::Trigger,
            MemberKind::Library => Self::Library,
            MemberKind::YamlSchema => Self::YamlSchema,
            MemberKind::Namespace => Self::Namespace,
            MemberKind::ExpressionSchema => Self::ExpressionSchema,
        }
    }
}

/// One file of a [`RegistrationInventory`], with the exact bytes published.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedFile {
    path: Cow<'static, str>,
    kind: FileKind,
    name: Option<Cow<'static, str>>,
    bytes: Cow<'static, [u8]>,
}

#[allow(missing_docs)]
impl PublishedFile {
    /// A file at `path` (relative to the output root, `/`-separated).
    #[must_use]
    pub fn new(
        path: impl Into<Cow<'static, str>>,
        kind: FileKind,
        name: Option<impl Into<Cow<'static, str>>>,
        bytes: impl Into<Cow<'static, [u8]>>,
    ) -> Self {
        Self { path: path.into(), kind, name: name.map(Into::into), bytes: bytes.into() }
    }

    /// Its path relative to the output root, written with `/`
    /// (`schemas/seq.yaml`).
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub const fn kind(&self) -> FileKind {
        self.kind
    }

    /// Its display `name`, when it declares one.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Its text; every Claudine registration file is UTF-8.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        std::str::from_utf8(&self.bytes).ok()
    }
}

/// The complete published set: the manifest, every member, and the
/// lifecycle artifact, sorted by [`PublishedFile::path`] (byte order).
///
/// The manifest is derived from the rest whenever the set changes, so it
/// always lists exactly the other files with their digests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrationInventory {
    version: String,
    files: Vec<PublishedFile>,
}

impl RegistrationInventory {
    /// The inventory compiled into this binary
    /// ([`crate::composition::inventory`]), built once per process.
    #[must_use]
    pub fn claudine() -> &'static Self {
        static INVENTORY: OnceLock<RegistrationInventory> = OnceLock::new();
        INVENTORY.get_or_init(|| {
            let members = inventory::members().iter().map(|member| {
                PublishedFile::new(member.bundle_path(), member.kind.into(), member.name, member.bytes())
            });
            let lifecycle = PublishedFile::new(
                inventory::LIFECYCLE_BUNDLE_PATH,
                FileKind::Lifecycle,
                None::<&'static str>,
                inventory::LIFECYCLE_ARTIFACT,
            );
            Self::from_members(env!("CARGO_PKG_VERSION"), members.chain([lifecycle]))
        })
    }

    /// An inventory of `members` (anything but the manifest, which is
    /// derived) under host release `version`.
    #[must_use]
    pub fn from_members(version: impl Into<String>, members: impl IntoIterator<Item = PublishedFile>) -> Self {
        let mut inventory = Self { version: version.into(), files: members.into_iter().collect() };
        inventory.rebuild_manifest();
        inventory
    }

    /// This inventory under another host release.
    #[must_use]
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self.rebuild_manifest();
        self
    }

    /// This inventory with `file` replacing the member at its path, or added.
    #[must_use]
    pub fn with_member(mut self, file: PublishedFile) -> Self {
        self.files.retain(|existing| existing.path != file.path);
        self.files.push(file);
        self.rebuild_manifest();
        self
    }

    /// This inventory without the member at `path`.
    #[must_use]
    pub fn without_member(mut self, path: &str) -> Self {
        self.files.retain(|existing| existing.path != path);
        self.rebuild_manifest();
        self
    }

    /// The host release the manifest records as `version`.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Every published file, the manifest included, in path order.
    #[must_use]
    pub fn files(&self) -> &[PublishedFile] {
        &self.files
    }

    /// The file published at `path` (`manifest.yaml`, `schemas/seq.yaml`).
    #[must_use]
    pub fn file(&self, path: &str) -> Option<&PublishedFile> {
        self.files.iter().find(|file| file.path == path)
    }

    /// The manifest.
    ///
    /// ## Panics
    ///
    /// Never: every constructor derives it.
    #[must_use]
    pub fn manifest(&self) -> &PublishedFile {
        self.file(MANIFEST_FILE).expect("every inventory derives its manifest")
    }

    /// Every file but the manifest, in path order.
    pub fn members(&self) -> impl Iterator<Item = &PublishedFile> {
        self.files.iter().filter(|file| file.kind != FileKind::Manifest)
    }

    fn rebuild_manifest(&mut self) {
        self.files.retain(|file| file.kind != FileKind::Manifest && file.path != MANIFEST_FILE);
        let text = manifest_text(&self.version, &self.files);
        self.files.push(PublishedFile::new(MANIFEST_FILE, FileKind::Manifest, None::<&'static str>, text.into_bytes()));
        self.files.sort_by(|left, right| left.path.cmp(&right.path));
    }

    /// The first path the publisher cannot write as given, and why.
    pub(super) fn invalid_path(&self) -> Option<(&str, String)> {
        let mut lifecycles = 0;
        for file in &self.files {
            if let Err(why) = portable_path(&file.path) {
                return Some((&file.path, why));
            }
            if file.kind == FileKind::Lifecycle {
                lifecycles += 1;
            }
            let nested = format!("{}/", file.path);
            if self.files.iter().any(|other| other.path.starts_with(&nested)) {
                return Some((&file.path, "is both a file and the folder of another file".to_string()));
            }
        }
        (lifecycles != 1).then(|| (MANIFEST_FILE, format!("lists {lifecycles} lifecycle artifacts; it lists one")))
    }
}

/// The format 2 manifest for `members`: `files` lists every schema member in
/// path order, `lifecycle` the lifecycle artifact, and `digests` each by its
/// reference.
fn manifest_text(version: &str, members: &[PublishedFile]) -> String {
    let reference = |file: &PublishedFile| quoted(&format!("./{}", file.path));
    let mut members: Vec<&PublishedFile> = members.iter().collect();
    members.sort_by(|left, right| left.path.cmp(&right.path));
    let files: String = members
        .iter()
        .filter(|file| file.kind != FileKind::Lifecycle)
        .map(|file| format!("    - {}\n", reference(file)))
        .collect();
    let lifecycle = members
        .iter()
        .find(|file| file.kind == FileKind::Lifecycle)
        .map_or_else(|| quoted(&format!("./{}", inventory::LIFECYCLE_BUNDLE_PATH)), |file| reference(file));
    let digests: String = members
        .iter()
        .map(|file| format!("    {}: {}\n", reference(file), quoted(&blake3_hex(&file.bytes))))
        .collect();
    let files = if files.is_empty() { " []\n".to_string() } else { format!("\n{files}") };
    format!(
        "# Written by Claudine from the registration compiled into it, and rewritten when that changes; do not edit.\n\
         kind: darkmatter-host\n\
         format_version: 3\n\
         host: {HOST_ID}\n\
         version: {version}\n\
         files:{files}\
         lifecycle: {lifecycle}\n\
         digests:\n{digests}",
        version = quoted(version),
    )
}

/// The BLAKE3 hash of `bytes` as 64 lowercase hexadecimal digits.
pub(super) fn blake3_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    biscuit_hash::blake3_hash_bytes(bytes).iter().fold(String::with_capacity(64), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    })
}

/// A JSON double-quoted scalar, which YAML reads as the same string.
fn quoted(text: &str) -> String {
    serde_json::Value::String(text.to_string()).to_string()
}

/// Whether `path` is a relative `/`-separated path every supported file
/// system stores unchanged and Darkmatter's manifest reader accepts: names
/// of ASCII letters, digits, `.`, `_`, and `-`, none `.`/`..`, ending in
/// `.`, a Windows device name, or starting with [`TEMP_PREFIX`].
pub(super) fn portable_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("is empty".to_string());
    }
    for name in path.split('/') {
        if name.is_empty() || name == "." || name == ".." {
            return Err(format!("has the segment `{name}`"));
        }
        if !name.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')) {
            return Err(format!("`{name}` uses a character outside `A-Z`, `a-z`, `0-9`, `.`, `_`, and `-`"));
        }
        if name.ends_with('.') {
            return Err(format!("`{name}` ends with `.`, which Windows drops"));
        }
        let stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
        let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || ((stem.starts_with("COM") || stem.starts_with("LPT"))
                && stem.len() == 4
                && matches!(stem.as_bytes()[3], b'1'..=b'9'));
        if device {
            return Err(format!("`{name}` is a Windows device name"));
        }
        if name.starts_with(TEMP_PREFIX) {
            return Err(format!("`{name}` starts with the publisher's temporary prefix `{TEMP_PREFIX}`"));
        }
    }
    Ok(())
}
