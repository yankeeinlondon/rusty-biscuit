//! [`PortablePath`]: evaluate a strategy against a target and verify the
//! chosen reference in one prepared context.

use std::borrow::Cow;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

use super::diagnostics::{
    Attempt, AttemptOutcome, ConfigurationProblem, EnvAnchorProblem, FilterProblem, Finding, InvalidTarget,
    NotApplicable, PortablePathError, ProbeError, ResolutionProblem, SpellingProblem,
};
use super::env_anchor::{self, PortableNames};
use super::path_identity::PathIdentity;
use super::strategy::{IntentForms, PortabilityPreference};
use super::text::{self, Lead};
use crate::file_reference::{
    DetailedOutcome, DetailedResolution, FileReference, FileReferenceKind, FileResolutionContext,
    ProbeDisposition, ResolutionFailure, RootProvenance, TemplateSegment, find_git_root, parse,
    resolve,
};

/// Turns an absolute path or an authored [`FileReference`] into the most
/// portable reference that verifiably names the same file.
///
/// Builders only record settings; [`file_reference`](Self::file_reference)
/// validates them and evaluates the strategy.
///
/// ## Examples
///
/// ```no_run
/// use biscuit_file::{FileResolutionContext, PortabilityPreference, PortablePath};
///
/// let ctx = FileResolutionContext::new("/opt/coding/repo/docs/topics")
///     .with_repository_root("/opt/coding/repo");
/// let portable = PortablePath::from_path("/opt/coding/repo/apps/web/README.md")
///     .with_ctx(&ctx)
///     .file_reference()?;
/// assert_eq!(portable.reference().raw(), "&apps/web/README.md");
/// assert_eq!(portable.strategy(), &PortabilityPreference::RepoRoot(None));
/// # Ok::<(), biscuit_file::PortablePathError>(())
/// ```
#[derive(Debug, Clone)]
pub struct PortablePath {
    input: Input,
    ctx: Option<FileResolutionContext>,
    cwd: Option<PathBuf>,
    base_dir: Option<PathBuf>,
    strategy: Vec<PortabilityPreference>,
    portable_env: Vec<String>,
}

#[derive(Debug, Clone)]
enum Input {
    Path(PathBuf),
    Reference(FileReference),
}

