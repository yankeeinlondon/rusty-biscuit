//! Contract tests for the prompt documents shipped with Claudine.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::{ExpressionFinder, parse_condition};
use darkmatter::markdown::schemas::{DarkmatterSchemas, SchemaPhase};
use ignore::WalkBuilder;
use serde_json::Value;
use std::path::{Path, PathBuf};

mod common;

fn repository_root() -> PathBuf {
    biscuit_test_harness::manifest_dir!()
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn shipped_prompt_paths() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = WalkBuilder::new(repository_root().join("prompts"))
        .hidden(false)
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
        .map(|entry| entry.into_path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
        .collect();
    paths.sort();
    paths
}

/// Copy the shipped prompt corpus into `destination`, byte-for-byte and with
/// its relative layout intact.
///
/// Composition anchors repository discovery on the *document's* directory, so a
/// test that composes the monorepo copy in place pays for a full rusty-biscuit
/// topology walk however isolated its workspace is. Relative `::file` spans and
/// `@` references resolve against that same layout, so the corpus has to move
/// as a tree rather than as one file.
fn copy_shipped_prompts(destination: &Path) {
    let source_root = repository_root().join("prompts");
    let sources = shipped_prompt_paths();
    assert!(
        !sources.is_empty(),
        "the shipped prompt corpus must not be empty"
    );
    for source in &sources {
        let relative = source
            .strip_prefix(&source_root)
            .expect("shipped prompt paths are rooted at the corpus directory");
        let target = destination.join(relative);
        std::fs::create_dir_all(target.parent().expect("a copied prompt has a parent")).unwrap();
        std::fs::copy(source, &target).unwrap();
    }
}

fn record_expression_errors(
    prompt: &Path,
    surface: &str,
    text: &str,
    skip_code_blocks: bool,
    errors: &mut Vec<String>,
) {
    let expressions = if skip_code_blocks {
        ExpressionFinder::new(text).find_all()
    } else {
        ExpressionFinder::find_all_plain(text)
    };
    for expression in expressions {
        if let Err(error) = parse_condition(&expression.expression) {
            errors.push(format!(
                "{} [{surface}] `{{{{ {} }}}}`: {error}",
                prompt.display(),
                expression.expression
            ));
        }
    }
}

fn inspect_frontmatter_value(
    prompt: &Path,
    property: &str,
    value: &Value,
    errors: &mut Vec<String>,
) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let property = if property.is_empty() {
                    key.clone()
                } else {
                    format!("{property}.{key}")
                };
                inspect_frontmatter_value(prompt, &property, child, errors);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                inspect_frontmatter_value(
                    prompt,
                    &format!("{property}[{index}]"),
                    child,
                    errors,
                );
            }
        }
        Value::String(text) => {
            record_expression_errors(prompt, property, text, false, errors);
            let leaf = property.rsplit('.').next().unwrap_or(property);
            if matches!(leaf, "when" | "until" | "while")
                && !text.contains("{{")
                && let Err(error) = parse_condition(text)
            {
                errors.push(format!(
                    "{} [{property}] `{text}`: {error}",
                    prompt.display()
                ));
            }
        }
        _ => {}
    }
}

fn inspect_block_conditions(prompt: &Path, body: &str, errors: &mut Vec<String>) {
    for (index, line) in body.lines().enumerate() {
        let line = line.trim();
        if !line.starts_with("::block ") {
            continue;
        }
        let Some(after_when) = line.split_once("when=\"").map(|(_, value)| value) else {
            continue;
        };
        let Some(condition) = after_when.strip_suffix('"') else {
            errors.push(format!(
                "{} [body line {}] malformed block condition: {line}",
                prompt.display(),
                index + 1
            ));
            continue;
        };
        if condition.contains("{{") {
            continue;
        }
        if let Err(error) = parse_condition(condition) {
            errors.push(format!(
                "{} [body line {}] `{condition}`: {error}",
                prompt.display(),
                index + 1
            ));
        }
    }
}

