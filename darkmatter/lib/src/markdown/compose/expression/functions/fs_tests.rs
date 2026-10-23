//! Unit tests of the context-aware handlers (`fs.rs` and the family files
//! that read a `ResolutionContext`).

use super::*;
use serde_json::json;

#[allow(dead_code)]
fn v(value: Value) -> Vec<Value> {
    vec![value]
}

/// Calls `name` through normal dispatch, binding included, as a document
/// would.
fn dispatch(name: &str, args: &[Value], ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    call_function(name, args, Some(ctx)).expect("registered context function")
}

// Each shim shadows the handler of the same name, so these tests reach the
// handler only through binding.
fn absolute_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    dispatch("absolute", args, ctx)
}
fn basename_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    dispatch("basename", args, ctx)
}
fn basename_without_index_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    dispatch("basename_without_index", args, ctx)
}
fn code_link_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    dispatch("code_link", args, ctx)
}
fn decrement_file_index_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    dispatch("decrement_file_index", args, ctx)
}
fn dir_leading_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    dispatch("dir_leading", args, ctx)
}
fn dirname_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    dispatch("dirname", args, ctx)
}
fn ext_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    dispatch("ext", args, ctx)
}
fn file_exists_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    dispatch("file_exists", args, ctx)
}
fn file_index_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    dispatch("file_index", args, ctx)
}
fn file_trailing_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    dispatch("file_trailing", args, ctx)
}
fn frontmatter_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    dispatch("frontmatter", args, ctx)
}
fn has_command_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    dispatch("has_command", args, ctx)
}
fn has_local_skill_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    dispatch("has_local_skill", args, ctx)
}
fn has_skill_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    dispatch("has_skill", args, ctx)
}
fn increment_file_index_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    dispatch("increment_file_index", args, ctx)
}
fn is_indexed_file_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    dispatch("is_indexed_file", args, ctx)
}
fn join_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    dispatch("join", args, ctx)
}
fn link_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    dispatch("link", args, ctx)
}
fn markdown_body_empty_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    dispatch("markdown_body_empty", args, ctx)
}
fn markdown_title_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    dispatch("markdown_title", args, ctx)
}
fn parent_dir_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    dispatch("parent_dir", args, ctx)
}
fn relative_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    dispatch("relative", args, ctx)
}
fn validate_schema_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    dispatch("validate_schema", args, ctx)
}

mod fn_filesystem {
    use super::*;

    #[test]
    fn pure_functions_dispatch_with_a_context_too() {
        let ctx = ResolutionContext::at(std::path::PathBuf::from("."));
        assert_eq!(call_function("lower", &[json!("x")], Some(&ctx)).unwrap().unwrap(), json!("x"));
    }

    #[test]
    fn dirname_renamed_without_dir_alias() {
        // A relative context directory fails resolution validation.
        let dir = tempfile::TempDir::new().unwrap();
        let ctx = ResolutionContext::at(dir.path().to_path_buf());
        assert_eq!(
            call_function("dirname", &[json!("sub/note.md")], Some(&ctx))
                .unwrap()
                .unwrap(),
            json!("sub")
        );
        assert!(call_function("dir", &[json!("sub/note.md")], Some(&ctx)).is_none());
    }

    #[test]
    fn absolute_and_file_exists_resolve_relative_to_base_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.md"), "# A\n").unwrap();
        let ctx = ResolutionContext::at(dir.path().to_path_buf());

        let abs = absolute_fn(&[json!("a.md")], &ctx).unwrap();
        assert_eq!(
            abs,
            json!(dir.path().join("a.md").to_string_lossy().to_string())
        );

