//! Environment anchors: the `PORTABLE_ENV_VARIABLES` declaration, the value of
//! each declared variable, and which anchor wins.

use biscuit_file::{
    Attempt, AttemptOutcome, EnvAnchorProblem, Finding, NotApplicable, PORTABLE_ENV_VARIABLES,
    PortabilityPreference as P, PortablePath,
};

use super::*;

/// Every candidate of an `EnvRootedPath` attempt: the matched text, or why a
/// candidate did not apply.
fn env_outcomes(attempt: &Attempt) -> Vec<Result<String, NotApplicable>> {
    attempt
        .rejected
        .iter()
        .chain(std::iter::once(&attempt.outcome))
        .map(|outcome| match outcome {
            AttemptOutcome::Matched(reference) => Ok(reference.raw().to_string()),
            AttemptOutcome::NotApplicable(reason) => Err(reason.clone()),
            other => panic!("unexpected outcome {other}"),
        })
        .collect()
}

fn unusable(name: &str, problem: EnvAnchorProblem) -> Result<String, NotApplicable> {
    Err(NotApplicable::EnvAnchor {
        name: name.to_string(),
        problem,
    })
}

fn invalid(name: &str) -> Finding {
    Finding::InvalidPortableVariableName { name: name.to_string() }
}

/// The input robustness matrix for both configuration reads: the
/// `PORTABLE_ENV_VARIABLES` list and the value of the declared `CONFIG_DIR`.
/// One fixture, one edit per row, asserted through the public result; the
/// control row proves the unedited fixture anchors the target.
#[test]
fn the_declaration_and_value_matrix_has_one_defined_outcome_per_cell() {
    let fx = Fixture::new();
    let cwd = fx.dir("work");
    let config = fx.dir("config");
    let target = fx.file("config/x.json");
    let config_value = config.to_str().unwrap().to_string();
    let anchored = "{{CONFIG_DIR}}/x.json".to_string();
    let absolute = absolute_text(&target);
    let trailing = format!("{config_value}{}", std::path::MAIN_SEPARATOR);
    let lexical_prefix = fx.path("conf");
    #[cfg(not(windows))]
    let foreign = r"C:\config".to_string();
    #[cfg(windows)]
    let foreign = "/opt/config".to_string();

    struct Row<'a> {
        cell: &'a str,
        list: Option<&'a str>,
        value: Option<&'a str>,
        builder: &'a [&'a str],
        expect_raw: &'a str,
        expect_env: Vec<Result<String, NotApplicable>>,
        expect_findings: Vec<Finding>,
    }
    let matched = || vec![Ok(anchored.clone())];
    let none_declared = || vec![Err(NotApplicable::NoPortableVariables)];
    let rows = [
        Row {
            cell: "control",
            list: Some("CONFIG_DIR"),
            value: Some(&config_value),
            builder: &[],
            expect_raw: &anchored,
            expect_env: matched(),
            expect_findings: vec![],
        },
        // ---- PORTABLE_ENV_VARIABLES ----
        Row {
            cell: "list absent",
            list: None,
            value: Some(&config_value),
            builder: &[],
            expect_raw: &absolute,
            expect_env: none_declared(),
            expect_findings: vec![],
        },
        Row {
            cell: "list explicitly empty",
            list: Some(""),
            value: Some(&config_value),
            builder: &[],
            expect_raw: &absolute,
            expect_env: none_declared(),
            expect_findings: vec![],
        },
        Row {
            cell: "list: one bad element",
            list: Some("CONFIG_DIR,bad-name"),
            value: Some(&config_value),
            builder: &[],
            expect_raw: &anchored,
            expect_env: matched(),
            expect_findings: vec![invalid("bad-name")],
        },
        Row {
            cell: "list: every element bad",
            list: Some("bad-name,lower"),
            value: Some(&config_value),
            builder: &[],
            expect_raw: &absolute,
            expect_env: none_declared(),
            expect_findings: vec![invalid("bad-name"), invalid("lower")],
        },
        Row {
            cell: "list: empty element",
            list: Some("CONFIG_DIR,,X"),
            value: Some(&config_value),
            builder: &[],
            expect_raw: &anchored,
            expect_env: vec![unusable("X", EnvAnchorProblem::Unset), Ok(anchored.clone())],
            expect_findings: vec![],
        },
        Row {
            cell: "list: duplicate",
            list: Some("CONFIG_DIR,CONFIG_DIR"),
            value: Some(&config_value),
            builder: &["CONFIG_DIR"],
            expect_raw: &anchored,
            expect_env: matched(),
            expect_findings: vec![],
        },
        Row {
            cell: "list: surrounding whitespace",
            list: Some("  CONFIG_DIR , "),
            value: Some(&config_value),
            builder: &[],
            expect_raw: &anchored,
            expect_env: matched(),
            expect_findings: vec![],
        },
        Row {
            cell: "list: lowercase entry",
            list: Some("config_dir"),
            value: Some(&config_value),
            builder: &[],
            expect_raw: &absolute,
            expect_env: none_declared(),
            expect_findings: vec![invalid("config_dir")],
        },
        Row {
            cell: "builder name only",
            list: None,
            value: Some(&config_value),
            builder: &["CONFIG_DIR"],
            expect_raw: &anchored,
            expect_env: matched(),
            expect_findings: vec![],
        },
        // ---- value of CONFIG_DIR ----
        Row {
            cell: "value absent",
            list: Some("CONFIG_DIR"),
            value: None,
            builder: &[],
            expect_raw: &absolute,
            expect_env: vec![unusable("CONFIG_DIR", EnvAnchorProblem::Unset)],
            expect_findings: vec![],
        },
        Row {
            cell: "value empty",
            list: Some("CONFIG_DIR"),
            value: Some(""),
            builder: &[],
            expect_raw: &absolute,
            expect_env: vec![unusable("CONFIG_DIR", EnvAnchorProblem::NotAbsolute { value: String::new() })],
            expect_findings: vec![],
        },
        Row {
            cell: "value relative",
            list: Some("CONFIG_DIR"),
            value: Some("../shared"),
            builder: &[],
            expect_raw: &absolute,
            expect_env: vec![unusable(
                "CONFIG_DIR",
                EnvAnchorProblem::NotAbsolute { value: "../shared".into() },
            )],
            expect_findings: vec![],
        },
        Row {
            cell: "value absolute only on another OS",
            list: Some("CONFIG_DIR"),
            value: Some(&foreign),
            builder: &[],
            expect_raw: &absolute,
            expect_env: vec![unusable(
                "CONFIG_DIR",
                EnvAnchorProblem::ForeignAbsolute { value: foreign.clone() },
            )],
            expect_findings: vec![],
        },
        Row {
            cell: "value: lexical-only prefix",
            list: Some("CONFIG_DIR"),
            value: Some(lexical_prefix.to_str().unwrap()),
            builder: &[],
            expect_raw: &absolute,
            expect_env: vec![unusable(
                "CONFIG_DIR",
                EnvAnchorProblem::NotAPrefix { value: lexical_prefix.clone() },
            )],
            expect_findings: vec![],
        },
        Row {
            cell: "value: trailing separator",
            list: Some("CONFIG_DIR"),
            value: Some(&trailing),
            builder: &[],
            expect_raw: &anchored,
            expect_env: matched(),
            expect_findings: vec![],
        },
    ];

    for row in rows {
        let mut env = Vec::new();
        if let Some(list) = row.list {
            env.push((PORTABLE_ENV_VARIABLES, list));
        }
        if let Some(value) = row.value {
            env.push(("CONFIG_DIR", value));
        }
        let ctx = fx.bare_ctx(&cwd, &env);
        let found = evaluate(
            PortablePath::from_path(&target)
                .with_ctx(&ctx)
                .with_portable_env(row.builder.iter().copied()),
        );
        let cell = row.cell;
        assert_eq!(found.reference().raw(), row.expect_raw, "{cell}");
        let expected_strategy = if row.expect_raw == anchored { P::EnvRootedPath } else { P::AbsolutePath };
        assert_eq!(found.strategy(), &expected_strategy, "{cell}");
        assert_eq!(
            env_outcomes(attempt_for(found.attempts(), &P::EnvRootedPath)),
            row.expect_env,
            "{cell}"
        );
        assert_eq!(found.findings(), row.expect_findings, "{cell}");
        if row.expect_raw == anchored {
            let resolved = found.reference().resolve_in_context(&ctx).unwrap().unwrap();
            assert!(same_file(&resolved, &target), "{cell}: the anchor resolves back");
        }
    }
}

