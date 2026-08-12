use super::*;

#[test]
fn all_groups_have_rules() {
    let groups = [
        RuleGroup::FilesystemDestruction,
        RuleGroup::DiskManipulation,
        RuleGroup::RemoteExecution,
        RuleGroup::GitDestructive,
        RuleGroup::SystemSabotage,
        RuleGroup::NetworkSabotage,
        RuleGroup::ContainerCloud,
        RuleGroup::DatabaseNukes,
        RuleGroup::ObfuscatedExecution,
        RuleGroup::PromptInjection,
        RuleGroup::CredentialExfiltration,
    ];

    for group in groups {
        let rules: Vec<_> = CATALOG.iter().filter(|r| r.group == group).collect();
        assert!(
            !rules.is_empty(),
            "group {:?} has no rules",
            group.config_key()
        );
    }
}

#[test]
fn macos_filter_excludes_linux_only() {
    let filtered = rules_for_platform(ProtectPlatform::MacOs);
    assert!(
        !filtered
            .iter()
            .any(|r| r.platforms == PlatformApplicability::LinuxOnly),
        "macOS catalog should not contain Linux-only rules"
    );
}

#[test]
fn linux_filter_excludes_macos_only() {
    let filtered = rules_for_platform(ProtectPlatform::Linux);
    assert!(
        !filtered
            .iter()
            .any(|r| r.platforms == PlatformApplicability::MacOsOnly),
        "Linux catalog should not contain macOS-only rules"
    );
}

#[test]
fn cross_platform_rules_appear_on_both() {
    let mac = rules_for_platform(ProtectPlatform::MacOs);
    let linux = rules_for_platform(ProtectPlatform::Linux);
    assert!(mac.iter().any(|r| r.rule_id == "curl_pipe_shell"));
    assert!(linux.iter().any(|r| r.rule_id == "curl_pipe_shell"));
}

#[test]
fn rule_ids_are_unique() {
    let mut seen = std::collections::HashSet::new();
    for rule in CATALOG.iter() {
        assert!(
            seen.insert(rule.rule_id),
            "duplicate rule_id: {}",
            rule.rule_id
        );
    }
}

#[test]
fn all_patterns_are_valid_regex() {
    for rule in CATALOG.iter() {
        regex::Regex::new(rule.pattern).unwrap_or_else(|e| {
            panic!("invalid regex for rule {}: {e}", rule.rule_id);
        });
    }
}

#[test]
fn config_key_round_trips() {
    for group in RuleGroup::all_builtin() {
        let key = group.config_key();
        assert!(!key.is_empty(), "config_key empty for {group:?}");
    }
}