        assert_eq!(file_exists_fn(&[json!("a.md")], &ctx).unwrap(), json!(true));
        assert_eq!(
            file_exists_fn(&[json!("missing.md")], &ctx).unwrap(),
            json!(false)
        );
        // An invalid reference is an error, never a missing file.
        assert!(matches!(
            file_exists_fn(&[json!("\0bad")], &ctx),
            Err(ExpressionError::FileReference(_))
        ));
    }

    #[test]
    fn absolute_repository_scoped_reference_uses_package_before_repository() {
        let repo = tempfile::TempDir::new().unwrap();
        let package_area = repo.path().join("darkmatter");
        let base_dir = package_area.join("docs");
        std::fs::create_dir_all(&base_dir).unwrap();
        std::fs::write(repo.path().join("shared.md"), "repository decoy").unwrap();
        std::fs::write(package_area.join("shared.md"), "package").unwrap();
        let catalog = biscuit_file::RepositoryScopeCatalog::new(
            repo.path(),
            vec![package_area.clone()],
            vec![package_area.clone()],
            biscuit_file::PackageAreaFallback::FirstComponent,
        )
        .unwrap();
        let snapshot = biscuit_file::FileResolutionContext::new(&base_dir)
            .with_repository_scope_catalog(catalog);
        let mut ctx = ResolutionContext::at(base_dir)
            .with_repository_root(repo.path())
            .with_package_area(&package_area);
        ctx.file_resolution_context = snapshot;

        assert_eq!(
            absolute_fn(&[json!("^shared.md")], &ctx).unwrap(),
            json!(package_area.join("shared.md").to_string_lossy().to_string()),
        );
    }

    #[test]
    #[serial_test::serial]
    fn file_exists_does_not_consult_launch_area_fallback_or_process_cwd() {
        // Per D2 the launch-area fallback is not a resolution input for a
        // reference authored inside a document. A file present only under the
        // configured fallback (and not under base_dir) reads as missing, and
        // the ambient process CWD is likewise never consulted.
        let launch_dir = tempfile::TempDir::new().unwrap();
        std::fs::write(launch_dir.path().join("plan.md"), "# Plan\n").unwrap();
        // base_dir deliberately lacks plan.md.
        let base_dir = tempfile::TempDir::new().unwrap();
        // An unrelated directory the process is chdir'd into: plan.md is NOT
        // here either.
        let unrelated_dir = tempfile::TempDir::new().unwrap();
        let ctx = ResolutionContext::at(base_dir.path().to_path_buf())
            .with_file_ref_fallback_dir(launch_dir.path().to_path_buf());

        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(unrelated_dir.path()).unwrap();
        let found = file_exists_fn(&[json!("plan.md")], &ctx);
        std::env::set_current_dir(&original).unwrap();

        assert_eq!(found.unwrap(), json!(false));
    }

    #[test]
    #[serial_test::serial]
    fn file_exists_without_fallback_ignores_process_cwd() {
        // With no explicit fallback, resolution stays document-relative and
        // does NOT consult the mutated ambient CWD — the legacy fallback is
        // gone. A file present only under the process CWD reads as missing.
        let cwd_dir = tempfile::TempDir::new().unwrap();
        std::fs::write(cwd_dir.path().join("ambient.md"), "# Ambient\n").unwrap();
        let base_dir = tempfile::TempDir::new().unwrap();
        let ctx = ResolutionContext::at(base_dir.path().to_path_buf());

        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(cwd_dir.path()).unwrap();
        let found = file_exists_fn(&[json!("ambient.md")], &ctx);
        std::env::set_current_dir(&original).unwrap();

        assert_eq!(found.unwrap(), json!(false));
    }

    #[test]
    fn relative_returns_repo_or_cwd_relative_path() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/a.md"), "# A\n").unwrap();
        let ctx = ResolutionContext::at(dir.path().to_path_buf());
        let rel = relative_fn(&[json!("sub/a.md")], &ctx).unwrap();
        assert_eq!(rel, json!("sub/a.md"));
    }

    #[test]
    fn frontmatter_reads_whole_map_and_single_prop() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("d.md"),
            "---\ntitle: Hi\nstatus: draft\n---\nBody\n",
        )
        .unwrap();
        let ctx = ResolutionContext::at(dir.path().to_path_buf());

        let whole = frontmatter_fn(&[json!("d.md")], &ctx).unwrap();
        assert_eq!(whole["title"], json!("Hi"));

        let one = frontmatter_fn(&[json!("d.md"), json!("status")], &ctx).unwrap();
        assert_eq!(one, json!("draft"));

        // Missing prop → null (not error).
        let missing = frontmatter_fn(&[json!("d.md"), json!("nope")], &ctx).unwrap();
        assert_eq!(missing, Value::Null);

        // Invalid filepath → error.
        assert!(frontmatter_fn(&[json!("does-not-exist.md")], &ctx).is_err());
    }

    #[test]
    #[cfg(unix)]
    fn load_markdown_unreadable_file_yields_typed_file_reference() {
        use std::os::unix::fs::PermissionsExt;
        // A reference that resolves to a real file but cannot be read
        // (permission denied) must surface through the shared FileReference
        // diagnostic carrying the typed `Io` cause — not an opaque `Other`
        // string — so it renders like any other invalid reference (finding #4).
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("locked.md");
        std::fs::write(&path, "---\ntitle: T\n---\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();

        // Running as root bypasses the permission bits — skip rather than flake.
        if std::fs::read_to_string(&path).is_ok() {
            return;
        }
        let ctx = ResolutionContext::at(dir.path().to_path_buf());

        match frontmatter_fn(&[json!("locked.md")], &ctx).unwrap_err() {
            ExpressionError::FileReference(diagnostic) => {
                assert_eq!(diagnostic.function, "frontmatter");
                assert!(
                    diagnostic.source.is_some(),
                    "the typed Io cause must be preserved"
                );
            }
            other => panic!("expected a FileReference diagnostic, got: {other:?}"),
        }
    }

    #[test]
    fn markdown_body_empty_and_title() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("empty.md"), "---\ntitle: T\n---\n\n   \n").unwrap();
        std::fs::write(dir.path().join("full.md"), "---\n---\n# Heading\n\nWords\n").unwrap();
        std::fs::write(dir.path().join("fm_title.md"), "---\ntitle: FM\n---\n# H1\n").unwrap();
        let ctx = ResolutionContext::at(dir.path().to_path_buf());

        assert_eq!(
            markdown_body_empty_fn(&[json!("empty.md")], &ctx).unwrap(),
            json!(true)
        );
        assert_eq!(
            markdown_body_empty_fn(&[json!("full.md")], &ctx).unwrap(),
            json!(false)
        );
        // Frontmatter title wins over H1.
        assert_eq!(
            markdown_title_fn(&[json!("fm_title.md")], &ctx).unwrap(),
            json!("FM")
        );
        // No frontmatter title → first H1.
        assert_eq!(
            markdown_title_fn(&[json!("full.md")], &ctx).unwrap(),
            json!("Heading")
        );
    }

    #[test]
    fn validate_schema_true_when_no_schema_property() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("plain.md"), "---\ntitle: T\n---\nBody\n").unwrap();
        let ctx = ResolutionContext::at(dir.path().to_path_buf());
        assert_eq!(
            validate_schema_fn(&[json!("plain.md")], &ctx).unwrap(),
            json!(true)
        );
    }

    // A near-universal shell binary, chosen per-OS so the PATH probe also
    // exercises Windows `PATHEXT` extension resolution (`cmd` → `cmd.exe`).
    #[cfg(windows)]
    const PROBE_BINARY: &str = "cmd";
    #[cfg(not(windows))]
    const PROBE_BINARY: &str = "sh";

    #[test]
    fn has_command_found_on_path_returns_true() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        // Guard behind an independent probe so a minimal CI image lacking
        // the binary skips the positive assertion instead of failing.
        if which::which(PROBE_BINARY).is_ok() {
            assert_eq!(
                has_command_fn(&[json!(PROBE_BINARY)], &ctx).unwrap(),
                json!(true)
            );
        }
    }

    #[test]
    fn has_command_missing_returns_false() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        assert_eq!(
            has_command_fn(&[json!("definitely-not-a-real-bin-zzz")], &ctx).unwrap(),
            json!(false)
        );
    }

    #[test]
    fn has_command_null_and_empty_read_as_false() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        for arg in [json!(null), json!("")] {
            assert_eq!(
                has_command_fn(std::slice::from_ref(&arg), &ctx).unwrap(),
                json!(false),
                "arg: {arg}"
            );
        }
    }

    /// Ledger row 81 and criterion 44: `has_command(cmd: string | null)`. A
    /// number or boolean converts to its text and is looked up by that name,
    /// so `has_command(true)` looks for a command named `true`; a list or an
    /// object is a type error raised before the handler.
    #[test]
    fn has_command_converts_scalars_and_rejects_containers() {
        use crate::markdown::compose::expression::functions::test_hooks::{handler_entries, reset_handler_entries};

        let ctx = ResolutionContext::at(std::env::temp_dir());
        for (converted, text) in [(json!(true), "true"), (json!(false), "false"), (json!(42), "42")] {
            assert_eq!(
                has_command_fn(std::slice::from_ref(&converted), &ctx).unwrap(),
                has_command_fn(&[json!(text)], &ctx).unwrap(),
                "{converted} probes {text:?}"
            );
        }
        // `true` is a POSIX utility on every Unix host's `PATH`.
        #[cfg(unix)]
        assert_eq!(has_command_fn(&[json!(true)], &ctx).unwrap(), json!(true));

        reset_handler_entries();
        for container in [json!([]), json!({"a": 1})] {
            let error = has_command_fn(std::slice::from_ref(&container), &ctx).unwrap_err();
            assert!(matches!(error, ExpressionError::Call(_)), "{container}: {error:?}");
        }
        assert_eq!(handler_entries("has_command"), 0);
    }

    #[test]
    fn has_command_absolute_path_to_missing_returns_false() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        let missing = std::env::temp_dir().join("definitely-not-a-real-zzz-bin");
        assert_eq!(
            has_command_fn(&[json!(missing.to_string_lossy().to_string())], &ctx).unwrap(),
            json!(false)
        );
    }

    #[test]
    #[cfg(unix)]
    fn has_command_absolute_path_to_executable_returns_true() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("tool");
        std::fs::write(&path, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        let ctx = ResolutionContext::at(dir.path().to_path_buf());
        assert_eq!(
            has_command_fn(&[json!(path.to_string_lossy().to_string())], &ctx).unwrap(),
            json!(true)
        );
    }

    #[test]
    #[cfg(unix)]
    fn has_command_absolute_path_to_non_executable_returns_false() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("data.txt");
        std::fs::write(&path, "not executable\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        let ctx = ResolutionContext::at(dir.path().to_path_buf());
        assert_eq!(
            has_command_fn(&[json!(path.to_string_lossy().to_string())], &ctx).unwrap(),
            json!(false)
        );
    }

    #[test]
    fn has_command_directory_returns_false() {
        let dir = tempfile::TempDir::new().unwrap();
        let ctx = ResolutionContext::at(dir.path().to_path_buf());
        assert_eq!(
            has_command_fn(&[json!(dir.path().to_string_lossy().to_string())], &ctx).unwrap(),
            json!(false)
        );
    }

    #[test]
    fn has_command_tilde_is_not_expanded() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        assert_eq!(
            has_command_fn(&[json!("~/bin/x")], &ctx).unwrap(),
            json!(false)
        );
    }

    #[test]
    fn has_command_relative_path_is_not_resolved() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        assert_eq!(
            has_command_fn(&[json!("./mytool")], &ctx).unwrap(),
            json!(false)
        );
        assert_eq!(
            has_command_fn(&[json!("bin/mytool")], &ctx).unwrap(),
            json!(false)
        );
    }

    #[test]
    #[serial_test::serial]
    fn has_command_relative_path_ignores_existing_cwd_executable() {
        // Regression: `which::which` resolves relative paths against the
        // process CWD, so a real executable at `./mytool` used to read as
        // `true`. The spec pins relative paths to `false` regardless of
        // what exists under the CWD.
        #[cfg(windows)]
        const TOOL: &str = "mytool.bat";
        #[cfg(not(windows))]
        const TOOL: &str = "mytool";

        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join("bin")).unwrap();
        for rel in [TOOL, &format!("bin/{TOOL}")] {
            let path = dir.path().join(rel);
            std::fs::write(&path, "#!/bin/sh\n").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(
                    &path,
                    std::fs::Permissions::from_mode(0o755),
                )
                .unwrap();
            }
        }
        let ctx = ResolutionContext::at(dir.path().to_path_buf());

        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let dot_relative = has_command_fn(&[json!(format!("./{TOOL}"))], &ctx);
        let bare_relative = has_command_fn(&[json!(format!("bin/{TOOL}"))], &ctx);
        std::env::set_current_dir(&original).unwrap();

        assert_eq!(dot_relative.unwrap(), json!(false));
        assert_eq!(bare_relative.unwrap(), json!(false));
    }

    #[test]
    fn has_command_requires_exactly_one_arg() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        assert!(has_command_fn(&[], &ctx).is_err());
        assert!(has_command_fn(&[json!("sh"), json!("extra")], &ctx).is_err());
    }

    #[test]
    fn has_command_dispatches_by_canonical_and_alias() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        assert_eq!(
            call_function("has_command", &[json!("definitely-not-a-real-bin-zzz")], Some(&ctx))
                .unwrap()
                .unwrap(),
            json!(false)
        );
        assert_eq!(
            call_function("hascommand", &[json!("definitely-not-a-real-bin-zzz")], Some(&ctx))
                .unwrap()
                .unwrap(),
            json!(false)
        );
    }
}