#[test]
fn shipped_prompts_have_parseable_schemas_and_expressions() {
    let paths = shipped_prompt_paths();
    assert!(!paths.is_empty(), "the shipped prompt corpus must not be empty");

    let mut errors = Vec::new();
    for prompt in paths {
        let markdown = match Markdown::try_from(prompt.as_path()) {
            Ok(markdown) => markdown,
            Err(error) => {
                errors.push(format!("{} [markdown]: {error}", prompt.display()));
                continue;
            }
        };
        if let Err(error) = DarkmatterSchemas::new().effective_for(&markdown) {
            errors.push(format!("{} [$schema]: {error}", prompt.display()));
        }

        for (property, value) in markdown.frontmatter().as_map() {
            inspect_frontmatter_value(&prompt, property, value, &mut errors);
        }
        record_expression_errors(&prompt, "body", markdown.content(), true, &mut errors);
        inspect_block_conditions(&prompt, markdown.content(), &mut errors);
    }

    assert!(
        errors.is_empty(),
        "shipped prompt contract failures:\n{}",
        errors.join("\n")
    );
}

/// Every shipped schema must successfully construct both phase projections.
#[test]
fn shipped_prompt_schemas_project_at_both_phases() {
    let mut failures = Vec::new();
    for prompt in shipped_prompt_paths() {
        let Ok(markdown) = Markdown::try_from(prompt.as_path()) else {
            continue;
        };
        let Ok(Some(effective)) = DarkmatterSchemas::new().effective_for(&markdown) else {
            continue;
        };
        let empty = Value::Object(serde_json::Map::new());
        for phase in [SchemaPhase::Launch, SchemaPhase::Completion] {
            if let Err(error) = effective.validate_for_phase(&empty, phase) {
                failures.push(format!("{} ({phase:?}): {error}", prompt.display()));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "shipped prompt phase-projection failures:\n{}",
        failures.join("\n")
    );
}

/// End-to-end through the real shipped artifact and the normal invocation
/// path: `implement-plan.md` derives `spec` from a sibling `spec.md` and
/// deliberately yields `null` when there is none. While `spec` was declared
/// `eager` that combination was a launch requirement the document could not
/// satisfy, and the run failed with ``spec — null is not of type "string"``.
///
/// `--dry-run` stops after provider resolution, so the shipped document's
/// `say:`/`effect:`/`shell:` lifecycle effects never run. The completion half
/// of the same document is covered end-to-end by
/// `level2_shipped_implement_plan_*`, whose fixture also stages no sibling
/// spec.
#[cfg(unix)]
#[test]
fn shipped_implement_plan_launches_without_a_sibling_spec() {
    use std::fs;

    use common::CliProcessFixture;
    use common::{strip_ansi, write_executable};

    let fixture = CliProcessFixture::named("implement-plan-no-spec");
    fixture.initialize_repository();
    fixture.seed_user_config();
    let prompts = fixture.cwd().join("prompts");
    copy_shipped_prompts(&prompts);
    let implement_plan = fs::read_to_string(prompts.join("_implement/implement-plan.md")).unwrap();
    assert!(
        implement_plan.contains("epilog: null"),
        "the normal invocation must exercise the shipped mapping-only lifecycle action"
    );
    let feature_dir = fixture.home().join("features/no-spec-here");
    fs::create_dir_all(&feature_dir).unwrap();
    write_executable(&fixture.bin_dir().join("codex"), "#!/bin/sh\nexit 0\n");

    let plan = feature_dir.join("plan.md");
    fs::write(&plan, "---\ntotal_phases: 3\nphase: 1\n---\n# Plan\n").unwrap();
    assert!(
        !feature_dir.join("spec.md").exists(),
        "the regression needs a plan with no sibling spec"
    );

    let assert = fixture
        .command()
        .arg("compose")
        .arg(prompts.join("_implement/implement-plan.md"))
        .arg(format!("plan={}", plan.display()))
        .args(["--dry-run", "-y", "--codex"])
        .assert()
        .success();

    let output = assert.get_output();
    let rendered = strip_ansi(&format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ));
    assert!(
        rendered.contains("spec: null"),
        "the run must actually reach the null branch this regression is about: {rendered}"
    );
    for marker in ["is not of type", "schema validation", "MissingProperties"] {
        assert!(
            !rendered.contains(marker),
            "an absent optional spec must not be a launch problem ({marker}): {rendered}"
        );
    }
}

/// The copy the CLI contract test composes must be the shipped corpus, not a
/// subset of it: a relative `::file` span that silently lost its target would
/// otherwise turn into a composition error the test reports as a product bug.
#[test]
fn copied_prompt_corpus_matches_the_shipped_tree() {
    let workspace = common::TestWorkspace::named("prompt-corpus-copy");
    let copied_root = workspace.path().join("prompts");
    copy_shipped_prompts(&copied_root);

    let source_root = repository_root().join("prompts");
    let sources = shipped_prompt_paths();
    let copied = markdown_files_under(&copied_root);

    let source_layout: Vec<PathBuf> = sources
        .iter()
        .map(|path| path.strip_prefix(&source_root).unwrap().to_path_buf())
        .collect();
    let copied_layout: Vec<PathBuf> = copied
        .iter()
        .map(|path| path.strip_prefix(&copied_root).unwrap().to_path_buf())
        .collect();
    assert_eq!(
        copied_layout, source_layout,
        "the copied corpus must hold every shipped prompt at its shipped relative path"
    );

    for (source, target) in sources.iter().zip(copied.iter()) {
        assert_eq!(
            std::fs::read(source).unwrap(),
            std::fs::read(target).unwrap(),
            "{} was not copied byte-for-byte",
            source.display()
        );
    }
}

fn markdown_files_under(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "md") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

#[cfg(unix)]
#[test]
fn feature_review_cli_preserves_numeric_iteration_and_dependent_paths() {
    use std::fs;

    use common::CliProcessFixture;
    use common::write_executable;

    let fixture = CliProcessFixture::named("feature-review-contract");
    // The shipped document composes from a copy of the corpus inside the
    // fixture's own repository. Composed in place it would anchor repository
    // discovery on the rusty-biscuit checkout and walk the whole workspace;
    // the copy keeps the shipped bytes under test and moves only their
    // location, so `::file ../_senior-reviewer.md` still resolves one level up.
    fixture.initialize_repository();
    let prompts = fixture.cwd().join("prompts");
    copy_shipped_prompts(&prompts);
    // The feature lives under the fixture HOME: `review:` is derived with
    // `dirname(spec)`, whose projection ladder (repo root → base dir → `~/`)
    // only yields a re-resolvable spelling for the `~/` arm here — the fixture
    // repository holds the prompt corpus, not the feature, so neither the repo
    // root nor the base dir contains the fixture's spec.
    let feature_dir = fixture.home().join("features/example");
    let spec = feature_dir.join("spec.md");
    let review = feature_dir.join("review-3.md");
    let captured_prompt = fixture.cwd().join("stdin.txt");
    fs::create_dir_all(&feature_dir).unwrap();
    fixture.seed_user_config();
    fs::write(&spec, "---\nreview_iterations: '2'\n---\n# Example\n").unwrap();
    write_executable(
        &fixture.bin_dir().join("codex"),
        "#!/bin/sh\n/bin/cat > \"$CLAUDINE_STDIN_FILE\"\nprintf '%s\\n' '---' 'ready: true' '---' > \"$CLAUDINE_REVIEW_FILE\"\nexit 0\n",
    );

    let prompt = prompts.join("_reviews/feature-review.md");
    fixture
        .command()
        .env("CLAUDINE_STDIN_FILE", &captured_prompt)
        .env("CLAUDINE_REVIEW_FILE", &review)
        .arg("compose")
        .arg(&prompt)
        .arg(format!("spec={}", spec.display()))
        .args(["-y", "--codex"])
        .assert()
        .success();
    // `ready: true` from the stub selects the shipped success branch, whose
    // `effect: small-group-cheer` is real playback; the fixture default dry-run
    // must be what kept it silent, so the spool it would have used is checked.
    assert!(
        !fixture.audio_spool().exists(),
        "the shipped review lifecycle must not publish audio from a test"
    );

    let composed = fs::read_to_string(captured_prompt).unwrap();
    assert!(
        composed.contains("Review Iteration: #3"),
        "quoted review_iterations must produce numeric iteration 3: {composed}"
    );
    assert!(
        composed.contains("review-3.md"),
        "the output review path must incorporate iteration 3: {composed}"
    );
    assert!(
        composed.contains("review-2.md"),
        "the previous-review instructions must target iteration 2: {composed}"
    );
    let senior_reviewer = fs::read_to_string(prompts.join("_senior-reviewer.md")).unwrap();
    let senior_reviewer_opening = senior_reviewer
        .lines()
        .next()
        .expect("the shipped senior-reviewer prompt is not empty");
    assert!(
        composed.contains(senior_reviewer_opening),
        "the relative `::file ../_senior-reviewer.md` span must resolve against the copied \
         corpus: {composed}"
    );
    assert!(
        !composed.contains("decrement_file_index(review)"),
        "the helper call must be evaluated rather than delivered as literal text: {composed}"
    );
}

/// The first nested-span-in-literal defect in `markdown`'s lifecycle, found by
/// the same validator shared preparation runs before any provider starts.
fn nested_span_defect(path: &Path, markdown: &Markdown) -> Option<String> {
    use claudine::composition::lifecycle::{
        parse_lifecycle_config, validate_no_nested_spans_in_literals,
    };

    let frontmatter = Value::Object(
        markdown
            .frontmatter()
            .as_map()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    );
    // A lifecycle block that does not parse is refused by preparation with its
    // own error; this rule has nothing to inspect there.
    let lifecycle = parse_lifecycle_config(&frontmatter, path).ok()?;
    validate_no_nested_spans_in_literals(&frontmatter, &lifecycle, path)
        .err()
        .map(|error| format!("{}: {error}", path.display()))
}

/// Passive corpus check (spec acceptance 3): no shipped prompt carries a
/// lifecycle that shared preparation would refuse, which covers both branches
/// of every ternary without composing anything. The pre-fix incident is the
/// negative control that keeps the walk from passing vacuously.
#[test]
fn shipped_prompt_lifecycles_have_no_nested_spans_in_literals() {
    let incident = Markdown::from(include_str!(
        "fixtures/nested_span_regression/review-spec-inline.md"
    ));
    let control = nested_span_defect(Path::new("review-spec-inline.md"), &incident)
        .expect("the pre-fix incident must be refused");
    assert!(control.contains("success.say"), "{control}");

    let paths = shipped_prompt_paths();
    assert!(!paths.is_empty(), "the shipped prompt corpus must not be empty");
    let defects: Vec<String> = paths
        .iter()
        .filter_map(|prompt| {
            let markdown = Markdown::try_from(prompt.as_path()).ok()?;
            nested_span_defect(prompt, &markdown)
        })
        .collect();
    assert!(
        defects.is_empty(),
        "shipped prompts refused by lifecycle validation:\n{}",
        defects.join("\n")
    );
}

/// Run the shipped `_reviews/review-spec-inline.md` through `compose` with a
/// `codex` stub exiting `exit_code`, returning `(success, rendered output,
/// prompt the provider received)`.
#[cfg(unix)]
fn compose_shipped_review_spec_inline(exit_code: i32) -> (bool, String, String) {
    use common::{CliProcessFixture, strip_ansi, write_executable};

    let fixture = CliProcessFixture::named(&format!("shipped-review-spec-inline-{exit_code}"));
    fixture.initialize_repository();
    fixture.seed_user_config();
    // Composed from a copy for the same reason as the feature-review contract:
    // in place it would anchor repository discovery on the rusty-biscuit
    // checkout, and `::file _writing-clearly.md` must still resolve.
    let prompts = fixture.cwd().join("prompts");
    copy_shipped_prompts(&prompts);
    let spec = fixture.home().join("features/2026-09-16-example/spec.md");
    std::fs::create_dir_all(spec.parent().unwrap()).unwrap();
    std::fs::write(&spec, "---\ncreated: 2026-09-16\n---\n# Example\n").unwrap();
    write_executable(
        &fixture.bin_dir().join("codex"),
        &format!("#!/bin/sh\n/bin/cat > \"$CLAUDINE_STDIN_FILE\"\nexit {exit_code}\n"),
    );
    let delivered = fixture.cwd().join("stdin.txt");

    let output = fixture
        .command()
        .env("CLAUDINE_STDIN_FILE", &delivered)
        .arg("compose")
        .arg(prompts.join("_reviews/review-spec-inline.md"))
        .arg(format!("spec={}", spec.display()))
        .args(["-y", "--codex"])
        .output()
        .unwrap();
    // `say` is real speech; the fixture's child-local dry-run must have kept
    // it off the spool.
    assert!(
        !fixture.audio_spool().exists(),
        "the shipped review lifecycle must not publish audio from a test"
    );
    let rendered = strip_ansi(&format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ));
    // Only a started provider writes the capture, so its presence separates a
    // lifecycle outcome from a run that failed before launch.
    let delivered = std::fs::read_to_string(&delivered).unwrap_or_else(|_| {
        panic!("the provider must have started; output was:\n{rendered}")
    });
    (output.status.success(), rendered, delivered)
}