#[test]
fn the_deepest_anchor_wins_and_names_break_ties() {
    let fx = Fixture::new();
    let cwd = fx.dir("work");
    let config = fx.dir("config");
    let app = fx.dir("config/app");
    let in_app = fx.file("config/app/x.json");
    let in_config = fx.file("config/y.json");
    let env = [
        (PORTABLE_ENV_VARIABLES, "CONFIG_DIR,APP,A_DIR"),
        ("CONFIG_DIR", config.to_str().unwrap()),
        ("A_DIR", config.to_str().unwrap()),
        ("APP", app.to_str().unwrap()),
    ];
    let ctx = fx.bare_ctx(&cwd, &env);

    let found = evaluate(PortablePath::from_path(&in_app).with_ctx(&ctx));
    assert_eq!(found.reference().raw(), "{{APP}}/x.json");
    let found = evaluate(PortablePath::from_path(&in_config).with_ctx(&ctx));
    assert_eq!(found.reference().raw(), "{{A_DIR}}/y.json", "equal depth: name order");
    assert_eq!(
        env_outcomes(attempt_for(found.attempts(), &P::EnvRootedPath)),
        [
            unusable("APP", EnvAnchorProblem::NotAPrefix { value: app.clone() }),
            Ok("{{A_DIR}}/y.json".to_string())
        ]
    );
}

