
use tempfile::TempDir;

use super::*;

use crate::linking::skills::test_helpers::{setup_skill, test_paths, test_paths_with_gemini};
use crate::provider::Provider;

#[test]
fn discovers_user_only_skill() {
    let tmp = TempDir::new().unwrap();
    let paths = test_paths(tmp.path());
    let user_dir = tmp.path().join("user/skills");
    setup_skill(&user_dir, "my-skill", "A test skill", "# Body\n");

    let report = list_skills(&paths, &[]).unwrap();
    assert_eq!(report.skills.len(), 1);
    assert_eq!(report.skills[0].name, "my-skill");
    assert_eq!(report.skills[0].scope, SkillScope::User);
    assert_eq!(
        report.skills[0].description.as_deref(),
        Some("A test skill")
    );
}

#[test]
fn discovers_repo_only_skill() {
    let tmp = TempDir::new().unwrap();
    let paths = test_paths(tmp.path());
    let repo_dir = tmp.path().join("repo/skills");
    setup_skill(&repo_dir, "repo-skill", "Repo only", "# Body\n");

    let report = list_skills(&paths, &[]).unwrap();
    assert_eq!(report.skills.len(), 1);
    assert_eq!(report.skills[0].scope, SkillScope::Repo);
}

#[test]
fn classifies_masked_skill() {
    let tmp = TempDir::new().unwrap();
    let paths = test_paths(tmp.path());
    let user_dir = tmp.path().join("user/skills");
    let repo_dir = tmp.path().join("repo/skills");
    setup_skill(&user_dir, "shared", "User version", "# User\n");
    setup_skill(&repo_dir, "shared", "Repo version", "# Repo\n");

    let report = list_skills(&paths, &[]).unwrap();
    assert_eq!(report.skills.len(), 1);
    assert_eq!(report.skills[0].scope, SkillScope::RepoMasked);
    // Description should come from repo (the winning version)
    assert_eq!(
        report.skills[0].description.as_deref(),
        Some("Repo version")
    );
}

#[test]
fn filters_skills_by_name() {
    let tmp = TempDir::new().unwrap();
    let paths = test_paths(tmp.path());
    let user_dir = tmp.path().join("user/skills");
    setup_skill(&user_dir, "alpha", "Alpha skill", "# A\n");
    setup_skill(&user_dir, "beta", "Beta skill", "# B\n");
    setup_skill(&user_dir, "gamma", "Gamma skill", "# G\n");

    let report = list_skills(&paths, &["bet".to_string()]).unwrap();
    assert_eq!(report.skills.len(), 1);
    assert_eq!(report.skills[0].name, "beta");
}

#[test]
fn filter_is_case_insensitive() {
    let tmp = TempDir::new().unwrap();
    let paths = test_paths(tmp.path());
    let user_dir = tmp.path().join("user/skills");
    setup_skill(&user_dir, "MySkill", "My skill", "# Body\n");

    let report = list_skills(&paths, &["myskill".to_string()]).unwrap();
    assert_eq!(report.skills.len(), 1);
}

#[test]
fn detects_invalid_missing_description() {
    let tmp = TempDir::new().unwrap();
    let paths = test_paths(tmp.path());
    let user_dir = tmp.path().join("user/skills");
    let skill_dir = user_dir.join("no-desc");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: no-desc\n---\n# Body\n",
    )
    .unwrap();

    let report = list_skills(&paths, &[]).unwrap();
    let invalid: Vec<_> = report
        .exceptions
        .iter()
        .filter(|e| e.exception_type == ExceptionType::Invalid)
        .collect();
    assert_eq!(invalid.len(), 1);
    assert_eq!(invalid[0].topic, "no-desc");
}

