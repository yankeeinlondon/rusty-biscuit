//! The authority-to-lockfile table: the single source of truth for which
//! files each [`MonorepoStandard`] may corroborate against.

use super::super::standard::MonorepoStandard;
use super::fallback::FallbackSource;
use super::{LockfileReason, Outcome};

/// A lockfile format with its own parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Format {
    Pnpm,
    Npm,
    Yarn,
    /// Text `bun.lock` (JSONC).
    Bun,
    /// Binary `bun.lockb`: selected by name and never read.
    BunBinary,
    Uv,
    Cargo,
}

impl Format {
    /// Whether the format can record workspace membership at all. A format
    /// that cannot is reported from metadata alone, with no content read.
    pub(crate) fn records_membership(self) -> bool {
        !matches!(self, Self::BunBinary)
    }

    /// Parse `content` as this format.
    ///
    /// `Cargo.lock` is parsed by the store's shared Cargo cache, which also
    /// serves dependency-version enrichment, and never reaches here.
    pub(crate) fn parse(self, content: &str) -> Outcome {
        match self {
            Self::Pnpm => super::pnpm::parse(content),
            Self::Npm => super::npm::parse(content),
            Self::Yarn => super::yarn::parse(content),
            Self::Bun => super::bun::parse(content),
            Self::Uv => super::uv::parse(content),
            Self::BunBinary | Self::Cargo => {
                unreachable!("{self:?} is never parsed through Format::parse")
            }
        }
    }
}

/// Where an authority's lockfile evidence comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Source {
    /// Files directly under the layer root, in precedence order. Only the
    /// first present candidate is selected (ruling R6).
    Candidates(&'static [(&'static str, Format)]),
    /// Dependency locks that prove no membership: metadata only.
    Fallback(FallbackSource),
    /// No applicable lockfile source.
    NotApplicable(LockfileReason),
    /// Rush: configuration chooses the manager and lockfile (ruling R3).
    Configured,
}

/// The lockfile source for `standard`.
pub(crate) fn source(standard: MonorepoStandard) -> Source {
    use MonorepoStandard as S;
    match standard {
        S::CargoWorkspace => Source::Candidates(&[("Cargo.lock", Format::Cargo)]),
        S::NpmWorkspaces => Source::Candidates(&[
            ("npm-shrinkwrap.json", Format::Npm),
            ("package-lock.json", Format::Npm),
        ]),
        S::PnpmWorkspaces => Source::Candidates(&[("pnpm-lock.yaml", Format::Pnpm)]),
        S::YarnWorkspaces => Source::Candidates(&[("yarn.lock", Format::Yarn)]),
        S::BunWorkspaces => Source::Candidates(&[
            ("bun.lock", Format::Bun),
            ("bun.lockb", Format::BunBinary),
        ]),
        S::UvWorkspace => Source::Candidates(&[("uv.lock", Format::Uv)]),
        S::RushStack => Source::Configured,
        S::GoWorkspace => Source::Fallback(FallbackSource::GoWorkSum),
        S::GradleMultiProject => Source::Fallback(FallbackSource::Gradle),
        S::Bazel => Source::Fallback(FallbackSource::BazelModule),
        S::MavenMultiModule | S::DotNetSolution | S::Pants | S::Buck2 => {
            Source::NotApplicable(LockfileReason::NoLockfileSource)
        }
        // Orchestrators never own a layer (ruling R7); the entry exists only
        // so the table stays exhaustive.
        S::Nx | S::Turborepo | S::Lerna => Source::NotApplicable(LockfileReason::NoLockfileSource),
        S::Unknown => Source::NotApplicable(LockfileReason::UnknownStandard),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_standard_has_the_specified_source() {
        use MonorepoStandard as S;
        let expected: &[(MonorepoStandard, Source)] = &[
            (S::CargoWorkspace, Source::Candidates(&[("Cargo.lock", Format::Cargo)])),
            (
                S::NpmWorkspaces,
                Source::Candidates(&[
                    ("npm-shrinkwrap.json", Format::Npm),
                    ("package-lock.json", Format::Npm),
                ]),
            ),
            (S::PnpmWorkspaces, Source::Candidates(&[("pnpm-lock.yaml", Format::Pnpm)])),
            (S::YarnWorkspaces, Source::Candidates(&[("yarn.lock", Format::Yarn)])),
            (
                S::BunWorkspaces,
                Source::Candidates(&[("bun.lock", Format::Bun), ("bun.lockb", Format::BunBinary)]),
            ),
            (S::UvWorkspace, Source::Candidates(&[("uv.lock", Format::Uv)])),
            (S::GoWorkspace, Source::Fallback(FallbackSource::GoWorkSum)),
            (S::GradleMultiProject, Source::Fallback(FallbackSource::Gradle)),
            (
                S::MavenMultiModule,
                Source::NotApplicable(LockfileReason::NoLockfileSource),
            ),
            (
                S::DotNetSolution,
                Source::NotApplicable(LockfileReason::NoLockfileSource),
            ),
            (S::Bazel, Source::Fallback(FallbackSource::BazelModule)),
            (S::Pants, Source::NotApplicable(LockfileReason::NoLockfileSource)),
            (S::Buck2, Source::NotApplicable(LockfileReason::NoLockfileSource)),
            (S::RushStack, Source::Configured),
            (S::Nx, Source::NotApplicable(LockfileReason::NoLockfileSource)),
            (S::Turborepo, Source::NotApplicable(LockfileReason::NoLockfileSource)),
            (S::Lerna, Source::NotApplicable(LockfileReason::NoLockfileSource)),
            (S::Unknown, Source::NotApplicable(LockfileReason::UnknownStandard)),
        ];
        assert_eq!(expected.len(), 18, "17 named standards plus Unknown");
        for (standard, source_expected) in expected {
            assert_eq!(source(*standard), *source_expected, "{standard:?}");
        }
    }

    #[test]
    fn only_the_binary_bun_lockfile_records_no_membership() {
        for format in [
            Format::Pnpm,
            Format::Npm,
            Format::Yarn,
            Format::Bun,
            Format::Uv,
            Format::Cargo,
        ] {
            assert!(format.records_membership(), "{format:?}");
        }
        assert!(!Format::BunBinary.records_membership());
    }
}
