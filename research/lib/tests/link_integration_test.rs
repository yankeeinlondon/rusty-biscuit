//! Integration tests for the link command
//!
//! These tests verify the complete end-to-end workflow of the link command,
//! including topic discovery, filtering, and symlink creation.

use research_lib::link::{link, SkillAction};
use serial_test::serial;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Helper to create a research library directory structure with topics
fn create_test_research_library(temp_dir: &Path) -> PathBuf {
    let library = temp_dir.join(".research").join("library");
    fs::create_dir_all(&library).unwrap();
    library
}

/// Helper to create a skill directory with SKILL.md
fn create_skill(library: &Path, name: &str, topic_type: &str) {
    let topic_dir = library.join(name);
    let skill_dir = topic_dir.join("skill");
    fs::create_dir_all(&skill_dir).unwrap();

    // Create SKILL.md
    let skill_md = skill_dir.join("SKILL.md");
    fs::write(&skill_md, format!("# {} Skill", name)).unwrap();

    // Create metadata.json
    let metadata = topic_dir.join("metadata.json");
    fs::write(
        &metadata,
        format!(
            r#"{{
  "name": "{}",
  "type": "{}",
  "brief": "Test skill for {}"
}}"#,
            name, topic_type, name
        ),
    )
    .unwrap();
}

/// Helper to create a skill directory without SKILL.md (invalid)
fn create_invalid_skill(library: &Path, name: &str, topic_type: &str) {
    let topic_dir = library.join(name);
    let skill_dir = topic_dir.join("skill");
    fs::create_dir_all(&skill_dir).unwrap();
    // Intentionally do NOT create SKILL.md

    // Create metadata.json
    let metadata = topic_dir.join("metadata.json");
    fs::write(
        &metadata,
        format!(
            r#"{{
  "name": "{}",
  "type": "{}",
  "brief": "Invalid test skill for {}"
}}"#,
            name, topic_type, name
        ),
    )
    .unwrap();
}

/// Helper to set up temporary home directories for Claude Code and OpenCode
fn setup_temp_home_dirs(temp_dir: &Path) -> (PathBuf, PathBuf) {
    let home = temp_dir.join("home");
    fs::create_dir_all(&home).unwrap();

    let claude_skills = home.join(".claude").join("skills");
    fs::create_dir_all(&claude_skills).unwrap();

    let opencode_skills = home.join(".config").join("opencode").join("skill");
    fs::create_dir_all(&opencode_skills).unwrap();

    (claude_skills, opencode_skills)
}

/// Helper to create a local skill definition (non-symlink directory)
fn create_local_definition(dir: &Path, name: &str) {
    let local_skill = dir.join(name);
    fs::create_dir_all(&local_skill).unwrap();
    fs::write(local_skill.join("README.md"), "Local definition").unwrap();
}

#[tokio::test]
#[serial]
async fn test_end_to_end_discover_filter_link_with_mixed_scenarios() {
    let temp = TempDir::new().unwrap();
    let library = create_test_research_library(temp.path());
    let _ = setup_temp_home_dirs(temp.path());

    // Create test skills
    create_skill(&library, "clap", "library");
    create_skill(&library, "thiserror", "library");
    create_skill(&library, "axum", "framework");
    create_skill(&library, "serde", "library");

    // Set RESEARCH_DIR and HOME environment variables
    unsafe {
        env::set_var("RESEARCH_DIR", temp.path());
        env::set_var("HOME", temp.path().join("home"));
    }

    // Run link command with no filters (link all)
    let result = link(vec![], vec![], false).await;

    // Reset environment variables
    unsafe {
        env::remove_var("RESEARCH_DIR");
        env::remove_var("HOME");
    }

    assert!(result.is_ok());
    let link_result = result.unwrap();

    // Should have processed 4 topics
    assert_eq!(link_result.total_processed(), 4);

    // Verify all topics were processed
    let topic_names: Vec<&str> = link_result.links.iter().map(|l| l.name.as_str()).collect();
    assert!(topic_names.contains(&"clap"));
    assert!(topic_names.contains(&"thiserror"));
    assert!(topic_names.contains(&"axum"));
    assert!(topic_names.contains(&"serde"));
}