/// Assert no lifecycle validation or evaluation error reached the output.
#[cfg(unix)]
fn assert_clean_lifecycle(rendered: &str) {
    for marker in [
        "lifecycle evaluation error",
        "nested interpolation inside a string literal",
        "after every interpolation pass",
    ] {
        assert!(
            !rendered.contains(marker),
            "the shipped lifecycle must fire cleanly ({marker}):\n{rendered}"
        );
    }
}

/// End-to-end over the real shipped artifact (spec acceptance 3): the repaired
/// `review-spec-inline.md` fires `success` with its `say` and `info` resolved.
/// A `say` that still carried a span would fail the event closed through the
/// runtime surviving-span guard, so a clean exit is the spoken-text proof.
#[cfg(unix)]
#[test]
fn shipped_review_spec_inline_fires_success_cleanly() {
    let (success, rendered, delivered) = compose_shipped_review_spec_inline(0);
    assert!(!rendered.contains("{{"), "no raw span may be written:\n{rendered}");
    assert!(success, "a successful review must exit zero:\n{rendered}");
    assert!(
        delivered.contains("2026-09-16-example/spec.md"),
        "the provider must receive the composed review prompt:\n{delivered}"
    );
    let flowed = rendered.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flowed.contains("2026-09-16-example/spec.md specification has been reviewed and updated inline"),
        "the `success.info` line must be written with its link resolved:\n{rendered}"
    );
    assert_clean_lifecycle(&rendered);
}

