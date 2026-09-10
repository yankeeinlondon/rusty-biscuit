use std::path::PathBuf;

use biscuit_file::serde_yaml_ng;
use tempfile::TempDir;

use super::frontmatter_io::parse_frontmatter_lines;
use super::properties::claude_specific_properties;
use super::{
    classify_canonical_candidate, classify_target_reference, fix_frontmatter_indentation_tabs,
    frontmatter_has_indentation_tabs, has_claude_specific_properties, parse_markdown_document,
};
use crate::linking::capabilities::LinkableResource;
use crate::linking::detector::DiscoveredResource;
use crate::linking::model::{ResourceReference, ResourceScope};
use crate::provider::Provider;

fn build_candidate(
    name: &str,
    path: PathBuf,
    provider: Provider,
    is_symlink: bool,
) -> DiscoveredResource {
    DiscoveredResource {
        name: name.to_string(),
        path,
        provider,
        is_symlink,
        hash: None,
    }
}

#[test]
fn skill_name_is_derived_and_written_in_place() {
    let tmp = TempDir::new().unwrap();
    let skill_dir = tmp.path().join("build-pipeline");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\ndescription: Build release artifacts\n---\n# Build\n",
    )
    .unwrap();

    let candidate =
        build_candidate("build-pipeline", skill_dir.clone(), Provider::Claude, false);
    let classified =
        classify_canonical_candidate(LinkableResource::Skill, &candidate, ResourceScope::User)
            .unwrap();

    match classified {
        ResourceReference::Source(definition) => {
            assert_eq!(
                definition.frontmatter.get("name"),
                Some(&"build-pipeline".to_string())
            );
        }
        other => panic!("expected Source, got {other:?}"),
    }

    let upgraded = std::fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
    assert!(upgraded.contains("name: build-pipeline"));
}

#[test]
fn missing_required_properties_mark_source_as_partial() {
    let tmp = TempDir::new().unwrap();
    let skill_dir = tmp.path().join("missing-description");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), "---\n---\n# Body\n").unwrap();

    let candidate = build_candidate("missing-description", skill_dir, Provider::Claude, false);
    let classified =
        classify_canonical_candidate(LinkableResource::Skill, &candidate, ResourceScope::User)
            .unwrap();

    match classified {
        ResourceReference::PartialSource(_, missing) => {
            assert!(missing.contains(&"description".to_string()));
        }
        other => panic!("expected PartialSource, got {other:?}"),
    }
}

#[test]
fn alias_duplication_adds_equivalent_keys() {
    let tmp = TempDir::new().unwrap();
    let agent_file = tmp.path().join("security-agent.md");
    std::fs::write(
        &agent_file,
        "---\nname: security-agent\ndescription: Security reviewer\nmax_turns: 8\n---\nReview changes for security defects.\n",
    )
    .unwrap();

    let candidate = build_candidate(
        "security-agent",
        agent_file.clone(),
        Provider::Claude,
        false,
    );
    let classified =
        classify_canonical_candidate(LinkableResource::Agent, &candidate, ResourceScope::User)
            .unwrap();

    let definition = match classified {
        ResourceReference::Source(definition)
        | ResourceReference::PartialSource(definition, _) => definition,
        other => panic!("expected Source/PartialSource, got {other:?}"),
    };

    assert_eq!(
        definition.frontmatter.get("max_turns"),
        Some(&"8".to_string())
    );
    assert_eq!(
        definition.frontmatter.get("maxTurns"),
        Some(&"8".to_string())
    );
}

#[test]
fn markdown_command_body_satisfies_prompt_requirement() {
    let tmp = TempDir::new().unwrap();
    let command_file = tmp.path().join("run-tests.md");
    std::fs::write(&command_file, "Run unit tests with coverage.").unwrap();

    let candidate = build_candidate("run-tests", command_file, Provider::Claude, false);
    let classified = classify_canonical_candidate(
        LinkableResource::Command,
        &candidate,
        ResourceScope::User,
    )
    .unwrap();

    assert!(matches!(classified, ResourceReference::Source(_)));
}

#[test]
fn script_file_is_classified_as_source() {
    let tmp = TempDir::new().unwrap();
    let script_file = tmp.path().join("run.sh");
    std::fs::write(&script_file, "#!/usr/bin/env bash\necho hi\n").unwrap();

    let candidate = build_candidate("run.sh", script_file, Provider::Codex, false);
    let classified =
        classify_canonical_candidate(LinkableResource::Script, &candidate, ResourceScope::User)
            .unwrap();

    assert!(matches!(classified, ResourceReference::Source(_)));
}