#[test]
fn detects_and_fixes_yaml_tabs() {
    let tmp = TempDir::new().unwrap();
    let paths = test_paths(tmp.path());
    let user_dir = tmp.path().join("user/skills");
    let skill_dir = user_dir.join("tabbed");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: tabbed\ndescription: Has tabbed yaml\nprompt: |-\n\tline one\n\t\tline two\n---\n# Body\n",
    )
    .unwrap();

    let report = list_skills(&paths, &[]).unwrap();
    let yaml_tabs: Vec<_> = report
        .exceptions
        .iter()
        .filter(|e| e.exception_type == ExceptionType::YamlTabs)
        .collect();
    assert_eq!(yaml_tabs.len(), 1);
    assert_eq!(yaml_tabs[0].topic, "tabbed");

    let summary = crate::linking::skills::fix_missing_skills(&paths).unwrap();
    assert_eq!(summary.yaml_tabs_fixed, 1);

    let content = fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
    assert!(!frontmatter_has_indentation_tabs(&content).unwrap());
}

#[test]
fn detects_broken_links() {
    let tmp = TempDir::new().unwrap();
    let paths = test_paths(tmp.path());
    let user_dir = tmp.path().join("user/skills");
    setup_skill(
        &user_dir,
        "broken",
        "Has broken link",
        "See [details](./nonexistent.md) for more.\n",
    );

    let report = list_skills(&paths, &[]).unwrap();
    let broken: Vec<_> = report
        .exceptions
        .iter()
        .filter(|e| e.exception_type == ExceptionType::BrokenLink)
        .collect();
    assert_eq!(broken.len(), 1);
    assert_eq!(broken[0].topic, "broken");
}

#[test]
fn detects_no_links_in_long_body() {
    let tmp = TempDir::new().unwrap();
    let paths = test_paths(tmp.path());
    let user_dir = tmp.path().join("user/skills");
    let long_body = "x".repeat(300);
    setup_skill(&user_dir, "verbose", "Long body", &long_body);

    let report = list_skills(&paths, &[]).unwrap();
    let no_links: Vec<_> = report
        .exceptions
        .iter()
        .filter(|e| e.exception_type == ExceptionType::NoLinks)
        .collect();
    assert_eq!(no_links.len(), 1);
    assert_eq!(no_links[0].topic, "verbose");
}

#[test]
fn no_exception_for_short_body_without_links() {
    let tmp = TempDir::new().unwrap();
    let paths = test_paths(tmp.path());
    let user_dir = tmp.path().join("user/skills");
    setup_skill(&user_dir, "short", "Short skill", "Brief content.\n");

    let report = list_skills(&paths, &[]).unwrap();
    let no_links: Vec<_> = report
        .exceptions
        .iter()
        .filter(|e| e.exception_type == ExceptionType::NoLinks)
        .collect();
    assert!(no_links.is_empty());
}

#[test]
fn skips_hidden_directories() {
    let tmp = TempDir::new().unwrap();
    let paths = test_paths(tmp.path());
    let user_dir = tmp.path().join("user/skills");
    setup_skill(&user_dir, ".hidden", "Hidden", "# Body\n");
    setup_skill(&user_dir, "visible", "Visible", "# Body\n");

    let report = list_skills(&paths, &[]).unwrap();
    assert_eq!(report.skills.len(), 1);
    assert_eq!(report.skills[0].name, "visible");
}

#[test]
fn sorts_by_scope_then_alpha() {
    let tmp = TempDir::new().unwrap();
    let paths = test_paths(tmp.path());
    let user_dir = tmp.path().join("user/skills");
    let repo_dir = tmp.path().join("repo/skills");
    setup_skill(&user_dir, "gamma", "Gamma skill", "# G\n");
    setup_skill(&user_dir, "alpha", "Alpha skill", "# A\n");
    setup_skill(&repo_dir, "beta", "Beta skill", "# B\n");

    let report = list_skills(&paths, &[]).unwrap();
    assert_eq!(report.skills.len(), 3);
    // User scope first (alpha, gamma), then Repo scope (beta)
    assert_eq!(report.skills[0].name, "alpha");
    assert_eq!(report.skills[0].scope, SkillScope::User);
    assert_eq!(report.skills[1].name, "gamma");
    assert_eq!(report.skills[1].scope, SkillScope::User);
    assert_eq!(report.skills[2].name, "beta");
    assert_eq!(report.skills[2].scope, SkillScope::Repo);
}

