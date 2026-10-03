//! Lexical path identity: one definition of whole-component prefix comparison
//! and relative-route computation.
//!
//! The rules are those of the "Path identity and text" section of the
//! file-references topic page: collapse `.` and `..` on ordinary paths without
//! walking above a root, keep literal dot segments under a Windows verbatim
//! prefix, treat different drives and shares as separate roots, and never
//! canonicalize, case-fold names, or equate symlink aliases.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// A lexical, lossless identity for a path, used for prefix tests and
/// relative-route arithmetic.
///
/// An identity is never rendered as reference text and never turned back into
/// a native spelling; render generated text through
/// [`try_portable_string`](crate::try_portable_string) instead.
///
/// Two identities are equal when they name the same location by the lexical
/// rules below. Conservative inequality (failing to recognize an alias) is
/// acceptable; equating two different files is not.
///
/// - **Ordinary paths:** `.` is dropped and `..` removes the preceding name. A
///   `..` at a root is dropped (`/..` is `/`); on a relative path it is kept as
///   a leading parent hop (`a/../../b` is `../b`).
/// - **Windows roots:** a drive letter is case-insensitive (`c:\x` equals
///   `C:\x`), and a verbatim drive or share equals its legacy spelling
///   (`\\?\C:\x` equals `C:\x`; `\\?\UNC\server\share\x` equals
///   `\\server\share\x`). This holds even when the whole verbatim path is too
///   long to be reduced, so a long descendant stays inside a short root.
///   Device (`\\.\`) and other verbatim (`\\?\Volume{…}`) prefixes keep their
///   own text. `C:\a` and the drive-relative `C:a` differ.
/// - **Verbatim names:** under `\\?\`, `.` and `..` are ordinary directory
///   names and `/` is not a separator, so they are kept literally.
/// - **Names:** compared exactly, as raw platform units. Names are not
///   case-folded, and non-Unicode names stay distinct.
///
/// ## Examples
///
/// ```
/// use std::path::Path;
/// use biscuit_file::PathIdentity;
///
/// let root = PathIdentity::new(Path::new("/opt/config"));
/// assert!(PathIdentity::new(Path::new("/opt/config/app.toml")).starts_with(&root));
/// assert!(!PathIdentity::new(Path::new("/opt/config-old/app.toml")).starts_with(&root));
///
/// let target = PathIdentity::new(Path::new("/repo/assets/logo.png"));
/// let route = target.relative_from(&PathIdentity::new(Path::new("/repo/docs/guide"))).unwrap();
/// assert_eq!(route.parent_hops(), 2);
/// assert_eq!(route.to_path_buf(), Path::new("../../assets/logo.png"));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PathIdentity {
    /// The namespace-independent root: empty, `C:`, `\\server\share`, or the
    /// raw text of a device or other verbatim prefix.
    root: OsString,
    /// Whether the path is anchored at its root (`/a`, `C:\a`, every UNC and
    /// verbatim path), as opposed to relative (`a`) or drive-relative (`C:a`).
    rooted: bool,
    /// `..` hops a relative path keeps above its first name.
    leading_parents: usize,
    components: Vec<OsString>,
}

impl PathIdentity {
    /// Build the identity of `path` by the host's path grammar.
    pub fn new(path: &Path) -> Self {
        #[cfg(windows)]
        {
            use std::os::windows::ffi::{OsStrExt, OsStringExt};

            let units: Vec<u16> = path.as_os_str().encode_wide().collect();
            let parts = windows::parse(&units);
            Self {
                root: OsString::from_wide(&parts.root),
                rooted: parts.rooted,
                leading_parents: parts.leading_parents,
                components: parts
                    .components
                    .iter()
                    .map(|name| OsString::from_wide(name))
                    .collect(),
            }
        }
        #[cfg(not(windows))]
        {
            unix_identity(path)
        }
    }

    /// The names below the root, after normalization.
    pub fn components(&self) -> &[OsString] {
        &self.components
    }

    /// Whether the path has no root and no leading `..` hops: plain names
    /// below wherever it is read from.
    pub(crate) fn is_unanchored(&self) -> bool {
        self.root.is_empty() && !self.rooted && self.leading_parents == 0
    }

    /// Whether `base` is a whole-component prefix of `self` (equal paths
    /// included). `/opt/config-old` does not start with `/opt/config`.
    pub fn starts_with(&self, base: &PathIdentity) -> bool {
        self.root == base.root
            && self.rooted == base.rooted
            && self.leading_parents == base.leading_parents
            && self.components.starts_with(&base.components)
    }