#[tokio::test]
#[serial]
async fn test_error_handling_continues_when_one_symlink_fails() {
    let temp = TempDir::new().unwrap();
    let library = create_test_research_library(temp.path());
    let (_claude_skills, _opencode_skills) = setup_temp_home_dirs(temp.path());

    // Create 5 test skills
    create_skill(&library, "skill1", "library");
    create_skill(&library, "skill2", "library");
    create_skill(&library, "skill3", "library");
    create_skill(&library, "skill4", "library");
    create_skill(&library, "skill5", "library");

    // Create a local definition for skill3 to prevent linking
    create_local_definition(&_claude_skills, "skill3");

    // Set RESEARCH_DIR and HOME environment variables
    unsafe {
        env::set_var("RESEARCH_DIR", temp.path());
        env::set_var("HOME", temp.path().join("home"));
    }

    // Run link command
    let result = link(vec![], vec![], false).await;

    // Reset environment variables
    unsafe {
        env::remove_var("RESEARCH_DIR");
        env::remove_var("HOME");
    }

    assert!(result.is_ok());
    let link_result = result.unwrap();

    // Should have processed all 5 topics
    assert_eq!(link_result.total_processed(), 5);

    // Find skill3 result - should have NoneLocalDefinition for claude_action
    let skill3 = link_result
        .links
        .iter()
        .find(|l| l.name == "skill3")
        .unwrap();
    assert_eq!(skill3.claude_action, SkillAction::NoneLocalDefinition);

    // Other skills should have succeeded or attempted creation
    let other_skills: Vec<_> = link_result
        .links
        .iter()
        .filter(|l| l.name != "skill3")
        .collect();
    assert_eq!(other_skills.len(), 4);
}

#[tokio::test]
#[serial]
async fn test_asymmetric_failure_claude_succeeds_opencode_fails() {
    let temp = TempDir::new().unwrap();
    let library = create_test_research_library(temp.path());
    let (_claude_skills, opencode_skills) = setup_temp_home_dirs(temp.path());

    // Create test skill
    create_skill(&library, "test-skill", "library");

    // Create a local definition for OpenCode only
    create_local_definition(&opencode_skills, "test-skill");

    // Set environment variables
    unsafe {
        env::set_var("RESEARCH_DIR", temp.path());
        env::set_var("HOME", temp.path().join("home"));
    }

    // Run link command
    let result = link(vec![], vec![], false).await;

    // Reset environment variables
    unsafe {
        env::remove_var("RESEARCH_DIR");
        env::remove_var("HOME");
    }

    assert!(result.is_ok());
    let link_result = result.unwrap();

    // Should have processed 1 topic
    assert_eq!(link_result.total_processed(), 1);

    let skill_link = &link_result.links[0];
    assert_eq!(skill_link.name, "test-skill");

    // Claude Code should succeed, OpenCode should have local definition
    assert!(matches!(
        skill_link.claude_action,
        SkillAction::CreatedLink | SkillAction::NoneAlreadyLinked
    ));
    assert_eq!(
        skill_link.opencode_action,
        SkillAction::NoneLocalDefinition
    );
}

#[tokio::test]
#[serial]
async fn test_empty_filter_result_produces_no_symlinks() {
    let temp = TempDir::new().unwrap();
    let library = create_test_research_library(temp.path());
    let _ = setup_temp_home_dirs(temp.path());

    // Create test skills
    create_skill(&library, "clap", "library");
    create_skill(&library, "axum", "framework");

    // Set RESEARCH_DIR and HOME environment variables
    unsafe {
        env::set_var("RESEARCH_DIR", temp.path());
        env::set_var("HOME", temp.path().join("home"));
    }

    // Run link command with filter that matches nothing
    let result = link(vec!["nonexistent*".to_string()], vec![], false).await;

    // Reset environment variables
    unsafe {
        env::remove_var("RESEARCH_DIR");
        env::remove_var("HOME");
    }

    assert!(result.is_ok());
    let link_result = result.unwrap();

    // Should have processed 0 topics
    assert_eq!(link_result.total_processed(), 0);
    assert_eq!(link_result.total_created(), 0);
    assert!(!link_result.has_errors());
}

