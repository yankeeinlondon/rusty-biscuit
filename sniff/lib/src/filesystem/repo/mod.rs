pub mod aggregate;
pub mod aggregate_view;
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
pub(crate) mod ownership;
pub mod polyglot;
pub mod python;
pub(crate) mod seed;
pub mod standard;
pub(crate) mod test_runner_usage;
pub(crate) mod topology;
pub mod types;
pub mod uv;

pub use aggregate::{
    AggregateResult, AggregateScope, PathAttribution, TestRunnerAttribution, VersionAttribution,
    VersionSource, VersionSourceAttribution, aggregate_package_values, aggregate_test_runners,
    aggregate_versions, attribute_paths, bare_aggregate_version, collect_external_dependencies,
    resolve_directory_version, resolve_scope, resolve_scope_with_overrides,
};
pub use aggregate_view::{AggregateCwdContext, RepoAggregate, observe_repo_aggregate, scope_paths};
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
    PackageEcosystem, RepoInfo, detect_repo, detect_repo_structure,
    detect_repo_structure_or_root_package, detect_repo_with_inventory, detect_repo_with_request,
    detect_repo_with_request_or_root_package,
};

pub use crate::package::{DependencyEntry, DependencyKind};
