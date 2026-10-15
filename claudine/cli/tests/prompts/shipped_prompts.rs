//! The repository's internal `prompts/`, read live: their frontmatter, their
//! lifecycle `set` payloads, and the implement router and plan prompts run
//! end to end. Opt-in through `prompt-tests`; CI never runs these.

use std::path::{Path, PathBuf};

use crate::common;
use common::lifecycle_set_corpus::mapping_only_set_findings;
use common::prompt_staging::workspace_root;
use common::{CliProcessFixture, write};
#[cfg(unix)]
use common::write_executable;
use darkmatter::markdown::Markdown;

fn collect_markdown_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()))
    {
        let path = entry.expect("prompt directory entry").path();
        if path.is_dir() {
            collect_markdown_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "md") {
            files.push(path);
        }
    }
}

#[test]
fn shipped_prompts_use_mapping_only_set() {
    let (_, defects) = mapping_only_set_findings(&[workspace_root().join("prompts")]);
    assert!(
        defects.is_empty(),
        "prompts with removed or invalid lifecycle `set` syntax:\n{}",
        defects.join("\n")
    );
}

#[test]
fn shipped_prompt_corpus_parses_frontmatter() {
    let prompts = workspace_root().join("prompts");
    let mut files = Vec::new();
    collect_markdown_files(&prompts, &mut files);
    files.sort();

    assert!(!files.is_empty(), "the shipped prompt corpus must not be empty");
    let implement_plan = std::fs::read_to_string(prompts.join("_implement/implement-plan.md"))
        .expect("shipped implementation prompt");
    assert!(
        implement_plan.contains("frontmatter(log, 'message_to_agent')"),
        "the passive corpus must include the shipped handoff reader"
    );
    assert!(
        !implement_plan.contains("set: ["),
        "the shipped implementation prompt must not retain positional lifecycle set syntax"
    );
    let failures = files
        .iter()
        .filter_map(|path| {
            Markdown::try_from(path.as_path())
                .err()
                .map(|error| format!("{}: {error}", path.display()))
        })
        .collect::<Vec<_>>();
    assert!(
        failures.is_empty(),
        "shipped prompts with invalid Markdown/frontmatter:\n{}",
        failures.join("\n")
    );
}

/// Internal drafts whose lifecycle is authored in verbs that do not exist yet
/// (`prompts/fix.md` sketches `prep` and `interview`, and says so in its own
/// comment). Migrating a draft's grammar cannot make it parse, so it is
/// excluded by name rather than left to fail the corpus.
const DRAFT_LIFECYCLE_PROMPTS: &[&str] = &["prompts/fix.md"];

/// Every internal document that carries frontmatter — the root, Claudine, and
/// Darkmatter `prompts/`, the research template, and every research fleet —
/// parses under the current lifecycle grammar, so a retired spelling surfaces
/// here as its `RemovedGrammar` hint rather than at someone's next launch.
///
/// [`DRAFT_LIFECYCLE_PROMPTS`] names the drafts that are excluded on purpose.
#[test]
fn internal_lifecycle_corpus_parses_under_current_grammar() {
    use claudine::composition::lifecycle::parse_lifecycle_config;
    use common::lifecycle_set_corpus::frontmatter_value;

    let root = workspace_root();
    let mut files = Vec::new();
    collect_markdown_files(&root.join("prompts"), &mut files);
    collect_markdown_files(&root.join("claudine/prompts"), &mut files);
    collect_markdown_files(&root.join("darkmatter/prompts"), &mut files);
    files.push(root.join("claudine/docs/research/_TEMPLATE.md"));
    let research = root.join("claudine/docs/research");
    for entry in std::fs::read_dir(&research).expect("research directory") {
        let fleet = entry.expect("research entry").path().join("_fleet.md");
        if fleet.is_file() {
            files.push(fleet);
        }
    }
    files.sort();

    let mut parsed = 0;
    let mut sequences = 0;
    let mut failures = Vec::new();
    for path in &files {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        if !text.starts_with("---") {
            continue;
        }
        let relative = path.strip_prefix(&root).unwrap_or(path).display().to_string();
        if DRAFT_LIFECYCLE_PROMPTS.contains(&relative.replace('\\', "/").as_str()) {
            continue;
        }
        match Markdown::try_from(path.as_path()) {
            Ok(markdown) => match parse_lifecycle_config(&frontmatter_value(&markdown), path) {
                Ok(lifecycle) => {
                    parsed += 1;
                    let frontmatter = frontmatter_value(&markdown);
                    // Schema-declared optional caller inputs are ordinary
                    // properties even when the passive read has no caller.
                    let declared_inputs = frontmatter.get("$schema").and_then(serde_json::Value::as_object)
                        .map(|schema| serde_json::Value::Object(schema.keys()
                            .map(|key| (key.clone(), serde_json::Value::Null)).collect()));
                    if let Err(error) = claudine::composition::lifecycle::bindings::reject_removed_sequence_reads(
                        path,
                        claudine::composition::lifecycle::bindings::AuthoredDocument {
                            frontmatter: &frontmatter,
                            body: markdown.content(),
                            body_line_offset: 0,
                        },
                        declared_inputs.as_ref(),
                        Some(&lifecycle),
                    ) {
                        failures.push(format!("{relative}: {error}"));
                    }
                    if markdown.frontmatter().as_map().contains_key("sequence") {
                        let resolution = biscuit_file::FileResolutionContext::new(path.parent().unwrap())
                            .with_repository_root(&root).with_source_path(path);
                        let source = claudine::composition::ResolvedCompositionSource {
                            original_ref: path.display().to_string(),
                            resolved_path: path.clone(),
                            original_text: text.clone(),
                            markdown,
                        };
                        match claudine::composition::resolve_sequence_plan(&source, &resolution) {
                            Ok(Some(_)) => sequences += 1,
                            Ok(None) => failures.push(format!("{relative}: sequence was not recognized")),
                            Err(error) => failures.push(format!("{relative}: {error}")),
                        }
                    }
                },
                Err(error) => failures.push(format!("{relative}: {error}")),
            },
            Err(error) => failures.push(format!("{relative}: frontmatter does not parse: {error}")),
        }
    }

    assert!(parsed > 0, "the internal lifecycle corpus must not be empty");
    assert!(sequences > 0, "the internal corpus must exercise sequence grammar");
    assert!(
        failures.is_empty(),
        "internal documents whose lifecycle does not parse:\n{}",
        failures.join("\n")
    );
}

