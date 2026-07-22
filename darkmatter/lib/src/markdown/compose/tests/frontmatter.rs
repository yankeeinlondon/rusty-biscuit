use super::*;

// ── Frontmatter Interpolation Integration Tests ─────────────────

#[test]
fn test_frontmatter_interpolation_spec_example() {
    let content = "---\nbase: /path/to/something\nspec: \"{{base}}/spec.md\"\nplan: \"{{base}}/plan.md\"\n---\nThe spec is located at: {{spec}}\nThe plan is located at: {{plan}}";
    let md: Markdown = content.into();
    let (composed, report) = md
        .compose_with(ComposeOptions::new().only(&[
            ComposeOperation::FrontmatterInterpolation,
            ComposeOperation::Interpolation,
        ]))
        .unwrap();

    assert_eq!(report.frontmatter_interpolations_applied, 2);
    assert!(
        composed
            .content()
            .contains("The spec is located at: /path/to/something/spec.md")
    );
    assert!(
        composed
            .content()
            .contains("The plan is located at: /path/to/something/plan.md")
    );
}

#[test]
fn test_frontmatter_interpolation_with_set_overrides() {
    let content = "---\nbase: /original\nspec: \"{{base}}/spec.md\"\n---\nSpec: {{spec}}";
    let md: Markdown = content.into();
    let (composed, report) = md
        .compose_with(
            ComposeOptions::new()
                .only(&[
                    ComposeOperation::FrontmatterInterpolation,
                    ComposeOperation::Interpolation,
                ])
                .with_set_overrides(serde_json::json!({"base": "/override"})),
        )
        .unwrap();

    assert_eq!(report.frontmatter_interpolations_applied, 1);
    assert!(composed.content().contains("Spec: /override/spec.md"));
}

#[test]
fn compose_reports_eager_spec_path_before_derived_nulls_mask_it() {
    let dir = tempfile::tempdir().unwrap();
    let prompt_dir = dir.path().join("prompts");
    let feature_dir = dir.path().join("features/2026-06-30-replace-expression");
    std::fs::create_dir_all(&prompt_dir).unwrap();
    std::fs::create_dir_all(&feature_dir).unwrap();
    std::fs::write(feature_dir.join("spec.md"), "---\ntitle: Real Spec\n---\n").unwrap();
    std::fs::write(
        feature_dir.join("plan.md"),
        "---\nstart_phase: 1\ntotal_phases: 2\n---\n",
    )
    .unwrap();

    let prompt_path = prompt_dir.join("implement-plan.md");
    std::fs::write(
        &prompt_path,
        "---\n\
         $schema:\n\
         \x20 phase: number(required)\n\
         \x20 total_phases: number(required)\n\
         \x20 plan: file(eager; required)\n\
         \x20 spec: file(eager)\n\
         plan: \"{{ spec ? dirname(spec) + '/plan.md' : null }}\"\n\
         phase: \"{{ file_exists(plan) ? frontmatter(plan, 'start_phase') || 1 : null }}\"\n\
         total_phases: \"{{ file_exists(plan) ? frontmatter(plan, 'total_phases') || frontmatter(plan, 'phases') : 0 }}\"\n\
         spec: \"{{ file_exists(plan) ? file_exists(dirname(plan) + '/spec.md') ? dirname(plan) + '/spec.md' : null : null }}\"\n\
         ---\n\
         Body\n",
    )
    .unwrap();

    let md = Markdown::try_from_content(std::fs::read_to_string(&prompt_path).unwrap()).unwrap();
    let err = md
        .compose_with(
            ComposeOptions::new()
                .with_source_file(&prompt_path)
                .with_file_ref_fallback_dir(dir.path())
                .with_set_overrides(serde_json::json!({
                    "spec": "reviews/2026-06-30-replace-expression/spec.md",
                })),
        )
        .expect_err("the stale reviews/ spec path should fail schema validation");

    match err {
        MarkdownError::SchemaValidationFailed { problems, .. } => {
            assert!(
                problems.iter().any(|problem| {
                    problem.path == "/spec"
                        && problem
                            .message
                            .contains("reviews/2026-06-30-replace-expression/spec.md")
                }),
                "expected the stale spec path to be reported directly, got {problems:?}",
            );
        }
        other => panic!("expected SchemaValidationFailed, got {other:?}"),
    }
}

