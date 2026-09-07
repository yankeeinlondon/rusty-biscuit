//! Unit tests for the harness error types.
//!
//! Relocated out of `error.rs` so its inline test volume stays under the
//! repository test-placement threshold; module paths are otherwise unchanged.

mod source_chain_tests {
    use std::error::Error;

    use super::super::*;

    /// `HarnessError` had no `#[source]` on any variant before this: the enum
    /// carried no cause chain at all, so every lower-layer failure reaching it
    /// died at the `detail` string. These assert the chain now exists.
    ///
    /// `ShellExecCause` is `#[error(transparent)]`, so it *replaces* the stage's
    /// error in the chain rather than adding a link above it. Downcasting to the
    /// cause enum and matching its arm is the recovery path — and the arm is the
    /// part that matters, since `Spawn` and `Wait` are indistinguishable from
    /// their `io::Error` alone.
    #[test]
    fn shell_exec_failure_publishes_the_stage_that_failed() {
        let err = HarnessError::ShellCommandExecutionFailed {
            detail: "boom".to_owned(),
            source: ShellExecCause::Spawn(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "denied",
            )),
        };

        let published = (&err as &(dyn Error + 'static))
            .source()
            .expect("no source published");
        let recovered = published
            .downcast_ref::<ShellExecCause>()
            .expect("source is not a `ShellExecCause`");

        let ShellExecCause::Spawn(io) = recovered else {
            panic!("wrong stage: {recovered:?}");
        };
        assert_eq!(io.kind(), std::io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn a_wait_failure_is_distinguishable_from_a_spawn_failure() {
        let err = HarnessError::ShellCommandExecutionFailed {
            detail: "boom".to_owned(),
            source: ShellExecCause::Wait(std::io::Error::other("wait")),
        };

        let recovered = (&err as &(dyn Error + 'static))
            .source()
            .and_then(|c| c.downcast_ref::<ShellExecCause>())
            .expect("source is a `ShellExecCause`");
        assert!(matches!(recovered, ShellExecCause::Wait(_)));
    }

    /// `Which` and `Timeout` were previously discarded outright (`|_|` and
    /// `Err(_)`), so retaining them is the sharper win of the two.
    #[test]
    fn a_path_lookup_failure_is_retained_rather_than_discarded() {
        let err = HarnessError::ShellCommandExecutionFailed {
            detail: "executable 'nope' not found in PATH".to_owned(),
            source: ShellExecCause::Which(which::Error::CannotFindBinaryPath),
        };

        let cause = (&err as &(dyn Error + 'static))
            .source()
            .and_then(|c| c.downcast_ref::<ShellExecCause>())
            .expect("source is a `ShellExecCause`");
        assert!(matches!(
            cause,
            ShellExecCause::Which(which::Error::CannotFindBinaryPath)
        ));
    }

    /// Darkmatter's typed parse error must survive in the value. It is
    /// deliberately *not* reachable by downcasting `Error::source()`: the field
    /// is a `Box`, which publishes `Box<ShellExpansionError>` to the chain (the
    /// D-7 trap), and the variant's rustdoc records why the box is forced. This
    /// pins both halves of that trade so neither drifts silently.
    #[test]
    fn a_shell_audit_parse_failure_retains_darkmatters_typed_error() {
        // The only construction site is `collect_auditable_commands`; drive the
        // real parser rather than fabricating a `ShellExpansionError`.
        let ctx = biscuit_terminal::errors::SourceContext::new(
            std::path::PathBuf::from("<t>"),
            std::path::PathBuf::from("<t>"),
            String::new(),
        );
        let Err(parse_error) = darkmatter::markdown::compose::shell_expansion::parse_directives(
            "::shell \"unterminated",
            ctx,
            0,
        ) else {
            panic!("fixture no longer produces a parse error");
        };

        let err = HarnessError::ShellAuditParseError {
            detail: parse_error.to_string(),
            source: Box::new(parse_error),
        };

        let HarnessError::ShellAuditParseError { detail, source } = &err else {
            unreachable!()
        };
        // The typed value is retained beside the prose, not replaced by it.
        assert_eq!(&source.to_string(), detail);

        let published = (&err as &(dyn Error + 'static))
            .source()
            .expect("a source is published");
        assert!(
            published.downcast_ref::<ShellExpansionError>().is_none(),
            "the box no longer hides the cause — if `ShellAuditParseError` was \
             unboxed, drop this assertion and assert reachability instead"
        );
        assert!(
            published.downcast_ref::<Box<ShellExpansionError>>().is_some(),
            "the chain publishes neither `ShellExpansionError` nor its box"
        );
    }

    /// Adding the `#[source]` fields must leave `Display` and the machine
    /// surface exactly where they were (spec §D10).
    #[test]
    fn adding_a_source_leaves_display_and_detail_unmoved() {
        let err = HarnessError::ShellCommandExecutionFailed {
            detail: "failed to spawn 'ls': denied".to_owned(),
            source: ShellExecCause::Spawn(std::io::Error::other("denied")),
        };

        assert_eq!(
            err.to_string(),
            "shell command execution failed: failed to spawn 'ls': denied"
        );
        assert_eq!(err.code(), "composition.shell_expansion");
        assert_eq!(err.detail()["command"], json!("failed to spawn 'ls': denied"));
    }
}

mod classification_tests {
    use super::super::*;

    #[test]
    fn shell_denied_classifies_as_shell_expansion() {
        let err = HarnessError::ShellCommandDenied {
            command: "rm -rf /".to_string(),
        };
        assert_eq!(err.code(), "composition.shell_expansion");
        assert_eq!(err.category(), Category::Composition);
        assert_eq!(err.origin(), Origin::Author);
        assert_eq!(err.detail()["command"], json!("rm -rf /"));
    }

    #[test]
    fn invalid_frontmatter_classifies_as_lifecycle_invalid() {
        let err = HarnessError::InvalidFrontmatter {
            source_path: PathBuf::from("p.md"),
            property: "timeout".to_string(),
            detail: "must be a string".to_string(),
        };
        assert_eq!(err.code(), "composition.lifecycle_invalid");
        assert_eq!(err.detail()["property"], json!("timeout"));
        assert_eq!(err.detail()["message"], json!("must be a string"));
    }

    #[test]
    fn repo_root_required_classifies_as_io_read() {
        let err = HarnessError::RepoRootRequired {
            path: "@/x".to_string(),
        };
        assert_eq!(err.code(), "io.read_failed");
        assert_eq!(err.category(), Category::Io);
        assert_eq!(err.detail()["path"], json!("@/x"));
    }

    #[test]
    fn path_resolution_classifies_as_an_authoring_failure() {
        // Regression: this reported an authoring mistake as `io.read_failed`
        // (`Category::Io` / `Origin::Operator`), sending the reader to check
        // the filesystem when the fix was in their frontmatter.
        let err = HarnessError::PathResolutionFailed {
            raw: "nope.md".to_string(),
            failure: PathResolutionFailure::TargetMissing,
            source_path: Some(PathBuf::from("/repo/run.md")),
            resolved: Some(PathBuf::from("/repo/nope.md")),
            resolution: None,
        };
        assert_eq!(err.code(), "composition.invalid_file_reference");
        assert_eq!(err.category(), Category::Composition);
        assert_eq!(err.origin(), Origin::Author);
        assert_eq!(err.disposition(), Disposition::Correctable);
    }

    #[test]
    fn path_resolution_detail_projects_only_what_the_resolver_knows() {
        // No `resolution` plan attached (the failure was drawn before a probe),
        // so `kind`, `repository_root`, and `candidates` stay `null`. The probed
        // no-match projection is covered in `resolve.rs`.
        let err = HarnessError::PathResolutionFailed {
            raw: "nope.md".to_string(),
            failure: PathResolutionFailure::TargetMissing,
            source_path: Some(PathBuf::from("/repo/run.md")),
            resolved: Some(PathBuf::from("/repo/nope.md")),
            resolution: None,
        };
        let detail = err.detail();

        assert_eq!(detail["reference"], json!("nope.md"));
        assert_eq!(detail["failure"], json!("no_match"));
        assert_eq!(detail["source_path"], json!("/repo/run.md"));

        // Every declared key is present; without a plan the resolver-supplied
        // ones stay `null` rather than invented (spec §D3). `kind` in particular
        // is not back-derived from `failure`.
        for field in [
            "kind",
            "base_dir",
            "suggestions",
            "fallback_dir",
            "property",
            "event",
            "repository_root",
            "candidates",
        ] {
            assert!(
                detail.get(field).is_some(),
                "declared field `{field}` is absent from the projection"
            );
            assert_eq!(detail[field], Value::Null, "`{field}` should be null");
        }
    }

    #[test]
    fn every_path_resolution_failure_projects_a_declared_failure_slug() {
        // `failure` is a closed vocabulary the catalog documents; a new arm
        // must pick from it rather than coining a slug.
        for failure in [
            PathResolutionFailure::EmptyReference,
            PathResolutionFailure::NoSourceParent,
            PathResolutionFailure::TargetMissing,
        ] {
            assert!(
                [
                    "invalid_syntax",
                    "missing_context",
                    "no_match",
                    "permission_io",
                    "unsupported_remote",
                ]
                .contains(&failure.as_str()),
                "`{failure:?}` projects undeclared slug `{}`",
                failure.as_str()
            );
        }
    }

    #[test]
    fn path_resolution_display_names_the_reference_and_the_reason() {
        let err = HarnessError::PathResolutionFailed {
            raw: "nope.md".to_string(),
            failure: PathResolutionFailure::TargetMissing,
            source_path: None,
            resolved: Some(PathBuf::from("/repo/nope.md")),
            resolution: None,
        };
        let text = err.to_string();
        assert!(text.contains("nope.md"), "{text}");
        assert!(text.contains("target does not exist"), "{text}");

        let empty = HarnessError::PathResolutionFailed {
            raw: "  ".to_string(),
            failure: PathResolutionFailure::EmptyReference,
            source_path: None,
            resolved: None,
            resolution: None,
        };
        assert!(empty.to_string().contains("path is empty"));
    }

    #[test]
    fn file_reference_unresolvable_classifies_as_an_authoring_failure() {
        let err = HarnessError::FileReferenceUnresolvable {
            reference: "{{MISSING}}/x.md".to_string(),
            source_path: Some(PathBuf::from("/repo/run.md")),
            resolution: None,
            source: Box::new(FileReferenceError::MissingEnvironmentVariable {
                name: "MISSING".to_string(),
            }),
        };
        assert_eq!(err.code(), "composition.invalid_file_reference");
        assert_eq!(err.category(), Category::Composition);
        assert_eq!(err.origin(), Origin::Author);

        let detail = err.detail();
        assert_eq!(detail["reference"], json!("{{MISSING}}/x.md"));
        // A missing interpolation variable is an absent context anchor.
        assert_eq!(detail["failure"], json!("missing_context"));
        assert_eq!(detail["source_path"], json!("/repo/run.md"));
    }

    #[test]
    fn file_reference_unresolvable_publishes_the_typed_cause() {
        use std::error::Error;

        let err = HarnessError::FileReferenceUnresolvable {
            reference: "http://example.com/x.md".to_string(),
            source_path: None,
            resolution: None,
            source: Box::new(FileReferenceError::RemoteNotLocal(
                "http://example.com/x.md".to_string(),
            )),
        };
        assert_eq!(err.detail()["failure"], json!("unsupported_remote"));

        // The typed cause is published (as the box — the D-7 trade, matching
        // `ShellAuditParseError`).
        let published = (&err as &(dyn Error + 'static))
            .source()
            .expect("a source is published");
        assert!(
            published.downcast_ref::<Box<FileReferenceError>>().is_some(),
            "the boxed typed cause must be reachable on the chain"
        );
    }

    #[test]
    fn every_file_reference_error_maps_to_a_declared_failure_slug() {
        // This second exhaustive match deliberately duplicates the expected
        // vocabulary decision. A new shared error variant must update both the
        // adapter and this regression test before either will compile.
        fn expected_slug(error: &FileReferenceError) -> &'static str {
            use FileReferenceError as E;
            match error {
                E::InvalidSyntax(_)
                | E::UnsupportedScheme { .. }
                | E::UnsupportedUserHome(_)
                | E::InvalidUrl(_) => "invalid_syntax",
                E::MissingEnvironmentVariable { .. }
                | E::BareRepository
                | E::VaultNotConfigured
                | E::MissingHomeContext
                | E::OutsideRepository { .. }
                | E::RepositoryRootNotContainingSource { .. } => "missing_context",
                E::CurrentDirectory(_)
                | E::Git(_)
                | E::RepositoryEscape { .. }
                | E::RelativePath { .. }
                | E::Io { .. } => "permission_io",
                E::RemoteNotLocal(_) => "unsupported_remote",
            }
        }

        let samples = [
            FileReferenceError::InvalidSyntax("x".to_string()),
            FileReferenceError::MissingEnvironmentVariable {
                name: "X".to_string(),
            },
            FileReferenceError::CurrentDirectory(std::io::Error::other("cwd")),
            FileReferenceError::UnsupportedScheme {
                scheme: "ftp".to_string(),
                reference: "ftp://example.com/spec.md".to_string(),
            },
            FileReferenceError::UnsupportedUserHome("other".to_string()),
            FileReferenceError::MissingHomeContext,
            FileReferenceError::OutsideRepository {
                sigil: '^',
                reference_cwd: PathBuf::from("/elsewhere"),
            },
            FileReferenceError::VaultNotConfigured,
            FileReferenceError::BareRepository,
            FileReferenceError::RepositoryRootNotContainingSource {
                repository_root: PathBuf::from("/repo"),
                source_path: PathBuf::from("/elsewhere/source.md"),
            },
            FileReferenceError::RelativePath {
                from: PathBuf::from("/from"),
                to: PathBuf::from("/to"),
            },
            FileReferenceError::RepositoryEscape {
                sigil: '&',
                reference: "&escape/spec.md".to_string(),
                repository_root: PathBuf::from("/repo"),
                escaped_candidate: PathBuf::from("/outside/spec.md"),
            },
            FileReferenceError::RemoteNotLocal("http://x".to_string()),
            FileReferenceError::InvalidUrl("not a URL".to_string()),
            FileReferenceError::Io {
                path: PathBuf::from("/x"),
                source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
            },
        ];
        for error in &samples {
            assert_eq!(
                file_reference_failure_slug(error),
                expected_slug(error),
                "unexpected vocabulary mapping for `{error:?}`"
            );
        }
    }
}