#[cfg(unix)]
#[test]
fn shipped_implement_plan_reads_persisted_handoff_before_provider_launch() {
    let fixture = CliProcessFixture::named("shipped-implement-handoff");
    fixture.initialize_repository();
    fixture.seed_user_config();

    let root = workspace_root();
    let prompts = fixture.cwd().join("prompts");
    let target = prompts.join("_implement/implement-plan.md");
    let source_prompts = root.join("prompts");
    let mut shipped = Vec::new();
    collect_markdown_files(&source_prompts, &mut shipped);
    for source in shipped {
        let relative = source.strip_prefix(&source_prompts).unwrap();
        let destination = prompts.join(relative);
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        std::fs::copy(source, &destination).unwrap();
    }

    let case = fixture.cwd().join("fixes/case");
    std::fs::create_dir_all(&case).unwrap();
    let spec = case.join("spec.md");
    let plan = case.join("plan.md");
    let log = case.join("implementation-log.md");
    write(&spec, "---\nimplemented: false\n---\n# Spec\n");
    write(&plan, "---\ntotal_phases: 2\nphase: 2\n---\n# Plan\n");
    let original_log = "---\nmessage_to_agent: PRIOR-HANDOFF\n---\n# Implementation Log\n";
    write(&log, original_log);

    let capture = fixture.cwd().join("delivered-prompt.txt");
    write_executable(
        &fixture.bin_dir().join("goose"),
        r#"#!/bin/sh
{
  printf '%s\n' "$@"
  cat
} > "$CLAUDINE_PROMPT_CAPTURE"
exit 17
"#,
    );
    let original_prompt = std::fs::read_to_string(&target).unwrap();
    assert!(
        original_prompt.contains("frontmatter(log, 'message_to_agent')"),
        "the regression must execute the shipped persisted handoff reader"
    );

    let output = fixture
        .command()
        .env("CLAUDINE_PROMPT_CAPTURE", &capture)
        .arg("compose")
        .arg(&target)
        .arg(format!("plan={}", plan.display()))
        .arg("phase=2")
        .args(["--goose", "-y"])
        .output()
        .unwrap();
    let rendered = common::strip_ansi(&format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    ));

    assert!(!output.status.success(), "the fake provider must fail:\n{rendered}");
    let delivered = std::fs::read_to_string(&capture)
        .unwrap_or_else(|_| panic!("the provider must launch after initialize:\n{rendered}"));
    assert!(delivered.contains("# Implement Phase 2 of 2"), "{delivered}");
    assert!(delivered.contains("PRIOR-HANDOFF"), "{delivered}");
    for rejected in [
        "object value not allowed here",
        "set: {property: value}",
        "unknown root `message_to_agent`",
    ] {
        assert!(
            !rendered.contains(rejected),
            "the shipped handoff failed before provider launch ({rejected}):\n{rendered}"
        );
    }
    assert_eq!(std::fs::read_to_string(&target).unwrap(), original_prompt);
    assert_eq!(std::fs::read_to_string(&log).unwrap(), original_log);
    assert!(!fixture.audio_spool().exists(), "no audio may be published");
}