/// The `failure` half: a failed review reports the provider failure, not a
/// lifecycle crash that replaces it (the incident's secondary hazard).
#[cfg(unix)]
#[test]
fn shipped_review_spec_inline_fires_failure_cleanly() {
    let (success, rendered, _) = compose_shipped_review_spec_inline(1);
    assert!(!rendered.contains("{{"), "no raw span may be written:\n{rendered}");
    assert!(!success, "a failed review must exit non-zero:\n{rendered}");
    assert_clean_lifecycle(&rendered);
}

/// End-to-end over the shipped `commit.md` (spec acceptance 3): its repaired
/// `resides_in` whole value composes into the delivered prompt with no raw
/// span, and the `success` lifecycle fires cleanly after the provider exits.
#[cfg(unix)]
#[test]
fn shipped_commit_prompt_composes_resides_in_and_fires_success_cleanly() {
    use common::{CliProcessFixture, strip_ansi, write, write_executable};

    let fixture = CliProcessFixture::named("shipped-commit");
    fixture.initialize_repository();
    fixture.seed_user_config();
    let prompts = fixture.cwd().join("prompts");
    copy_shipped_prompts(&prompts);
    write(
        &fixture.cwd().join(".claudine/memory/commits.md"),
        "# Commit lessons\n",
    );
    write(&fixture.cwd().join("staged.txt"), "staged\n");
    let staged = common::helper_command("git")
        .arg("-C")
        .arg(fixture.cwd())
        .args(["add", "staged.txt"])
        .status()
        .unwrap();
    assert!(staged.success(), "the fixture needs one staged file");

    let delivered = fixture.cwd().join("delivered.txt");
    write_executable(
        &fixture.bin_dir().join("opencode"),
        "#!/bin/sh\nprintf '%s\\n' \"$@\" >> \"$CLAUDINE_PROMPT_CAPTURE\"\n/bin/cat >> \"$CLAUDINE_PROMPT_CAPTURE\"\nexit 0\n",
    );
    let output = fixture
        .command()
        .env("CLAUDINE_PROMPT_CAPTURE", &delivered)
        .arg("compose")
        .arg(prompts.join("commit.md"))
        .arg("-y")
        .output()
        .unwrap();
    let rendered = strip_ansi(&format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ));
    assert!(output.status.success(), "the commit prompt must succeed:\n{rendered}");
    assert!(!fixture.audio_spool().exists(), "no audio may be published");

    let delivered = std::fs::read_to_string(&delivered)
        .unwrap_or_else(|_| panic!("the provider must have started:\n{rendered}"));
    let flowed = delivered.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flowed.contains("There are 1 staged files to commit which are"),
        "`resides_in` must compose into the delivered prompt:\n{delivered}"
    );
    assert!(!delivered.contains("{{"), "no raw span may reach the provider:\n{delivered}");
    assert_clean_lifecycle(&rendered);
}