#[test]
fn declaring_home_writes_the_variable_instead_of_tilde() {
    let fx = Fixture::new();
    let cwd = fx.dir("work");
    let target = fx.file("home/notes/x.md");
    let home = fx.home.to_str().unwrap().to_string();

    let plain = evaluate(PortablePath::from_path(&target).with_ctx(&fx.bare_ctx(&cwd, &[("HOME", &home)])));
    assert_eq!(plain.reference().raw(), "~/notes/x.md");

    let declared = evaluate(
        PortablePath::from_path(&target)
            .with_ctx(&fx.bare_ctx(&cwd, &[("HOME", &home)]))
            .with_portable_env(["HOME"]),
    );
    assert_eq!(declared.reference().raw(), "{{HOME}}/notes/x.md");
    assert_eq!(declared.strategy(), &P::EnvRootedPath);
}

#[test]
fn portable_names_accumulate_across_builder_calls() {
    let fx = Fixture::new();
    let cwd = fx.dir("work");
    let notes = fx.dir("notes");
    let target = fx.file("notes/x.md");
    let ctx = fx.bare_ctx(&cwd, &[("NOTES", notes.to_str().unwrap())]);

    let found = evaluate(
        PortablePath::from_path(&target)
            .with_ctx(&ctx)
            .with_portable_env(["OTHER"])
            .with_portable_env(["NOTES", "OTHER"]),
    );
    assert_eq!(found.reference().raw(), "{{NOTES}}/x.md");
    assert_eq!(
        env_outcomes(attempt_for(found.attempts(), &P::EnvRootedPath)),
        [unusable("OTHER", EnvAnchorProblem::Unset), Ok("{{NOTES}}/x.md".to_string())],
        "OTHER is evaluated once"
    );
}