#[test]
fn test_frontmatter_interpolation_arrays_and_objects() {
    let content = "---\nbase: /root\npaths:\n  - \"{{base}}/a\"\n  - \"{{base}}/b\"\nmeta:\n  home: \"{{base}}/home\"\n---\n";
    let md: Markdown = content.into();
    let (_, report) = md
        .compose_with(ComposeOptions::new().only(&[ComposeOperation::FrontmatterInterpolation]))
        .unwrap();

    assert!(report.frontmatter_interpolations_applied >= 3);
}

#[test]
fn test_frontmatter_interpolation_disabled() {
    let content = "---\nbase: /path\nspec: \"{{base}}/spec.md\"\n---\n{{spec}}";
    let md: Markdown = content.into();
    let (composed, report) = md
        .compose_with(
            ComposeOptions::new()
                .disable(ComposeOperation::FrontmatterInterpolation)
                .only(&[ComposeOperation::Interpolation]),
        )
        .unwrap();

    assert_eq!(report.frontmatter_interpolations_applied, 0);
    // body interpolation resolves {{spec}} to {{base}}/spec.md and then
    // recursively resolves {{base}} in the same pass.
    assert!(composed.content().contains("/path/spec.md"));
}

#[test]
fn test_frontmatter_interpolation_body_still_skips_fenced_code() {
    // Inline code spans interpolate, but fenced blocks remain untouched
    // unless `interpolate_code_blocks` is set.
    let content =
        "---\nname: World\n---\nHello {{ name }}! Code: `{{ name }}`\n\n```\n{{ name }}\n```";
    let md: Markdown = content.into();
    let (composed, _) = md
        .compose_with(ComposeOptions::new().only(&[
            ComposeOperation::FrontmatterInterpolation,
            ComposeOperation::Interpolation,
        ]))
        .unwrap();

    assert!(composed.content().contains("Hello World!"));
    assert!(composed.content().contains("Code: `World`"));
    assert!(composed.content().contains("```\n{{ name }}\n```"));
}

#[test]
fn test_frontmatter_interpolation_report_counted_separately() {
    let content = "---\nbase: /path\nspec: \"{{base}}/spec.md\"\n---\nHello {{ spec }}!";
    let md: Markdown = content.into();
    let (_, report) = md
        .compose_with(ComposeOptions::new().only(&[
            ComposeOperation::FrontmatterInterpolation,
            ComposeOperation::Interpolation,
        ]))
        .unwrap();

    assert_eq!(report.frontmatter_interpolations_applied, 1);
    assert_eq!(report.interpolations_applied, 1);
}

#[test]
fn test_frontmatter_interpolation_summary() {
    let mut report = ComposeReport::new();
    report.frontmatter_interpolations_applied = 2;
    let summary = report.summary();
    assert!(summary.contains("2 frontmatter interpolation(s)"));
}

#[test]
fn test_frontmatter_interpolation_report_merge() {
    let mut r1 = ComposeReport::new();
    r1.frontmatter_interpolations_applied = 3;
    let mut r2 = ComposeReport::new();
    r2.frontmatter_interpolations_applied = 5;
    r1.merge(r2);
    assert_eq!(r1.frontmatter_interpolations_applied, 8);
}

// ── DM1: exclude-keys integration tests ───────────────────────────

#[test]
fn dm1_excluded_key_survives_raw_through_compose() {
    let content = "---\n\
        base: /root\n\
        failure:\n\
        \x20 message: \"{{err.msg}}\"\n\
        ---\n\
        body\n";
    let md: Markdown = content.into();
    let (composed, report) = md
        .compose_with(
            ComposeOptions::new()
                .only(&[ComposeOperation::FrontmatterInterpolation])
                .with_exclude_keys(["failure"]),
        )
        .unwrap();

    // The excluded key keeps its raw `{{err.msg}}` span.
    assert_eq!(
        composed.frontmatter().as_map().get("failure").unwrap().get("message"),
        Some(&serde_json::json!("{{err.msg}}")),
        "excluded key must survive raw through compose"
    );
    // The deferred-key metadata is surfaced in the report.
    assert!(
        report.deferred_frontmatter_keys.contains("failure"),
        "report must list 'failure' as deferred"
    );
}