/// Composes `implement-plan.md` once through the normal CLI path and returns
/// the prompt the provider received.
///
/// The shipped document's success stack runs `git`/`just`/`gitnexus` and its
/// lifecycle speaks, so this drives the side-effect-free Level 2 copy, whose
/// body `shipped_prompt_route_drift` pins byte-identical to the shipped body.
#[cfg(unix)]
fn deliver_implement_plan_prompt(
    fixture: &common::CliProcessFixture,
    prompts: &Path,
    case: &Path,
    phase: u32,
) -> String {
    use common::{strip_ansi, write, write_executable};

    write(
        &prompts.join("_implement/implement-plan.md"),
        include_str!("fixtures/shipped_implement_route/_implement/implement-plan.md"),
    );
    write(&case.join("spec.md"), "---\nstatus: draft\n---\n# Spec\n");
    write(
        &case.join("plan.md"),
        &format!("---\ntotal_phases: {phase}\nphase: {phase}\n---\n# Plan\n"),
    );
    let delivered = fixture.cwd().join("delivered.txt");
    let _ = std::fs::remove_file(&delivered);
    write_executable(
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$CLAUDINE_PROMPT_CAPTURE\"\nexit 0\n",
    );

    let output = fixture
        .command()
        .env("CLAUDINE_PROMPT_CAPTURE", &delivered)
        .arg("compose")
        .arg(prompts.join(if phase == 3 { "implement.md" } else { "_implement/implement-plan.md" }))
        .arg(format!("phase={phase}"))
        .arg(format!("spec={}", case.join("spec.md").display()))
        .args(["--goose", "-y"])
        .output()
        .unwrap();
    let rendered = strip_ansi(&format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ));
    assert!(output.status.success(), "implement-plan must succeed:\n{rendered}");
    assert!(!fixture.audio_spool().exists(), "no audio may be published");
    std::fs::read_to_string(&delivered)
        .unwrap_or_else(|_| panic!("the provider must have started:\n{rendered}"))
}