mod fn_phase4 {
    use super::*;

    fn ctx_with_temp_dir() -> (tempfile::TempDir, ResolutionContext) {
        let dir = tempfile::TempDir::new().unwrap();
        let ctx = ResolutionContext::at(dir.path().to_path_buf());
        (dir, ctx)
    }

    #[test]
    fn is_indexed_file_detects_indexed_stems() {
        let (dir, ctx) = ctx_with_temp_dir();
        std::fs::write(dir.path().join("review-1.md"), "x").unwrap();
        std::fs::write(dir.path().join("review.md"), "x").unwrap();
        std::fs::write(dir.path().join("review_1.md"), "x").unwrap();

        assert_eq!(is_indexed_file_fn(&[json!("review-1.md")], &ctx).unwrap(), json!(true));
        assert_eq!(is_indexed_file_fn(&[json!("review-100.md")], &ctx).unwrap(), json!(true));
        assert_eq!(is_indexed_file_fn(&[json!("review-001.md")], &ctx).unwrap(), json!(true));
        assert_eq!(is_indexed_file_fn(&[json!("review.md")], &ctx).unwrap(), json!(false));
        assert_eq!(is_indexed_file_fn(&[json!("review_1.md")], &ctx).unwrap(), json!(false));
        assert_eq!(is_indexed_file_fn(&[json!("review1.md")], &ctx).unwrap(), json!(false));
    }

    #[test]
    fn file_index_parses_or_negative_one() {
        let (dir, ctx) = ctx_with_temp_dir();
        std::fs::write(dir.path().join("review-42.md"), "x").unwrap();
        std::fs::write(dir.path().join("review.md"), "x").unwrap();

        assert_eq!(file_index_fn(&[json!("review-42.md")], &ctx).unwrap(), json!(42));
        assert_eq!(file_index_fn(&[json!("review.md")], &ctx).unwrap(), json!(-1));
    }

    #[test]
    fn increment_file_index_preserves_zero_padding() {
        let (dir, ctx) = ctx_with_temp_dir();
        std::fs::write(dir.path().join("review-1.md"), "x").unwrap();
        std::fs::write(dir.path().join("review-001.md"), "x").unwrap();
        std::fs::write(dir.path().join("review.md"), "x").unwrap();

        assert_eq!(
            increment_file_index_fn(&[json!("review-1.md")], &ctx).unwrap(),
            json!("review-2.md")
        );
        assert_eq!(
            increment_file_index_fn(&[json!("review-001.md")], &ctx).unwrap(),
            json!("review-002.md")
        );
        assert_eq!(
            increment_file_index_fn(&[json!("review.md")], &ctx).unwrap(),
            json!("review-2.md")
        );
    }

    #[test]
    fn decrement_file_index_clamps_at_zero() {
        let (dir, ctx) = ctx_with_temp_dir();
        std::fs::write(dir.path().join("review-001.md"), "x").unwrap();
        std::fs::write(dir.path().join("review-0.md"), "x").unwrap();
        std::fs::write(dir.path().join("review.md"), "x").unwrap();

        assert_eq!(
            decrement_file_index_fn(&[json!("review-001.md")], &ctx).unwrap(),
            json!("review-000.md")
        );
        assert_eq!(
            decrement_file_index_fn(&[json!("review-0.md")], &ctx).unwrap(),
            json!("review-0.md")
        );
        assert_eq!(
            decrement_file_index_fn(&[json!("review.md")], &ctx).unwrap(),
            json!("review-0.md")
        );
    }