#[cfg(unix)]
#[test]
fn shipped_implement_router_runs_real_proxy_handoff() {
    let fixture = CliProcessFixture::named("shipped-prompts-router");
    // The router prompt is resolved from its own repository, so the launch
    // directory is a second, separate repository inside the same workspace.
    let prompt_fixture = fixture.cwd().join("prompt-repo");
    let feature = fixture.cwd().join("features/2026-07-20-router-fixture");
    let review = feature.join("review.md");
    write(&review, "---\nimplemented: false\n---\n# Router fixture\n");

    let capture = fixture.cwd().join("delivered-prompt.txt");
    write_executable(
        &fixture.bin_dir().join("claude"),
        r#"#!/bin/sh
{
  for arg in "$@"; do
    if [ -f "$arg" ]; then cat "$arg"; else printf '%s\n' "$arg"; fi
  done
  cat
} >> "$CLAUDINE_PROMPT_CAPTURE" 2>/dev/null
exit 0
"#,
    );

    let root = workspace_root();
    let prompt_dir = prompt_fixture.join("prompts");
    let target_dir = prompt_dir.join("_implement");
    std::fs::create_dir_all(&target_dir).expect("prompt fixture directory");
    // The *shipped* router is the artifact under test; only its repository is
    // isolated, so its relative references still resolve as authored.
    std::fs::copy(root.join("prompts/implement.md"), prompt_dir.join("implement.md"))
        .expect("copy shipped router prompt");
    write(
        &target_dir.join("implement-review.md"),
        "---\ntitle: Router target fixture\n---\n# Implementation of Review Findings\n",
    );
    assert!(common::init_git_repo(&prompt_fixture));

    let router = prompt_dir.join("implement.md");
    let review_arg = format!("review={}", review.display());
    fixture
        .command_builder()
        // The router resolves its target relative to the repository it is
        // launched from, which is the prompt repository staged above.
        .ambient_context(&prompt_fixture)
        .build()
        .env("CLAUDINE_PROMPT_CAPTURE", &capture)
        .args([
            "compose",
            "--claude",
            router.to_str().expect("UTF-8 router path"),
            &review_arg,
        ])
        .assert()
        .success();

    let delivered = std::fs::read_to_string(&capture).expect("captured provider prompt");
    assert!(
        delivered.contains("Implementation of Review Findings"),
        "the shipped router must deliver the resolved target prompt:\n{delivered}"
    );
    assert!(
        !delivered.contains("This prompt should never be reached"),
        "the router body must not reach the provider after proxy handoff:\n{delivered}"
    );
}

/// The shipped router, verbatim, reads its optional `plan`/`review` inputs
/// without a fallback guard. With only `spec` supplied (already implemented,
/// no pending review) every route is false and the run must reach the
/// router's own authored `error`, not a lifecycle evaluation failure.
#[test]
fn shipped_implement_router_reads_absent_optional_inputs_unguarded() {
    const ROUTER: &str = include_str!("../../../../prompts/implement.md");
    for guarded in ["plan ? '- a plan file was identified: ' + plan : ''", "when: \"review\""] {
        assert!(ROUTER.contains(guarded), "the shipped router must read `{guarded}` unguarded");
    }
    for input in ["spec", "plan", "review"] {
        let guard = format!("{input} || false");
        assert!(!ROUTER.contains(&guard), "the shipped router must not guard `{input}` with `{guard}`");
    }

    let fixture = CliProcessFixture::named("shipped-router-absent-inputs");
    write(&fixture.cwd().join("prompts/implement.md"), ROUTER);
    let spec = fixture.cwd().join("fixes/2026-09-17-router-fixture/spec.md");
    write(&spec, "---\nimplemented: true\n---\n# Router fixture spec\n");

    let launched = fixture.cwd().join("provider-launched");
    #[cfg(unix)]
    write_executable(
        &fixture.bin_dir().join("claude"),
        &format!("#!/bin/sh\ntouch '{}'\nexit 0\n", launched.display()),
    );

    let output = fixture
        .command()
        .args([
            "compose",
            "prompts/implement.md",
            "spec=fixes/2026-09-17-router-fixture/spec.md",
            "-y",
            "--claude",
        ])
        .output()
        .expect("run claudine compose");
    let rendered = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(!output.status.success(), "the authored routing error must fail the run:\n{rendered}");
    assert!(
        rendered.contains("Unable to _route_ the implementation")
            || rendered.contains("Unable to route the implementation"),
        "the run must end at the router's authored error:\n{rendered}"
    );
    assert!(
        rendered.contains("a specification file was identified:"),
        "the spec line must render:\n{rendered}"
    );
    for absent in ["a plan file was identified", "a review file was identified", "no spec, plan, or review"] {
        assert!(!rendered.contains(absent), "an absent input rendered `{absent}`:\n{rendered}");
    }
    for rejected in [
        "unknown root",
        "undefined variable",
        "Undefined variable",
        "|| false",
        "fallback",
        "LifecycleEvaluation",
        "lifecycle evaluation",
    ] {
        assert!(!rendered.contains(rejected), "unexpected `{rejected}` in output:\n{rendered}");
    }
    assert!(!launched.exists(), "no provider may start:\n{rendered}");
    assert!(!fixture.audio_spool().exists(), "no audio may be published");
}

