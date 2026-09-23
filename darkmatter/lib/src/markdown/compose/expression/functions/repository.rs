//! `package(where)` and `package_area(where)`: lexical lookups over the
//! request's captured package topology (R12, R26).

use serde_json::Value;

use super::{EvaluationMode, FunctionBinding, FunctionHandler, ResolutionContext, resolve_path_shape};
use crate::markdown::compose::context::ContextGroup;
use crate::markdown::compose::expression::ExpressionError;

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding {
        canonical: "package_area",
        aliases: &[],
        evaluation: EvaluationMode::Context,
        handler: Some(FunctionHandler::Context(package_area_fn)),
    },
    FunctionBinding {
        canonical: "package",
        aliases: &[],
        evaluation: EvaluationMode::Context,
        handler: Some(FunctionHandler::Context(package_fn)),
    },
];

fn package_area_fn(args: &[Value], context: &ResolutionContext) -> Result<Value, ExpressionError> {
    lookup("package_area", args, context).map(|found| Value::String(found.area))
}

fn package_fn(args: &[Value], context: &ResolutionContext) -> Result<Value, ExpressionError> {
    lookup("package", args, context).map(|found| Value::String(found.package))
}

/// Resolves `where` to a path shape through the request's file-resolution
/// snapshot, then looks it up in the captured topology.
///
/// A missing target still has a shape, so a nonexistent descendant matches
/// lexically. A remote reference is rejected before resolution, and typed
/// `FileReference` failures keep their own classification; neither is a miss.
fn lookup(
    function: &'static str,
    args: &[Value],
    context: &ResolutionContext,
) -> Result<crate::markdown::compose::context::PackageMatch, ExpressionError> {
    let [where_value] = args else {
        return Err(ExpressionError::Other {
            function: function.to_string(),
            message: format!("requires 1 argument, got {}", args.len()),
        });
    };
    let Some(raw) = where_value.as_str() else {
        return Err(ExpressionError::ArgType {
            function,
            index: 0,
            expected: "file | string",
            actual_type: super::git::value_type(where_value),
        });
    };
    // R12 exempts only lookup misses from erroring; these functions take a
    // local path, so a URL is a contract violation rather than a miss.
    if super::is_remote_url(raw) {
        return Err(ExpressionError::ContractViolation {
            function: function.to_string(),
            message: format!("{function}() does not accept HTTP(S) URLs"),
        });
    }
    let path = resolve_path_shape(function, raw, context)?;
    let packages = context.observations.packages().ok_or_else(|| {
        ExpressionError::FunctionContextNotCaptured {
            function: function.to_string(),
            group: ContextGroup::Repo,
        }
    })?;
    packages
        .lookup(&path, context.repository_root.as_deref())
        .map_err(|message| ExpressionError::Other {
            function: function.to_string(),
            message: format!("the captured repository topology is invalid: {message}"),
        })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use serde_json::{Value, json};
    use sniff::filesystem::repo::{Package, RepoInfo};

    use super::super::dispatch_fs;
    use crate::markdown::compose::context::capture::CapturedObservations;
    use crate::markdown::compose::context::{ContextGroup, repository_scope_catalog};
    use crate::markdown::compose::expression::{ExpressionError, ResolutionContext};

    fn package(root: &Path, name: &str, relative: &str, area: &str) -> Package {
        Package {
            name: name.to_owned(),
            relative: relative.to_owned(),
            package_area: area.to_owned(),
            path: root.join(relative),
            ..Package::default()
        }
    }

    fn monorepo(root: &Path) -> RepoInfo {
        RepoInfo {
            is_monorepo: true,
            root: root.to_path_buf(),
            packages: Some(vec![
                package(root, "tool", "tool", ""),
                package(root, "foo-lib", "foo/lib", "foo"),
                package(root, "foo-nested", "foo/lib/nested", "foo"),
            ]),
            ..RepoInfo::default()
        }
    }

    struct Fixture {
        _temp: tempfile::TempDir,
        repo: PathBuf,
        context: ResolutionContext,
    }

    /// A document in `<repo>/docs` whose request captured `topology`, wired the
    /// way `ComposeOptions` builds a function context.
    fn fixture_with(topology: impl FnOnce(&Path) -> RepoInfo) -> Fixture {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        let base_dir = repo.join("docs");
        std::fs::create_dir_all(&base_dir).unwrap();
        let topology = topology(&repo);
        let snapshot = biscuit_file::FileResolutionContext::new(&base_dir)
            .with_repository_scope_catalog(repository_scope_catalog(&topology, &repo).unwrap());
        let mut context = ResolutionContext::new(base_dir)
            .with_repository_root(&repo)
            .with_observations(CapturedObservations::for_test_packages(&repo, &topology));
        context.file_resolution_context = Some(snapshot);
        Fixture { _temp: temp, repo, context }
    }

    fn fixture() -> Fixture {
        fixture_with(monorepo)
    }

    fn names(fixture: &Fixture, where_value: Value) -> (Value, Value) {
        let call = |name| {
            dispatch_fs(name, std::slice::from_ref(&where_value), &fixture.context)
                .expect("registered context function")
                .unwrap_or_else(|error| panic!("{name}({where_value}): {error}"))
        };
        (call("package"), call("package_area"))
    }

    #[test]
    fn nonexistent_descendants_select_the_deepest_package_and_area() {
        let fixture = fixture();
        assert_eq!(names(&fixture, json!("../foo/lib/src/lib.rs")), (json!("foo-lib"), json!("foo")));
        assert_eq!(
            names(&fixture, json!("../foo/lib/nested/src/new.rs")),
            (json!("foo-nested"), json!("foo"))
        );
        assert_eq!(names(&fixture, json!("../tool/main.rs")), (json!("tool"), json!("")));
        // An area directory with no containing package.
        assert_eq!(names(&fixture, json!("../foo/docs/guide.md")), (json!(""), json!("foo")));
        let absolute = fixture.repo.join("foo/lib/Cargo.toml");
        assert_eq!(
            names(&fixture, json!(absolute.to_string_lossy())),
            (json!("foo-lib"), json!("foo"))
        );
    }

    /// An implicit bare reference that exists only under the repository root
    /// resolves there through `FileReference`, not against the document directory.
    #[test]
    fn existing_implicit_references_resolve_before_lookup() {
        let fixture = fixture();
        std::fs::create_dir_all(fixture.repo.join("foo/lib/src")).unwrap();
        std::fs::write(fixture.repo.join("foo/lib/src/lib.rs"), "").unwrap();
        assert_eq!(names(&fixture, json!("foo/lib/src/lib.rs")), (json!("foo-lib"), json!("foo")));
    }

    /// AC17 misses are `""`, never `null`: a sibling with a shared string
    /// prefix, a repository-root file, an unobserved directory, and a path
    /// outside the repository.
    #[test]
    fn valid_misses_are_empty_strings() {
        let fixture = fixture();
        for where_value in [
            json!("../foobar/lib/src/lib.rs"),
            json!("../README.md"),
            json!("../scaffold/new/file.md"),
            json!("../../outside/foo/lib/src/lib.rs"),
        ] {
            assert_eq!(names(&fixture, where_value.clone()), (json!(""), json!("")), "{where_value}");
        }
    }

    #[test]
    fn non_monorepo_misses_everywhere() {
        let fixture = fixture_with(|root| RepoInfo { is_monorepo: false, ..monorepo(root) });
        assert_eq!(names(&fixture, json!("../foo/lib/src/lib.rs")), (json!(""), json!("")));
    }

    #[test]
    fn remote_and_unresolvable_references_are_errors_not_misses() {
        let fixture = fixture();
        for name in ["package", "package_area"] {
            let remote = dispatch_fs(name, &[json!("https://example.com/foo/lib")], &fixture.context)
                .unwrap()
                .unwrap_err();
            assert!(matches!(remote, ExpressionError::ContractViolation { .. }), "{remote:?}");
            assert!(remote.is_authoring_fatal());
            assert!(remote.to_string().contains("does not accept HTTP(S) URLs"), "{remote}");

            let vault = dispatch_fs(name, &[json!("vault:notes/x.md")], &fixture.context)
                .unwrap()
                .unwrap_err();
            assert!(
                matches!(&vault, ExpressionError::FileReference(diagnostic)
                    if diagnostic.function == name
                        && matches!(
                            diagnostic.source.as_deref(),
                            Some(biscuit_file::FileReferenceError::VaultNotConfigured)
                        )),
                "{vault:?}"
            );
            assert!(vault.is_authoring_fatal());
        }
    }

    #[test]
    fn non_string_arguments_are_type_errors() {
        let fixture = fixture();
        for argument in [json!(null), json!(3), json!(["foo"])] {
            assert!(matches!(
                dispatch_fs("package", &[argument], &fixture.context),
                Some(Err(ExpressionError::ArgType { function: "package", index: 0, .. }))
            ));
        }
    }

    #[test]
    fn uncaptured_repository_observation_is_fatal() {
        let mut context = fixture().context;
        context.observations = CapturedObservations::default();
        let error = dispatch_fs("package_area", &[json!("../foo/lib")], &context).unwrap().unwrap_err();
        assert!(matches!(
            error,
            ExpressionError::FunctionContextNotCaptured { group: ContextGroup::Repo, .. }
        ));
        assert!(error.is_authoring_fatal());
    }
}