#[test]
fn dm1_non_excluded_key_resolves_through_compose() {
    let content = "---\n\
        base: /root\n\
        summary: \"{{base}}/summary\"\n\
        failure:\n\
        \x20 message: \"{{err.msg}}\"\n\
        ---\n\
        body\n";
    let md: Markdown = content.into();
    let (composed, report) = md
        .compose_with(
            ComposeOptions::new()
                .only(&[ComposeOperation::FrontmatterInterpolation])
                .with_exclude_keys(["failure"]),
        )
        .unwrap();

    // Non-excluded key resolves normally.
    assert_eq!(
        composed.frontmatter().as_map().get("summary"),
        Some(&serde_json::json!("/root/summary"))
    );
    // Excluded key stays raw.
    assert_eq!(
        composed.frontmatter().as_map().get("failure").unwrap().get("message"),
        Some(&serde_json::json!("{{err.msg}}"))
    );
    assert_eq!(report.frontmatter_interpolations_applied, 1);
}

#[test]
fn dm1_empty_exclude_set_is_byte_identical_to_default() {
    let content = "---\nbase: /root\nspec: \"{{base}}/spec.md\"\n---\n{{spec}}";
    let md1: Markdown = content.into();
    let md2: Markdown = content.into();

    let (composed_default, _) = md1
        .compose_with(ComposeOptions::new().only(&[
            ComposeOperation::FrontmatterInterpolation,
            ComposeOperation::Interpolation,
        ]))
        .unwrap();

    let (composed_excluded, report) = md2
        .compose_with(
            ComposeOptions::new()
                .only(&[
                    ComposeOperation::FrontmatterInterpolation,
                    ComposeOperation::Interpolation,
                ])
                .with_exclude_keys(std::iter::empty::<&str>()),
        )
        .unwrap();

    assert_eq!(
        composed_default.content(),
        composed_excluded.content(),
        "empty exclude set must be byte-identical to default"
    );
    assert!(
        report.deferred_frontmatter_keys.is_empty(),
        "no keys deferred with empty exclude set"
    );
}

#[test]
fn dm1a_composed_key_referencing_deferred_fails_through_compose() {
    let content = "---\n\
        summary: \"{{ failure.message }}\"\n\
        failure:\n\
        \x20 message: \"{{err.msg}}\"\n\
        ---\n\
        body\n";
    let md: Markdown = content.into();
    let result = md.compose_with(
        ComposeOptions::new()
            .only(&[ComposeOperation::FrontmatterInterpolation])
            .with_exclude_keys(["failure"]),
    );

    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("summary"), "error names the composed key: {msg}");
    assert!(
        msg.contains("failure"),
        "error names the deferred key: {msg}"
    );
}

// ── DM2: subtree compose integration tests ──────────────────────────

#[test]
fn dm2_subtree_resolves_injected_eager_and_lazy_globals() {
    use super::subtree::{InjectedGlobal, SubtreeStrictness, compose_subtree};
    use crate::markdown::compose::EffectiveStateBuilder;
    use std::collections::HashMap;

    let fm: HashMap<String, serde_json::Value> =
        [("phase".to_string(), serde_json::json!(2))].into();
    let state = EffectiveStateBuilder::new()
        .with_frontmatter(fm)
        .build()
        .unwrap();

    let mut globals = HashMap::new();
    globals.insert(
        "err".to_string(),
        InjectedGlobal::eager(serde_json::json!({"msg": "disk full"})),
    );
    globals.insert(
        "current".to_string(),
        InjectedGlobal::lazy(|| serde_json::json!({"ctx": {"today": "2026-06-24"}})),
    );

    let result = compose_subtree(
        &serde_json::json!("phase {{phase}} failed: {{err.msg}} on {{current.ctx.today}}"),
        &state,
        globals,
        SubtreeStrictness::Lenient,
    )
    .unwrap();

    assert_eq!(
        result,
        serde_json::json!("phase 2 failed: disk full on 2026-06-24")
    );
}

