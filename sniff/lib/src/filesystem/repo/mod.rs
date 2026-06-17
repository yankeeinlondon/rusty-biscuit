pub mod aggregate;
pub mod area;
pub mod cargo;
pub mod detection;
pub mod dotnet;
pub mod glob;
pub mod go;
pub mod gradle;
pub mod identity;
pub mod manifest_index;
pub mod maven;
pub(crate) mod nested;
pub mod npm;
pub mod nx_turbo;
pub mod polyglot;
pub mod python;
pub mod standard;
pub(crate) mod test_runner_usage;
pub(crate) mod topology;
pub mod types;
pub mod uv;

pub use aggregate::{
    AggregateResult, AggregateScope, aggregate_package_values, collect_external_dependencies,
    resolve_scope,
};
pub use area::{AreaError, detect_area};
pub use identity::{RepoIdentity, detect_repo_identity, detect_repo_identity_with_repo};
pub use standard::{
    BinarySource, BinarySpec, DetectedStandard, DetectionConfidence, GlobDialect,
    InvocationTemplate, Marker, MarkerConfidence, MarkerContent, MembershipModel, MonorepoLayer,
    MonorepoStandard, MonorepoStandardSpec, NestingPolicy, PackageProvenance, ResolvedBinary, Role,
    RootMembership, Token, WorkspaceMultiplicity, WrapperScript,
};
pub use test_runner_usage::{TestRunnerSource, TestRunnerUsage, detect_test_runners_for_dir};
pub use types::{
    ExternalDependency, ExternalDependencyFamily, ExternalDependencyFilter, Package,
    PackageEcosystem, RepoInfo, detect_repo, detect_repo_structure, detect_repo_with_inventory,
};

pub use crate::package::{DependencyEntry, DependencyKind};