#[test]
fn empty_directories_produce_empty_report() {
    let tmp = TempDir::new().unwrap();
    let paths = test_paths(tmp.path());

    let report = list_skills(&paths, &[]).unwrap();
    assert!(report.skills.is_empty());
}

#[test]
fn diagnostic_when_skills_dir_missing_but_parent_exists() {
    let tmp = TempDir::new().unwrap();
    let paths = test_paths_with_gemini(tmp.path());

    // Create Claude skills
    let user_dir = tmp.path().join("user/skills");
    setup_skill(&user_dir, "my-skill", "A skill", "# Body\n");

    // Create Gemini parent but NOT the skills dir
    fs::create_dir_all(tmp.path().join("gemini")).unwrap();

    let report = list_skills(&paths, &[]).unwrap();
    let gemini_diags: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.provider == Provider::Gemini)
        .collect();
    assert!(!gemini_diags.is_empty());
    assert!(
        gemini_diags
            .iter()
            .any(|d| d.message.contains("skills directory"))
    );
}

#[test]
fn diagnostic_when_base_config_dir_missing() {
    let tmp = TempDir::new().unwrap();
    let paths = test_paths_with_gemini(tmp.path());

    // Create Claude skills
    let user_dir = tmp.path().join("user/skills");
    setup_skill(&user_dir, "my-skill", "A skill", "# Body\n");

    // Don't create the Gemini dir at all (no gemini/ parent)

    let report = list_skills(&paths, &[]).unwrap();
    let gemini_diags: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.provider == Provider::Gemini)
        .collect();
    assert!(!gemini_diags.is_empty());
    assert!(
        gemini_diags
            .iter()
            .any(|d| d.message.contains("base configuration directory"))
    );
}

#[test]
fn no_diagnostic_when_skills_dir_exists() {
    let tmp = TempDir::new().unwrap();
    let paths = test_paths_with_gemini(tmp.path());

    // Create Claude skills in user scope only
    let user_dir = tmp.path().join("user/skills");
    setup_skill(&user_dir, "my-skill", "A skill", "# Body\n");

    // Create Gemini skills directories for both scopes (exist but empty)
    fs::create_dir_all(tmp.path().join("gemini/skills")).unwrap();
    fs::create_dir_all(tmp.path().join("repo/.gemini/skills")).unwrap();

    let report = list_skills(&paths, &[]).unwrap();
    // Should have individual missing exceptions, NOT diagnostics
    let gemini_diags: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.provider == Provider::Gemini)
        .collect();
    assert!(gemini_diags.is_empty());

    let gemini_missing: Vec<_> = report
        .exceptions
        .iter()
        .filter(|e| {
            e.provider == Provider::Gemini && e.exception_type == ExceptionType::Missing
        })
        .collect();
    // Only user scope: user-only skills are NOT expected in repo scope
    assert_eq!(gemini_missing.len(), 1);
    assert_eq!(gemini_missing[0].topic, "my-skill");
}

#[test]
fn missing_exceptions_scope_aware_both_scopes() {
    let tmp = TempDir::new().unwrap();
    let paths = test_paths_with_gemini(tmp.path());

    // Create Claude skills in BOTH scopes
    let user_dir = tmp.path().join("user/skills");
    let repo_dir = tmp.path().join("repo/skills");
    setup_skill(&user_dir, "user-tool", "User tool", "# Body\n");
    setup_skill(&repo_dir, "repo-tool", "Repo tool", "# Body\n");

    // Create Gemini skills directories (exist but empty)
    fs::create_dir_all(tmp.path().join("gemini/skills")).unwrap();
    fs::create_dir_all(tmp.path().join("repo/.gemini/skills")).unwrap();

    let report = list_skills(&paths, &[]).unwrap();

    let gemini_missing: Vec<_> = report
        .exceptions
        .iter()
        .filter(|e| {
            e.provider == Provider::Gemini && e.exception_type == ExceptionType::Missing
        })
        .collect();
    // 1 from user scope (user-tool) + 1 from repo scope (repo-tool)
    assert_eq!(gemini_missing.len(), 2);
    let topics: BTreeSet<&str> = gemini_missing.iter().map(|e| e.topic.as_str()).collect();
    assert!(topics.contains("user-tool"));
    assert!(topics.contains("repo-tool"));
}