/// Copies the shipped prompt corpus into the fixture and stages a plan case
/// at `fixes/case/` with the given phase position.
#[cfg(unix)]
fn stage_implement_plan_case(fixture: &CliProcessFixture, phase: u32, total_phases: u32) -> (PathBuf, PathBuf, PathBuf) {
    let source_prompts = workspace_root().join("prompts");
    let prompts = fixture.cwd().join("prompts");
    let mut shipped = Vec::new();
    collect_markdown_files(&source_prompts, &mut shipped);
    for source in shipped {
        let destination = prompts.join(source.strip_prefix(&source_prompts).unwrap());
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        std::fs::copy(&source, &destination).unwrap();
    }

    let case = fixture.cwd().join("fixes/case");
    std::fs::create_dir_all(&case).unwrap();
    let spec = case.join("spec.md");
    let plan = case.join("plan.md");
    write(&spec, "---\nimplemented: false\n---\n# Spec\n");
    write(&plan, &format!("---\ntotal_phases: {total_phases}\nphase: {phase}\n---\n# Plan\n"));
    (prompts.join("_implement/implement-plan.md"), plan, spec)
}

#[cfg(unix)]
fn run_implement_plan(fixture: &CliProcessFixture, target: &Path, plan: &Path, phase: u32) -> (std::process::Output, String) {
    let output = fixture
        .command()
        .arg("compose")
        .arg(target)
        .arg(format!("plan={}", plan.display()))
        .arg(format!("phase={phase}"))
        .args(["--goose", "-y"])
        .output()
        .unwrap();
    let rendered = common::strip_ansi(&format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    ));
    (output, rendered)
}

/// Every prompt that ends by writing completion frontmatter transcludes the
/// rule that a non-interactive agent must not end its turn while background
/// work is pending: ending the turn exits the session, which is how final
/// phases lost their `implemented` flags.
#[test]
fn shipped_implement_prompts_transclude_finish_before_exit() {
    let prompts = workspace_root().join("prompts");
    let snippet = std::fs::read_to_string(prompts.join("_finish-before-exit.md")).expect("shipped snippet");
    assert!(snippet.contains("never end your turn with work pending"), "{snippet}");
    for prompt in [
        "implement-plan.md",
        "implement-review-findings-plan.md",
        "implement-suggestions.md",
        "implement-review.md",
    ] {
        let body = std::fs::read_to_string(prompts.join("_implement").join(prompt)).unwrap();
        assert!(
            body.contains("::file \"../_finish-before-exit.md\""),
            "{prompt} must transclude the finish-before-exit rule"
        );
    }
}