    /// The names of `self` below `base`, or `None` when `base` is not a
    /// prefix. Empty when the two are equal.
    pub fn strip_prefix(&self, base: &PathIdentity) -> Option<&[OsString]> {
        self.starts_with(base)
            .then(|| &self.components[base.components.len()..])
    }

    /// The route from the directory `dir` to `self`.
    ///
    /// `dir` is always read as a directory; no file-versus-directory guess is
    /// made from an extension.
    ///
    /// ## Returns
    ///
    /// `None` when no lexical route exists: the two have different roots
    /// (another drive or share, or one rooted and one not), or `dir` keeps more
    /// leading `..` hops than `self`, so the route would have to name a
    /// directory the path does not spell.
    pub fn relative_from(&self, dir: &PathIdentity) -> Option<RelativeRoute> {
        if self.root != dir.root
            || self.rooted != dir.rooted
            || dir.leading_parents > self.leading_parents
        {
            return None;
        }
        // Names below different numbers of leading hops are relative to
        // different directories, so none of them are shared.
        let common = if self.leading_parents == dir.leading_parents {
            self.components
                .iter()
                .zip(&dir.components)
                .take_while(|(target, base)| target == base)
                .count()
        } else {
            0
        };
        Some(RelativeRoute {
            parent_hops: dir.components.len() - common
                + (self.leading_parents - dir.leading_parents),
            forward: self.components[common..].to_vec(),
        })
    }
}

#[cfg(test)]
impl PathIdentity {
    /// The identity a Windows host would build for `text`, on any host.
    pub(crate) fn from_windows_text(text: &str) -> Self {
        let units: Vec<u16> = text.encode_utf16().collect();
        let parts = windows::parse(&units);
        let text = |units: &[u16]| OsString::from(String::from_utf16_lossy(units));
        Self {
            root: text(&parts.root),
            rooted: parts.rooted,
            leading_parents: parts.leading_parents,
            components: parts.components.iter().map(|name| text(name)).collect(),
        }
    }
}

impl From<&Path> for PathIdentity {
    fn from(path: &Path) -> Self {
        Self::new(path)
    }
}

/// Keep the first item for each [`PathIdentity`], in input order.
///
/// The one ordering rule behind candidate and `@` root dedupe: a later item
/// whose identity was already seen is dropped, so the earlier item keeps its
/// provenance and its own path spelling. Taking identities rather than paths
/// lets every host test the Windows equalities (verbatim versus legacy, drive
/// case, mixed separators) through this function.
pub(crate) fn first_seen_by_identity<T>(
    items: impl IntoIterator<Item = T>,
    identity: impl Fn(&T) -> PathIdentity,
) -> Vec<T> {
    let mut seen = std::collections::HashSet::new();
    items
        .into_iter()
        .filter(|item| seen.insert(identity(item)))
        .collect()
}

/// A relative route: generated parent hops followed by names copied from the
/// target.
///
/// The halves are kept apart so a generated `..` hop stays distinct from a
/// literal `..` name, which a Windows verbatim path can contain; joining them
/// into one path would make the two indistinguishable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelativeRoute {
    parent_hops: usize,
    forward: Vec<OsString>,
}

impl RelativeRoute {
    /// The number of `..` hops before the forward names.
    pub fn parent_hops(&self) -> usize {
        self.parent_hops
    }

    /// The names below the point where the two paths diverge.
    pub fn forward(&self) -> &[OsString] {
        &self.forward
    }

    /// The route as a native relative path, or `.` when it is empty.
    ///
    /// Joining loses the hop/name distinction described on the type, so use
    /// this only where no literal `..` name can occur.
    pub fn to_path_buf(&self) -> PathBuf {
        let mut path = PathBuf::new();
        for _ in 0..self.parent_hops {
            path.push("..");
        }
        for name in &self.forward {
            path.push(name);
        }
        if path.as_os_str().is_empty() {
            path.push(".");
        }
        path
    }
}

/// One parsed segment, before collapsing.
enum Step<S> {
    Current,
    Parent,
    Name(S),
}

/// Apply one segment to the names collected so far.
fn apply<S>(components: &mut Vec<S>, leading_parents: &mut usize, rooted: bool, step: Step<S>) {
    match step {
        Step::Current => {}
        Step::Parent => {
            if components.pop().is_none() && !rooted {
                *leading_parents += 1;
            }
        }
        Step::Name(name) => components.push(name),
    }
}