impl PortablePath {
    /// Start from an absolute host path, which carries no authored intent.
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self::new(Input::Path(path.into()))
    }

    /// Start from an authored reference: intent forms (`~`, `@`, `^`, `&`,
    /// `vault:`, URLs, `%`, a leading portable `{{VAR}}`) can be kept, and
    /// position forms (`./`, `../`, bare, absolute, a non-portable `{{VAR}}`)
    /// are resolved to their target and rewritten.
    pub fn from_reference(reference: FileReference) -> Self {
        Self::new(Input::Reference(reference))
    }

    fn new(input: Input) -> Self {
        Self {
            input,
            ctx: None,
            cwd: None,
            base_dir: None,
            strategy: PortabilityPreference::DEFAULT_STRATEGY.to_vec(),
            portable_env: Vec::new(),
        }
    }

    /// Evaluate against a clone of `ctx`: its `cwd`, tree, home, environment,
    /// and roots are used as captured, with no discovery and no live reads.
    /// A context with no repository stays non-repository.
    ///
    /// Combining this with [`with_cwd`](Self::with_cwd) or
    /// [`with_base_dir`](Self::with_base_dir) is an
    /// [`InvalidConfiguration`](PortablePathError::InvalidConfiguration);
    /// derive the context instead.
    #[must_use]
    pub fn with_ctx(mut self, ctx: &FileResolutionContext) -> Self {
        self.ctx = Some(ctx.clone());
        self
    }

    /// Use `cwd` instead of the process working directory. Must be absolute.
    #[must_use]
    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Supply the tree root outside a repository; inside one it must equal the
    /// repository root. Does not set `cwd`. Must be absolute.
    #[must_use]
    pub fn with_base_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.base_dir = Some(dir.into());
        self
    }

    /// Replace the strategy. An empty strategy is valid and matches nothing.
    #[must_use]
    pub fn with_strategy(mut self, strategy: impl IntoIterator<Item = PortabilityPreference>) -> Self {
        self.strategy = strategy.into_iter().collect();
        self
    }

    /// Declare more portable variable names, in addition to
    /// [`PORTABLE_ENV_VARIABLES`](super::PORTABLE_ENV_VARIABLES). Names
    /// accumulate across calls; values always come from the evaluation's
    /// environment.
    #[must_use]
    pub fn with_portable_env<I, S>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for name in names {
            let name = name.into();
            if !self.portable_env.contains(&name) {
                self.portable_env.push(name);
            }
        }
        self
    }

    /// Evaluate the strategy and return the first verified reference.
    ///
    /// Despite the getter-like name this does real work: without a context it
    /// captures the working directory, home, and environment and discovers
    /// the repository once; it resolves a reference input, probes the
    /// filesystem, and resolves every candidate back to the target before
    /// accepting it.
    ///
    /// ## Errors
    ///
    /// See [`PortablePathError`]. Settings and the context are validated before
    /// any preference runs.
    pub fn file_reference(&self) -> Result<PortableReference, PortablePathError> {
        if self.ctx.is_some() && self.cwd.is_some() {
            return Err(PortablePathError::InvalidConfiguration(ConfigurationProblem::ContextWithCwd));
        }
        if self.ctx.is_some() && self.base_dir.is_some() {
            return Err(PortablePathError::InvalidConfiguration(
                ConfigurationProblem::ContextWithBaseDir,
            ));
        }
        validate_filters(&self.strategy)?;
        if let Input::Path(path) = &self.input {
            validate_target(path)?;
        }
        let ctx = self.prepare_context()?;
        let names = env_anchor::portable_names(ctx.env(), &self.portable_env);
        let findings = names
            .invalid
            .iter()
            .map(|name| Finding::InvalidPortableVariableName { name: name.clone() })
            .collect();
        Evaluation {
            ctx,
            names,
            input: &self.input,
            attempts: Vec::new(),
            findings,
            target: None,
        }
        .run(&self.strategy)
    }

    fn prepare_context(&self) -> Result<FileResolutionContext, PortablePathError> {
        let ctx = match &self.ctx {
            Some(ctx) => ctx.clone(),
            None => {
                let cwd = match &self.cwd {
                    Some(cwd) => absolute_dir(cwd)?,
                    None => std::env::current_dir().map_err(|error| {
                        PortablePathError::CwdUnavailable(ProbeError::new(Path::new("."), &error))
                    })?,
                };
                let base_dir = self.base_dir.as_deref().map(absolute_dir).transpose()?;
                let mut ctx = FileResolutionContext::new(cwd.clone());
                match find_git_root(&cwd) {
                    Ok(Some(root)) => ctx = ctx.with_repository_root(root),
                    Ok(None) => {}
                    Err(error) => {
                        return Err(PortablePathError::RepositoryDiscoveryFailed {
                            cwd,
                            problem: ResolutionProblem::from_error(&error),
                        });
                    }
                }
                match base_dir {
                    Some(base_dir) => ctx.with_base_dir(base_dir),
                    None => ctx,
                }
            }
        };
        ctx.validate().map_err(|error| {
            PortablePathError::InvalidConfiguration(ConfigurationProblem::from_validation(&error))
        })?;
        Ok(ctx)
    }
}

fn absolute_dir(dir: &Path) -> Result<PathBuf, PortablePathError> {
    if dir.is_absolute() {
        Ok(dir.to_path_buf())
    } else {
        Err(PortablePathError::InvalidConfiguration(
            ConfigurationProblem::RelativeDirectory {
                path: dir.to_path_buf(),
            },
        ))
    }
}

fn validate_target(path: &Path) -> Result<(), PortablePathError> {
    if path.is_absolute() {
        return Ok(());
    }
    let reason = match path.to_str() {
        Some(text) if parse::is_absolute_reference(text) => InvalidTarget::ForeignAbsolute,
        _ => InvalidTarget::Relative,
    };
    Err(PortablePathError::InvalidTarget {
        target: path.to_path_buf(),
        reason,
    })
}

fn validate_filters(strategy: &[PortabilityPreference]) -> Result<(), PortablePathError> {
    for preference in strategy {
        let problem = match preference {
            PortabilityPreference::RepoRoot(Some(filter))
            | PortabilityPreference::RepoMultiPath(Some(filter)) => subdirectory_filter(filter).err(),
            PortabilityPreference::MagicPath(Some(filter)) => magic_filter(filter).err(),
            _ => None,
        };
        if let Some(problem) = problem {
            return Err(PortablePathError::InvalidConfiguration(
                ConfigurationProblem::InvalidFilter {
                    strategy: preference.clone(),
                    problem,
                },
            ));
        }
    }
    Ok(())
}

/// The names of a `RepoRoot`/`RepoMultiPath` filter, which must stay below
/// its root.
fn subdirectory_filter(filter: &str) -> Result<Vec<OsString>, FilterProblem> {
    if filter.is_empty() {
        return Err(FilterProblem::Empty);
    }
    let identity = PathIdentity::new(Path::new(filter));
    if !identity.is_unanchored() || identity.components().is_empty() {
        return Err(FilterProblem::NotARelativeSubdirectory);
    }
    Ok(identity.components().to_vec())
}

