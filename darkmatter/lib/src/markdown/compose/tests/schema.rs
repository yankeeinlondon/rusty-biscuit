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
}