/// Collapse `.` and `..` by the [`PathIdentity`] rules while keeping the native
/// spelling of the prefix, root, and names.
///
/// This is the one native-path normalization behind resolution, context
/// selection, and containment, so they cannot disagree with identity
/// comparison: a `..` at a root or drive root is dropped (`/../a` is `/a`), a
/// relative path keeps its leading `..` hops, and under a Windows `\\?\` prefix
/// dot segments are literal names and are kept. No verbatim-prefix reduction
/// happens here.
pub(crate) fn normalize_native(path: &Path) -> PathBuf {
    use std::path::Component;

    let rooted = path.has_root();
    let mut head: Vec<Component<'_>> = Vec::new();
    let mut names: Vec<Component<'_>> = Vec::new();
    let mut leading_parents = 0;
    let mut verbatim = false;
    for component in path.components() {
        let step = match component {
            Component::Prefix(prefix) => {
                verbatim = prefix.kind().is_verbatim();
                head.push(component);
                Step::Current
            }
            Component::RootDir => {
                head.push(component);
                Step::Current
            }
            // `Path::components` reports `.`/`..` under `\\?\`, but Win32 reads
            // them there as ordinary directory names.
            _ if verbatim => Step::Name(component),
            Component::CurDir => Step::Current,
            Component::ParentDir => Step::Parent,
            Component::Normal(_) => Step::Name(component),
        };
        apply(&mut names, &mut leading_parents, rooted, step);
    }
    // Assembled as text: `PathBuf::push` drops `.`/`..` onto a verbatim
    // buffer, which would undo the literal names kept above.
    let mut text = OsString::new();
    for component in &head {
        text.push(component.as_os_str());
    }
    let tail = std::iter::repeat_n(Component::ParentDir, leading_parents).chain(names);
    for (index, component) in tail.enumerate() {
        if index > 0 {
            text.push(std::path::MAIN_SEPARATOR_STR);
        }
        text.push(component.as_os_str());
    }
    PathBuf::from(text)
}

/// Off Windows, [`Path::components`] is faithful and lossless: `/` is the only
/// separator and `.`/`..` are never literal names.
#[cfg(not(windows))]
fn unix_identity(path: &Path) -> PathIdentity {
    use std::path::Component;

    let rooted = path.has_root();
    let mut components = Vec::new();
    let mut leading_parents = 0;
    for component in path.components() {
        let step = match component {
            Component::Prefix(_) | Component::RootDir | Component::CurDir => Step::Current,
            Component::ParentDir => Step::Parent,
            Component::Normal(name) => Step::Name(name.to_os_string()),
        };
        apply(&mut components, &mut leading_parents, rooted, step);
    }
    PathIdentity {
        root: OsString::new(),
        rooted,
        leading_parents,
        components,
    }
}

/// The Windows grammar over UTF-16 units.
///
/// Compiled on every host so the Windows rules are tested everywhere; only
/// Windows builds call it on real paths. The prefix grammar mirrors the
/// standard library's `parse_prefix`, and a Windows-only test pins the two to
/// the same classification.
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
pub(crate) mod windows {
    use super::{Step, apply};

    const SEPARATOR: u16 = b'\\' as u16;
    const ALT_SEPARATOR: u16 = b'/' as u16;
    const DOT: u16 = b'.' as u16;

    /// A Windows path split into identity parts, still as UTF-16 units.
    #[derive(Debug, PartialEq, Eq)]
    pub(crate) struct Parts {
        pub(crate) root: Vec<u16>,
        pub(crate) rooted: bool,
        pub(crate) leading_parents: usize,
        pub(crate) components: Vec<Vec<u16>>,
    }