#[test]
fn repo_scope_diagnostic_when_repo_dir_missing() {
    let tmp = TempDir::new().unwrap();
    let paths = test_paths_with_gemini(tmp.path());

    // Create Claude skills (repo scope)
    let repo_dir = tmp.path().join("repo/skills");
    setup_skill(&repo_dir, "repo-skill", "Repo skill", "# Body\n");

    // Don't create the Gemini repo dir at all

    let report = list_skills(&paths, &[]).unwrap();
    let gemini_diags: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.provider == Provider::Gemini)
        .collect();
    // Should have diagnostics for both user and repo scopes
    // Message contains Prose markup: "<b>repo</b> scoped"
    assert!(
        gemini_diags
            .iter()
            .any(|d| d.message.contains("repo</b> scoped"))
    );
}

// ── SkillFilter parsing tests ────────────────────────────────────

#[test]
fn parse_simple_fuzzy() {
    let f = SkillFilter::parse("rust").unwrap();
    assert_eq!(f.pattern, "rust");
    assert!(!f.negated);
    assert!(!f.exact);
}

#[test]
fn parse_exact_suffix() {
    let f = SkillFilter::parse("rust!").unwrap();
    assert_eq!(f.pattern, "rust");
    assert!(!f.negated);
    assert!(f.exact);
}

#[test]
fn parse_negation_dash_prefix() {
    let f = SkillFilter::parse("-rust").unwrap();
    assert_eq!(f.pattern, "rust");
    assert!(f.negated);
    assert!(!f.exact);
}

#[test]
fn parse_negation_bang_prefix() {
    let f = SkillFilter::parse("!rust").unwrap();
    assert_eq!(f.pattern, "rust");
    assert!(f.negated);
    assert!(!f.exact);
}

#[test]
fn parse_negation_and_exact() {
    let f = SkillFilter::parse("-rust!").unwrap();
    assert_eq!(f.pattern, "rust");
    assert!(f.negated);
    assert!(f.exact);
}

#[test]
fn parse_is_case_insensitive() {
    let f = SkillFilter::parse("Rust").unwrap();
    assert_eq!(f.pattern, "rust");
}

#[test]
fn parse_empty_returns_none() {
    assert!(SkillFilter::parse("").is_none());
    assert!(SkillFilter::parse("-").is_none());
    assert!(SkillFilter::parse("!").is_none());
    assert!(SkillFilter::parse("-!").is_none());
}

#[test]
fn parse_all_filters_empty() {
    let raw = vec!["rust".to_string(), "".to_string(), "-!".to_string()];
    let filters = SkillFilter::parse_all(&raw);
    assert_eq!(filters.len(), 1);
    assert_eq!(filters[0].pattern, "rust");
}

// ── SkillFilter matching tests ───────────────────────────────────

#[test]
fn fuzzy_matches_substring() {
    let f = SkillFilter::parse("us").unwrap();
    assert!(f.matches("rust"));
    assert!(f.matches("RUST"));
    assert!(!f.matches("python"));
}

#[test]
fn exact_matches_only_full_name() {
    let f = SkillFilter::parse("rust!").unwrap();
    assert!(f.matches("rust"));
    assert!(f.matches("Rust"));
    assert!(!f.matches("rusty"));
    assert!(!f.matches("my-rust"));
}

// ── SkillFilter::retain tests ────────────────────────────────────

#[test]
fn retain_positive_fuzzy_only() {
    let filters = SkillFilter::parse_all(&["us".to_string()]);
    assert!(SkillFilter::retain(&filters, "rust"));
    assert!(!SkillFilter::retain(&filters, "python"));
}