    #[test]
    fn indexed_functions_null_propagate_and_reject_remote() {
        let ctx = ResolutionContext::at(std::env::temp_dir());

        // Ledger row 83: `is_indexed_file` is a predicate, so `null` is `false`.
        assert_eq!(is_indexed_file_fn(&[json!(null)], &ctx).unwrap(), json!(false));
        assert_eq!(file_index_fn(&[json!(null)], &ctx).unwrap(), json!(null));
        assert_eq!(
            increment_file_index_fn(&[json!(null)], &ctx).unwrap(),
            json!(null)
        );

        let err = is_indexed_file_fn(&[json!("https://example.com/doc.md")], &ctx)
            .unwrap_err();
        assert!(err.contains("HTTP(S)"), "got: {err}");
    }

    #[test]
    fn indexed_functions_require_arity() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        assert!(is_indexed_file_fn(&[], &ctx).is_err());
        assert!(is_indexed_file_fn(&[json!("a"), json!("b")], &ctx).is_err());
        assert!(file_index_fn(&[], &ctx).is_err());
        assert!(increment_file_index_fn(&[json!("a"), json!("b")], &ctx).is_err());
    }

    #[test]
    fn path_components_split_display_path() {
        let (dir, ctx) = ctx_with_temp_dir();
        std::fs::create_dir_all(dir.path().join("foo/bar/baz")).unwrap();
        std::fs::write(dir.path().join("foo/bar/baz/test.md"), "x").unwrap();
        std::fs::write(dir.path().join("foo/review-1.md"), "x").unwrap();
        std::fs::write(dir.path().join("no-ext"), "x").unwrap();

        assert_eq!(basename_fn(&[json!("foo/bar/baz/test.md")], &ctx).unwrap(), json!("test.md"));
        assert_eq!(
            basename_without_index_fn(&[json!("foo/review-1.md")], &ctx).unwrap(),
            json!("review.md")
        );
        assert_eq!(dirname_fn(&[json!("foo/bar/baz/test.md")], &ctx).unwrap(), json!("foo/bar/baz"));
        assert_eq!(ext_fn(&[json!("foo/bar/baz/test.md")], &ctx).unwrap(), json!("md"));
        assert_eq!(ext_fn(&[json!("no-ext")], &ctx).unwrap(), json!(""));
        assert_eq!(
            parent_dir_fn(&[json!("foo/bar/baz/test.md")], &ctx).unwrap(),
            json!("baz")
        );
        assert_eq!(
            file_trailing_fn(&[json!("foo/bar/baz/test.md")], &ctx).unwrap(),
            json!("baz/test.md")
        );
        assert_eq!(
            dir_leading_fn(&[json!("foo/bar/baz/test.md")], &ctx).unwrap(),
            json!("foo/bar")
        );
    }

    #[test]
    fn path_components_handle_bare_basename() {
        let (dir, ctx) = ctx_with_temp_dir();
        std::fs::write(dir.path().join("test.md"), "x").unwrap();

        assert_eq!(basename_fn(&[json!("test.md")], &ctx).unwrap(), json!("test.md"));
        assert_eq!(dirname_fn(&[json!("test.md")], &ctx).unwrap(), json!(""));
        assert_eq!(parent_dir_fn(&[json!("test.md")], &ctx).unwrap(), json!(""));
        assert_eq!(file_trailing_fn(&[json!("test.md")], &ctx).unwrap(), json!("test.md"));
        assert_eq!(dir_leading_fn(&[json!("test.md")], &ctx).unwrap(), json!(""));
    }

    #[test]
    fn join_normalizes_separators_and_resolves() {
        let (dir, ctx) = ctx_with_temp_dir();
        std::fs::create_dir_all(dir.path().join("foo/bar")).unwrap();
        std::fs::write(dir.path().join("foo/bar/baz.md"), "x").unwrap();

        assert_eq!(
            join_fn(&[json!("foo/bar/"), json!("/baz.md")], &ctx).unwrap(),
            json!("foo/bar/baz.md")
        );
        assert_eq!(
            join_fn(&[json!("foo/bar"), json!("baz/bax.md")], &ctx).unwrap(),
            json!("foo/bar/baz/bax.md")
        );
        assert_eq!(
            join_fn(&[json!("foo//bar//"), json!("//baz.md")], &ctx).unwrap(),
            json!("foo/bar/baz.md")
        );
    }

    #[test]
    fn join_rejects_http_and_null_propagates() {
        let ctx = ResolutionContext::at(std::env::temp_dir());

        assert_eq!(join_fn(&[json!(null), json!("b")], &ctx).unwrap(), json!(null));
        let err = join_fn(&[json!("https://example.com"), json!("b")], &ctx).unwrap_err();
        assert!(err.contains("HTTP(S)"), "got: {err}");
        let err = join_fn(&[json!("a"), json!("https://example.com/b")], &ctx).unwrap_err();
        assert!(err.contains("HTTP(S)"), "got: {err}");
    }

    #[test]
    fn join_requires_strings_and_correct_arity() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        assert!(join_fn(&[], &ctx).is_err());
        assert!(join_fn(&[json!("a")], &ctx).is_err());
        assert!(join_fn(&[json!("a"), json!([])], &ctx).is_err());
        // Ledger row 96: a number converts to its text.
        assert_eq!(join_fn(&[json!(123), json!("b")], &ctx).unwrap(), join_fn(&[json!("123"), json!("b")], &ctx).unwrap());
    }

    /// A missing implicit bare reference is shaped from the document CWD
    /// first. The explicit `./` form pins to the same source directory.
    #[test]
    fn missing_implicit_path_shape_is_document_cwd_first() {
        let repo = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        let base = repo.path().join("prompts");
        std::fs::create_dir_all(&base).unwrap();
        let ctx = ResolutionContext::at(base.clone())
            .with_repository_root(repo.path().to_path_buf());

        // Implicit bare miss → document-CWD candidate.
        let implicit = resolve_path_shape("dirname", "sub/missing.md", &ctx).unwrap();
        assert_eq!(implicit, base.join("sub/missing.md"));

        // Explicit `./` miss → source-directory candidate only.
        let explicit = resolve_path_shape("dirname", "./sub/missing.md", &ctx).unwrap();
        assert_eq!(explicit, base.join("./sub/missing.md"));
    }

    #[test]
    fn path_functions_do_not_require_existence() {
        let ctx = ResolutionContext::at(std::env::temp_dir());

        assert_eq!(basename_fn(&[json!("foo/bar/missing.md")], &ctx).unwrap(), json!("missing.md"));
        assert_eq!(dirname_fn(&[json!("foo/bar/missing.md")], &ctx).unwrap(), json!("foo/bar"));
        assert_eq!(
            increment_file_index_fn(&[json!("foo/bar/missing.md")], &ctx).unwrap(),
            json!("foo/bar/missing-2.md")
        );
    }

    #[test]
    fn path_functions_dispatch_by_name() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        assert_eq!(
            call_function("is_indexed_file", &[json!("foo-1.md")], Some(&ctx))
                .unwrap()
                .unwrap(),
            json!(true)
        );
        assert_eq!(
            call_function("file_index", &[json!("foo-7.md")], Some(&ctx)).unwrap().unwrap(),
            json!(7)
        );
        assert_eq!(
            call_function("basename", &[json!("foo/bar.md")], Some(&ctx)).unwrap().unwrap(),
            json!("bar.md")
        );
        assert_eq!(
            call_function("dirname", &[json!("foo/bar.md")], Some(&ctx)).unwrap().unwrap(),
            json!("foo")
        );
        assert_eq!(
            call_function("join", &[json!("foo"), json!("bar.md")], Some(&ctx))
                .unwrap()
                .unwrap(),
            json!("foo/bar.md")
        );
    }
}
mod fn_phase5 {
    use super::*;