/// `initialize` runs `ensure_file: log` before the body composes, so the log
/// always exists by the time the logging blocks are evaluated. The "start the
/// log" instructions must therefore key on an *unstarted* log (missing, or
/// present with no frontmatter and a blank body), not on a missing one —
/// otherwise a fresh implementation is told the log "already exists" and never
/// learns to write its title and metadata.
#[cfg(unix)]
#[test]
fn shipped_implement_plan_logging_instructions_follow_log_content() {
    use common::CliProcessFixture;

    const UNSTARTED: &str = "the log file for this implementation has not been started yet";
    const STARTED: &str = "the log file already has content from earlier work";

    let fixture = CliProcessFixture::named("implement-plan-logging");
    fixture.initialize_repository();
    fixture.seed_user_config();
    let prompts = fixture.cwd().join("prompts");
    copy_shipped_prompts(&prompts);
    let case = fixture.cwd().join("fixes/case");
    let log = case.join("implementation-log.md");

    // Missing log: initialize creates it empty, and the prompt starts it.
    assert!(!log.exists());
    let fresh = deliver_implement_plan_prompt(&fixture, &prompts, &case, 1);
    assert_eq!(
        std::fs::read_to_string(&log).expect("initialize must create the log"),
        "",
        "ensure_file creates an empty log"
    );
    assert!(fresh.contains(UNSTARTED), "a fresh log must be started:\n{fresh}");
    assert!(fresh.contains("# Implementation Log for"), "title instructions:\n{fresh}");
    assert!(!fresh.contains(STARTED), "a fresh log has no prior content:\n{fresh}");

    // An empty or whitespace-only log left by an earlier initialize is still unstarted.
    for unstarted in ["", "\n  \n"] {
        std::fs::write(&log, unstarted).unwrap();
        let prompt = deliver_implement_plan_prompt(&fixture, &prompts, &case, 1);
        assert!(prompt.contains(UNSTARTED), "{unstarted:?} is unstarted:\n{prompt}");
        assert!(!prompt.contains(STARTED), "{unstarted:?} is unstarted:\n{prompt}");
    }

    // Frontmatter alone, or a body alone, means an earlier phase started the log;
    // ensure_file must keep those bytes and the prompt must append to them.
    for (started, phase) in [
        ("---\nmessage_to_agent: Phase 2 is complete; begin Phase 3.\n---\n# Implementation Log\n", 3),
        ("---\nspec: fixes/case/spec.md\n---\n", 2),
        ("# Implementation Log\n\n## Phase 1\n\n- did things\n", 2),
        ("---\nstarted_phase: 1\n---\n# Implementation Log\n\n## Phase 1\n", 1),
    ] {
        std::fs::write(&log, started).unwrap();
        let prompt = deliver_implement_plan_prompt(&fixture, &prompts, &case, phase);
        assert_eq!(std::fs::read_to_string(&log).unwrap(), started, "log preserved");
        assert!(prompt.contains(STARTED), "{started:?} is started:\n{prompt}");
        assert!(!prompt.contains(UNSTARTED), "{started:?} is started:\n{prompt}");
        assert!(!prompt.contains("# Implementation Log for"), "no re-title:\n{prompt}");
        assert_eq!(
            prompt.contains(&format!("we do have the log entries for {}", phase - 1)),
            phase > 1,
            "prior-phase pointer tracks the phase:\n{prompt}"
        );
    }
}