#[test]
fn retain_negation_only() {
    let filters = SkillFilter::parse_all(&["-rust".to_string()]);
    assert!(!SkillFilter::retain(&filters, "rust"));
    assert!(!SkillFilter::retain(&filters, "rusty"));
    assert!(SkillFilter::retain(&filters, "python"));
}

#[test]
fn retain_negation_wins_over_positive() {
    let filters = SkillFilter::parse_all(&["us".to_string(), "-rust!".to_string()]);
    // "rust" matches positive "us" but is excluded by exact negation "-rust!"
    assert!(!SkillFilter::retain(&filters, "rust"));
    // "rusty" matches positive "us" and is NOT excluded by exact negation "-rust!"
    assert!(SkillFilter::retain(&filters, "rusty"));
}

#[test]
fn retain_combined_positive_and_negation() {
    let filters = SkillFilter::parse_all(&["a".to_string(), "-alpha".to_string()]);
    // "gamma" contains "a" → included, not negated → kept
    assert!(SkillFilter::retain(&filters, "gamma"));
    // "alpha" contains "a" → included, but negated by "-alpha" → excluded
    assert!(!SkillFilter::retain(&filters, "alpha"));
    // "beta" contains "a" → included, not negated → kept
    assert!(SkillFilter::retain(&filters, "beta"));
    // "xyz" does not contain "a" → not included → excluded
    assert!(!SkillFilter::retain(&filters, "xyz"));
}

#[test]
fn retain_only_negations_keeps_non_matches() {
    let filters = SkillFilter::parse_all(&["-beta".to_string()]);
    assert!(SkillFilter::retain(&filters, "alpha"));
    assert!(!SkillFilter::retain(&filters, "beta"));
    assert!(SkillFilter::retain(&filters, "gamma"));
}

// ── list_skills with new filter modes ────────────────────────────

#[test]
fn list_skills_negation_excludes_match() {
    let tmp = TempDir::new().unwrap();
    let paths = test_paths(tmp.path());
    let user_dir = tmp.path().join("user/skills");
    setup_skill(&user_dir, "alpha", "Alpha", "# A\n");
    setup_skill(&user_dir, "beta", "Beta", "# B\n");
    setup_skill(&user_dir, "gamma", "Gamma", "# G\n");

    let report = list_skills(&paths, &["-beta".to_string()]).unwrap();
    let names: Vec<&str> = report.skills.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"alpha"));
    assert!(!names.contains(&"beta"));
    assert!(names.contains(&"gamma"));
}

#[test]
fn list_skills_exact_matches_only_full_name() {
    let tmp = TempDir::new().unwrap();
    let paths = test_paths(tmp.path());
    let user_dir = tmp.path().join("user/skills");
    setup_skill(&user_dir, "alpha", "Alpha", "# A\n");
    setup_skill(&user_dir, "alpha-extended", "Alpha Ext", "# AE\n");

    let report = list_skills(&paths, &["alpha!".to_string()]).unwrap();
    assert_eq!(report.skills.len(), 1);
    assert_eq!(report.skills[0].name, "alpha");
}

#[test]
fn list_skills_negation_exact_combo() {
    let tmp = TempDir::new().unwrap();
    let paths = test_paths(tmp.path());
    let user_dir = tmp.path().join("user/skills");
    setup_skill(&user_dir, "rust", "Rust", "# R\n");
    setup_skill(&user_dir, "rusty", "Rusty", "# Ry\n");
    setup_skill(&user_dir, "python", "Python", "# P\n");

    // Fuzzy "rust" but exclude exact "rust"
    let report = list_skills(&paths, &["rust".to_string(), "-rust!".to_string()]).unwrap();
    let names: Vec<&str> = report.skills.iter().map(|s| s.name.as_str()).collect();
    assert!(!names.contains(&"rust"));
    assert!(names.contains(&"rusty"));
    assert!(!names.contains(&"python"));
}