#[tokio::test]
#[serial]
async fn test_topic_has_no_skill_directory_verification() {
    let temp = TempDir::new().unwrap();
    let library = create_test_research_library(temp.path());
    let _ = setup_temp_home_dirs(temp.path());

    // Create one valid skill and one invalid skill (missing SKILL.md)
    create_skill(&library, "valid-skill", "library");
    create_invalid_skill(&library, "invalid-skill", "library");

    // Set RESEARCH_DIR and HOME environment variables
    unsafe {
        env::set_var("RESEARCH_DIR", temp.path());
        env::set_var("HOME", temp.path().join("home"));
    }

    // Run link command
    let result = link(vec![], vec![], false).await;

    // Reset environment variables
    unsafe {
        env::remove_var("RESEARCH_DIR");
        env::remove_var("HOME");
    }

    assert!(result.is_ok());
    let link_result = result.unwrap();

    // Should have processed 2 topics
    assert_eq!(link_result.total_processed(), 2);

    // Find invalid-skill - should have NoneSkillDirectoryInvalid
    let invalid = link_result
        .links
        .iter()
        .find(|l| l.name == "invalid-skill")
        .unwrap();
    assert_eq!(
        invalid.claude_action,
        SkillAction::NoneSkillDirectoryInvalid
    );
    assert_eq!(
        invalid.opencode_action,
        SkillAction::NoneSkillDirectoryInvalid
    );

    // Valid skill should succeed or attempt creation
    let valid = link_result
        .links
        .iter()
        .find(|l| l.name == "valid-skill")
        .unwrap();
    assert_ne!(
        valid.claude_action,
        SkillAction::NoneSkillDirectoryInvalid
    );
}

#[tokio::test]
#[serial]
async fn test_idempotency_running_twice_produces_same_result() {
    let temp = TempDir::new().unwrap();
    let library = create_test_research_library(temp.path());
    let _ = setup_temp_home_dirs(temp.path());

    // Create test skills
    create_skill(&library, "skill1", "library");
    create_skill(&library, "skill2", "library");

    // Set environment variables
    unsafe {
        env::set_var("RESEARCH_DIR", temp.path());
        env::set_var("HOME", temp.path().join("home"));
    }

    // Run link command first time
    let result1 = link(vec![], vec![], false).await;
    assert!(result1.is_ok());
    let link_result1 = result1.unwrap();

    // Run link command second time
    let result2 = link(vec![], vec![], false).await;
    assert!(result2.is_ok());
    let link_result2 = result2.unwrap();

    // Reset environment variables
    unsafe {
        env::remove_var("RESEARCH_DIR");
        env::remove_var("HOME");
    }

    // Should have same number of processed topics
    assert_eq!(link_result1.total_processed(), link_result2.total_processed());
    assert_eq!(link_result1.total_processed(), 2);

    // First run should create links
    assert_eq!(link_result1.total_created(), 2);

    // Second run should find existing links (no new creations)
    assert_eq!(link_result2.total_created(), 0);

    // Verify actions changed from CreatedLink to NoneAlreadyLinked
    for link in &link_result2.links {
        assert!(matches!(
            link.claude_action,
            SkillAction::NoneAlreadyLinked
        ));
        assert!(matches!(
            link.opencode_action,
            SkillAction::NoneAlreadyLinked
        ));
    }
}

#[tokio::test]
#[serial]
async fn test_filtering_works_correctly_glob_patterns() {
    let temp = TempDir::new().unwrap();
    let library = create_test_research_library(temp.path());
    let _ = setup_temp_home_dirs(temp.path());

    // Create test skills with different names
    create_skill(&library, "clap", "library");
    create_skill(&library, "clap-derive", "library");
    create_skill(&library, "thiserror", "library");
    create_skill(&library, "axum", "framework");

    // Set RESEARCH_DIR and HOME environment variables
    unsafe {
        env::set_var("RESEARCH_DIR", temp.path());
        env::set_var("HOME", temp.path().join("home"));
    }

    // Run link command with glob filter "clap*"
    let result = link(vec!["clap*".to_string()], vec![], false).await;

    // Reset environment variables
    unsafe {
        env::remove_var("RESEARCH_DIR");
        env::remove_var("HOME");
    }

    assert!(result.is_ok());
    let link_result = result.unwrap();

    // Should have processed 2 topics matching "clap*"
    assert_eq!(link_result.total_processed(), 2);

    let topic_names: Vec<&str> = link_result.links.iter().map(|l| l.name.as_str()).collect();
    assert!(topic_names.contains(&"clap"));
    assert!(topic_names.contains(&"clap-derive"));
    assert!(!topic_names.contains(&"thiserror"));
    assert!(!topic_names.contains(&"axum"));
}