fn magic_filter(filter: &str) -> Result<FileReference, FilterProblem> {
    if filter.is_empty() {
        return Err(FilterProblem::Empty);
    }
    let reference = FileReference::new(filter).map_err(|error| FilterProblem::InvalidReference {
        message: error.to_string(),
    })?;
    let class = reference.class();
    if class.recursive || class.kind == FileReferenceKind::Url {
        return Err(FilterProblem::InvalidReference {
            message: format!("`{filter}` is not a single local directory"),
        });
    }
    Ok(reference)
}

/// The chosen reference and how it was chosen.
#[derive(Debug, Clone)]
pub struct PortableReference {
    reference: FileReference,
    strategy: PortabilityPreference,
    attempts: Vec<Attempt>,
    findings: Vec<Finding>,
}

#[allow(missing_docs)]
impl PortableReference {
    pub fn reference(&self) -> &FileReference {
        &self.reference
    }

    /// The preference that matched; [`PortabilityPreference::AbsolutePath`]
    /// is the caller's cue to warn that the result is not portable.
    pub fn strategy(&self) -> &PortabilityPreference {
        &self.strategy
    }

    /// Every preference tried, in order, ending with the one that matched.
    pub fn attempts(&self) -> &[Attempt] {
        &self.attempts
    }

    /// Problems with the returned reference; empty when there are none.
    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }

    pub fn into_reference(self) -> FileReference {
        self.reference
    }
}

impl AsRef<FileReference> for PortableReference {
    fn as_ref(&self) -> &FileReference {
        &self.reference
    }
}

/// The target every non-intent preference writes.
#[derive(Debug, Clone)]
struct Target {
    path: PathBuf,
    identity: PathIdentity,
    state: TargetState,
    /// For a bare reference input: whether it reached the target from `cwd`
    /// rather than through the repository-root fallback.
    reached_from_cwd: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetState {
    File,
    Missing,
    NotFile,
}

impl TargetState {
    fn finding(self) -> Option<Finding> {
        match self {
            Self::File => None,
            Self::Missing => Some(Finding::TargetMissing),
            Self::NotFile => Some(Finding::TargetNotFile),
        }
    }

    /// The reason a search form (`@`, `^`) cannot be used, if any.
    fn search_obstacle(self) -> Option<NotApplicable> {
        match self {
            Self::File => None,
            Self::Missing => Some(NotApplicable::TargetMissing),
            Self::NotFile => Some(NotApplicable::TargetNotFile),
        }
    }
}

/// What verifying one generated spelling decided.
enum Check {
    Verified(FileReference),
    Rejected(AttemptOutcome),
    /// The lookup failed for a reason other than absence; the preference
    /// tries no further candidate.
    Failed(ProbeError),
}

impl Check {
    /// Whether the preference stops at this candidate: it either matched or
    /// hit a probe failure.
    fn ends_attempt(&self) -> bool {
        !matches!(self, Self::Rejected(_))
    }
}

/// One preference's candidate outcomes, in order. A probe failure is the last
/// outcome, never an early return, so the rejections before it survive into
/// the attempt the error reports.
type Outcomes = Vec<AttemptOutcome>;

fn not_applicable(reason: NotApplicable) -> Outcomes {
    vec![AttemptOutcome::NotApplicable(reason)]
}

struct Evaluation<'a> {
    ctx: FileResolutionContext,
    names: PortableNames,
    input: &'a Input,
    attempts: Vec<Attempt>,
    findings: Vec<Finding>,
    /// Established on first use, so an input that only `AuthoredIntent`
    /// decides is never resolved twice.
    target: Option<Target>,
}