#[test]
fn target_links_become_incomplete_when_provider_requirements_unmet() {
    let tmp = TempDir::new().unwrap();
    let agent_file = tmp.path().join("reviewer.md");
    std::fs::write(
        &agent_file,
        "---\nname: reviewer\ndescription: Reviews pull requests\n---\nFocus on maintainability.\n",
    )
    .unwrap();

    let candidate = build_candidate("reviewer", agent_file, Provider::Claude, false);
    let canonical =
        classify_canonical_candidate(LinkableResource::Agent, &candidate, ResourceScope::User)
            .unwrap();

    let for_kimi = classify_target_reference(
        LinkableResource::Agent,
        &canonical,
        Provider::KimiCode,
        ResourceScope::User,
    );
    assert!(matches!(
        for_kimi,
        ResourceReference::IncompleteLink(Provider::KimiCode, ResourceScope::User, _)
    ));

    let for_opencode = classify_target_reference(
        LinkableResource::Agent,
        &canonical,
        Provider::OpenCode,
        ResourceScope::User,
    );
    assert!(matches!(
        for_opencode,
        ResourceReference::LinkMissing(Provider::OpenCode, ResourceScope::User)
    ));
}

#[test]
fn frontmatter_with_yaml_incompatible_brackets_uses_fallback_parser() {
    let content = "---\ndescription: Create commits\nargument-hint: [--force] [commit-message]\n---\nDo the thing.\n";
    let parsed = parse_markdown_document(content).unwrap();
    assert!(parsed.had_frontmatter);
    assert_eq!(
        parsed
            .frontmatter
            .get(serde_yaml_ng::Value::String("description".to_string())),
        Some(&serde_yaml_ng::Value::String("Create commits".to_string()))
    );
    assert_eq!(
        parsed
            .frontmatter
            .get(serde_yaml_ng::Value::String("argument-hint".to_string())),
        Some(&serde_yaml_ng::Value::String(
            "[--force] [commit-message]".to_string()
        ))
    );
    assert_eq!(parsed.body, "Do the thing.\n");
}

#[test]
fn single_bracket_value_parsed_as_yaml_sequence() {
    // serde_yaml interprets `[bug-description]` as a flow sequence.
    // This is correct YAML behavior but means the value is a Sequence,
    // not the literal string `[bug-description]`. Downstream consumers
    // (mapping_to_string_map) re-serialize it as `- bug-description`.
    let content = "---\nargument-hint: [bug-description]\n---\nBody.\n";
    let parsed = parse_markdown_document(content).unwrap();
    let hint = parsed
        .frontmatter
        .get(serde_yaml_ng::Value::String("argument-hint".to_string()))
        .unwrap();
    assert!(
        hint.is_sequence(),
        "YAML parses [value] as a flow sequence, not a string"
    );
}

#[test]
fn angle_bracket_placeholders_are_plain_strings() {
    let content = "---\nargument-hint: <skill-name>\ndescription: Do something\n---\nBody.\n";
    let parsed = parse_markdown_document(content).unwrap();
    assert_eq!(
        parsed
            .frontmatter
            .get(serde_yaml_ng::Value::String("argument-hint".to_string())),
        Some(&serde_yaml_ng::Value::String("<skill-name>".to_string()))
    );
}

#[test]
fn mixed_angle_and_square_brackets_are_plain_strings() {
    // `<source-file> [test-glob]` starts with `<`, so YAML treats
    // the whole value as a plain scalar (the `[` is not at value start).
    let content = "---\nargument-hint: <source-file> [test-glob]\n---\nBody.\n";
    let parsed = parse_markdown_document(content).unwrap();
    assert_eq!(
        parsed
            .frontmatter
            .get(serde_yaml_ng::Value::String("argument-hint".to_string())),
        Some(&serde_yaml_ng::Value::String(
            "<source-file> [test-glob]".to_string()
        ))
    );
}

#[test]
fn allowed_tools_with_colons_and_parens_parse_as_strings() {
    let content = "---\nallowed-tools: Bash(git:*), Read\n---\nBody.\n";
    let parsed = parse_markdown_document(content).unwrap();
    assert_eq!(
        parsed
            .frontmatter
            .get(serde_yaml_ng::Value::String("allowed-tools".to_string())),
        Some(&serde_yaml_ng::Value::String(
            "Bash(git:*), Read".to_string()
        ))
    );
}

#[test]
fn allowed_tools_with_multiple_colons_parse_as_strings() {
    let content =
        "---\nallowed-tools: Bash(ping :*), Bash(traceroute :*), Bash(dig :*)\n---\nBody.\n";
    let parsed = parse_markdown_document(content).unwrap();
    assert_eq!(
        parsed
            .frontmatter
            .get(serde_yaml_ng::Value::String("allowed-tools".to_string())),
        Some(&serde_yaml_ng::Value::String(
            "Bash(ping :*), Bash(traceroute :*), Bash(dig :*)".to_string()
        ))
    );
}