#[test]
fn dm2_subtree_layered_seed_state_still_resolves() {
    use super::subtree::{SubtreeStrictness, compose_subtree};
    use crate::markdown::compose::EffectiveStateBuilder;
    use std::collections::HashMap;

    let fm: HashMap<String, serde_json::Value> = [
        ("phase".to_string(), serde_json::json!(3)),
        (
            "config".to_string(),
            serde_json::json!({"artifact": {"path": "/tmp/out"}}),
        ),
    ]
    .into();
    let state = EffectiveStateBuilder::new()
        .with_frontmatter(fm)
        .build()
        .unwrap();

    let result = compose_subtree(
        &serde_json::json!("artifact={{config.artifact.path}} phase={{phase}}"),
        &state,
        HashMap::new(),
        SubtreeStrictness::Lenient,
    )
    .unwrap();
    assert_eq!(result, serde_json::json!("artifact=/tmp/out phase=3"));
}

#[test]
fn dm2_subtree_parity_with_main_compose_whole_value() {
    // A whole-value single `{{ expr }}` yields the same typed Value in subtree
    // compose as main compose's frontmatter interpolation does.
    use super::subtree::{SubtreeStrictness, compose_subtree};
    use crate::markdown::compose::EffectiveStateBuilder;
    use std::collections::HashMap;

    let fm: HashMap<String, serde_json::Value> =
        [("count".to_string(), serde_json::json!(5))].into();
    let state = EffectiveStateBuilder::new()
        .with_frontmatter(fm)
        .build()
        .unwrap();

    // Whole-value span: typed Number result, not a string.
    let result = compose_subtree(
        &serde_json::json!("{{count}}"),
        &state,
        HashMap::new(),
        SubtreeStrictness::Strict,
    )
    .unwrap();
    assert_eq!(result, serde_json::json!(5));
}

#[test]
fn dm2_subtree_parity_with_main_compose_mixed_string() {
    use super::subtree::{SubtreeStrictness, compose_subtree};
    use crate::markdown::compose::EffectiveStateBuilder;
    use std::collections::HashMap;

    let fm: HashMap<String, serde_json::Value> =
        [("count".to_string(), serde_json::json!(5))].into();
    let state = EffectiveStateBuilder::new()
        .with_frontmatter(fm)
        .build()
        .unwrap();

    let result = compose_subtree(
        &serde_json::json!("count={{count}}"),
        &state,
        HashMap::new(),
        SubtreeStrictness::Strict,
    )
    .unwrap();
    assert_eq!(result, serde_json::json!("count=5"));
}