/// Only the final phase asks the agent to set `implemented: true` on the plan,
/// log and spec.
#[cfg(unix)]
#[test]
fn shipped_implement_plan_final_phase_delivers_completion_flags() {
    for (phase, is_final) in [(2, true), (1, false)] {
        let fixture = CliProcessFixture::named(&format!("shipped-implement-final-phase-{phase}"));
        fixture.initialize_repository();
        fixture.seed_user_config();
        let (target, plan, spec) = stage_implement_plan_case(&fixture, phase, 2);
        let capture = fixture.cwd().join("delivered-prompt.txt");
        write_executable(
            &fixture.bin_dir().join("goose"),
            &format!("#!/bin/sh\n{{ printf '%s\\n' \"$@\"; cat; }} > '{}'\nexit 17\n", capture.display()),
        );

        let (_, rendered) = run_implement_plan(&fixture, &target, &plan, phase);
        let delivered = std::fs::read_to_string(&capture)
            .unwrap_or_else(|_| panic!("the provider must launch for phase {phase}:\n{rendered}"));

        assert!(delivered.contains(&format!("# Implement Phase {phase} of 2")), "{delivered}");
        assert_eq!(
            delivered.contains("### Final Phase: Mark the Implementation Complete"),
            is_final,
            "phase {phase}:\n{delivered}"
        );
        if is_final {
            assert!(spec.ends_with("fixes/case/spec.md"));
            assert!(delivered.contains("on the spec (fixes/case/spec.md)"), "the spec must be named:\n{delivered}");
            assert!(delivered.contains("`completed_phase`: `2`"), "{delivered}");
            assert_eq!(delivered.matches("`implemented`: `true`").count(), 2, "plan/log and spec:\n{delivered}");
        } else {
            assert!(!delivered.contains("`implemented`: `true`"), "a non-final phase must not mark completion:\n{delivered}");
        }
    }
}

/// A final phase whose agent exits cleanly without writing the completion
/// flags is surfaced by the success stack; one that wrote them is not.
#[cfg(unix)]
#[test]
fn shipped_implement_plan_warns_when_final_phase_exits_without_flags() {
    const WARNING: &str = "finished without setting `implemented: true`";
    for writes_flags in [false, true] {
        let fixture = CliProcessFixture::named(&format!("shipped-implement-missing-flags-{writes_flags}"));
        fixture.initialize_repository();
        fixture.seed_user_config();
        let (target, plan, spec) = stage_implement_plan_case(&fixture, 2, 2);
        let script = if writes_flags {
            format!(
                "#!/bin/sh\ncat > /dev/null\nprintf -- '---\\ntotal_phases: 2\\nphase: 2\\nimplemented: true\\n---\\n# Plan\\n' > '{}'\nprintf -- '---\\nimplemented: true\\n---\\n# Spec\\n' > '{}'\nexit 0\n",
                plan.display(),
                spec.display()
            )
        } else {
            "#!/bin/sh\ncat > /dev/null\nexit 0\n".to_string()
        };
        write_executable(&fixture.bin_dir().join("goose"), &script);

        let (_, rendered) = run_implement_plan(&fixture, &target, &plan, 2);

        assert!(rendered.contains("phase **2**") || rendered.contains("phase 2"), "the phase must run:\n{rendered}");
        assert_eq!(
            rendered.contains(WARNING),
            !writes_flags,
            "writes_flags={writes_flags}:\n{rendered}"
        );
    }
}

/// The shipped sequence stops on its first failed prompt without an authored
/// failure policy. Stubs replace its work prompts, keeping the sequence itself
/// byte-identical while making every downstream side effect observable.
#[test]
fn shipped_review_loop_stops_after_its_first_failed_prompt() {
    let fixture = CliProcessFixture::named("shipped-review-loop-failure");
    fixture.initialize_repository();
    fixture.seed_user_config();
    common::write_dry_run_provider_stub(fixture.bin_dir(), "claude");
    common::write_dry_run_provider_stub(fixture.bin_dir(), "codex");
    let target = fixture.cwd().join("prompts/review-loop.md");
    write(&target, include_str!("../../../../prompts/review-loop.md"));
    write(&fixture.cwd().join("spec.md"), "---\nimplemented: false\n---\n# Spec\n");
    write(
        &fixture.cwd().join("prompts/_implement/implement-plan.md"),
        "---\ninitialize:\n  - error: p8-failed\n---\nMust not launch.\n",
    );
    for path in [
        "prompts/_reviews/feature-review.md",
        "prompts/_implement/implement-suggestions.md",
        "prompts/commit.md",
    ] {
        write(
            &fixture.cwd().join(path),
            "---\ninitialize:\n  - append_line: [later-prompt.txt, admitted]\n---\nMust not launch.\n",
        );
    }
    let output = fixture.command()
        .arg("sequence").arg(&target).arg("spec=spec.md").args(["--yolo", "--claude"])
        .output().expect("sequence runs");
    let rendered = common::strip_ansi(&format!("{}{}",
        String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr)));
    assert!(!output.status.success(), "{rendered}");
    assert!(rendered.contains("p8-failed"), "{rendered}");
    assert!(!rendered.contains("removed"), "{rendered}");
    assert!(!fixture.cwd().join("later-prompt.txt").exists(), "a later prompt was admitted: {rendered}");
    assert!(!fixture.cwd().join(".git/index").exists(), "a later git-add task ran: {rendered}");
    assert!(!fixture.audio_spool().exists(), "no audio may be published");
}