impl Evaluation<'_> {
    fn run(mut self, strategy: &[PortabilityPreference]) -> Result<PortableReference, PortablePathError> {
        for preference in strategy {
            let outcomes = match preference {
                PortabilityPreference::AuthoredIntent(forms) => self.authored_intent(*forms),
                _ if self.not_rewritable().is_some() => {
                    vec![AttemptOutcome::NotApplicable(NotApplicable::NotRewritable)]
                }
                _ => {
                    let target = self.target()?;
                    self.target_preference(preference, &target)
                }
            };
            let attempt = Attempt::from_outcomes(preference.clone(), outcomes);
            let failed = match &attempt.outcome {
                AttemptOutcome::ProbeFailed(error) => Some(error.clone()),
                _ => None,
            };
            let matched = attempt.matched().cloned();
            self.attempts.push(attempt);
            if let Some(error) = failed {
                // Every preference's probe failure ends evaluation here, after
                // its attempt is recorded. Only target preferences probe, so
                // the target is already established.
                let target = self.target.take().map_or_else(|| error.path.clone(), |target| target.path);
                return Err(PortablePathError::ProbeFailed {
                    target,
                    error,
                    attempts: self.attempts,
                });
            }
            if let Some(reference) = matched {
                if !matches!(preference, PortabilityPreference::AuthoredIntent(_))
                    && let Some(finding) = self.target.as_ref().and_then(|target| target.state.finding())
                {
                    self.findings.push(finding);
                }
                return Ok(PortableReference {
                    reference,
                    strategy: preference.clone(),
                    attempts: self.attempts,
                    findings: self.findings,
                });
            }
        }
        if let Some(reference) = self.not_rewritable() {
            return Err(PortablePathError::NormalizationUnsupported {
                reference: Box::new(reference.clone()),
                attempts: self.attempts,
            });
        }
        let target = self.target()?;
        Err(PortablePathError::NoStrategyMatched {
            target: target.path,
            attempts: self.attempts,
            findings: self.findings,
        })
    }

    /// A URL or recursive reference input, which no target preference rewrites.
    fn not_rewritable(&self) -> Option<&FileReference> {
        match self.input {
            Input::Reference(reference) => {
                let class = reference.class();
                (class.recursive || class.kind == FileReferenceKind::Url).then_some(reference)
            }
            Input::Path(_) => None,
        }
    }

    // ---- AuthoredIntent ---------------------------------------------------

    fn authored_intent(&mut self, forms: IntentForms) -> Vec<AttemptOutcome> {
        let input = self.input;
        let Input::Reference(reference) = input else {
            return vec![AttemptOutcome::NotApplicable(NotApplicable::NotAReference)];
        };
        if !self.is_intent(reference, forms) {
            return vec![AttemptOutcome::NotApplicable(NotApplicable::PositionForm)];
        }
        let findings = self.intent_findings(reference);
        self.findings.extend(findings);
        vec![AttemptOutcome::Matched(reference.clone())]
    }

    fn is_intent(&self, reference: &FileReference, _forms: IntentForms) -> bool {
        let class = reference.class();
        if class.recursive {
            return true;
        }
        match class.kind {
            FileReferenceKind::Home
            | FileReferenceKind::Magic
            | FileReferenceKind::RepositoryRoot
            | FileReferenceKind::RepositoryScoped
            | FileReferenceKind::Vault
            | FileReferenceKind::Url => true,
            FileReferenceKind::ImplicitRelative => self.leading_portable_variable(reference).is_some(),
            FileReferenceKind::ExplicitRelative | FileReferenceKind::Absolute => false,
        }
    }

    /// The leading `{{VAR}}` of a reference, when `VAR` is portable.
    fn leading_portable_variable<'r>(&self, reference: &'r FileReference) -> Option<&'r str> {
        match reference.parsed.kind.template().segments.first()? {
            TemplateSegment::EnvVar(name) if self.names.contains(name) => Some(name),
            _ => None,
        }
    }

    /// Findings for a reference kept as authored: unusable or non-portable
    /// variables, then one lookup for a missing target or a failed resolution.
    /// URLs get no lookup.
    fn intent_findings(&self, reference: &FileReference) -> Vec<Finding> {
        if reference.class().kind == FileReferenceKind::Url {
            return Vec::new();
        }
        let mut findings = Vec::new();
        let leading = self.leading_portable_variable(reference);
        let mut leading_unset = false;
        if let Some(name) = leading
            && let Err(problem) = env_anchor::anchor_dir(self.ctx.env().get(name).map(String::as_str))
        {
            leading_unset = problem == EnvAnchorProblem::Unset;
            findings.push(Finding::PortableVariableUnusable {
                name: name.to_string(),
                problem,
            });
        }
        let mut seen: Vec<&str> = leading.into_iter().collect();
        for segment in &reference.parsed.kind.template().segments {
            if let TemplateSegment::EnvVar(name) = segment
                && !seen.contains(&name.as_str())
            {
                seen.push(name.as_str());
                if !self.names.contains(name) {
                    findings.push(Finding::NonPortableVariable { name: name.clone() });
                }
            }
        }
        // An unset leading anchor is already reported; the lookup could only
        // repeat it.
        if !leading_unset {
            findings.extend(lookup_finding(&reference.resolve_detailed(&self.ctx)));
        }
        findings
    }

    // ---- the target ---------------------------------------------------------

    fn target(&mut self) -> Result<Target, PortablePathError> {
        if let Some(target) = &self.target {
            return Ok(target.clone());
        }
        let input = self.input;
        let target = match input {
            Input::Path(path) => self.path_target(path)?,
            Input::Reference(reference) => self.reference_target(reference)?,
        };
        if target.path.to_str().is_none() {
            return Err(PortablePathError::UnrenderableTarget {
                target: target.path,
                attempts: std::mem::take(&mut self.attempts),
            });
        }
        self.target = Some(target.clone());
        Ok(target)
    }

    fn path_target(&mut self, path: &Path) -> Result<Target, PortablePathError> {
        let state = match std::fs::metadata(path) {
            Ok(metadata) if metadata.is_file() => TargetState::File,
            Ok(_) => TargetState::NotFile,
            // Only `NotFound` is absence, as in the resolver's own probe: a
            // path through a regular file (`ENOTDIR`) is a probe failure.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => TargetState::Missing,
            Err(error) => {
                return Err(PortablePathError::ProbeFailed {
                    target: path.to_path_buf(),
                    error: ProbeError::new(path, &error),
                    attempts: std::mem::take(&mut self.attempts),
                });
            }
        };
        Ok(Target {
            path: path.to_path_buf(),
            identity: PathIdentity::new(path),
            state,
            reached_from_cwd: false,
        })
    }

    /// Resolve a reference input to its one target. A single-candidate form
    /// names its candidate even when it is missing; a multi-candidate form
    /// with no match, and any failure other than absence, is
    /// [`PortablePathError::UnresolvableInput`].
    fn reference_target(&mut self, reference: &FileReference) -> Result<Target, PortablePathError> {
        let detailed = reference.resolve_detailed(&self.ctx);
        let candidates = detailed.candidates();
        let resolved = match detailed.outcome() {
            DetailedOutcome::Matched(path) => {
                let provenance = candidates
                    .iter()
                    .find(|probed| probed.disposition() == ProbeDisposition::Matched)
                    .map(|probed| probed.candidate().provenance());
                Ok((path.clone(), TargetState::File, provenance))
            }
            DetailedOutcome::Failed(ResolutionFailure::NoMatch) => match candidates {
                [only] => {
                    let state = match only.disposition() {
                        ProbeDisposition::NonFile => TargetState::NotFile,
                        _ => TargetState::Missing,
                    };
                    let path = resolve::normalize_components(only.candidate().path());
                    Ok((path, state, Some(only.candidate().provenance())))
                }
                _ => Err(Finding::TargetMissing),
            },
            DetailedOutcome::Failed(_) => Err(Finding::ResolutionFailed(resolution_problem(&detailed))),
        };
        match resolved {
            Ok((path, state, provenance)) => Ok(Target {
                identity: PathIdentity::new(&path),
                path,
                state,
                reached_from_cwd: provenance == Some(RootProvenance::Source),
            }),
            Err(finding) => {
                let mut findings = std::mem::take(&mut self.findings);
                findings.push(finding);
                Err(PortablePathError::UnresolvableInput {
                    reference: Box::new(reference.clone()),
                    attempts: std::mem::take(&mut self.attempts),
                    findings,
                })
            }
        }
    }

    // ---- target preferences -------------------------------------------------

    fn target_preference(&self, preference: &PortabilityPreference, target: &Target) -> Outcomes {
        match preference {
            // Decided in `run` without a target.
            PortabilityPreference::AuthoredIntent(_) => not_applicable(NotApplicable::NoCandidate),
            PortabilityPreference::SameDirRelative
            | PortabilityPreference::ChildDir
            | PortabilityPreference::ImmediateParentDir
            | PortabilityPreference::PeerDir
            | PortabilityPreference::ParentDir
            | PortabilityPreference::ExternalRelativePath => self.relative(preference, target),
            PortabilityPreference::RepoRoot(filter) => self.repository_root(filter.as_deref(), target),
            PortabilityPreference::RepoMultiPath(filter) => {
                self.repository_scoped(filter.as_deref(), target)
            }
            PortabilityPreference::MagicPath(filter) => self.magic(filter.as_deref(), target),
            PortabilityPreference::EnvRootedPath => self.env_rooted(target),
            PortabilityPreference::HomeDir => self.home(target),
            PortabilityPreference::AbsolutePath => self.absolute(target),
        }
    }

    fn relative(&self, preference: &PortabilityPreference, target: &Target) -> Outcomes {
        let external = *preference == PortabilityPreference::ExternalRelativePath;
        let in_tree = target
            .identity
            .starts_with(&PathIdentity::new(self.ctx.base_dir()));
        if external && in_tree {
            return not_applicable(NotApplicable::InsideBaseDir);
        }
        if !external && !in_tree {
            return not_applicable(NotApplicable::OutsideBaseDir);
        }
        let Some(route) = target.identity.relative_from(&PathIdentity::new(self.ctx.cwd())) else {
            return not_applicable(NotApplicable::NoSharedRoot);
        };
        let (parent_hops, names) = (route.parent_hops(), route.forward().len());
        let fits = match preference {
            PortabilityPreference::SameDirRelative => parent_hops == 0 && names <= 1,
            PortabilityPreference::ChildDir => parent_hops == 0 && names >= 2,
            PortabilityPreference::ImmediateParentDir => parent_hops == 1 && names == 1,
            PortabilityPreference::PeerDir => parent_hops == 1 && names >= 2,
            PortabilityPreference::ParentDir => parent_hops >= 1,
            _ => true,
        };
        if !fits {
            return not_applicable(NotApplicable::RouteShape { parent_hops, names });
        }
        if let Some(authored) = self.authored_route(parent_hops, route.forward(), target) {
            return vec![AttemptOutcome::Matched(authored.clone())];
        }
        let text = match text::render_reference(Lead::Relative { parent_hops }, route.forward()) {
            Ok(text) => text,
            Err(rejection) => return not_applicable(NotApplicable::UnsafeSpelling(rejection.into())),
        };
        // `ExternalRelativePath` is verified the way its readers must resolve
        // it: with the reader opt-in.
        let ctx = if external {
            Cow::Owned(self.ctx.clone().allow_external_relative())
        } else {
            Cow::Borrowed(&self.ctx)
        };
        vec![verify(&text, target, Form::Single, &ctx).into_outcome()]
    }

    /// Minimal churn: a relative reference input already spelled as this
    /// route (`./x.md` or bare `x.md` for a same-directory target) is kept. A
    /// bare input qualifies only when it reached the target from `cwd`, not
    /// through the repository-root fallback.
    fn authored_route(&self, parent_hops: usize, forward: &[OsString], target: &Target) -> Option<&FileReference> {
        let Input::Reference(reference) = self.input else {
            return None;
        };
        let reached = match reference.class().kind {
            FileReferenceKind::ExplicitRelative => true,
            FileReferenceKind::ImplicitRelative => target.reached_from_cwd,
            _ => false,
        };
        let has_variables = reference
            .parsed
            .kind
            .template()
            .segments
            .iter()
            .any(|segment| matches!(segment, TemplateSegment::EnvVar(_)));
        if !reached || has_variables || reference.class().recursive {
            return None;
        }
        let (hops, names) = literal_route(reference.payload())?;
        (hops == parent_hops && names == forward).then_some(reference)
    }

    fn repository_root(&self, filter: Option<&str>, target: &Target) -> Outcomes {
        let Some(root) = self.ctx.repository_root() else {
            return not_applicable(NotApplicable::NoRepository);
        };
        let Some(rest) = target.identity.strip_prefix(&PathIdentity::new(root)) else {
            return not_applicable(NotApplicable::OutsideRepository);
        };
        if let Some(reason) = outside_filter(root, filter, rest) {
            return not_applicable(reason);
        }
        match text::render_reference(Lead::RepositoryRoot, rest) {
            Ok(text) => vec![verify(&text, target, Form::Single, &self.ctx).into_outcome()],
            Err(rejection) => not_applicable(NotApplicable::UnsafeSpelling(rejection.into())),
        }
    }

    fn repository_scoped(&self, filter: Option<&str>, target: &Target) -> Outcomes {
        let Some(repository_root) = self.ctx.repository_root() else {
            return not_applicable(NotApplicable::NoRepository);
        };
        let mut roots: Vec<&Path> = Vec::new();
        for root in [self.ctx.package_root(), self.ctx.package_area(), Some(repository_root)]
            .into_iter()
            .flatten()
        {
            if !roots.iter().any(|seen| PathIdentity::new(seen) == PathIdentity::new(root)) {
                roots.push(root);
            }
        }
        self.searched(Lead::RepositoryScoped, &roots, filter, NotApplicable::OutsideRepository, target)
    }

    fn magic(&self, filter: Option<&str>, target: &Target) -> Outcomes {
        let chain = self.ctx.magic_search_roots();
        let roots: Vec<&Path> = chain.iter().map(|root| root.path()).collect();
        let Some(filter) = filter else {
            return self.searched(Lead::Magic, &roots, None, NotApplicable::NotUnderMagicRoot, target);
        };
        // Validated before evaluation, so a parse failure cannot occur here.
        let Ok(reference) = magic_filter(filter) else {
            return not_applicable(NotApplicable::NoCandidate);
        };
        let candidates = match reference.candidate_plan(&self.ctx) {
            Ok(candidates) => candidates,
            Err(error) => {
                return not_applicable(NotApplicable::FilterUnavailable {
                    filter: filter.to_string(),
                    problem: ResolutionProblem::from_error(&error),
                });
            }
        };
        // The filter names one search root; spellings are written from it, so
        // the result reads relative to the directory the caller chose.
        let filter_root = candidates.iter().find_map(|candidate| {
            let identity = PathIdentity::new(candidate.path());
            roots.iter().copied().find(|root| PathIdentity::new(root) == identity)
        });
        let Some(filter_root) = filter_root else {
            let named = candidates
                .first()
                .map(|candidate| candidate.path().to_path_buf())
                .unwrap_or_else(|| PathBuf::from(filter));
            return not_applicable(NotApplicable::FilterNotASearchRoot { filter: named });
        };
        if !target.identity.starts_with(&PathIdentity::new(filter_root)) {
            return not_applicable(NotApplicable::OutsideFilter {
                filter: filter_root.to_path_buf(),
            });
        }
        self.searched(Lead::Magic, &[filter_root], None, NotApplicable::NotUnderMagicRoot, target)
    }

    /// Try a searched form from each root containing the target, in resolver
    /// order, keeping the first spelling whose lookup finds the target. A root
    /// whose spelling is shadowed is recorded, never skipped silently.
    fn searched(
        &self,
        lead: Lead<'_>,
        roots: &[&Path],
        filter: Option<&str>,
        outside: NotApplicable,
        target: &Target,
    ) -> Outcomes {
        let mut outcomes = Vec::new();
        let mut eligible = Vec::new();
        for root in roots {
            let Some(rest) = target.identity.strip_prefix(&PathIdentity::new(root)) else {
                continue;
            };
            match outside_filter(root, filter, rest) {
                Some(reason) => outcomes.push(AttemptOutcome::NotApplicable(reason)),
                None => eligible.push(rest.to_vec()),
            }
        }
        if eligible.is_empty() {
            if outcomes.is_empty() {
                outcomes.push(AttemptOutcome::NotApplicable(outside));
            }
            return outcomes;
        }
        if let Some(obstacle) = target.state.search_obstacle() {
            outcomes.push(AttemptOutcome::NotApplicable(obstacle));
            return outcomes;
        }
        for rest in eligible {
            let text = match text::render_reference(lead, &rest) {
                Ok(text) => text,
                Err(rejection) => {
                    outcomes.push(AttemptOutcome::NotApplicable(NotApplicable::UnsafeSpelling(
                        rejection.into(),
                    )));
                    continue;
                }
            };
            let check = verify(&text, target, Form::Search, &self.ctx);
            let done = check.ends_attempt();
            outcomes.push(check.into_outcome());
            if done {
                break;
            }
        }
        outcomes
    }

    fn env_rooted(&self, target: &Target) -> Outcomes {
        if self.names.names.is_empty() {
            return not_applicable(NotApplicable::NoPortableVariables);
        }
        let evaluation = env_anchor::evaluate_anchors(&self.names, self.ctx.env(), &target.identity);
        let mut outcomes: Vec<AttemptOutcome> = evaluation
            .rejected
            .into_iter()
            .map(|(name, problem)| AttemptOutcome::NotApplicable(NotApplicable::EnvAnchor { name, problem }))
            .collect();
        for anchor in &evaluation.eligible {
            match text::render_reference(Lead::Env(&anchor.name), &anchor.rest) {
                Ok(text) => {
                    let check = verify(&text, target, Form::Single, &self.ctx);
                    let done = check.ends_attempt();
                    outcomes.push(check.into_outcome());
                    if done {
                        break;
                    }
                }
                Err(rejection) => outcomes.push(AttemptOutcome::NotApplicable(
                    NotApplicable::UnsafeSpelling(rejection.into()),
                )),
            }
        }
        outcomes
    }

    fn home(&self, target: &Target) -> Outcomes {
        let Some(home) = self.ctx.home_dir() else {
            return not_applicable(NotApplicable::HomeUnavailable);
        };
        let Some(rest) = target.identity.strip_prefix(&PathIdentity::new(home)) else {
            return not_applicable(NotApplicable::NotUnderHome);
        };
        match text::render_reference(Lead::Home, rest) {
            Ok(text) => vec![verify(&text, target, Form::Single, &self.ctx).into_outcome()],
            Err(rejection) => not_applicable(NotApplicable::UnsafeSpelling(rejection.into())),
        }
    }

    fn absolute(&self, target: &Target) -> Outcomes {
        match text::render_absolute(&target.path) {
            Ok(text) => vec![verify(&text, target, Form::Single, &self.ctx).into_outcome()],
            Err(rejection) => not_applicable(NotApplicable::UnsafeSpelling(rejection.into())),
        }
    }
}