#[test]
fn dm2_subtree_lazy_global_only_evaluated_when_referenced() {
    use super::subtree::{InjectedGlobal, SubtreeStrictness, compose_subtree};
    use crate::markdown::compose::EffectiveStateBuilder;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let state = EffectiveStateBuilder::new().build().unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let count_for_closure = count.clone();
    let mut globals = HashMap::new();
    globals.insert(
        "current".to_string(),
        InjectedGlobal::lazy(move || {
            count_for_closure.fetch_add(1, Ordering::SeqCst);
            serde_json::json!({"phase": 1})
        }),
    );

    // String does NOT reference `current`: closure must not run.
    let result = compose_subtree(
        &serde_json::json!("no reference"),
        &state,
        globals,
        SubtreeStrictness::Lenient,
    )
    .unwrap();
    assert_eq!(result, serde_json::json!("no reference"));
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

#[test]
fn dm2_subtree_lazy_global_evaluated_at_most_once() {
    use super::subtree::{InjectedGlobal, SubtreeStrictness, compose_subtree};
    use crate::markdown::compose::EffectiveStateBuilder;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let state = EffectiveStateBuilder::new().build().unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let count_for_closure = count.clone();
    let mut globals = HashMap::new();
    globals.insert(
        "current".to_string(),
        InjectedGlobal::lazy(move || {
            count_for_closure.fetch_add(1, Ordering::SeqCst);
            serde_json::json!({"phase": 7})
        }),
    );

    // Two references to `current.phase`: closure runs at most once.
    let result = compose_subtree(
        &serde_json::json!("{{current.phase}} then {{current.phase}}"),
        &state,
        globals,
        SubtreeStrictness::Lenient,
    )
    .unwrap();
    assert_eq!(result, serde_json::json!("7 then 7"));
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn dm2_subtree_strict_rejects_unknown_root() {
    use super::subtree::{SubtreeStrictness, compose_subtree};
    use crate::markdown::compose::EffectiveStateBuilder;
    use std::collections::HashMap;

    let fm: HashMap<String, serde_json::Value> =
        [("phase".to_string(), serde_json::json!(2))].into();
    let state = EffectiveStateBuilder::new()
        .with_frontmatter(fm)
        .build()
        .unwrap();

    let err = compose_subtree(
        &serde_json::json!("{{spec_fil}}"),
        &state,
        HashMap::new(),
        SubtreeStrictness::Strict,
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("unknown root"), "error: {err}");
    assert!(err.contains("spec_fil"), "error names the typo: {err}");
}

#[test]
fn dm2_subtree_strict_known_but_empty_renders_empty() {
    use super::subtree::{SubtreeStrictness, compose_subtree};
    use crate::markdown::compose::EffectiveStateBuilder;
    use std::collections::HashMap;

    let fm: HashMap<String, serde_json::Value> =
        [("spec_file".to_string(), serde_json::Value::Null)].into();
    let state = EffectiveStateBuilder::new()
        .with_frontmatter(fm)
        .build()
        .unwrap();

    // `spec_file` is a known root that resolves to null: renders empty.
    let result = compose_subtree(
        &serde_json::json!("spec={{spec_file}}"),
        &state,
        HashMap::new(),
        SubtreeStrictness::Strict,
    )
    .unwrap();
    assert_eq!(result, serde_json::json!("spec="));
}

#[test]
fn dm2_subtree_strict_rejects_malformed_span() {
    use super::subtree::{SubtreeStrictness, compose_subtree};
    use crate::markdown::compose::EffectiveStateBuilder;
    use std::collections::HashMap;

    let state = EffectiveStateBuilder::new().build().unwrap();

    let err = compose_subtree(
        &serde_json::json!("{{ > broken }}"),
        &state,
        HashMap::new(),
        SubtreeStrictness::Strict,
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("failed to parse"), "error: {err}");
}

#[test]
fn dm2_subtree_strict_rejects_unknown_function() {
    use super::subtree::{SubtreeStrictness, compose_subtree};
    use crate::markdown::compose::EffectiveStateBuilder;
    use std::collections::HashMap;

    let fm: HashMap<String, serde_json::Value> =
        [("phase".to_string(), serde_json::json!(2))].into();
    let state = EffectiveStateBuilder::new()
        .with_frontmatter(fm)
        .build()
        .unwrap();

    let err = compose_subtree(
        &serde_json::json!("{{ bogus_fn(phase) }}"),
        &state,
        HashMap::new(),
        SubtreeStrictness::Strict,
    )
    .unwrap_err()
    .to_string();

    assert!(
        err.contains("Unknown function") || err.to_lowercase().contains("bogus_fn"),
        "error names the unknown function: {err}"
    );
}

#[test]
fn dm2_subtree_strict_rejects_unknown_root_in_function_argument() {
    // The strict root check walks the AST, so a typo buried in a function
    // argument also fails.
    use super::subtree::{SubtreeStrictness, compose_subtree};
    use crate::markdown::compose::EffectiveStateBuilder;
    use std::collections::HashMap;

    let fm: HashMap<String, serde_json::Value> =
        [("phase".to_string(), serde_json::json!(2))].into();
    let state = EffectiveStateBuilder::new()
        .with_frontmatter(fm)
        .build()
        .unwrap();

    let err = compose_subtree(
        &serde_json::json!("{{ parent_dir(typo_var) }}"),
        &state,
        HashMap::new(),
        SubtreeStrictness::Strict,
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("unknown root"), "error: {err}");
    assert!(err.contains("typo_var"), "error names the typo: {err}");
}

// ── Nested external state regression tests ────────────────────────

#[test]
fn test_frontmatter_interpolation_nested_external_state() {
    // External state has nested keys; frontmatter references them.
    let content = "---\nmeta:\n  author: Local\nspec: \"{{meta.base}}/spec.md\"\n---\n{{spec}}";
    let md: Markdown = content.into();
    let (composed, report) = md
        .compose_with(
            ComposeOptions::new()
                .with_external_state(serde_json::json!({
                    "meta": {"base": "/root", "author": "Parent"}
                }))
                .only(&[
                    ComposeOperation::FrontmatterInterpolation,
                    ComposeOperation::Interpolation,
                ]),
        )
        .unwrap();

    // meta.base from external state should be deep-merged in
    assert!(
        composed.content().contains("/root/spec.md"),
        "Expected /root/spec.md but got: {}",
        composed.content()
    );
    // frontmatter author should win over external
    assert_eq!(
        composed
            .frontmatter()
            .as_map()
            .get("meta")
            .and_then(|v| v.get("author")),
        Some(&serde_json::json!("Local"))
    );
    assert!(report.frontmatter_interpolations_applied >= 1);
}

#[test]
fn test_external_state_deep_merge_preserves_frontmatter_values() {
    // Both frontmatter and external have nested objects; frontmatter wins on conflict.
    let content =
        "---\nconfig:\n  theme: dark\n---\ntheme={{config.theme}} lang={{config.lang}}";
    let md: Markdown = content.into();
    let (composed, _) = md
        .compose_with(
            ComposeOptions::new()
                .with_external_state(serde_json::json!({
                    "config": {"theme": "light", "lang": "en"}
                }))
                .only(&[ComposeOperation::Interpolation]),
        )
        .unwrap();

    assert!(
        composed.content().contains("theme=dark"),
        "Frontmatter should win: {}",
        composed.content()
    );
    assert!(
        composed.content().contains("lang=en"),
        "External nested key should fill in: {}",
        composed.content()
    );
}

// ── Child document frontmatter from parent state ──────────────────

#[test]
fn test_child_frontmatter_interpolation_from_parent_state() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let child = dir.path().join("child.md");

    std::fs::write(&root, "---\nbase: /docs\n---\n::file ./child.md").unwrap();
    std::fs::write(
        &child,
        "---\nspec: \"{{base}}/spec.md\"\n---\nSpec: {{spec}}",
    )
    .unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, _) = md.compose_with(options).unwrap();

    assert!(
        composed.content().contains("Spec: /docs/spec.md"),
        "Child should derive frontmatter from parent state: {}",
        composed.content()
    );
}

