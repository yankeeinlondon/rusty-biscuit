use super::*;

// ============================================
// Schema Validation integration tests
// ============================================

mod schema_validation_integration {
    use super::*;

    #[test]
    fn schema_validation_fails_fast_before_shell_expansion() {
        // Document matching the shape of the failing planner prompt:
        // spec is empty, and dir uses shell expansion that would fail
        // if spec stays empty.
        let content = "---\n$schema:\n  spec: 'file(required)'\nspec: \"\"\ndir: \"$(dirname '{{ spec }}')\"\n---\nBody\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[
            ComposeOperation::FrontmatterInterpolation,
            ComposeOperation::FrontmatterShellExpansion,
            ComposeOperation::Interpolation,
        ]);

        let err = md.compose_with(options).unwrap_err();
        let err_string = format!("{err}");
        assert!(
            err_string.contains("Schema validation failed"),
            "Expected schema validation error, got: {err_string}"
        );
        assert!(
            !err_string.contains("dirname"),
            "Shell expansion should not have run, got: {err_string}"
        );

        // The error variant itself should name the failing property.
        match err {
            MarkdownError::SchemaValidationFailed { problems, .. } => {
                assert!(
                    problems.iter().any(|p| {
                        p.property.as_deref() == Some("spec") || p.path == "/spec"
                    }),
                    "Error should mention the spec property, got: {problems:?}"
                );
            }
            other => panic!("Expected SchemaValidationFailed, got {other:?}"),
        }
    }

    #[test]
    fn schema_violation_on_shell_value_reported_when_shell_expansion_disabled() {
        // A `$(...)` frontmatter value violates the schema, but
        // FrontmatterShellExpansion is NOT in the enabled set. Because no
        // later stage will expand or re-validate `spec`, the violation must
        // surface here rather than being deferred and silently accepted.
        let content =
            "---\n$schema:\n  spec: 'number(required)'\nspec: \"$(echo 1)\"\n---\nBody\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let err = md.compose_with(options).unwrap_err();
        match err {
            MarkdownError::SchemaValidationFailed { problems, .. } => {
                assert!(
                    problems
                        .iter()
                        .any(|p| p.property.as_deref() == Some("spec") || p.path == "/spec"),
                    "Error should mention the spec property, got: {problems:?}"
                );
            }
            other => panic!("Expected SchemaValidationFailed, got {other:?}"),
        }
    }

    #[test]
    fn schema_validation_reports_zero_shell_replacements() {
        let content = "---\n$schema:\n  spec: 'file(required)'\nspec: \"\"\ndir: \"$(dirname '{{ spec }}')\"\n---\nBody\n";
        let md: Markdown = content.into();

        // Even with fail_fast=false, schema validation is a hard error.
        let options = ComposeOptions::new()
            .only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::FrontmatterShellExpansion,
                ComposeOperation::Interpolation,
            ])
            .with_fail_fast(false);

        let err = md.compose_with(options).unwrap_err();
        match err {
            MarkdownError::SchemaValidationFailed { .. } => {}
            other => panic!("Expected SchemaValidationFailed, got {other:?}"),
        }
    }

    #[test]
    fn coercion_write_back_flows_to_composed_frontmatter() {
        // `has_spec` derives from a ternary, resolves to the string "true"
        // during frontmatter interpolation, and is coerced to a real JSON
        // bool by schema validation. The composed frontmatter must hold the
        // bool, not the string.
        let content = "---\n$schema:\n  spec: string(required)\n  has_spec: boolean\nspec: design.md\nhas_spec: \"{{spec ? true : false}}\"\n---\nBody\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[
            ComposeOperation::FrontmatterInterpolation,
            ComposeOperation::Interpolation,
        ]);

        let (composed, _report) = md.compose_with(options).unwrap();
        assert_eq!(
            composed.frontmatter().as_map().get("has_spec"),
            Some(&serde_json::json!(true))
        );
    }

    #[test]
    fn schema_number_increment_accepts_native_and_quoted_source_numbers() {
        for review_iterations in ["2", "'2'", "\"2\""] {
            let dir = tempfile::tempdir().unwrap();
            let spec = dir.path().join("spec.md");
            let prompt = dir.path().join("prompt.md");
            std::fs::write(
                &spec,
                format!("---\nreview_iterations: {review_iterations}\n---\nSpec\n"),
            )
            .unwrap();
            std::fs::write(
                &prompt,
                "---\n\
                 $schema:\n\
                 \x20 spec: file(required;eager)\n\
                 \x20 iteration: number\n\
                 spec: spec.md\n\
                 iteration: \"{{ file_exists(spec) ? (frontmatter(spec, 'review_iterations') || 0) + 1 : 1 }}\"\n\
                 ---\nBody\n",
            )
            .unwrap();

            let md = Markdown::try_from(prompt.as_path()).unwrap();
            let options = ComposeOptions::new()
                .with_source_file(prompt)
                .only(&[
                    ComposeOperation::FrontmatterInterpolation,
                    ComposeOperation::Interpolation,
                ]);
            let (composed, _) = md.compose_with(options).unwrap();
            assert_eq!(
                composed.frontmatter().as_map().get("iteration"),
                Some(&serde_json::json!(3)),
                "source review_iterations was {review_iterations}"
            );
        }
    }

    #[test]
    fn schema_number_increment_survives_quoted_persistence_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let spec = dir.path().join("spec.md");
        let prompt = dir.path().join("prompt.md");
        std::fs::write(&spec, "---\n---\nSpec\n").unwrap();
        std::fs::write(
            &prompt,
            "---\n\
             $schema:\n\
             \x20 spec: file(required;eager)\n\
             \x20 iteration: number\n\
             \x20 review: file\n\
             \x20 previous: file\n\
             spec: spec.md\n\
             iteration: \"{{ file_exists(spec) ? (frontmatter(spec, 'review_iterations') || 0) + 1 : 1 }}\"\n\
             review: \"{{ dirname(spec) + '/review-' + iteration + '.md' }}\"\n\
             previous: \"{{ iteration < 2 ? null : decrement_file_index(review) }}\"\n\
             ---\nBody\n",
        )
        .unwrap();

        for expected_iteration in 1..=3 {
            let md = Markdown::try_from(prompt.as_path()).unwrap();
            let options = ComposeOptions::new()
                .with_source_file(prompt.clone())
                .only(&[
                    ComposeOperation::FrontmatterInterpolation,
                    ComposeOperation::Interpolation,
                ]);
            let (composed, _) = md.compose_with(options).unwrap();
            let frontmatter = composed.frontmatter().as_map();

            assert_eq!(
                frontmatter.get("iteration"),
                Some(&serde_json::json!(expected_iteration))
            );
            assert!(
                frontmatter
                    .get("review")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|path| path.ends_with(&format!("review-{expected_iteration}.md"))),
                "review should use iteration {expected_iteration}: {frontmatter:?}"
            );
            if expected_iteration == 1 {
                assert_eq!(frontmatter.get("previous"), Some(&serde_json::Value::Null));
            } else {
                assert!(
                    frontmatter
                        .get("previous")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|path| {
                            path.ends_with(&format!("review-{}.md", expected_iteration - 1))
                        }),
                    "previous should point to the prior iteration: {frontmatter:?}"
                );
            }

            std::fs::write(
                &spec,
                format!(
                    "---\nreview_iterations: '{expected_iteration}'\n---\nSpec\n"
                ),
            )
            .unwrap();
        }
    }

    #[test]
    fn implement_md_three_arm_union_ternaries_coerce_and_defer_shell() {
        // Faithful reproduction of the original failing `claudine compose
        // prompts/implement.md spec=… --claude` invocation: a 3-arm root
        // union where every arm types the `has_*` trio as strict `boolean`,
        // computed `has_*` ternaries that render into quoted scalars
        // ("true"/"false"), and a `$(...)`-bearing `dir`. A `spec=` value is
        // supplied via --set, so arm 2 (`spec: string(required)`) validates
        // post-coercion. Before this feature the strict `boolean` arms
        // rejected the "false"/"true" strings; now they coerce.
        //
        // Frontmatter shell expansion is left disabled to keep the test
        // hermetic (no real `dirname` invocation). `dir` is typed `string`,
        // so its literal `$(...)` value is already a valid string: coercion
        // skips it and validation raises no type problem, so it survives
        // untouched into the composed output as a deferred shell expression.
        let content = "---\n\
            $schema:\n\
            \x20 - review: string(required)\n\
            \x20   spec: string\n\
            \x20   iteration: number\n\
            \x20   has_plan: boolean\n\
            \x20   has_spec: boolean\n\
            \x20   has_review: boolean\n\
            \x20 - spec: string(required)\n\
            \x20   has_plan: boolean\n\
            \x20   has_spec: boolean\n\
            \x20   has_review: boolean\n\
            \x20 - plan: string(required)\n\
            \x20   spec: string\n\
            \x20   iteration: number\n\
            \x20   has_plan: boolean\n\
            \x20   has_spec: boolean\n\
            \x20   has_review: boolean\n\
            has_spec: \"{{spec ? true : false}}\"\n\
            has_plan: \"{{plan ? true : false}}\"\n\
            has_review: \"{{review ? true : false}}\"\n\
            dir: \"$(dirname '{{spec || plan}}')\"\n\
            ---\nBody\n";
        let md: Markdown = content.into();

        // `spec=` provided via --set; no `plan`/`review` → second arm wins.
        let options = ComposeOptions::new()
            .with_set_overrides(serde_json::json!({ "spec": "features/plan.md" }))
            .only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::Interpolation,
            ]);

        let (composed, _report) = md
            .compose_with(options)
            .expect("compose should succeed once the has_* strings coerce");

        let fm = composed.frontmatter();
        let map = fm.as_map();
        // The motivating fix: the ternary-derived strings become real bools.
        assert_eq!(map.get("has_spec"), Some(&serde_json::json!(true)));
        assert_eq!(map.get("has_plan"), Some(&serde_json::json!(false)));
        assert_eq!(map.get("has_review"), Some(&serde_json::json!(false)));
        // The `$(...)` `dir` value is deferred: coercion skips it pre-shell,
        // and the unresolved interpolation/shell template never errored.
        let dir = map.get("dir").and_then(serde_json::Value::as_str).unwrap();
        assert!(
            dir.contains("$(") && dir.contains("dirname"),
            "dir should remain a deferred shell expression, got: {dir}"
        );
    }

    #[test]
    fn parent_set_overlay_satisfies_child_schema() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        // Child has a schema requiring child_input
        std::fs::write(
            &child,
            "---\n$schema:\n  child_input: 'string(required)'\n---\nChild body\n",
        )
        .unwrap();

        // Parent transcludes child with set.child_input="ok"
        std::fs::write(
            &root,
            "# Parent\n\n::file ./child.md set.child_input=\"ok\"\n",
        )
        .unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("Child body"));
        assert_eq!(report.transclusions_applied, 1);
    }

    #[test]
    fn parent_set_overlay_missing_child_schema_fails() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        // Child has a schema requiring child_input
        std::fs::write(
            &child,
            "---\n$schema:\n  child_input: 'string(required)'\n---\nChild body\n",
        )
        .unwrap();

        // Parent transcludes child WITHOUT the set overlay
        std::fs::write(&root, "# Parent\n\n::file ./child.md\n").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        // fail_fast=true so the schema validation error propagates rather
        // than being downgraded to a transclusion warning.
        let options = ComposeOptions::new()
            .with_source_file(root)
            .with_fail_fast(true);
        let err = md.compose_with(options).unwrap_err();

        match err {
            MarkdownError::SchemaValidationFailed { problems, .. } => {
                assert!(
                    problems.iter().any(|p| p.property.as_deref() == Some("child_input")),
                    "Expected problem on child_input, got: {problems:?}"
                );
            }
            other => panic!("Expected SchemaValidationFailed, got {other:?}"),
        }
    }

    /// Different baseline schemas must not share cache entries for the same
    /// transcluded child. Compose the same parent+child three times against
    /// a shared persistent cache:
    ///
    /// 1. baseline A → cold cache, child is computed and written to the
    ///    persistent store (`persistent_hits == 0`, `persistent_writes >= 1`).
    /// 2. baseline A again → cache is warm and the child compose entry is
    ///    reused (`persistent_hits >= 1`).
    /// 3. baseline B → baseline differs, so the persistent cache key
    ///    differs; the child must be recomputed rather than reuse the
    ///    baseline-A entry (`persistent_hits == 0` again).
    ///
    /// This proves `options_hash` includes `baseline_schema` in a way that
    /// actually invalidates the persistent cache — guarding against the
    /// "stale success keyed without baseline" regression.
    #[test]
    fn baseline_cache_does_not_reuse_across_distinct_baselines() {
        use crate::markdown::compose::CacheAccessMode;
        use crate::markdown::schemas::{
            Constraint, PropertyAtom, PropertyDef, SchemaShape, SimplifiedSchema,
            SimplifiedType, TypeExpr,
        };
        use indexmap::IndexMap;

        fn baseline_required(prop: &str) -> SimplifiedSchema {
            let mut properties = IndexMap::new();
            properties.insert(
                prop.into(),
                PropertyDef::Single(PropertyAtom {
                    ty: TypeExpr::Primitive(SimplifiedType::String),
                    is_array: false,
                    constraints: vec![Constraint::Required],
                    array_constraints: vec![],
                    description: None,
                }),
            );
            SimplifiedSchema::Single(SchemaShape {
                properties,
                ..Default::default()
            })
        }

        let dir = tempfile::tempdir().unwrap();
        let cache_root = dir.path().join("cache");
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        // Parent supplies both `alpha` and `beta` so it (and its effective
        // state inherited by the child) satisfies either baseline under
        // test. Cache invalidation is the contract we care about here, not
        // the validation outcome.
        std::fs::write(&child, "---\nalpha: ok\nbeta: ok\n---\nChild body\n").unwrap();
        std::fs::write(
            &root,
            "---\nalpha: ok\nbeta: ok\n---\n# Parent\n\n::file ./child.md\n",
        )
        .unwrap();

        let mk_options = |baseline_prop: &str| {
            ComposeOptions::new()
                .with_source_file(&root)
                .with_baseline_schema(baseline_required(baseline_prop))
                .with_cache_access_mode(CacheAccessMode::ReadWrite)
                .with_cache_root(&cache_root)
                .with_cache_namespace("baseline_cache_regression")
                .with_fail_fast(true)
        };

        // ── Run 1: cold cache under baseline A ─────────────────────
        let md1 = Markdown::try_from(root.as_path()).unwrap();
        let (_, report1) = md1
            .compose_with(mk_options("alpha"))
            .expect("run 1 (baseline alpha, cold cache) should succeed");
        let stats1 = report1
            .cache_stats
            .expect("expected cache stats with cache enabled");
        assert_eq!(
            stats1.persistent_hits, 0,
            "run 1 should have a cold persistent cache, got {stats1:?}"
        );
        assert!(
            stats1.persistent_writes >= 1,
            "run 1 must write the child compose to the persistent cache, got {stats1:?}"
        );

        // ── Run 2: same baseline A → cache should be warm ──────────
        let md2 = Markdown::try_from(root.as_path()).unwrap();
        let (_, report2) = md2
            .compose_with(mk_options("alpha"))
            .expect("run 2 (baseline alpha, warm cache) should succeed");
        let stats2 = report2
            .cache_stats
            .expect("expected cache stats with cache enabled");
        assert!(
            stats2.persistent_hits >= 1,
            "run 2 must reuse the warmed persistent entry, got {stats2:?}",
        );

        // ── Run 3: baseline B → distinct key, must not reuse run 1 ─
        let md3 = Markdown::try_from(root.as_path()).unwrap();
        let (_, report3) = md3
            .compose_with(mk_options("beta"))
            .expect("run 3 (baseline beta) should succeed");
        let stats3 = report3
            .cache_stats
            .expect("expected cache stats with cache enabled");
        assert_eq!(
            stats3.persistent_hits, 0,
            "run 3 must NOT reuse the baseline-A entry — options_hash must include \
             baseline_schema. got {stats3:?}",
        );
        assert!(
            stats3.persistent_writes >= 1,
            "run 3 must compute and write a fresh entry under the new baseline, got {stats3:?}"
        );
    }

    /// Per D2 the launch-area anchor (`file_ref_fallback_dir`) is **not** a
    /// resolution input for a reference authored inside a document, so a file
    /// present only under the fallback never resolves — the interpolated
    /// `file_exists("anchored.md")` is `false` regardless of which fallback is
    /// configured.
    ///
    /// The anchor nevertheless remains part of `ComposeOptions` identity
    /// (`options_hash`), so two runs that differ only in their anchor still get
    /// distinct persistent cache keys (a conservative over-invalidation): run 2
    /// must not reuse run 1's entry.
    #[test]
    fn distinct_file_ref_fallback_dirs_do_not_share_a_cache_entry_and_never_resolve_via_fallback() {
        use crate::markdown::compose::CacheAccessMode;

        let dir = tempfile::tempdir().unwrap();
        let cache_root = dir.path().join("cache");
        let doc_dir = dir.path().join("doc");
        std::fs::create_dir_all(&doc_dir).unwrap();
        let root = doc_dir.join("root.md");
        std::fs::write(&root, "anchored exists: {{ file_exists(\"anchored.md\") }}\n").unwrap();

        // Both launch areas exist; only the first HAS anchored.md — but neither
        // is consulted for resolution (D2), so the result is `false` either way.
        let fallback_present = dir.path().join("launch-present");
        let fallback_absent = dir.path().join("launch-absent");
        std::fs::create_dir_all(&fallback_present).unwrap();
        std::fs::create_dir_all(&fallback_absent).unwrap();
        std::fs::write(fallback_present.join("anchored.md"), "# Anchored\n").unwrap();

        let mk_options = |fallback: &std::path::Path| {
            ComposeOptions::new()
                .with_source_file(&root)
                .with_file_ref_fallback_dir(fallback.to_path_buf())
                .with_cache_access_mode(CacheAccessMode::ReadWrite)
                .with_cache_root(&cache_root)
                .with_cache_namespace("file_ref_fallback_cache_regression")
                .with_fail_fast(true)
        };

        // ── Run 1: cold cache; the launch-area copy of anchored.md is NOT
        // consulted, so file_exists is false. ─────────────────────
        let md1 = Markdown::try_from(root.as_path()).unwrap();
        let (composed1, report1) = md1
            .compose_with(mk_options(&fallback_present))
            .expect("run 1 (present fallback, cold cache) should succeed");
        assert!(
            composed1.content().contains("anchored exists: false"),
            "the launch-area fallback must not resolve a document-authored reference: {}",
            composed1.content(),
        );
        let stats1 = report1
            .cache_stats
            .expect("expected cache stats with cache enabled");
        assert_eq!(
            stats1.persistent_hits, 0,
            "run 1 should have a cold persistent cache, got {stats1:?}"
        );

        // ── Run 2: different launch area → distinct options_hash → must not
        // reuse run 1's entry. Same (false) resolution outcome. ────
        let md2 = Markdown::try_from(root.as_path()).unwrap();
        let (composed2, report2) = md2
            .compose_with(mk_options(&fallback_absent))
            .expect("run 2 (absent fallback) should succeed");
        assert!(
            composed2.content().contains("anchored exists: false"),
            "the launch-area fallback is inert for resolution: {}",
            composed2.content(),
        );
        let stats2 = report2
            .cache_stats
            .expect("expected cache stats with cache enabled");
        assert_eq!(
            stats2.persistent_hits, 0,
            "run 2 must NOT reuse run 1's entry — options_hash still includes \
             file_ref_fallback_dir. got {stats2:?}",
        );
    }

    fn compose_from_launch(
        prompt: &std::path::Path,
        repository: &std::path::Path,
        launch_dir: &std::path::Path,
        overrides: serde_json::Value,
        exclude_keys: impl IntoIterator<Item = &'static str>,
    ) -> Markdown {
        let context = biscuit_file::FileResolutionContext::new(launch_dir)
            .with_repository_root(repository)
            .with_source_path(prompt);
        let options = ComposeOptions::new()
            .with_source_file(prompt)
            .with_file_resolution_context(context)
            .with_file_ref_fallback_dir(launch_dir)
            .with_set_overrides(overrides)
            .with_exclude_keys(exclude_keys)
            .only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::Interpolation,
            ]);
        Markdown::try_from(prompt)
            .unwrap()
            .compose_with(options)
            .unwrap()
            .0
    }

    #[test]
    fn eager_caller_file_projection_is_identical_from_root_and_package_launches() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        let prompt = repo.path().join("prompts/plan.md");
        let package = repo.path().join("claudine");
        let case_dir = package.join("fixes/file-param-anchoring");
        std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
        std::fs::create_dir_all(&case_dir).unwrap();
        let spec = case_dir.join("spec.md");
        std::fs::write(&spec, "# Specification\n").unwrap();
        std::fs::write(
            &prompt,
            "---\n\
             $schema:\n\
             \x20 spec: file(eager; required)\n\
             \x20 x: string(required)\n\
             \x20 plan: file\n\
             spec: authored.md\n\
             x: \"{{ spec }}\"\n\
             plan: \"{{ dirname(spec) + '/plan.md' }}\"\n\
             ---\n\
             SPEC={{ spec }}\n\
             PLAN={{ plan }}\n",
        )
        .unwrap();

        let launches = [
            (repo.path(), "claudine/fixes/file-param-anchoring/spec.md"),
            (&package, "fixes/file-param-anchoring/spec.md"),
        ];
        for (launch, authored) in launches {
            let composed = compose_from_launch(
                &prompt,
                repo.path(),
                launch,
                serde_json::json!({ "spec": authored }),
                [],
            );
            let frontmatter = composed.frontmatter().as_map();
            let native_spec = spec.to_string_lossy().into_owned();
            assert_eq!(frontmatter.get("spec"), Some(&serde_json::json!(native_spec)));
            assert_eq!(frontmatter.get("x"), frontmatter.get("spec"));
            assert_eq!(
                frontmatter.get("plan"),
                Some(&serde_json::json!(
                    "claudine/fixes/file-param-anchoring/plan.md"
                )),
            );
            assert!(
                composed.content().contains(&format!(
                    "SPEC={}",
                    biscuit_file::to_portable_string(&spec)
                )),
                "direct body interpolation should use the portable eager-file presentation: {}",
                composed.content(),
            );
            assert!(
                composed
                    .content()
                    .contains("PLAN=claudine/fixes/file-param-anchoring/plan.md"),
                "derived plan should remain beside the specification: {}",
                composed.content(),
            );
            assert!(
                !composed
                    .content()
                    .contains("prompts/fixes/file-param-anchoring/plan.md"),
                "the launch-relative input must not be retargeted under the prompt directory",
            );
        }
    }

    #[test]
    fn lazy_target_reuses_the_callers_file_origin_after_an_eager_router() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        let package = repo.path().join("claudine");
        let case_dir = package.join("fixes/proxy-provenance");
        let router = repo.path().join("prompts/implement.md");
        let target = repo.path().join("prompts/_implement/implement-suggestions.md");
        std::fs::create_dir_all(&case_dir).unwrap();
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        let spec = case_dir.join("spec.md");
        std::fs::write(&spec, "---\nreview_iterations: 2\n---\nSpecification\n").unwrap();
        std::fs::write(
            &router,
            "---\n$schema:\n  spec: file(eager; required)\nspec: authored.md\nimplemented: \"{{ frontmatter(spec, 'review_iterations') }}\"\n---\n{{ spec }}\n",
        )
        .unwrap();
        std::fs::write(
            &target,
            "---\n$schema:\n  spec: file(required)\n  iteration: number(required)\nspec: authored.md\niteration: \"{{ frontmatter(spec, 'review_iterations') }}\"\n---\n{{ spec }}\n",
        )
        .unwrap();

        let raw = "fixes/proxy-provenance/spec.md";
        let origin = biscuit_file::FileResolutionContext::new(&package)
            .with_repository_root(repo.path());
        let records: CallerInputRecords = [(
            "spec".to_string(),
            CallerInputRecord::new(serde_json::json!(raw), origin.clone()),
        )]
        .into_iter()
        .collect();
        let compose = |prompt: &std::path::Path| {
            Markdown::try_from(prompt)
                .unwrap()
                .compose_with(
                    ComposeOptions::new()
                        .with_source_file(prompt)
                        .with_file_resolution_context(origin.for_source(prompt))
                        .with_set_overrides(serde_json::json!({ "spec": raw }))
                        .with_caller_input_records(records.clone())
                        .only(&[
                            ComposeOperation::FrontmatterInterpolation,
                            ComposeOperation::Interpolation,
                        ]),
                )
                .unwrap()
                .0
        };

        let routed = compose(&router);
        assert_eq!(routed.frontmatter().as_map()["implemented"], serde_json::json!(2));
        let prepared_target = compose(&target);
        assert_eq!(prepared_target.frontmatter().as_map()["iteration"], serde_json::json!(2));
        assert_eq!(
            prepared_target.frontmatter().as_map()["spec"],
            serde_json::json!(spec.to_string_lossy().into_owned()),
        );
    }

    #[test]
    fn lazy_caller_file_uses_authoritative_candidate_order_for_every_local_reference_kind() {
        use biscuit_file::{CandidatePlanOrder, PathPosition, RootProvenance};

        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        let source = repo.path().join("area");
        let package = repo.path().join("package");
        let magic = repo.path().join("magic");
        let home = repo.path().join("home");
        let appended_magic = repo.path().join("appended-magic");
        let prompt = repo.path().join("prompts/candidate-order.md");
        for root in [
            repo.path(),
            source.as_path(),
            package.as_path(),
            magic.as_path(),
            home.as_path(),
            appended_magic.as_path(),
        ] {
            std::fs::create_dir_all(root.join("collision")).unwrap();
            std::fs::write(root.join("collision/spec.md"), root.to_string_lossy().as_bytes())
                .unwrap();
        }
        std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
        std::fs::write(
            &prompt,
            "---\n$schema:\n  spec: file(required)\nspec: authored.md\n---\n{{ spec }}\n",
        )
        .unwrap();

        let origin = biscuit_file::FileResolutionContext::new(&source)
            .with_repository_root(repo.path())
            .with_package_area(&package)
            .with_home_dir(&home)
            .add_magic_path(&magic, PathPosition::Start)
            .add_magic_path(&appended_magic, PathPosition::End);
        let cases = [
            (
                "collision/spec.md",
                vec![RootProvenance::Source, RootProvenance::Repository],
            ),
            (
                "@collision/spec.md",
                vec![
                    RootProvenance::Magic,
                    RootProvenance::Repository,
                    RootProvenance::Home,
                    RootProvenance::Magic,
                ],
            ),
            ("!collision/spec.md", vec![RootProvenance::Package]),
            ("./collision/spec.md", vec![RootProvenance::Source]),
        ];

        for (raw, expected_provenance) in cases {
            let reference = biscuit_file::FileReference::new(raw).unwrap();
            let plan = reference
                .candidate_plan_with_order(&origin, CandidatePlanOrder::AuthoringBaseFirst)
                .unwrap();
            assert_eq!(
                plan.iter()
                    .map(biscuit_file::ResolutionCandidate::provenance)
                    .collect::<Vec<_>>(),
                expected_provenance,
                "fixture must exercise the authoritative plan for `{raw}`",
            );
            let expected = plan.first().unwrap().path();
            assert!(
                expected.exists(),
                "the first candidate for `{raw}` must participate in the collision",
            );

            let records = [(
                "spec".to_string(),
                CallerInputRecord::new(serde_json::json!(raw), origin.clone()),
            )]
            .into_iter()
            .collect();
            let composed = Markdown::try_from(prompt.as_path())
                .unwrap()
                .compose_with(
                    ComposeOptions::new()
                        .with_source_file(&prompt)
                        .with_set_overrides(serde_json::json!({ "spec": raw }))
                        .with_caller_input_records(records)
                        .only(&[
                            ComposeOperation::FrontmatterInterpolation,
                            ComposeOperation::Interpolation,
                        ]),
                )
                .unwrap()
                .0;

            assert_eq!(
                composed.frontmatter().as_map()["spec"],
                serde_json::json!(expected.to_string_lossy().into_owned()),
                "lazy projection must consume the first candidate for `{raw}`",
            );
            assert_eq!(
                composed.content().trim(),
                biscuit_file::to_portable_string(expected),
                "presentation must preserve the selected identity for `{raw}`",
            );
        }
    }

    #[test]
    fn caller_file_projection_does_not_guess_ambiguous_or_zero_match_unions() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        let launch = repo.path().join("area");
        let prompt = repo.path().join("prompts/union.md");
        std::fs::create_dir_all(&launch).unwrap();
        std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
        std::fs::write(launch.join("spec.md"), "# Specification\n").unwrap();
        let origin = biscuit_file::FileResolutionContext::new(&launch)
            .with_repository_root(repo.path());

        std::fs::write(
            &prompt,
            "---\n$schema:\n  spec:\n    - string(required)\n    - file(required)\nspec: authored.md\n---\n{{ spec }}\n",
        )
        .unwrap();
        let ambiguous_records = [(
            "spec".to_string(),
            CallerInputRecord::new(serde_json::json!("spec.md"), origin.clone()),
        )]
        .into_iter()
        .collect();
        let ambiguous = Markdown::try_from(prompt.as_path())
            .unwrap()
            .compose_with(
                ComposeOptions::new()
                    .with_source_file(&prompt)
                    .with_set_overrides(serde_json::json!({ "spec": "spec.md" }))
                    .with_caller_input_records(ambiguous_records)
                    .only(&[ComposeOperation::Interpolation]),
            )
            .unwrap()
            .0;
        assert_eq!(
            ambiguous.frontmatter().as_map()["spec"],
            serde_json::json!("spec.md")
        );

        std::fs::write(
            &prompt,
            "---\n$schema:\n  spec:\n    - number(required)\n    - file(required)\nspec: authored.md\n---\nBody\n",
        )
        .unwrap();
        let zero_records = [(
            "spec".to_string(),
            CallerInputRecord::new(serde_json::json!({ "unexpected": true }), origin),
        )]
        .into_iter()
        .collect();
        let error = Markdown::try_from(prompt.as_path())
            .unwrap()
            .compose_with(
                ComposeOptions::new()
                    .with_source_file(&prompt)
                    .with_set_overrides(
                        serde_json::json!({ "spec": { "unexpected": true } }),
                    )
                    .with_caller_input_records(zero_records)
                    .only(&[ComposeOperation::Interpolation]),
            )
            .expect_err("normal schema validation owns a zero-match union");
        assert!(matches!(error, MarkdownError::SchemaValidationFailed { .. }));
    }

    #[test]
    fn discriminated_root_union_selects_one_file_schema_path() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        let launch = repo.path().join("area");
        let prompt = repo.path().join("prompts/root-union.md");
        std::fs::create_dir_all(&launch).unwrap();
        std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
        let spec = launch.join("spec.md");
        std::fs::write(&spec, "# Specification\n").unwrap();
        std::fs::write(
            &prompt,
            "---\n\
             $schema:\n\
             \x20 - kind: literal(alpha; required)\n\
             \x20   spec: file(required)\n\
             \x20 - kind: literal(beta; required)\n\
             \x20   spec: file(required)\n\
             kind: alpha\n\
             spec: authored.md\n\
             ---\n\
             {{ spec }}\n",
        )
        .unwrap();

        let composed = compose_from_launch(
            &prompt,
            repo.path(),
            &launch,
            serde_json::json!({ "spec": "spec.md" }),
            [],
        );

        assert_eq!(
            composed.frontmatter().as_map()["spec"],
            serde_json::json!(spec.to_string_lossy().into_owned()),
        );
        assert_eq!(
            composed.content().trim(),
            biscuit_file::to_portable_string(&spec),
        );
    }

    #[test]
    fn root_union_uses_document_origin_for_an_eager_file_sibling() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        let launch = repo.path().join("launch");
        let prompt = repo.path().join("prompts/root-union.md");
        std::fs::create_dir_all(&launch).unwrap();
        std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
        let spec = launch.join("spec.md");
        std::fs::write(&spec, "# Specification\n").unwrap();
        std::fs::write(prompt.parent().unwrap().join("document.md"), "# Document\n").unwrap();
        std::fs::write(
            &prompt,
            "---\n\
             $schema:\n\
             \x20 - kind: literal(alpha; required)\n\
             \x20   spec: file(required)\n\
             \x20   document: file(eager; required)\n\
             \x20 - kind: literal(beta; required)\n\
             \x20   spec: string(required)\n\
             \x20   document: file(eager; required)\n\
             kind: alpha\n\
             spec: authored.md\n\
             document: document.md\n\
             ---\n\
             {{ spec }}\n",
        )
        .unwrap();
        let origin = biscuit_file::FileResolutionContext::new(&launch)
            .with_repository_root(repo.path());
        let records = [(
            "spec".to_string(),
            CallerInputRecord::new(serde_json::json!("spec.md"), origin.clone()),
        )]
        .into_iter()
        .collect();

        let composed = Markdown::try_from(prompt.as_path())
            .unwrap()
            .compose_with(
                ComposeOptions::new()
                    .with_source_file(&prompt)
                    .with_file_resolution_context(origin.for_source(&prompt))
                    .with_set_overrides(serde_json::json!({ "spec": "spec.md" }))
                    .with_caller_input_records(records)
                    .only(&[
                        ComposeOperation::FrontmatterInterpolation,
                        ComposeOperation::Interpolation,
                    ]),
            )
            .unwrap()
            .0;

        assert_eq!(
            composed.frontmatter().as_map()["spec"],
            serde_json::json!(spec.to_string_lossy().into_owned()),
        );
        assert_eq!(
            composed.content().trim(),
            biscuit_file::to_portable_string(&spec),
        );
    }

    #[test]
    fn root_union_materializes_each_caller_file_from_its_own_origin() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        let first_origin_dir = repo.path().join("first-origin");
        let second_origin_dir = repo.path().join("second-origin");
        let prompt = repo.path().join("prompts/root-union.md");
        std::fs::create_dir_all(&first_origin_dir).unwrap();
        std::fs::create_dir_all(&second_origin_dir).unwrap();
        std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
        let first = first_origin_dir.join("first.md");
        let second = second_origin_dir.join("second.md");
        std::fs::write(&first, "# First\n").unwrap();
        std::fs::write(&second, "# Second\n").unwrap();
        std::fs::write(
            &prompt,
            "---\n\
             $schema:\n\
             \x20 - kind: literal(alpha; required)\n\
             \x20   first: file(eager; required)\n\
             \x20   second: file(eager; required)\n\
             \x20 - kind: literal(beta; required)\n\
             \x20   first: string(required)\n\
             \x20   second: string(required)\n\
             kind: alpha\n\
             first: authored-first.md\n\
             second: authored-second.md\n\
             ---\n\
             {{ first }}\n\
             {{ second }}\n",
        )
        .unwrap();
        let document_context = biscuit_file::FileResolutionContext::new(prompt.parent().unwrap())
            .with_repository_root(repo.path())
            .with_source_path(&prompt);
        let records = [
            (
                "first".to_string(),
                CallerInputRecord::new(
                    serde_json::json!("first.md"),
                    biscuit_file::FileResolutionContext::new(&first_origin_dir)
                        .with_repository_root(repo.path()),
                ),
            ),
            (
                "second".to_string(),
                CallerInputRecord::new(
                    serde_json::json!("second.md"),
                    biscuit_file::FileResolutionContext::new(&second_origin_dir)
                        .with_repository_root(repo.path()),
                ),
            ),
        ]
        .into_iter()
        .collect();

        let composed = Markdown::try_from(prompt.as_path())
            .unwrap()
            .compose_with(
                ComposeOptions::new()
                    .with_source_file(&prompt)
                    .with_file_resolution_context(document_context)
                    .with_set_overrides(serde_json::json!({
                        "first": "first.md",
                        "second": "second.md",
                    }))
                    .with_caller_input_records(records)
                    .only(&[
                        ComposeOperation::FrontmatterInterpolation,
                        ComposeOperation::Interpolation,
                    ]),
            )
            .unwrap()
            .0;

        assert_eq!(
            composed.frontmatter().as_map()["first"],
            serde_json::json!(first.to_string_lossy().into_owned()),
        );
        assert_eq!(
            composed.frontmatter().as_map()["second"],
            serde_json::json!(second.to_string_lossy().into_owned()),
        );
    }

    #[test]
    fn discriminated_root_union_distinguishes_file_and_non_file_arms() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        let launch = repo.path().join("area");
        std::fs::create_dir_all(&launch).unwrap();
        let spec = launch.join("spec.md");
        std::fs::write(&spec, "# Specification\n").unwrap();

        for (kind, expected) in [
            ("file", spec.to_string_lossy().into_owned()),
            ("text", "spec.md".to_string()),
        ] {
            let prompt = repo.path().join(format!("prompts/root-{kind}.md"));
            std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
            std::fs::write(
                &prompt,
                format!(
                    "---\n\
                     $schema:\n\
                     \x20 - kind: literal(file; required)\n\
                     \x20   spec: file(required)\n\
                     \x20 - kind: literal(text; required)\n\
                     \x20   spec: string(required)\n\
                     kind: {kind}\n\
                     spec: authored.md\n\
                     ---\n\
                     {{{{ spec }}}}\n"
                ),
            )
            .unwrap();

            let composed = compose_from_launch(
                &prompt,
                repo.path(),
                &launch,
                serde_json::json!({ "spec": "spec.md" }),
                [],
            );
            assert_eq!(
                composed.frontmatter().as_map()["spec"],
                serde_json::json!(expected),
                "root arm `{kind}` selected the wrong property schema",
            );
        }
    }

    #[test]
    fn ambiguous_root_union_does_not_guess_a_file_arm() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        let launch = repo.path().join("area");
        let prompt = repo.path().join("prompts/root-ambiguous.md");
        std::fs::create_dir_all(&launch).unwrap();
        std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
        std::fs::write(launch.join("spec.md"), "# Specification\n").unwrap();
        std::fs::write(
            &prompt,
            "---\n\
             $schema:\n\
             \x20 - kind: literal(shared; required)\n\
             \x20   spec: file(required)\n\
             \x20 - kind: literal(shared; required)\n\
             \x20   spec: file(required)\n\
             kind: shared\n\
             spec: authored.md\n\
             ---\n\
             {{ spec }}\n",
        )
        .unwrap();

        let composed = compose_from_launch(
            &prompt,
            repo.path(),
            &launch,
            serde_json::json!({ "spec": "spec.md" }),
            [],
        );
        assert_eq!(
            composed.frontmatter().as_map()["spec"],
            serde_json::json!("spec.md"),
        );
        assert_eq!(composed.content().trim(), "spec.md");
    }

    #[test]
    fn zero_match_root_union_leaves_the_verdict_to_schema_validation() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        let launch = repo.path().join("area");
        let prompt = repo.path().join("prompts/root-zero.md");
        std::fs::create_dir_all(&launch).unwrap();
        std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
        std::fs::write(launch.join("spec.md"), "# Specification\n").unwrap();
        std::fs::write(
            &prompt,
            "---\n\
             $schema:\n\
             \x20 - kind: literal(alpha; required)\n\
             \x20   spec: file(required)\n\
             \x20 - kind: literal(beta; required)\n\
             \x20   spec: file(required)\n\
             kind: neither\n\
             spec: authored.md\n\
             ---\n\
             Body\n",
        )
        .unwrap();

        let context = biscuit_file::FileResolutionContext::new(&launch)
            .with_repository_root(repo.path())
            .with_source_path(&prompt);
        let error = Markdown::try_from(prompt.as_path())
            .unwrap()
            .compose_with(
                ComposeOptions::new()
                    .with_source_file(&prompt)
                    .with_file_resolution_context(context)
                    .with_file_ref_fallback_dir(&launch)
                    .with_set_overrides(serde_json::json!({ "spec": "spec.md" }))
                    .only(&[
                        ComposeOperation::FrontmatterInterpolation,
                        ComposeOperation::Interpolation,
                    ]),
            )
            .expect_err("normal validation must reject a zero-match root union");
        assert!(matches!(error, MarkdownError::SchemaValidationFailed { .. }));
    }

    #[test]
    fn recursive_lazy_caller_file_requires_eager_binding() {
        use crate::markdown::schemas::FileReferenceDiagnostic;

        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        let launch = repo.path().join("area");
        let prompt = repo.path().join("prompts/recursive.md");
        std::fs::create_dir_all(&launch).unwrap();
        std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
        std::fs::write(
            &prompt,
            "---\n$schema:\n  spec: file(required)\nspec: authored.md\n---\n{{ spec }}\n",
        )
        .unwrap();
        let origin = biscuit_file::FileResolutionContext::new(&launch)
            .with_repository_root(repo.path());
        let records = [(
            "spec".to_string(),
            CallerInputRecord::new(serde_json::json!("%spec.md"), origin),
        )]
        .into_iter()
        .collect();

        let error = Markdown::try_from(prompt.as_path())
            .unwrap()
            .compose_with(
                ComposeOptions::new()
                    .with_source_file(&prompt)
                    .with_set_overrides(serde_json::json!({ "spec": "%spec.md" }))
                    .with_caller_input_records(records)
                    .only(&[
                        ComposeOperation::FrontmatterInterpolation,
                        ComposeOperation::Interpolation,
                    ]),
            )
            .expect_err("a recursive lazy caller file has no single identity");
        let MarkdownError::SchemaValidationFailed { problems, .. } = error else {
            panic!("expected typed schema failure, got {error:?}");
        };
        assert_eq!(problems.len(), 1);
        assert!(problems[0].message.contains("file(eager)"));
        assert!(matches!(
            problems[0].file_reference,
            Some(FileReferenceDiagnostic::ResolutionFailed { ref raw }) if raw == "%spec.md"
        ));
    }

    #[test]
    fn lazy_remote_caller_file_preserves_remote_identity_without_local_resolution() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        let launch = repo.path().join("area");
        let prompt = repo.path().join("prompts/remote.md");
        std::fs::create_dir_all(&launch).unwrap();
        std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
        std::fs::write(
            &prompt,
            "---\n$schema:\n  spec: file(required)\nspec: authored.md\nselected: \"{{ spec }}\"\n---\n{{ spec }}\n",
        )
        .unwrap();
        let raw = "https://example.com/spec.md?revision=2";
        let origin = biscuit_file::FileResolutionContext::new(&launch)
            .with_repository_root(repo.path());
        let records = [(
            "spec".to_string(),
            CallerInputRecord::new(serde_json::json!(raw), origin),
        )]
        .into_iter()
        .collect();

        let composed = Markdown::try_from(prompt.as_path())
            .unwrap()
            .compose_with(
                ComposeOptions::new()
                    .with_source_file(&prompt)
                    .with_set_overrides(serde_json::json!({ "spec": raw }))
                    .with_caller_input_records(records)
                    .only(&[
                        ComposeOperation::FrontmatterInterpolation,
                        ComposeOperation::Interpolation,
                    ]),
            )
            .unwrap()
            .0;
        assert_eq!(composed.frontmatter().as_map()["spec"], serde_json::json!(raw));
        assert_eq!(
            composed.frontmatter().as_map()["selected"],
            serde_json::json!(raw)
        );
        assert!(composed.content().contains(raw));
    }

    #[test]
    fn null_caller_files_and_non_caller_strings_are_not_materialized() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        let launch = repo.path().join("area");
        let prompt = repo.path().join("prompts/controls.md");
        std::fs::create_dir_all(&launch).unwrap();
        std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
        std::fs::write(
            &prompt,
            "---\n$schema:\n  spec: file\n  label: string(required)\nspec: authored.md\nlabel: document-owned\n---\n{{ label }}\n",
        )
        .unwrap();
        let origin = biscuit_file::FileResolutionContext::new(&launch)
            .with_repository_root(repo.path());
        let records = [(
            "spec".to_string(),
            CallerInputRecord::new(serde_json::Value::Null, origin),
        )]
        .into_iter()
        .collect();

        let composed = Markdown::try_from(prompt.as_path())
            .unwrap()
            .compose_with(
                ComposeOptions::new()
                    .with_source_file(&prompt)
                    .with_set_overrides(serde_json::json!({ "spec": null }))
                    .with_caller_input_records(records)
                    .only(&[
                        ComposeOperation::FrontmatterInterpolation,
                        ComposeOperation::Interpolation,
                    ]),
            )
            .unwrap()
            .0;
        assert_eq!(
            composed.frontmatter().as_map()["spec"],
            serde_json::Value::Null
        );
        assert_eq!(
            composed.frontmatter().as_map()["label"],
            serde_json::json!("document-owned")
        );
        assert_eq!(composed.content(), "document-owned");
    }

    #[test]
    fn absent_caller_property_does_not_materialize_a_schema_file_default() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        let launch = repo.path().join("area");
        let prompt = repo.path().join("prompts/default.md");
        std::fs::create_dir_all(&launch).unwrap();
        std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
        std::fs::write(
            &prompt,
            "---\n$schema:\n  spec: \"file(default('./defaults/spec.md'))\"\n  caller_label: string(required)\n---\nBody.\n",
        )
        .unwrap();
        let origin = biscuit_file::FileResolutionContext::new(&launch)
            .with_repository_root(repo.path());
        let records = [(
            "caller_label".to_string(),
            CallerInputRecord::new(serde_json::json!("present"), origin),
        )]
        .into_iter()
        .collect();

        let composed = Markdown::try_from(prompt.as_path())
            .unwrap()
            .compose_with(
                ComposeOptions::new()
                    .with_source_file(&prompt)
                    .with_set_overrides(serde_json::json!({ "caller_label": "present" }))
                    .with_caller_input_records(records)
                    .only(&[
                        ComposeOperation::FrontmatterInterpolation,
                        ComposeOperation::Interpolation,
                    ]),
            )
            .unwrap()
            .0;

        assert!(
            !composed.frontmatter().as_map().contains_key("spec"),
            "an absent caller property must not turn a document-owned schema default into an effective caller value",
        );
        assert_eq!(
            composed.frontmatter().as_map()["$schema"]["spec"],
            serde_json::json!("file(default('./defaults/spec.md'))"),
            "the schema retains ownership of its unchanged default declaration",
        );
    }

    #[test]
    #[serial_test::serial(caller_file_process_cwd)]
    fn caller_file_materialization_ignores_a_post_capture_process_cwd_change() {
        struct RestoreCwd(std::path::PathBuf);
        impl Drop for RestoreCwd {
            fn drop(&mut self) {
                let _ = std::env::set_current_dir(&self.0);
            }
        }

        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        let launch = repo.path().join("area");
        let unrelated = repo.path().join("unrelated");
        let prompt = repo.path().join("prompts/cwd.md");
        let spec = launch.join("cases/spec.md");
        std::fs::create_dir_all(spec.parent().unwrap()).unwrap();
        std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
        std::fs::create_dir_all(&unrelated).unwrap();
        std::fs::write(&spec, "---\nmarker: captured\n---\n").unwrap();
        std::fs::write(
            &prompt,
            "---\n$schema:\n  spec: file(required)\nmarker: \"{{ frontmatter(spec, 'marker') }}\"\n---\n{{ marker }}\n",
        )
        .unwrap();
        let origin = biscuit_file::FileResolutionContext::new(&launch)
            .with_repository_root(repo.path());
        let records = [(
            "spec".to_string(),
            CallerInputRecord::new(serde_json::json!("cases/spec.md"), origin),
        )]
        .into_iter()
        .collect();
        let options = ComposeOptions::new()
            .with_source_file(&prompt)
            .with_set_overrides(serde_json::json!({ "spec": "cases/spec.md" }))
            .with_caller_input_records(records)
            .only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::Interpolation,
            ]);
        let prior = std::env::current_dir().unwrap();
        let _restore = RestoreCwd(prior);
        std::env::set_current_dir(&unrelated).unwrap();

        let composed = Markdown::try_from(prompt.as_path())
            .unwrap()
            .compose_with(options)
            .unwrap()
            .0;
        assert_eq!(composed.frontmatter().as_map()["marker"], serde_json::json!("captured"));
        assert_eq!(
            composed.frontmatter().as_map()["spec"],
            serde_json::json!(spec.to_string_lossy().into_owned()),
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_caller_file_native_and_presentation_values_share_one_identity() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        let launch = repo.path().join("area");
        let prompt = repo.path().join("prompts/windows.md");
        let spec = launch.join("cases/spec.md");
        std::fs::create_dir_all(spec.parent().unwrap()).unwrap();
        std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
        std::fs::write(&spec, "# Windows identity\n").unwrap();
        std::fs::write(
            &prompt,
            "---\n$schema:\n  spec: file(required)\n---\n{{ spec }}\n",
        )
        .unwrap();
        let origin = biscuit_file::FileResolutionContext::new(&launch)
            .with_repository_root(repo.path());
        let records = [(
            "spec".to_string(),
            CallerInputRecord::new(serde_json::json!("cases/spec.md"), origin),
        )]
        .into_iter()
        .collect();
        let composed = Markdown::try_from(prompt.as_path())
            .unwrap()
            .compose_with(
                ComposeOptions::new()
                    .with_source_file(&prompt)
                    .with_set_overrides(serde_json::json!({ "spec": "cases/spec.md" }))
                    .with_caller_input_records(records)
                    .only(&[
                        ComposeOperation::FrontmatterInterpolation,
                        ComposeOperation::Interpolation,
                    ]),
            )
            .unwrap()
            .0;
        let native = composed.frontmatter().as_map()["spec"].as_str().unwrap();
        let presentation = composed.content().trim();

        assert_eq!(std::path::PathBuf::from(native), spec);
        assert_eq!(presentation, biscuit_file::to_portable_string(&spec));
        assert!(native.contains('\\'));
        assert!(presentation.contains('/'));
    }

    #[test]
    fn eager_array_and_property_union_project_before_frontmatter_expressions() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        let package = repo.path().join("claudine");
        let prompt = repo.path().join("prompts/array-union.md");
        std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
        std::fs::create_dir_all(&package).unwrap();
        let first = package.join("first.md");
        let union = package.join("union.md");
        std::fs::write(&first, "# First\n").unwrap();
        std::fs::write(&union, "# Union\n").unwrap();
        std::fs::write(
            &prompt,
            "---\n\
             $schema:\n\
             \x20 specs: file(eager)[]\n\
             \x20 selected: string(required)\n\
             \x20 union_spec:\n\
             \x20   - file(eager; required)\n\
             \x20   - number\n\
             \x20 union_selected: string(required)\n\
             specs: []\n\
             selected: \"{{ specs[0] }}\"\n\
             union_spec: authored.md\n\
             union_selected: \"{{ union_spec }}\"\n\
             ---\n\
             DIRECT={{ specs[0] }}\n\
             UNION={{ union_spec }}\n",
        )
        .unwrap();

        let composed = compose_from_launch(
            &prompt,
            repo.path(),
            &package,
            serde_json::json!({
                "specs": ["first.md"],
                "union_spec": "union.md",
            }),
            [],
        );
        let frontmatter = composed.frontmatter().as_map();
        let native_first = first.to_string_lossy().into_owned();
        let native_union = union.to_string_lossy().into_owned();
        assert_eq!(frontmatter.get("specs"), Some(&serde_json::json!([native_first])));
        assert_eq!(frontmatter.get("selected"), frontmatter["specs"].get(0));
        assert_eq!(
            frontmatter.get("union_spec"),
            Some(&serde_json::json!(native_union)),
        );
        assert_eq!(frontmatter.get("union_selected"), frontmatter.get("union_spec"));
        assert!(composed.content().contains(&format!(
            "DIRECT={}",
            biscuit_file::to_portable_string(&first)
        )));
        assert!(composed.content().contains(&format!(
            "UNION={}",
            biscuit_file::to_portable_string(&union)
        )));

        #[cfg(windows)]
        {
            assert!(frontmatter["selected"].as_str().unwrap().contains('\\'));
            assert!(!biscuit_file::to_portable_string(&first).contains('\\'));
        }
    }

    #[test]
    fn non_eager_and_excluded_caller_values_are_not_projected() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        let package = repo.path().join("claudine");
        std::fs::create_dir_all(&package).unwrap();

        for (name, declaration, excluded) in [
            ("ordinary", "string(required)", false),
            ("excluded", "file(eager; required)", true),
        ] {
            let prompt = repo.path().join(format!("prompts/{name}.md"));
            std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
            std::fs::write(
                &prompt,
                format!(
                    "---\n$schema:\n  spec: '{declaration}'\nspec: authored.md\n---\nSPEC={{{{ spec }}}}\n"
                ),
            )
            .unwrap();
            let composed = compose_from_launch(
                &prompt,
                repo.path(),
                &package,
                serde_json::json!({ "spec": "missing.md" }),
                excluded.then_some("spec"),
            );
            assert_eq!(
                composed.frontmatter().as_map().get("spec"),
                Some(&serde_json::json!("missing.md")),
                "{name} caller value should retain its authored spelling",
            );
            assert!(
                composed.content().contains("SPEC=missing.md"),
                "{name} caller value should stay unprojected in body output",
            );
        }

        let prompt = repo.path().join("prompts/document-owned.md");
        let authored = prompt.parent().unwrap().join("authored.md");
        std::fs::write(&authored, "# Authored\n").unwrap();
        std::fs::write(
            &prompt,
            "---\n$schema:\n  spec: 'file(eager; required)'\nspec: ./authored.md\n---\nSPEC={{ spec }}\n",
        )
        .unwrap();
        let context = biscuit_file::FileResolutionContext::new(repo.path())
            .with_repository_root(repo.path())
            .for_source(&prompt);
        let composed = Markdown::try_from(prompt.as_path())
            .unwrap()
            .compose_with(
                ComposeOptions::new()
                    .with_source_file(&prompt)
                    .with_file_resolution_context(context)
                    .only(&[
                        ComposeOperation::FrontmatterInterpolation,
                        ComposeOperation::Interpolation,
                    ]),
            )
            .unwrap()
            .0;
        assert_eq!(
            composed.frontmatter().as_map().get("spec"),
            Some(&serde_json::json!("prompts/authored.md")),
            "document-owned eager files keep the existing repository-relative normalization",
        );
    }

    #[test]
    fn eager_caller_file_classification_drift_fails_before_shell_expansion() {
        for (case, match_type) in [
            ("non-eager-to-eager", "boolean(required)"),
            ("eager-to-non-eager", "string(required)"),
        ] {
            let repo = tempfile::tempdir().unwrap();
            std::fs::create_dir_all(repo.path().join(".git")).unwrap();
            std::fs::create_dir_all(repo.path().join("schemas")).unwrap();
            let package = repo.path().join("claudine");
            std::fs::create_dir_all(&package).unwrap();
            std::fs::write(package.join("spec.md"), "# Specification\n").unwrap();
            std::fs::write(
                repo.path().join("schemas/eager-file.yaml"),
                format!(
                    "kind: trigger-schema\nmatch:\n  gate: {match_type}\n$schema: payload.yaml\n"
                ),
            )
            .unwrap();
            std::fs::write(
                repo.path().join("schemas/payload.yaml"),
                "$schema:\n  spec: file(eager; required)\n",
            )
            .unwrap();
            let prompt = repo.path().join("prompts/drift.md");
            std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
            std::fs::write(
                &prompt,
                "---\nenabled: true\ngate: \"{{ enabled }}\"\nspec: authored.md\nsentinel: \"$(darkmatter_test_should_not_execute)\"\n---\n{{ spec }}\n",
            )
            .unwrap();
            let context = biscuit_file::FileResolutionContext::new(&package)
                .with_repository_root(repo.path())
                .with_source_path(&prompt);
            let err = Markdown::try_from(prompt.as_path())
                .unwrap()
                .compose_with(
                    ComposeOptions::new()
                        .with_source_file(&prompt)
                        .with_file_resolution_context(context)
                        .with_file_ref_fallback_dir(&package)
                        .with_set_overrides(serde_json::json!({ "spec": "spec.md" }))
                        .with_trigger_schemas(true)
                        .only(&[
                            ComposeOperation::FrontmatterInterpolation,
                            ComposeOperation::FrontmatterShellExpansion,
                            ComposeOperation::Interpolation,
                        ]),
                )
                .expect_err("phase-unstable caller file typing must fail closed");
            assert!(
                matches!(
                    err,
                    MarkdownError::CallerFileClassificationChanged { ref property }
                        if property == "spec"
                ),
                "{case} should report the typed spec classification error, got {err:?}",
            );
        }
    }

    #[test]
    fn eager_caller_file_failures_retain_typed_reference_diagnostics() {
        use crate::markdown::schemas::FileReferenceDiagnostic;

        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        let package = repo.path().join("claudine");
        let prompt = repo.path().join("prompts/failure.md");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
        std::fs::write(
            &prompt,
            "---\n$schema:\n  spec: file(eager; required)\nspec: authored.md\n---\n{{ spec }}\n",
        )
        .unwrap();

        for (raw, expected_kind) in [("@//rooted", "syntax"), ("missing.md", "no-match")] {
            let context = biscuit_file::FileResolutionContext::new(&package)
                .with_repository_root(repo.path())
                .with_source_path(&prompt);
            let err = Markdown::try_from(prompt.as_path())
                .unwrap()
                .compose_with(
                    ComposeOptions::new()
                        .with_source_file(&prompt)
                        .with_file_resolution_context(context)
                        .with_file_ref_fallback_dir(&package)
                        .with_set_overrides(serde_json::json!({ "spec": raw }))
                        .only(&[
                            ComposeOperation::FrontmatterInterpolation,
                            ComposeOperation::Interpolation,
                        ]),
                )
                .expect_err("invalid eager caller input must fail before interpolation");
            let MarkdownError::SchemaValidationFailed { problems, .. } = err else {
                panic!("expected typed schema failure for {raw}, got {err:?}");
            };
            assert_eq!(problems.len(), 1, "{raw} should produce one focused problem");
            assert_eq!(problems[0].path, "/spec");
            assert!(
                matches!(
                    (&problems[0].file_reference, expected_kind),
                    (Some(FileReferenceDiagnostic::InvalidSyntax { .. }), "syntax")
                        | (Some(FileReferenceDiagnostic::NoMatch { .. }), "no-match")
                ),
                "{raw} should retain its {expected_kind} diagnostic: {:?}",
                problems[0],
            );
        }
    }

    #[test]
    fn lazy_caller_read_failure_retains_raw_origin_and_selected_candidate() {
        use crate::markdown::compose::expression::ExpressionError;

        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        let launch = repo.path().join("claudine");
        let prompt = repo.path().join("prompts/target.md");
        std::fs::create_dir_all(&launch).unwrap();
        std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
        std::fs::write(
            &prompt,
            "---\n$schema:\n  spec: file(required)\nvalue: \"{{ frontmatter(spec, 'value') }}\"\n---\nBody\n",
        )
        .unwrap();
        let raw = "fixes/missing/spec.md";
        let origin = biscuit_file::FileResolutionContext::new(&launch)
            .with_repository_root(repo.path());
        let records = [(
            "spec".to_string(),
            CallerInputRecord::new(serde_json::json!(raw), origin.clone()),
        )]
        .into_iter()
        .collect();

        let error = Markdown::try_from(prompt.as_path())
            .unwrap()
            .compose_with(
                ComposeOptions::new()
                    .with_source_file(&prompt)
                    .with_set_overrides(serde_json::json!({ "spec": raw }))
                    .with_caller_input_records(records)
                    .only(&[
                        ComposeOperation::FrontmatterInterpolation,
                        ComposeOperation::Interpolation,
                    ]),
            )
            .expect_err("the later frontmatter read must report the missing lazy candidate");
        let MarkdownError::Interpolation { cause, .. } = error else {
            panic!("expected interpolation failure, got {error:?}");
        };
        let ExpressionError::FileReference(diagnostic) = cause.as_ref() else {
            panic!("expected typed file-reference cause, got {cause:?}");
        };
        let caller = diagnostic
            .caller
            .as_ref()
            .expect("schema-projected lazy value must retain caller evidence");
        let expected = launch.join(raw);
        assert_eq!(diagnostic.reference, raw);
        assert_eq!(diagnostic.base_dir, launch);
        assert_eq!(caller.property, "spec");
        assert_eq!(caller.origin.base_dir(), origin.base_dir());
        assert_eq!(caller.candidate, expected);
        assert_eq!(
            caller.candidate_provenance,
            biscuit_file::RootProvenance::Source
        );
    }

    #[test]
    fn equal_lazy_candidates_retain_the_selected_property_occurrence() {
        use crate::markdown::compose::expression::{ExpressionError, FileReferenceDiagnostic};

        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        let prompt = repo.path().join("prompts/target.md");
        let launch = repo.path().join("launch");
        let first_source = launch.join("caller-first.md");
        let second_source = launch.join("caller-second.md");
        std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
        std::fs::create_dir_all(&launch).unwrap();
        let first_raw = "missing.md";
        let second_raw = "./missing.md";
        let first_origin =
            biscuit_file::FileResolutionContext::new(&launch).with_source_path(&first_source);
        let second_origin =
            biscuit_file::FileResolutionContext::new(&launch).with_source_path(&second_source);
        let records = [
            (
                "first".to_string(),
                CallerInputRecord::new(serde_json::json!(first_raw), first_origin.clone()),
            ),
            (
                "second".to_string(),
                CallerInputRecord::new(serde_json::json!(second_raw), second_origin.clone()),
            ),
        ]
        .into_iter()
        .collect::<CallerInputRecords>();

        let failure_for = |property: &str| -> FileReferenceDiagnostic {
            std::fs::write(
                &prompt,
                format!(
                    "---\n$schema:\n  first: file(required)\n  second: file(required)\nvalue: \"{{{{ frontmatter({property}, 'value') }}}}\"\n---\nBody\n"
                ),
            )
            .unwrap();
            let error = Markdown::try_from(prompt.as_path())
                .unwrap()
                .compose_with(
                    ComposeOptions::new()
                        .with_source_file(&prompt)
                        .with_set_overrides(serde_json::json!({
                            "first": first_raw,
                            "second": second_raw,
                        }))
                        .with_caller_input_records(records.clone())
                        .only(&[
                            ComposeOperation::FrontmatterInterpolation,
                            ComposeOperation::Interpolation,
                        ]),
                )
                .expect_err("the selected lazy caller file must be missing");
            let MarkdownError::Interpolation { cause, .. } = error else {
                panic!("expected interpolation failure, got {error:?}");
            };
            let ExpressionError::FileReference(diagnostic) = cause.as_ref() else {
                panic!("expected typed file-reference cause, got {cause:?}");
            };
            diagnostic.clone()
        };

        let first = failure_for("first");
        let second = failure_for("second");
        let first_caller = first.caller.as_ref().unwrap();
        let second_caller = second.caller.as_ref().unwrap();
        assert_eq!(first.reference, first_raw);
        assert_eq!(first_caller.property, "first");
        assert_eq!(first.base_dir, launch);
        assert_eq!(first_caller.origin.source_path(), Some(first_source.as_path()));
        assert_eq!(first_caller.candidate, launch.join(first_raw));
        assert_eq!(second.reference, second_raw);
        assert_eq!(second_caller.property, "second");
        assert_eq!(second.base_dir, launch);
        assert_eq!(second_caller.origin.source_path(), Some(second_source.as_path()));
        assert_eq!(second_caller.candidate, launch.join("missing.md"));
        assert_eq!(first_caller.candidate, second_caller.candidate);
    }

    #[test]
    fn duplicate_lazy_array_identities_retain_each_raw_occurrence() {
        use crate::markdown::compose::expression::{ExpressionError, FileReferenceDiagnostic};

        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        let launch = repo.path().join("launch");
        let prompt = repo.path().join("prompts/target.md");
        std::fs::create_dir_all(&launch).unwrap();
        std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
        let raw = serde_json::json!(["missing.md", "./missing.md"]);
        let origin = biscuit_file::FileResolutionContext::new(&launch);
        let records = [(
            "files".to_string(),
            CallerInputRecord::new(raw.clone(), origin.clone()),
        )]
        .into_iter()
        .collect::<CallerInputRecords>();

        let failure_for = |index: usize| -> FileReferenceDiagnostic {
            std::fs::write(
                &prompt,
                format!(
                    "---\n$schema:\n  files: file(required)[]\nvalue: \"{{{{ frontmatter(files[{index}], 'value') }}}}\"\n---\nBody\n"
                ),
            )
            .unwrap();
            let error = Markdown::try_from(prompt.as_path())
                .unwrap()
                .compose_with(
                    ComposeOptions::new()
                        .with_source_file(&prompt)
                        .with_set_overrides(serde_json::json!({ "files": raw }))
                        .with_caller_input_records(records.clone())
                        .only(&[
                            ComposeOperation::FrontmatterInterpolation,
                            ComposeOperation::Interpolation,
                        ]),
                )
                .expect_err("the selected lazy caller array item must be missing");
            let MarkdownError::Interpolation { cause, .. } = error else {
                panic!("expected interpolation failure, got {error:?}");
            };
            let ExpressionError::FileReference(diagnostic) = cause.as_ref() else {
                panic!("expected typed file-reference cause, got {cause:?}");
            };
            diagnostic.clone()
        };

        let first = failure_for(0);
        let second = failure_for(1);
        assert_eq!(first.reference, "missing.md");
        assert_eq!(second.reference, "./missing.md");
        assert_eq!(first.base_dir, launch);
        assert_eq!(second.base_dir, launch);
        assert_eq!(first.caller.as_ref().unwrap().property, "files");
        assert_eq!(second.caller.as_ref().unwrap().property, "files");
        assert_eq!(first.caller.as_ref().unwrap().origin.base_dir(), launch);
        assert_eq!(second.caller.as_ref().unwrap().origin.base_dir(), launch);
        assert_eq!(first.caller.as_ref().unwrap().candidate, launch.join("missing.md"));
        assert_eq!(first.caller.as_ref().unwrap().candidate, second.caller.as_ref().unwrap().candidate);
    }

    #[test]
    fn dynamic_lazy_array_index_retains_the_selected_raw_occurrence() {
        use crate::markdown::compose::expression::{ExpressionError, FileReferenceDiagnostic};

        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        let launch = repo.path().join("launch");
        let prompt = repo.path().join("prompts/target.md");
        let source = launch.join("caller.md");
        std::fs::create_dir_all(&launch).unwrap();
        std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
        let raw = serde_json::json!(["missing.md", "./missing.md"]);
        let origin = biscuit_file::FileResolutionContext::new(&launch)
            .with_repository_root(repo.path())
            .with_source_path(&source);
        let records = [(
            "files".to_string(),
            CallerInputRecord::new(raw.clone(), origin.clone()),
        )]
        .into_iter()
        .collect::<CallerInputRecords>();

        let failure_for = |index: usize| -> FileReferenceDiagnostic {
            std::fs::write(
                &prompt,
                format!(
                    "---\n$schema:\n  files: file(required)[]\nindex: {index}\nvalue: \"{{{{ frontmatter(files[index], 'value') }}}}\"\n---\nBody\n"
                ),
            )
            .unwrap();
            let error = Markdown::try_from(prompt.as_path())
                .unwrap()
                .compose_with(
                    ComposeOptions::new()
                        .with_source_file(&prompt)
                        .with_set_overrides(serde_json::json!({ "files": raw }))
                        .with_caller_input_records(records.clone())
                        .only(&[
                            ComposeOperation::FrontmatterInterpolation,
                            ComposeOperation::Interpolation,
                        ]),
                )
                .expect_err("the dynamically selected caller array item must be missing");
            let MarkdownError::Interpolation { cause, .. } = error else {
                panic!("expected interpolation failure, got {error:?}");
            };
            let ExpressionError::FileReference(diagnostic) = cause.as_ref() else {
                panic!("expected typed file-reference cause, got {cause:?}");
            };
            diagnostic.clone()
        };

        let first = failure_for(0);
        let second = failure_for(1);
        for (diagnostic, expected_raw) in [(&first, "missing.md"), (&second, "./missing.md")] {
            let caller = diagnostic.caller.as_ref().unwrap();
            assert_eq!(diagnostic.reference, expected_raw);
            assert_eq!(diagnostic.base_dir, launch);
            assert_eq!(caller.property, "files");
            assert_eq!(caller.origin.base_dir(), origin.base_dir());
            assert_eq!(caller.origin.repository_root(), origin.repository_root());
            assert_eq!(caller.origin.source_path(), Some(source.as_path()));
            assert_eq!(caller.candidate, launch.join("missing.md"));
            assert_eq!(
                caller.candidate_provenance,
                biscuit_file::RootProvenance::Source
            );
        }
        assert_eq!(first.caller.as_ref().unwrap().candidate, second.caller.as_ref().unwrap().candidate);
    }
}