/// Whether the target's names below `root` fall outside the filter.
fn outside_filter(root: &Path, filter: Option<&str>, rest: &[OsString]) -> Option<NotApplicable> {
    let filter = filter?;
    // Validated before evaluation; an invalid filter admits nothing.
    let names = subdirectory_filter(filter).unwrap_or_default();
    (names.is_empty() || !rest.starts_with(&names)).then(|| NotApplicable::OutsideFilter {
        filter: root.join(filter),
    })
}

/// A plain relative spelling: leading `..` hops followed by names, with
/// nothing that would make the text differ from that route.
fn literal_route(text: &str) -> Option<(usize, Vec<OsString>)> {
    let mut hops = 0;
    let mut names = Vec::new();
    for component in Path::new(text).components() {
        match component {
            Component::CurDir if hops == 0 && names.is_empty() => {}
            Component::ParentDir if names.is_empty() => hops += 1,
            Component::Normal(name) => names.push(name.to_os_string()),
            _ => return None,
        }
    }
    Some((hops, names))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Form {
    /// One location (`./`, `../`, `&`, `~`, absolute, an absolute `{{VAR}}`),
    /// which may name a file not yet created.
    Single,
    /// A search (`@`, `^`), usable only when the lookup finds the target.
    Search,
}

impl Check {
    fn into_outcome(self) -> AttemptOutcome {
        match self {
            Self::Verified(reference) => AttemptOutcome::Matched(reference),
            Self::Rejected(outcome) => outcome,
            Self::Failed(error) => AttemptOutcome::ProbeFailed(error),
        }
    }
}

/// Parse `text` and resolve it in `ctx`, including the boundary and
/// real-landing checks, and require it to name the target.
fn verify(text: &str, target: &Target, form: Form, ctx: &FileResolutionContext) -> Check {
    let Ok(reference) = FileReference::new(text) else {
        return Check::Rejected(AttemptOutcome::NotApplicable(NotApplicable::UnsafeSpelling(
            SpellingProblem::GrammarMismatch,
        )));
    };
    let detailed = reference.resolve_detailed(ctx);
    let landed = match detailed.outcome() {
        DetailedOutcome::Matched(path) => path.clone(),
        DetailedOutcome::Failed(ResolutionFailure::NoMatch) => match (form, detailed.candidates()) {
            (Form::Single, [only]) => resolve::normalize_components(only.candidate().path()),
            _ => {
                return Check::Rejected(AttemptOutcome::NotApplicable(NotApplicable::TargetMissing));
            }
        },
        DetailedOutcome::Failed(ResolutionFailure::Io) => {
            return Check::Failed(match resolution_problem(&detailed) {
                ResolutionProblem::Probe(error) => error,
                other => ProbeError {
                    path: target.path.clone(),
                    kind: std::io::ErrorKind::Other,
                    os_code: None,
                    message: format!("{other:?}"),
                },
            });
        }
        DetailedOutcome::Failed(_) => {
            return Check::Rejected(AttemptOutcome::NotApplicable(NotApplicable::Rejected(
                resolution_problem(&detailed),
            )));
        }
    };
    if PathIdentity::new(&landed) == target.identity {
        Check::Verified(reference)
    } else {
        Check::Rejected(AttemptOutcome::Shadowed {
            reference,
            resolves_to: landed,
        })
    }
}

fn resolution_problem(detailed: &DetailedResolution) -> ResolutionProblem {
    match detailed.error() {
        Some(error) => ResolutionProblem::from_error(error),
        None => ResolutionProblem::Other {
            failure: match detailed.outcome() {
                DetailedOutcome::Failed(failure) => *failure,
                DetailedOutcome::Matched(_) => ResolutionFailure::NoMatch,
            },
            message: format!("`{}` did not resolve", detailed.raw()),
        },
    }
}

/// The finding one lookup of a kept reference produces, if any.
fn lookup_finding(detailed: &DetailedResolution) -> Option<Finding> {
    match detailed.outcome() {
        DetailedOutcome::Matched(_) => None,
        DetailedOutcome::Failed(ResolutionFailure::NoMatch) => Some(
            if detailed
                .candidates()
                .iter()
                .any(|probed| probed.disposition() == ProbeDisposition::NonFile)
            {
                Finding::TargetNotFile
            } else {
                Finding::TargetMissing
            },
        ),
        DetailedOutcome::Failed(_) => Some(Finding::ResolutionFailed(resolution_problem(detailed))),
    }
}