// ── Interpolated prologue/epilogue paths ──────────────────────────

#[test]
fn test_interpolated_prologue_path() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let intro = dir.path().join("intro.md");

    std::fs::write(
        &root,
        "---\nparts: .\nprologue: \"{{parts}}/intro.md\"\n---\nBody",
    )
    .unwrap();
    std::fs::write(&intro, "Prologue content").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, report) = md.compose_with(options).unwrap();

    assert!(
        composed.content().contains("Prologue content"),
        "Interpolated prologue path should resolve: {}",
        composed.content()
    );
    assert!(report.frontmatter_interpolations_applied >= 1);
}

#[test]
fn test_interpolated_epilogue_path() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.md");
    let outro = dir.path().join("outro.md");

    std::fs::write(
        &root,
        "---\nparts: .\nepilogue: \"{{parts}}/outro.md\"\n---\nBody",
    )
    .unwrap();
    std::fs::write(&outro, "Epilogue content").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let options = ComposeOptions::new().with_source_file(root);
    let (composed, report) = md.compose_with(options).unwrap();

    assert!(
        composed.content().contains("Epilogue content"),
        "Interpolated epilogue path should resolve: {}",
        composed.content()
    );
    assert!(report.frontmatter_interpolations_applied >= 1);
}

// ── Page blocks consuming interpolated frontmatter values ─────────

#[test]
fn test_page_block_uses_interpolated_frontmatter() {
    // Frontmatter interpolation produces a value that page blocks consume.
    let content = "---\nbase: show\nflag: \"{{base}}\"\n---\n\n::block when=\"flag\"\n\nVisible\n\n::end-block\n";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[
        ComposeOperation::FrontmatterInterpolation,
        ComposeOperation::PageBlocks,
    ]);
    let (composed, report) = md.compose_with(options).unwrap();

    assert!(
        composed.content().contains("Visible"),
        "Page block should see interpolated frontmatter value: {}",
        composed.content()
    );
    assert!(report.frontmatter_interpolations_applied >= 1);
    assert!(report.page_blocks_rendered >= 1);
}