    fn ctx_with_temp_dir() -> (tempfile::TempDir, ResolutionContext) {
        let dir = tempfile::TempDir::new().unwrap();
        let ctx = ResolutionContext::at(dir.path().to_path_buf());
        (dir, ctx)
    }

    #[test]
    fn link_one_arg_uses_relative_text_and_absolute_destination() {
        let (dir, ctx) = ctx_with_temp_dir();
        std::fs::create_dir_all(dir.path().join("foo/bar")).unwrap();
        std::fs::write(dir.path().join("foo/bar/baz.md"), "x").unwrap();

        let result = link_fn(&[json!("foo/bar/baz.md")], &ctx)
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        assert!(result.starts_with("[foo/bar/baz.md]("));
        assert!(result.contains("/foo/bar/baz.md)"));
    }

    #[test]
    fn link_two_arg_file_uses_provided_description() {
        let (dir, ctx) = ctx_with_temp_dir();
        std::fs::write(dir.path().join("doc.md"), "x").unwrap();

        let result = link_fn(&[json!("doc.md"), json!("My Doc")], &ctx)
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        assert_eq!(
            result,
            format!(
                "[My Doc]({})",
                biscuit_file::to_portable_string(&dir.path().join("doc.md"))
            )
        );
    }

    #[test]
    fn link_two_arg_https_emits_url_destination() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        let result = link_fn(
            &[
                json!("https://example.com/page"),
                json!("Example"),
            ],
            &ctx,
        )
        .unwrap();
        assert_eq!(result, json!("[Example](https://example.com/page)"));
    }

    #[test]
    fn link_escapes_brackets_in_text() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        let result = link_fn(
            &[
                json!("https://example.com"),
                json!("click [here]"),
            ],
            &ctx,
        )
        .unwrap();
        assert_eq!(
            result,
            json!("[click \\[here\\]](https://example.com)")
        );
    }

    #[test]
    fn link_wraps_destination_with_spaces_and_parens() {
        let (dir, ctx) = ctx_with_temp_dir();
        std::fs::write(dir.path().join("my doc (v1).md"), "x").unwrap();

        let result = link_fn(
            &[
                json!("my doc (v1).md"),
                json!("spaces"),
            ],
            &ctx,
        )
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
        assert!(
            result.starts_with("[spaces](<"),
            "expected angle-bracket wrapping for destination with spaces, got {result:?}"
        );
        assert!(result.ends_with(">)"));
    }

    /// Returns the visible text of the single link in `markdown`.
    fn link_label(markdown: &str) -> String {
        use pulldown_cmark::{Event, Parser, Tag, TagEnd};

        let mut label = String::new();
        let mut inside = false;
        for event in Parser::new(markdown) {
            match event {
                Event::Start(Tag::Link { .. }) => inside = true,
                Event::End(TagEnd::Link) => inside = false,
                Event::Text(text) if inside => label.push_str(&text),
                _ => {}
            }
        }
        label
    }

    /// A label carrying a native Windows spelling must survive a CommonMark
    /// round-trip character for character.
    ///
    /// `make_portable_relative_in_context` falls back to the native
    /// spelling when no faithful portable one exists, so a `link()`
    /// description can contain literal backslashes. CommonMark honors a
    /// backslash before ASCII punctuation inside link text, so escaping
    /// only `[` and `]` would silently delete the separators — and the
    /// leading `\\` would arrive as a single `\`.
    #[test]
    fn native_fallback_label_round_trips_through_commonmark() {
        let label = r"\\server\share\notes\[draft].md";

        let rendered = format_markdown_link(label, "C:/repo/notes.md");

        assert_eq!(link_label(&rendered), label);
    }

    /// Both local-destination arms refuse a path with no faithful portable
    /// spelling rather than emitting Markdown that parses back as a
    /// different destination.
    ///
    /// Windows-only: off Windows every path portabilizes, so there is no
    /// decline to simulate.
    #[cfg(windows)]
    #[test]
    fn link_local_destination_arms_reject_declined_paths() {
        // A literal `.` component under a verbatim prefix is a real
        // directory name, so `dunce` refuses to reduce the prefix and the
        // path has no portable spelling.
        let ctx = ResolutionContext::at(std::path::PathBuf::from(r"\\?\C:\repo\.\docs"));

        let one_arg = link_fn(&[json!("sibling.md")], &ctx).unwrap_err();
        assert!(
            one_arg.contains("no faithful portable spelling"),
            "got: {one_arg}"
        );

        let two_arg = link_fn(&[json!("sibling.md"), json!("Sibling")], &ctx).unwrap_err();
        assert!(
            two_arg.contains("no faithful portable spelling"),
            "got: {two_arg}"
        );
    }

    #[test]
    fn link_one_arg_rejects_remote_url() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        let err = link_fn(
            &[json!("https://example.com/doc.md")],
            &ctx,
        )
        .unwrap_err();
        assert!(err.contains("HTTP(S)"), "got: {err}");
    }

    #[test]
    fn basename_rejects_uppercase_scheme_urls() {
        let ctx = ResolutionContext::at(std::env::temp_dir());

        let err = basename_fn(&[json!("HTTPS://example.com/doc.md")], &ctx)
            .unwrap_err();
        assert!(err.contains("HTTP(S)"), "got: {err}");
        let err = basename_fn(&[json!("hTtPs://example.com/doc.md")], &ctx)
            .unwrap_err();
        assert!(err.contains("HTTP(S)"), "got: {err}");
    }

    #[test]
    fn join_rejects_uppercase_scheme_urls() {
        let ctx = ResolutionContext::at(std::env::temp_dir());

        let err = join_fn(&[json!("HTTPS://example.com"), json!("b")], &ctx)
            .unwrap_err();
        assert!(err.contains("HTTP(S)"), "got: {err}");
        let err = join_fn(&[json!("a"), json!("HTTP://example.com/b")], &ctx)
            .unwrap_err();
        assert!(err.contains("HTTP(S)"), "got: {err}");
    }

    #[test]
    fn link_one_arg_rejects_uppercase_scheme_urls() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        let err = link_fn(&[json!("HTTPS://example.com/doc.md")], &ctx).unwrap_err();
        assert!(err.contains("HTTP(S)"), "got: {err}");
    }

    #[test]
    fn link_two_arg_accepts_uppercase_scheme_urls() {
        let ctx = ResolutionContext::at(std::env::temp_dir());

        let result = link_fn(
            &[json!("HTTPS://example.com/page"), json!("Example")],
            &ctx,
        )
        .unwrap();
        assert_eq!(result, json!("[Example](HTTPS://example.com/page)"));
        let result = link_fn(
            &[json!("hTtP://example.com/page"), json!("Example")],
            &ctx,
        )
        .unwrap();
        assert_eq!(result, json!("[Example](hTtP://example.com/page)"));
    }

    #[test]
    fn link_null_propagates_and_arity_errors() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        assert_eq!(
            link_fn(&[json!(null), json!("desc")], &ctx).unwrap(),
            json!(null)
        );
        assert_eq!(
            link_fn(&[json!("https://example.com"), json!(null)], &ctx).unwrap(),
            json!(null)
        );
        assert!(link_fn(&[], &ctx).is_err());
        assert!(link_fn(&[json!("a"), json!("b"), json!("c")], &ctx).is_err());
    }

    #[test]
    fn link_type_mismatch_errors() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        assert!(matches!(link_fn(&[json!([])], &ctx), Err(ExpressionError::Call(_))));
        assert!(matches!(link_fn(&[json!("a"), json!({})], &ctx), Err(ExpressionError::Call(_))));
        // Ledger row 97: numbers convert to text, for the target and the
        // description alike.
        assert_eq!(link_fn(&[json!("a"), json!(123)], &ctx).unwrap(), link_fn(&[json!("a"), json!("123")], &ctx).unwrap());
        assert_eq!(link_fn(&[json!(123)], &ctx).unwrap(), link_fn(&[json!("123")], &ctx).unwrap());
    }

    /// Returns the visible text, code spans included, of the single link
    /// in `markdown`, and whether that text came from exactly one code span.
    fn link_code_label(markdown: &str) -> (String, bool) {
        use pulldown_cmark::{Event, Parser, Tag, TagEnd};

        let mut label = String::new();
        let mut code_spans = 0;
        let mut other_text = false;
        let mut inside = false;
        for event in Parser::new(markdown) {
            match event {
                Event::Start(Tag::Link { .. }) => inside = true,
                Event::End(TagEnd::Link) => inside = false,
                Event::Code(code) if inside => {
                    code_spans += 1;
                    label.push_str(&code);
                }
                Event::Text(text) if inside => {
                    other_text = true;
                    label.push_str(&text);
                }
                _ => {}
            }
        }
        (label, code_spans == 1 && !other_text)
    }

    fn code_link(args: &[Value], ctx: &ResolutionContext) -> String {
        code_link_fn(args, ctx).unwrap().as_str().unwrap().to_string()
    }

    /// Splits `[label](destination)` at the `](` that ends the label.
    fn split_link(markdown: &str) -> (&str, &str) {
        let index = markdown.rfind("](").expect("a Markdown link");
        (&markdown[1..index], &markdown[index + 2..markdown.len() - 1])
    }

    #[test]
    fn code_link_one_arg_matches_link_destination_with_a_code_label() {
        let (dir, ctx) = ctx_with_temp_dir();
        std::fs::create_dir_all(dir.path().join("plans")).unwrap();
        std::fs::write(dir.path().join("plans/foo.md"), "x").unwrap();
        std::fs::write(dir.path().join("my doc (v1).md"), "x").unwrap();

        for file in ["plans/foo.md", "my doc (v1).md"] {
            let plain = link_fn(&[json!(file)], &ctx).unwrap();
            let plain = plain.as_str().unwrap();
            let code = code_link(&[json!(file)], &ctx);

            let (plain_label, plain_dest) = split_link(plain);
            let (code_label, code_dest) = split_link(&code);
            assert_eq!(code_dest, plain_dest, "same destination spelling and quoting");
            assert_eq!(code_label, format!("`{plain_label}`"));
            assert_eq!(link_code_label(&code), (file.to_string(), true), "{code}");
        }
        assert_eq!(
            code_link(&[json!("plans/foo.md")], &ctx),
            format!(
                "[`plans/foo.md`]({})",
                biscuit_file::to_portable_string(&dir.path().join("plans/foo.md"))
            )
        );
    }

    #[test]
    fn code_link_two_arg_file_and_url_destinations_match_link() {
        let (dir, ctx) = ctx_with_temp_dir();
        std::fs::write(dir.path().join("doc.md"), "x").unwrap();

        for target in ["doc.md", "https://example.com/page", "HTTPS://example.com/page"] {
            let plain = link_fn(&[json!(target), json!("md hash")], &ctx).unwrap();
            let code = code_link(&[json!(target), json!("md hash")], &ctx);
            assert_eq!(split_link(&code).1, split_link(plain.as_str().unwrap()).1);
            assert_eq!(split_link(&code).0, "`md hash`");
        }
    }

    #[test]
    fn code_link_keeps_brackets_unescaped_inside_the_code_span() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        let url = json!("https://example.com");

        let rendered = code_link(&[url.clone(), json!("a]b")], &ctx);
        assert_eq!(rendered, "[`a]b`](https://example.com)");
        assert_eq!(link_code_label(&rendered), ("a]b".to_string(), true));

        let rendered = code_link(&[url, json!("click [here]")], &ctx);
        assert_eq!(rendered, "[`click [here]`](https://example.com)");
        assert_eq!(link_code_label(&rendered), ("click [here]".to_string(), true));
    }

    #[test]
    fn code_link_emits_backslashes_as_is() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        let rendered = code_link(&[json!("https://example.com"), json!(r"a\_b")], &ctx);
        assert_eq!(rendered, r"[`a\_b`](https://example.com)");
        assert_eq!(link_code_label(&rendered), (r"a\_b".to_string(), true));
    }

    #[test]
    fn code_link_fences_backticks_and_pads_edges() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        let url = json!("https://example.com");
        for (label, expected) in [
            ("a`b", "[``a`b``](https://example.com)"),
            ("``x``", "[``` ``x`` ```](https://example.com)"),
            ("`a`", "[`` `a` ``](https://example.com)"),
            (" a ", "[`  a  `](https://example.com)"),
        ] {
            let rendered = code_link(&[url.clone(), json!(label)], &ctx);
            assert_eq!(rendered, expected, "{label:?}");
            assert_eq!(link_code_label(&rendered), (label.to_string(), true), "{label:?}");
        }
    }

    #[test]
    fn code_link_turns_line_endings_into_spaces() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        for label in ["one\ntwo", "one\r\ntwo", "one\rtwo"] {
            let rendered = code_link(&[json!("https://example.com"), json!(label)], &ctx);
            assert_eq!(rendered, "[`one two`](https://example.com)", "{label:?}");
        }
    }

    #[test]
    fn code_link_empty_label_is_an_ordinary_empty_link() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        let rendered = code_link(&[json!("https://example.com"), json!("")], &ctx);
        assert_eq!(rendered, "[](https://example.com)");
        assert_eq!(
            link_fn(&[json!("https://example.com"), json!("")], &ctx).unwrap(),
            json!(rendered)
        );
    }

    #[test]
    fn code_link_null_propagates_like_link() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        for args in [
            vec![json!(null)],
            vec![json!(null), json!("desc")],
            vec![json!("https://example.com"), json!(null)],
        ] {
            assert_eq!(code_link_fn(&args, &ctx).unwrap(), json!(null), "{args:?}");
            assert_eq!(link_fn(&args, &ctx).unwrap(), json!(null), "{args:?}");
        }
    }

    /// Each failure carries `link()`'s cause under the name the caller
    /// invoked, and `link()`'s own error text is unchanged.
    #[test]
    fn code_link_errors_name_code_link_with_the_link_cause() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        let cases: [(Vec<Value>, &str); 3] = [
            (vec![json!("https://example.com/doc.md")], "one-argument form does not accept HTTP(S) URLs"),
            // Neither parses as a URL, so both fail as file paths.
            (vec![json!("http://[::1"), json!("x")], "invalid file path"),
            (vec![json!("https://exa mple.com"), json!("x")], ""),
        ];
        for (args, cause) in cases {
            let link_err = link_fn(&args, &ctx).unwrap_err();
            let code_err = code_link_fn(&args, &ctx).unwrap_err();
            let function = match &code_err {
                ExpressionError::Other { function, .. } => Some(function.as_str()),
                ExpressionError::FileReference(diagnostic) => Some(diagnostic.function),
                _ => None,
            };
            assert!(matches!(function, None | Some("code_link")), "{code_err:?}");
            assert!(code_err.to_string().contains(cause), "{code_err}");
            assert!(link_err.to_string().contains(cause), "{link_err}");
            assert_eq!(
                code_err.to_string(),
                link_err.to_string().replace("link(", "code_link("),
                "{args:?}"
            );
        }
        assert_eq!(
            link_fn(&[json!("https://example.com/doc.md")], &ctx).unwrap_err().to_string(),
            "link(): link() one-argument form does not accept HTTP(S) URLs; use link(target, desc)"
        );
        // A wrong argument count fails at binding, under the called name.
        for args in [vec![], vec![json!("a"), json!("b"), json!("c")]] {
            let error = code_link_fn(&args, &ctx).unwrap_err();
            assert!(matches!(error, ExpressionError::Call(_)), "{error:?}");
            assert!(error.to_string().starts_with("code_link:"), "{error}");
        }
        assert!(code_link_fn(&[json!("a"), json!([])], &ctx).is_err());
    }

    #[test]
    fn code_link_is_dispatched_by_name_and_alias() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        let args = [json!("https://example.com"), json!("md hash")];
        for name in ["code_link", "codelink"] {
            assert_eq!(
                call_function(name, &args, Some(&ctx)).unwrap().unwrap(),
                json!("[`md hash`](https://example.com)"),
                "{name}"
            );
        }
    }

    #[test]
    fn has_skill_finds_user_and_local_roots() {
        let home = tempfile::TempDir::new().unwrap();
        let local = tempfile::TempDir::new().unwrap();
        let ctx = ResolutionContext::at(local.path().to_path_buf())
            .with_home_dir(home.path().to_path_buf())
            .with_ctx_value("agent", json!("claude"));

        std::fs::create_dir_all(home.path().join(".claude/skills/user-skill")).unwrap();
        std::fs::create_dir_all(local.path().join(".claude/skills/local-skill")).unwrap();

        assert_eq!(
            has_skill_fn(&[json!("user-skill")], &ctx).unwrap(),
            json!(true)
        );
        assert_eq!(
            has_skill_fn(&[json!("local-skill")], &ctx).unwrap(),
            json!(true)
        );
    }

    #[test]
    fn skill_lookup_reuses_request_snapshot_roots() {
        let request_repo = tempfile::TempDir::new().unwrap();
        let nested_repo = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(request_repo.path().join(".claude/skills/captured")).unwrap();
        std::fs::create_dir_all(nested_repo.path().join(".git")).unwrap();
        let snapshot = biscuit_file::FileResolutionContext::new(request_repo.path())
            .with_repository_root(request_repo.path())
            .without_home_dir();
        let mut ctx = ResolutionContext::at(nested_repo.path().to_path_buf())
            .with_ctx_value("agent", json!("claude"));
        ctx.file_resolution_context = snapshot;

        assert_eq!(ctx.home_dir(), None);
        assert_eq!(
            has_local_skill_fn(&[json!("captured")], &ctx).unwrap(),
            json!(true),
        );
    }

    #[test]
    fn has_skill_finds_cross_agent_local_roots() {
        // Local-scoped roots are shared across all recognized agents: a skill
        // placed in any agent's local directory is discoverable by any other
        // recognized agent via both `has_skill` and `has_local_skill`.
        let home = tempfile::TempDir::new().unwrap();
        let local = tempfile::TempDir::new().unwrap();

        std::fs::create_dir_all(local.path().join(".opencode/skill/cross-skill")).unwrap();
        std::fs::create_dir_all(local.path().join(".claude/skills/cross-skill")).unwrap();

        let claude_ctx = ResolutionContext::at(local.path().to_path_buf())
            .with_home_dir(home.path().to_path_buf())
            .with_ctx_value("agent", json!("claude"));
        assert_eq!(
            has_skill_fn(&[json!("cross-skill")], &claude_ctx).unwrap(),
            json!(true)
        );
        assert_eq!(
            has_local_skill_fn(&[json!("cross-skill")], &claude_ctx).unwrap(),
            json!(true)
        );

        let opencode_ctx = ResolutionContext::at(local.path().to_path_buf())
            .with_home_dir(home.path().to_path_buf())
            .with_ctx_value("agent", json!("opencode"));
        assert_eq!(
            has_skill_fn(&[json!("cross-skill")], &opencode_ctx).unwrap(),
            json!(true)
        );
        assert_eq!(
            has_local_skill_fn(&[json!("cross-skill")], &opencode_ctx).unwrap(),
            json!(true)
        );
    }

    #[test]
    fn has_local_skill_excludes_user_roots() {
        let home = tempfile::TempDir::new().unwrap();
        let local = tempfile::TempDir::new().unwrap();
        let ctx = ResolutionContext::at(local.path().to_path_buf())
            .with_home_dir(home.path().to_path_buf())
            .with_ctx_value("agent", json!("claude"));

        std::fs::create_dir_all(home.path().join(".claude/skills/user-only")).unwrap();
        std::fs::create_dir_all(local.path().join(".claude/skills/local-only")).unwrap();

        assert_eq!(
            has_local_skill_fn(&[json!("user-only")], &ctx).unwrap(),
            json!(false)
        );
        assert_eq!(
            has_local_skill_fn(&[json!("local-only")], &ctx).unwrap(),
            json!(true)
        );
    }

    #[test]
    fn has_skill_uses_env_agent_when_ctx_not_set() {
        let local = tempfile::TempDir::new().unwrap();
        let ctx = ResolutionContext::at(local.path().to_path_buf())
            .with_home_dir(local.path().to_path_buf());

        // Unknown agent: only .agents/skills and .codex/skills are searched.
        std::fs::create_dir_all(local.path().join(".agents/skills/env-skill")).unwrap();

        assert_eq!(
            has_skill_fn(&[json!("env-skill")], &ctx).unwrap(),
            json!(true)
        );
    }

    #[test]
    fn has_skill_rejects_path_separators_and_dotdot() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        assert!(has_skill_fn(&[json!("foo/bar")], &ctx).is_err());
        assert!(has_skill_fn(&[json!("..")], &ctx).is_err());
        assert!(has_skill_fn(&[json!(".")], &ctx).is_err());
    }

    #[test]
    fn has_skill_empty_name_returns_false() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        assert_eq!(has_skill_fn(&[json!("")], &ctx).unwrap(), json!(false));
        assert_eq!(
            has_local_skill_fn(&[json!("")], &ctx).unwrap(),
            json!(false)
        );
    }

    #[test]
    fn has_skill_nested_directory_does_not_count() {
        let local = tempfile::TempDir::new().unwrap();
        let ctx = ResolutionContext::at(local.path().to_path_buf())
            .with_home_dir(local.path().to_path_buf())
            .with_ctx_value("agent", json!("claude"));

        std::fs::create_dir_all(local.path().join(".claude/skills/parent/nested")).unwrap();

        assert_eq!(
            has_skill_fn(&[json!("parent")], &ctx).unwrap(),
            json!(true)
        );
        assert_eq!(
            has_skill_fn(&[json!("nested")], &ctx).unwrap(),
            json!(false)
        );
    }

    #[test]
    fn has_skill_missing_root_returns_false() {
        let local = tempfile::TempDir::new().unwrap();
        let ctx = ResolutionContext::at(local.path().to_path_buf())
            .with_home_dir(local.path().to_path_buf())
            .with_ctx_value("agent", json!("claude"));

        assert_eq!(
            has_skill_fn(&[json!("missing")], &ctx).unwrap(),
            json!(false)
        );
    }

    #[test]
    fn has_skill_null_is_false_and_arity_errors() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        // Ledger rows 104-105: both are predicates, so `null` is `false`.
        assert_eq!(
            has_skill_fn(&[json!(null)], &ctx).unwrap(),
            json!(false)
        );
        assert_eq!(
            has_local_skill_fn(&[json!(null)], &ctx).unwrap(),
            json!(false)
        );
        assert!(has_skill_fn(&[], &ctx).is_err());
        assert!(has_skill_fn(&[json!("a"), json!("b")], &ctx).is_err());
    }

    #[test]
    fn phase5_functions_dispatch_by_name() {
        let ctx = ResolutionContext::at(std::env::temp_dir());
        assert_eq!(
            call_function("link", &[json!("https://example.com"), json!("x")], Some(&ctx))
                .unwrap()
                .unwrap(),
            json!("[x](https://example.com)")
        );
        assert_eq!(
            call_function("hasskill", &[json!("missing")], Some(&ctx))
                .unwrap()
                .unwrap(),
            json!(false)
        );
        assert_eq!(
            call_function("haslocalskill", &[json!("missing")], Some(&ctx))
                .unwrap()
                .unwrap(),
            json!(false)
        );
    }
}

