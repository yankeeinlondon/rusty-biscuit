//! The ordered preferences [`PortablePath`](super::PortablePath) evaluates.

use std::fmt;

/// Which authored intent forms [`PortabilityPreference::AuthoredIntent`] keeps.
///
/// [`ALL`](Self::ALL) is the only set defined: `~`, `@`, `^`, `&`, `vault:`,
/// URLs, a leading portable `{{VAR}}`, and recursive `%` searches. Narrower
/// sets (for example only the searched forms) are added when a caller needs
/// one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntentForms {
    _all: (),
}

impl IntentForms {
    /// Every intent form.
    pub const ALL: Self = Self { _all: () };
}

/// One way of writing a target as a file reference.
///
/// A strategy is an ordered list of preferences; evaluation stops at the first
/// one whose reference verifies (see
/// [`PortablePath::file_reference`](super::PortablePath::file_reference)).
/// The relative preferences compute a component-based route from `cwd` to the
/// target and, except for [`ExternalRelativePath`](Self::ExternalRelativePath),
/// require the target to lie inside `base_dir`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PortabilityPreference {
    /// Keep a reference input exactly as authored when it is one of the given
    /// intent forms; never applies to a path input, and is the only preference
    /// that returns the input rather than computing a form from the target.
    ///
    /// Its position in the strategy is meaningful. First means "never touch
    /// intent forms". After [`SameDirRelative`](Self::SameDirRelative) means
    /// "keep sigils unless a plain `./x` reaches the same file", so an
    /// authored `&docs/x.md` next to the document becomes `./x.md`. Absent
    /// means "normalize everything", except URLs and recursive searches,
    /// which no other preference rewrites.
    AuthoredIntent(IntentForms),
    /// A target whose parent is `cwd` (`./x.md`); a target equal to `cwd` is
    /// written `./`.
    SameDirRelative,
    /// A target inside a subdirectory of `cwd`, at any depth (`./a/b/x.md`).
    ChildDir,
    /// A target whose parent is the parent of `cwd` (`../x.md`); never applies
    /// when `cwd` is already the root of `base_dir`.
    ImmediateParentDir,
    /// A target below a sibling of `cwd`: one parent hop, then a descent
    /// (`../sibling/x.md`).
    PeerDir,
    /// Any route that goes up at least one level and stays inside `base_dir`
    /// (`../../x.md`, `../../shared/x.md`): the in-tree catch-all. Not part of
    /// the default strategy.
    ParentDir,
    /// The `&` sigil from the repository root (`&docs/x.md`). Does not apply
    /// outside a repository. The optional relative directory is an
    /// eligibility filter, never a new root: `RepoRoot(Some("docs"))` only
    /// considers targets below `<repository>/docs`, and still writes
    /// `&docs/x.md`.
    RepoRoot(Option<String>),
    /// The `^` sigil, searched from the package root, then the package area,
    /// then the repository root. Does not apply outside a repository. The
    /// optional relative directory is joined to each `^` root as an
    /// eligibility filter.
    RepoMultiPath(Option<String>),
    /// The `@` sigil over the context's ordered search roots, keeping the
    /// first spelling whose lookup finds the target. The optional filter is a
    /// file reference (such as `~/.claudine/prompts`) that must name one of
    /// those roots; only targets below it are considered, and the spelling is
    /// written from that root (`@x.md`), still verified by lookup.
    MagicPath(Option<String>),
    /// A relative route that leaves `base_dir`. Opt-in only and never part of
    /// a default strategy, because a link that escapes its file tree rarely
    /// survives the tree moving; readers of such links must opt in with
    /// [`FileResolutionContext::allow_external_relative`](crate::FileResolutionContext::allow_external_relative).
    ExternalRelativePath,
    /// `{{NAME}}/rest`, rooted in a portable environment variable whose value
    /// is an absolute prefix of the target.
    EnvRootedPath,
    /// `~/rest`, under the context's home directory (or the OS home directory
    /// when no context was supplied).
    HomeDir,
    /// The absolute path itself; the last resort, and the caller's signal to
    /// warn that the result is not portable.
    AbsolutePath,
}

impl PortabilityPreference {
    /// The default strategy.
    ///
    /// `RepoRoot` precedes `HomeDir` because a repository usually lives under
    /// `~`; `EnvRootedPath` precedes `HomeDir` because a declared variable
    /// survives a host where the directory lives elsewhere. `ParentDir` and
    /// `ExternalRelativePath` are deliberately absent: a deep-parent in-tree
    /// target outside a repository falls through to the anchored forms.
    pub const DEFAULT_STRATEGY: &'static [Self] = &[
        Self::AuthoredIntent(IntentForms::ALL),
        Self::SameDirRelative,
        Self::ChildDir,
        Self::PeerDir,
        Self::ImmediateParentDir,
        Self::RepoRoot(None),
        Self::EnvRootedPath,
        Self::HomeDir,
        Self::AbsolutePath,
    ];
}

impl fmt::Display for PortabilityPreference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let filtered = |f: &mut fmt::Formatter<'_>, name: &str, filter: &Option<String>| match filter {
            Some(filter) => write!(f, "{name}({filter})"),
            None => f.write_str(name),
        };
        match self {
            Self::AuthoredIntent(_) => f.write_str("AuthoredIntent"),
            Self::SameDirRelative => f.write_str("SameDirRelative"),
            Self::ChildDir => f.write_str("ChildDir"),
            Self::ImmediateParentDir => f.write_str("ImmediateParentDir"),
            Self::PeerDir => f.write_str("PeerDir"),
            Self::ParentDir => f.write_str("ParentDir"),
            Self::RepoRoot(filter) => filtered(f, "RepoRoot", filter),
            Self::RepoMultiPath(filter) => filtered(f, "RepoMultiPath", filter),
            Self::MagicPath(filter) => filtered(f, "MagicPath", filter),
            Self::ExternalRelativePath => f.write_str("ExternalRelativePath"),
            Self::EnvRootedPath => f.write_str("EnvRootedPath"),
            Self::HomeDir => f.write_str("HomeDir"),
            Self::AbsolutePath => f.write_str("AbsolutePath"),
        }
    }
}