    /// The prefix kinds of the standard library's `std::path::Prefix`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum PrefixKind {
        Disk,
        VerbatimDisk,
        Unc,
        VerbatimUnc,
        DeviceNs,
        Verbatim,
    }

    struct Prefix {
        kind: PrefixKind,
        root: Vec<u16>,
        len: usize,
    }

    pub(crate) fn parse(units: &[u16]) -> Parts {
        let prefix = parse_prefix(units);
        let verbatim = prefix.as_ref().is_some_and(|prefix| {
            matches!(
                prefix.kind,
                PrefixKind::VerbatimDisk | PrefixKind::VerbatimUnc | PrefixKind::Verbatim
            )
        });
        let (root, consumed, implicit_root) = match prefix {
            Some(prefix) => (prefix.root, prefix.len, prefix.kind != PrefixKind::Disk),
            None => (Vec::new(), 0, false),
        };

        let is_separator = |unit: u16| unit == SEPARATOR || (!verbatim && unit == ALT_SEPARATOR);
        let rest = &units[consumed..];
        let rooted = implicit_root || rest.first().copied().is_some_and(is_separator);

        let mut components = Vec::new();
        let mut leading_parents = 0;
        for segment in rest.split(|unit| is_separator(*unit)) {
            let step = match segment {
                [] => Step::Current,
                // Under `\\?\` a dot segment is a directory whose name happens
                // to be a dot.
                _ if verbatim => Step::Name(segment.to_vec()),
                [DOT] => Step::Current,
                [DOT, DOT] => Step::Parent,
                _ => Step::Name(segment.to_vec()),
            };
            apply(&mut components, &mut leading_parents, rooted, step);
        }

        Parts {
            root,
            rooted,
            leading_parents,
            components,
        }
    }

    /// The prefix kind `units` starts with, if any.
    #[cfg(test)]
    pub(crate) fn prefix_kind(units: &[u16]) -> Option<PrefixKind> {
        parse_prefix(units).map(|prefix| prefix.kind)
    }

    fn parse_prefix(units: &[u16]) -> Option<Prefix> {
        // The standard library compares the first eight units with `/` read as
        // `\`, then insists that a verbatim `\\?\` was spelled with backslashes.
        let normalized: Vec<u16> = units
            .iter()
            .take(8)
            .map(|&unit| if unit == ALT_SEPARATOR { SEPARATOR } else { unit })
            .collect();
        let starts = |offset: usize, text: &str| {
            let expected: Vec<u16> = text.encode_utf16().collect();
            normalized.get(offset..offset + expected.len()) == Some(&expected[..])
        };

        if !starts(0, r"\\") {
            return parse_drive(units).map(|drive| Prefix {
                kind: PrefixKind::Disk,
                root: drive_root(drive),
                len: 2,
            });
        }

        if starts(2, r"?\") && !units[..4].contains(&ALT_SEPARATOR) {
            if starts(4, r"UNC\") {
                let (server, rest) = next_component(&units[8..], true);
                let (share, _) = next_component(rest, true);
                let len = 8 + server.len() + if share.is_empty() { 0 } else { 1 + share.len() };
                return Some(Prefix {
                    kind: PrefixKind::VerbatimUnc,
                    root: unc_root(server, share),
                    len,
                });
            }
            let path = &units[4..];
            if let Some(drive) = parse_drive_exact(path) {
                return Some(Prefix {
                    kind: PrefixKind::VerbatimDisk,
                    root: drive_root(drive),
                    len: 6,
                });
            }
            let (prefix, _) = next_component(path, true);
            return Some(Prefix {
                kind: PrefixKind::Verbatim,
                root: units[..4 + prefix.len()].to_vec(),
                len: 4 + prefix.len(),
            });
        }

        if starts(2, r".\") {
            let (device, _) = next_component(&units[4..], false);
            return Some(Prefix {
                kind: PrefixKind::DeviceNs,
                root: units[..4 + device.len()].to_vec(),
                len: 4 + device.len(),
            });
        }

        let (server, rest) = next_component(&units[2..], false);
        let (share, _) = next_component(rest, false);
        (!server.is_empty() && !share.is_empty()).then(|| Prefix {
            kind: PrefixKind::Unc,
            root: unc_root(server, share),
            len: 2 + server.len() + 1 + share.len(),
        })
    }

    fn next_component(units: &[u16], verbatim: bool) -> (&[u16], &[u16]) {
        let is_separator = |unit: &u16| *unit == SEPARATOR || (!verbatim && *unit == ALT_SEPARATOR);
        match units.iter().position(is_separator) {
            Some(index) => (&units[..index], &units[index + 1..]),
            None => (units, &[]),
        }
    }

    fn parse_drive(units: &[u16]) -> Option<u16> {
        match units {
            [drive, colon, ..] if is_drive_letter(*drive) && *colon == u16::from(b':') => Some(*drive),
            _ => None,
        }
    }

    /// A verbatim drive is recognized only as exactly `C:` or `C:\…`.
    fn parse_drive_exact(units: &[u16]) -> Option<u16> {
        let drive = parse_drive(units)?;
        (units.len() == 2 || units[2] == SEPARATOR).then_some(drive)
    }

    fn is_drive_letter(unit: u16) -> bool {
        u8::try_from(unit).is_ok_and(|byte| byte.is_ascii_alphabetic())
    }

    fn drive_root(drive: u16) -> Vec<u16> {
        let upper = u8::try_from(drive).map_or(drive, |byte| u16::from(byte.to_ascii_uppercase()));
        vec![upper, u16::from(b':')]
    }

    fn unc_root(server: &[u16], share: &[u16]) -> Vec<u16> {
        let mut root = vec![SEPARATOR, SEPARATOR];
        root.extend_from_slice(server);
        root.push(SEPARATOR);
        root.extend_from_slice(share);
        root
    }
}

#[cfg(test)]
mod tests;