mod fn_remote_tests {
    use super::*;
    use crate::markdown::compose::remote_fetch::RemoteFetchRuntime;
    use biscuit_file::file_reference::fetch::FetchPolicy;
    use serde_json::json;
    use std::time::Duration;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Builds a context whose remote-fetch runtime has already fetched `url`.
    async fn ready_ctx(url: &str, allow: bool) -> ResolutionContext {
        let policy = if allow {
            FetchPolicy::deny_all().allow_host("127.0.0.1")
        } else {
            FetchPolicy::deny_all()
        };
        let rt = RemoteFetchRuntime::with_policy(policy);
        rt.register_and_fetch(url::Url::parse(url).unwrap());
        // Allow the eager fetch task to settle before reading point-of-use.
        tokio::time::sleep(Duration::from_millis(200)).await;
        ResolutionContext {
            remote_fetch: Some(rt),
            ..ResolutionContext::at(std::path::PathBuf::from("."))
        }
    }

    #[tokio::test]
    async fn frontmatter_title_and_exists_read_remote_url() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/doc.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                "---\ntitle: Remote Title\nstatus: draft\n---\n# H1\n\nBody\n",
            ))
            .mount(&server)
            .await;
        let url = format!("{}/doc.md", server.uri());
        let ctx = ready_ctx(&url, true).await;

        assert_eq!(
            frontmatter_fn(&[json!(url.clone()), json!("status")], &ctx).unwrap(),
            json!("draft")
        );
        assert_eq!(
            markdown_title_fn(&[json!(url.clone())], &ctx).unwrap(),
            json!("Remote Title")
        );
        assert_eq!(
            markdown_body_empty_fn(&[json!(url.clone())], &ctx).unwrap(),
            json!(false)
        );
        assert_eq!(file_exists_fn(&[json!(url)], &ctx).unwrap(), json!(true));
    }

    #[tokio::test]
    async fn file_exists_errors_for_policy_denied_remote() {
        let server = MockServer::start().await;
        // No mock mounted and a deny-all policy: the fetch never reaches the
        // network. A denial says nothing about existence, so it is an error.
        let url = format!("{}/blocked.md", server.uri());
        let ctx = ready_ctx(&url, false).await;
        let err = file_exists_fn(&[json!(url)], &ctx)
            .expect_err("a denied remote read must not read as a missing file");
        assert!(err.contains("denied"), "unexpected message: {err}");
    }

    #[test]
    fn file_exists_remote_url_fails_loudly_in_local_only_context() {
        // Frontmatter's resolution context carries no remote runtime, so a
        // remote URL argument is unreadable and must error rather than
        // silently reporting the URL as absent (Decision B).
        let ctx = ResolutionContext::at(std::path::PathBuf::from("."));
        let err = file_exists_fn(&[json!("https://example.com/doc.md")], &ctx)
            .expect_err("local-only remote URL must fail loudly");
        assert!(err.contains("local-only"), "unexpected message: {err}");
    }

    #[test]
    fn load_markdown_remote_url_fails_loudly_in_local_only_context() {
        // The document-reading functions share the same local-only contract.
        let ctx = ResolutionContext::at(std::path::PathBuf::from("."));
        let err = markdown_title_fn(&[json!("https://example.com/doc.md")], &ctx)
            .expect_err("local-only remote URL must fail loudly");
        assert!(
            err.contains("remote reads are not enabled"),
            "unexpected message: {err}"
        );
    }
}