#[test]
fn test_page_block_false_from_interpolated_frontmatter() {
    let content = "---\nbase: \"\"\nflag: \"{{base}}\"\n---\n\n::block when=\"flag\"\n\nHidden\n\n::end-block\n\nAfter\n";
    let md: Markdown = content.into();

    let options = ComposeOptions::new().only(&[
        ComposeOperation::FrontmatterInterpolation,
        ComposeOperation::PageBlocks,
    ]);
    let (composed, _) = md.compose_with(options).unwrap();

    assert!(
        !composed.content().contains("Hidden"),
        "Page block with falsy interpolated value should be removed: {}",
        composed.content()
    );
    assert!(composed.content().contains("After"));
}

// ── Named-object string coercion (Sequence Plus) ────────────────

#[test]
fn name_coercion_renders_name_in_inline_body_context() {
    let content = "---\ntitle: t\n---\nName is {{state}}";
    let md: Markdown = content.into();
    let (composed, _report) = md
        .compose_with(
            ComposeOptions::new()
                .only(&[
                    ComposeOperation::FrontmatterInterpolation,
                    ComposeOperation::Interpolation,
                ])
                .with_set_overrides(serde_json::json!({"state": {"name": "alpha", "index": 1}}))
                .with_name_coercion_keys(vec!["state".to_string()]),
        )
        .unwrap();

    assert!(
        composed.content().contains("Name is alpha"),
        "inline {{{{state}}}} must render the name field, got: {}",
        composed.content()
    );
}

#[test]
fn name_coercion_is_opt_in_body_renders_json_without_keys() {
    // Same document WITHOUT with_name_coercion_keys: the object renders as JSON,
    // proving the coercion is opt-in and off by default.
    let content = "---\ntitle: t\n---\nName is {{state}}";
    let md: Markdown = content.into();
    let (composed, _report) = md
        .compose_with(
            ComposeOptions::new()
                .only(&[
                    ComposeOperation::FrontmatterInterpolation,
                    ComposeOperation::Interpolation,
                ])
                .with_set_overrides(serde_json::json!({"state": {"name": "alpha", "index": 1}})),
        )
        .unwrap();

    assert!(
        !composed.content().contains("Name is alpha\n") && !composed.content().ends_with("Name is alpha"),
        "without coercion keys the object must not collapse to the bare name, got: {}",
        composed.content()
    );
    assert!(
        composed.content().contains("\"name\":\"alpha\""),
        "without coercion keys the object renders as JSON, got: {}",
        composed.content()
    );
}

#[test]
fn name_coercion_whole_value_frontmatter_span_keeps_object() {
    // A whole-value `{{ state }}` span keeps the TYPED object even when the key
    // is registered for name coercion.
    let content = "---\nx: \"{{ state }}\"\n---\nBody";
    let md: Markdown = content.into();
    let (composed, _report) = md
        .compose_with(
            ComposeOptions::new()
                .only(&[ComposeOperation::FrontmatterInterpolation])
                .with_set_overrides(serde_json::json!({"state": {"name": "alpha", "index": 1}}))
                .with_name_coercion_keys(vec!["state".to_string()]),
        )
        .unwrap();

    assert_eq!(
        composed.frontmatter().as_map().get("x"),
        Some(&serde_json::json!({"name": "alpha", "index": 1})),
        "whole-value span must keep the typed object"
    );
}

#[test]
fn name_coercion_inline_frontmatter_value_coerces() {
    // Inline-compose path: a frontmatter value with mixed text coerces the
    // named object to its `name` when the key is registered.
    let content = "---\nprompt: \"Work on {{state}}\"\n---\nBody";
    let md: Markdown = content.into();
    let (composed, _report) = md
        .compose_with(
            ComposeOptions::new()
                .only(&[ComposeOperation::FrontmatterInterpolation])
                .with_set_overrides(serde_json::json!({"state": {"name": "alpha", "index": 1}}))
                .with_name_coercion_keys(vec!["state".to_string()]),
        )
        .unwrap();

    assert_eq!(
        composed.frontmatter().as_map().get("prompt"),
        Some(&serde_json::json!("Work on alpha")),
        "inline frontmatter value must coerce to the name"
    );
}