#[test]
fn quoted_field_value_strips_yaml_quotes() {
    let content = "---\nname: \"code-review\"\n---\nBody.\n";
    let parsed = parse_markdown_document(content).unwrap();
    assert_eq!(
        parsed
            .frontmatter
            .get(serde_yaml_ng::Value::String("name".to_string())),
        Some(&serde_yaml_ng::Value::String("code-review".to_string()))
    );
}

#[test]
fn single_quoted_brackets_are_plain_strings() {
    let content = "---\nargument-hint: '[--force] [commit-message-hint]'\n---\nBody.\n";
    let parsed = parse_markdown_document(content).unwrap();
    assert_eq!(
        parsed
            .frontmatter
            .get(serde_yaml_ng::Value::String("argument-hint".to_string())),
        Some(&serde_yaml_ng::Value::String(
            "[--force] [commit-message-hint]".to_string()
        ))
    );
}

#[test]
fn description_with_nested_quotes_and_parens() {
    let content = "---\ndescription: Evaluate \"drift\" (aka, docs out of sync)\n---\nBody.\n";
    let parsed = parse_markdown_document(content).unwrap();
    let desc = parsed
        .frontmatter
        .get(serde_yaml_ng::Value::String("description".to_string()))
        .unwrap();
    assert!(desc.is_string(), "description should parse as string");
}

#[test]
fn crlf_line_endings_in_frontmatter() {
    let content = "---\r\ndescription: test\r\nname: example\r\n---\r\nBody.\r\n";
    let parsed = parse_markdown_document(content).unwrap();
    assert!(parsed.had_frontmatter);
    assert_eq!(
        parsed
            .frontmatter
            .get(serde_yaml_ng::Value::String("description".to_string())),
        Some(&serde_yaml_ng::Value::String("test".to_string()))
    );
    assert_eq!(
        parsed
            .frontmatter
            .get(serde_yaml_ng::Value::String("name".to_string())),
        Some(&serde_yaml_ng::Value::String("example".to_string()))
    );
}

#[test]
fn bom_prefix_is_stripped_before_parsing() {
    let content = "\u{feff}---\ndescription: test\n---\nBody.\n";
    let parsed = parse_markdown_document(content).unwrap();
    assert!(parsed.had_frontmatter);
    assert_eq!(
        parsed
            .frontmatter
            .get(serde_yaml_ng::Value::String("description".to_string())),
        Some(&serde_yaml_ng::Value::String("test".to_string()))
    );
}

#[test]
fn no_frontmatter_returns_full_body() {
    let content = "Just a body with no frontmatter.\n";
    let parsed = parse_markdown_document(content).unwrap();
    assert!(!parsed.had_frontmatter);
    assert!(parsed.frontmatter.is_empty());
    assert_eq!(parsed.body, content);
}

#[test]
fn empty_frontmatter_returns_empty_mapping() {
    let content = "---\n---\nBody.\n";
    let parsed = parse_markdown_document(content).unwrap();
    assert!(parsed.had_frontmatter);
    assert!(parsed.frontmatter.is_empty());
    assert_eq!(parsed.body, "Body.\n");
}

#[test]
fn fallback_parser_skips_comments_and_empty_lines() {
    let mapping = parse_frontmatter_lines("# comment\n\nname: test\n\ndescription: hello\n");
    assert_eq!(mapping.len(), 2);
    assert_eq!(
        mapping.get(serde_yaml_ng::Value::String("name".to_string())),
        Some(&serde_yaml_ng::Value::String("test".to_string()))
    );
    assert_eq!(
        mapping.get(serde_yaml_ng::Value::String("description".to_string())),
        Some(&serde_yaml_ng::Value::String("hello".to_string()))
    );
}

#[test]
fn fallback_parser_ignores_lines_without_separator() {
    let mapping = parse_frontmatter_lines("name: valid\nno-separator\nother: also-valid\n");
    assert_eq!(mapping.len(), 2);
    assert!(
        mapping
            .get(serde_yaml_ng::Value::String("name".to_string()))
            .is_some()
    );
    assert!(
        mapping
            .get(serde_yaml_ng::Value::String("other".to_string()))
            .is_some()
    );
}