#[tokio::test]
#[serial]
async fn test_filtering_works_correctly_type_filters() {
    let temp = TempDir::new().unwrap();
    let library = create_test_research_library(temp.path());
    let _ = setup_temp_home_dirs(temp.path());

    // Create test skills with different types
    create_skill(&library, "clap", "library");
    create_skill(&library, "thiserror", "library");
    create_skill(&library, "axum", "framework");
    create_skill(&library, "tokio", "runtime");

    // Set RESEARCH_DIR and HOME environment variables
    unsafe {
        env::set_var("RESEARCH_DIR", temp.path());
        env::set_var("HOME", temp.path().join("home"));
    }

    // Run link command with type filter "library"
    let result = link(vec![], vec!["library".to_string()], false).await;

    // Reset environment variables
    unsafe {
        env::remove_var("RESEARCH_DIR");
        env::remove_var("HOME");
    }

    assert!(result.is_ok());
    let link_result = result.unwrap();

    // Should have processed 2 topics of type "library"
    assert_eq!(link_result.total_processed(), 2);

    let topic_names: Vec<&str> = link_result.links.iter().map(|l| l.name.as_str()).collect();
    assert!(topic_names.contains(&"clap"));
    assert!(topic_names.contains(&"thiserror"));
    assert!(!topic_names.contains(&"axum"));
    assert!(!topic_names.contains(&"tokio"));
}

#[tokio::test]
#[serial]
async fn test_combined_glob_and_type_filters() {
    let temp = TempDir::new().unwrap();
    let library = create_test_research_library(temp.path());
    let _ = setup_temp_home_dirs(temp.path());

    // Create test skills
    create_skill(&library, "clap", "library");
    create_skill(&library, "clap-derive", "library");
    create_skill(&library, "clap-builder", "framework");
    create_skill(&library, "thiserror", "library");

    // Set RESEARCH_DIR and HOME environment variables
    unsafe {
        env::set_var("RESEARCH_DIR", temp.path());
        env::set_var("HOME", temp.path().join("home"));
    }

    // Run link command with both glob and type filters
    let result = link(
        vec!["clap*".to_string()],
        vec!["library".to_string()],
        false,
    )
    .await;

    // Reset environment variables
    unsafe {
        env::remove_var("RESEARCH_DIR");
        env::remove_var("HOME");
    }

    assert!(result.is_ok());
    let link_result = result.unwrap();

    // Should have processed 2 topics matching both "clap*" AND type "library"
    assert_eq!(link_result.total_processed(), 2);

    let topic_names: Vec<&str> = link_result.links.iter().map(|l| l.name.as_str()).collect();
    assert!(topic_names.contains(&"clap"));
    assert!(topic_names.contains(&"clap-derive"));
    assert!(!topic_names.contains(&"clap-builder")); // Wrong type
    assert!(!topic_names.contains(&"thiserror")); // Wrong glob pattern
}

#[tokio::test]
#[serial]
async fn test_symlinks_created_are_accessible() {
    let temp = TempDir::new().unwrap();
    let library = create_test_research_library(temp.path());
    let (claude_skills, _opencode_skills) = setup_temp_home_dirs(temp.path());

    // Create test skill with some content
    create_skill(&library, "test-skill", "library");
    let skill_dir = library.join("test-skill").join("skill");
    fs::write(skill_dir.join("extra.md"), "Extra content").unwrap();

    // Set environment variables
    unsafe {
        env::set_var("RESEARCH_DIR", temp.path());
        env::set_var("HOME", temp.path().join("home"));
    }

    // Run link command
    let result = link(vec![], vec![], false).await;

    // Reset environment variables
    unsafe {
        env::remove_var("RESEARCH_DIR");
        env::remove_var("HOME");
    }

    assert!(result.is_ok());

    // Verify symlink was created and is accessible
    let symlink_path = claude_skills.join("test-skill");
    assert!(symlink_path.exists());
    assert!(symlink_path.is_symlink());

    // Verify we can access files through the symlink
    assert!(symlink_path.join("SKILL.md").exists());
    assert!(symlink_path.join("extra.md").exists());

    let content = fs::read_to_string(symlink_path.join("extra.md")).unwrap();
    assert_eq!(content, "Extra content");
}