#[test]
fn fallback_parser_rejects_keys_with_spaces() {
    let mapping = parse_frontmatter_lines("good-key: value\nbad key: value\n");
    assert_eq!(mapping.len(), 1);
    assert!(
        mapping
            .get(serde_yaml_ng::Value::String("good-key".to_string()))
            .is_some()
    );
}

#[test]
fn classify_target_reference_captures_missing_property_names() {
    use crate::linking::model::IncompleteCause;

    let tmp = TempDir::new().unwrap();
    let agent_file = tmp.path().join("reviewer.md");
    std::fs::write(
        &agent_file,
        "---\nname: reviewer\ndescription: Reviews changes\n---\nUse strict review standards.\n",
    )
    .unwrap();

    let candidate = build_candidate("reviewer", agent_file, Provider::Claude, false);
    let canonical =
        classify_canonical_candidate(LinkableResource::Agent, &candidate, ResourceScope::User)
            .unwrap();

    let for_kimi = classify_target_reference(
        LinkableResource::Agent,
        &canonical,
        Provider::KimiCode,
        ResourceScope::User,
    );
    match for_kimi {
        ResourceReference::IncompleteLink(_, _, IncompleteCause::MissingProperties(props)) => {
            assert!(!props.is_empty(), "should capture missing property names");
        }
        ResourceReference::IncompleteLink(_, _, cause) => {
            // CustomNotSupported is also valid if Kimi doesn't support custom agents
            assert!(
                matches!(cause, IncompleteCause::CustomNotSupported),
                "unexpected cause: {cause}"
            );
        }
        other => panic!("expected IncompleteLink, got {other:?}"),
    }
}

#[test]
fn claude_specific_detects_model() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("test.md");
    std::fs::write(&file, "---\ndescription: Test\nmodel: sonnet\n---\nBody\n").unwrap();
    let props = claude_specific_properties(&file);
    assert_eq!(props, vec!["model"]);
    assert!(has_claude_specific_properties(&file));
}

#[test]
fn claude_specific_detects_tools() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("test.md");
    std::fs::write(
        &file,
        "---\ndescription: Test\ntools: Bash, Read\n---\nBody\n",
    )
    .unwrap();
    let props = claude_specific_properties(&file);
    assert_eq!(props, vec!["tools"]);
}

#[test]
fn claude_specific_detects_skills() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("test.md");
    std::fs::write(&file, "---\ndescription: Test\nskills: rust\n---\nBody\n").unwrap();
    let props = claude_specific_properties(&file);
    assert_eq!(props, vec!["skills"]);
}

#[test]
fn claude_specific_ignores_allowed_tools() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("test.md");
    std::fs::write(
        &file,
        "---\ndescription: Test\nallowed-tools: Bash(git:*), Read\n---\nBody\n",
    )
    .unwrap();
    // allowed-tools is not in the non-portable list — other providers ignore it
    assert!(claude_specific_properties(&file).is_empty());
    assert!(!has_claude_specific_properties(&file));
}

#[test]
fn claude_specific_detects_multiple() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("test.md");
    std::fs::write(
        &file,
        "---\ndescription: Test\nmodel: opus\ntools: Bash\nskills: rust\n---\nBody\n",
    )
    .unwrap();
    let props = claude_specific_properties(&file);
    assert_eq!(props, vec!["model", "tools", "skills"]);
}

#[test]
fn claude_specific_returns_empty_for_shareable() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("test.md");
    std::fs::write(&file, "---\ndescription: A shareable agent\n---\nBody\n").unwrap();
    assert!(claude_specific_properties(&file).is_empty());
    assert!(!has_claude_specific_properties(&file));
}

#[test]
fn claude_specific_returns_empty_for_missing_file() {
    let path = std::path::PathBuf::from("/nonexistent/file.md");
    assert!(claude_specific_properties(&path).is_empty());
    assert!(!has_claude_specific_properties(&path));
}

#[test]
fn detects_frontmatter_indentation_tabs() {
    let content = "---\nprompt: |-\n\tline one\n    \tline two\n---\nBody.\n";
    assert!(frontmatter_has_indentation_tabs(content).unwrap());
}

#[test]
fn fixes_frontmatter_indentation_tabs_in_place() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("SKILL.md");
    std::fs::write(
        &path,
        "---\nprompt: |-\n\tline one\n\t\tline two\n---\nBody.\n",
    )
    .unwrap();

    assert!(fix_frontmatter_indentation_tabs(&path).unwrap());

    let rewritten = std::fs::read_to_string(&path).unwrap();
    assert!(!frontmatter_has_indentation_tabs(&rewritten).unwrap());
    assert!(rewritten.contains("    line one"));
    assert!(rewritten.contains("        line two"));
    assert!(rewritten.ends_with("---\nBody.\n"));
}
