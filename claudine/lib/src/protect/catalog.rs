use std::fmt;

use serde::{Deserialize, Serialize};

/// Platform for which protection rules should be loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectPlatform {
    MacOs,
    Linux,
}

impl ProtectPlatform {
    /// Detect the current platform from compile-time target.
    pub fn current() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self::MacOs
        }
        #[cfg(target_os = "linux")]
        {
            Self::Linux
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            // Default to Linux behavior for other Unix-like systems
            Self::Linux
        }
    }
}

/// Surface where a protection rule can match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanSurface {
    /// Bash tool command strings.
    BashCommand,
    /// Write/Edit tool target paths.
    WritePath,
    /// MCP server response payloads.
    McpResponse,
}

/// Rule group identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleGroup {
    FilesystemDestruction,
    DiskManipulation,
    RemoteExecution,
    GitDestructive,
    SystemSabotage,
    NetworkSabotage,
    ContainerCloud,
    DatabaseNukes,
    ObfuscatedExecution,
    PromptInjection,
    CredentialExfiltration,
    SensitivePaths,
    Custom,
}

impl RuleGroup {
    /// Returns the configuration key for this group.
    pub fn config_key(&self) -> &'static str {
        match self {
            Self::FilesystemDestruction => "filesystem_destruction",
            Self::DiskManipulation => "disk_manipulation",
            Self::RemoteExecution => "remote_execution",
            Self::GitDestructive => "git_destructive",
            Self::SystemSabotage => "system_sabotage",
            Self::NetworkSabotage => "network_sabotage",
            Self::ContainerCloud => "container_cloud",
            Self::DatabaseNukes => "database_nukes",
            Self::ObfuscatedExecution => "obfuscated_execution",
            Self::PromptInjection => "prompt_injection",
            Self::CredentialExfiltration => "credential_exfiltration",
            Self::SensitivePaths => "sensitive_paths",
            Self::Custom => "custom",
        }
    }

    /// Returns all built-in rule groups (excludes Custom).
    pub fn all_builtin() -> Vec<Self> {
        vec![
            Self::FilesystemDestruction,
            Self::DiskManipulation,
            Self::RemoteExecution,
            Self::GitDestructive,
            Self::SystemSabotage,
            Self::NetworkSabotage,
            Self::ContainerCloud,
            Self::DatabaseNukes,
            Self::ObfuscatedExecution,
            Self::PromptInjection,
            Self::CredentialExfiltration,
            Self::SensitivePaths,
        ]
    }
}

impl fmt::Display for RuleGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.config_key())
    }
}

/// Platform applicability for a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformApplicability {
    /// Rule applies to all platforms.
    All,
    /// Rule applies only to macOS.
    MacOsOnly,
    /// Rule applies only to Linux.
    LinuxOnly,
}

/// A single protection rule definition.
#[derive(Debug, Clone)]
pub struct RuleDefinition {
    /// Rule group this belongs to.
    pub group: RuleGroup,
    /// Unique identifier for this rule within its group.
    pub rule_id: &'static str,
    /// Surface this rule scans.
    pub surface: ScanSurface,
    /// Regex pattern to match.
    pub pattern: &'static str,
    /// Platform applicability.
    pub platforms: PlatformApplicability,
    /// Whether this rule supports allow_paths configuration.
    pub supports_allow_paths: bool,
}

/// The complete catalog of protection rules.
pub static CATALOG: &[RuleDefinition] = &[
    // ========== filesystem_destruction ==========
    RuleDefinition {
        group: RuleGroup::FilesystemDestruction,
        rule_id: "rm_root",
        surface: ScanSurface::BashCommand,
        pattern: r"(sudo\s+)?\\?rm\s+-[a-zA-Z]*[rf][a-zA-Z]*[rf][a-zA-Z]*\s+/\s*$",
        platforms: PlatformApplicability::All,
        supports_allow_paths: true,
    },
    RuleDefinition {
        group: RuleGroup::FilesystemDestruction,
        rule_id: "rm_root_glob",
        surface: ScanSurface::BashCommand,
        pattern: r"(sudo\s+)?\\?rm\s+-[a-zA-Z]*[rf][a-zA-Z]*[rf][a-zA-Z]*\s+/\*",
        platforms: PlatformApplicability::All,
        supports_allow_paths: true,
    },
    RuleDefinition {
        group: RuleGroup::FilesystemDestruction,
        rule_id: "find_delete",
        surface: ScanSurface::BashCommand,
        pattern: r"find\s+.*\s+-delete",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::FilesystemDestruction,
        rule_id: "shred",
        surface: ScanSurface::BashCommand,
        pattern: r"shred\s+-u\s+",
        platforms: PlatformApplicability::All,
        supports_allow_paths: true,
    },
    RuleDefinition {
        group: RuleGroup::FilesystemDestruction,
        rule_id: "chmod_recursive_777",
        surface: ScanSurface::BashCommand,
        pattern: r"(sudo\s+)?chmod\s+-R\s+777\s+/",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::FilesystemDestruction,
        rule_id: "chmod_recursive_000",
        surface: ScanSurface::BashCommand,
        pattern: r"(sudo\s+)?chmod\s+-R\s+000\s+/",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::FilesystemDestruction,
        rule_id: "chown_recursive_root",
        surface: ScanSurface::BashCommand,
        pattern: r"(sudo\s+)?chown\s+-R\s+.*\s+/\s*$",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::FilesystemDestruction,
        rule_id: "chattr_immutable",
        surface: ScanSurface::BashCommand,
        pattern: r"chattr\s+\+i\s+",
        platforms: PlatformApplicability::LinuxOnly,
        supports_allow_paths: true,
    },
    RuleDefinition {
        group: RuleGroup::FilesystemDestruction,
        rule_id: "rm_boot",
        surface: ScanSurface::BashCommand,
        pattern: r"(sudo\s+)?rm\s+-rf\s+/boot",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    // ========== disk_manipulation ==========
    RuleDefinition {
        group: RuleGroup::DiskManipulation,
        rule_id: "mkfs",
        surface: ScanSurface::BashCommand,
        pattern: r"(sudo\s+)?mkfs\.",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::DiskManipulation,
        rule_id: "dd_to_device",
        surface: ScanSurface::BashCommand,
        pattern: r"dd\s+.*of=/dev/sd[a-z]",
        platforms: PlatformApplicability::LinuxOnly,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::DiskManipulation,
        rule_id: "dd_to_disk",
        surface: ScanSurface::BashCommand,
        pattern: r"dd\s+.*of=/dev/disk",
        platforms: PlatformApplicability::MacOsOnly,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::DiskManipulation,
        rule_id: "fdisk",
        surface: ScanSurface::BashCommand,
        pattern: r"(sudo\s+)?fdisk\s+",
        platforms: PlatformApplicability::LinuxOnly,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::DiskManipulation,
        rule_id: "parted",
        surface: ScanSurface::BashCommand,
        pattern: r"(sudo\s+)?parted\s+",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::DiskManipulation,
        rule_id: "lvremove",
        surface: ScanSurface::BashCommand,
        pattern: r"(sudo\s+)?lvremove\s+",
        platforms: PlatformApplicability::LinuxOnly,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::DiskManipulation,
        rule_id: "zfs_destroy",
        surface: ScanSurface::BashCommand,
        pattern: r"(sudo\s+)?zfs\s+destroy\s+",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::DiskManipulation,
        rule_id: "diskutil_erase",
        surface: ScanSurface::BashCommand,
        pattern: r"(sudo\s+)?diskutil\s+erase",
        platforms: PlatformApplicability::MacOsOnly,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::DiskManipulation,
        rule_id: "truncate_device",
        surface: ScanSurface::BashCommand,
        pattern: r">\s*/dev/sd[a-z]",
        platforms: PlatformApplicability::LinuxOnly,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::DiskManipulation,
        rule_id: "swapoff",
        surface: ScanSurface::BashCommand,
        pattern: r"(sudo\s+)?swapoff\s+-a",
        platforms: PlatformApplicability::LinuxOnly,
        supports_allow_paths: false,
    },
    // ========== remote_execution ==========
    RuleDefinition {
        group: RuleGroup::RemoteExecution,
        rule_id: "curl_pipe_shell",
        surface: ScanSurface::BashCommand,
        pattern: r"curl\s+.*\|\s*(bash|sh)",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::RemoteExecution,
        rule_id: "wget_pipe_shell",
        surface: ScanSurface::BashCommand,
        pattern: r"wget\s+.*-O-\s*\|\s*(sh|bash)",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::RemoteExecution,
        rule_id: "bash_reverse_shell",
        surface: ScanSurface::BashCommand,
        pattern: r"bash\s+-i\s+>&\s*/dev/tcp/",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::RemoteExecution,
        rule_id: "python_reverse_shell",
        surface: ScanSurface::BashCommand,
        pattern: r"python.*-c.*import\s+socket.*dup2",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::RemoteExecution,
        rule_id: "perl_reverse_shell",
        surface: ScanSurface::BashCommand,
        pattern: r"perl\s+-e\s+.*use\s+Socket",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::RemoteExecution,
        rule_id: "ruby_open_uri_eval",
        surface: ScanSurface::BashCommand,
        pattern: r#"ruby\s+-e.*require\s+["']open-uri["'].*eval"#,
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    // ========== git_destructive ==========
    RuleDefinition {
        group: RuleGroup::GitDestructive,
        rule_id: "git_push_force",
        surface: ScanSurface::BashCommand,
        pattern: r"git\s+push\s+.*--force",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::GitDestructive,
        rule_id: "git_push_f",
        surface: ScanSurface::BashCommand,
        pattern: r"git\s+push\s+.*\s+-f(\s|$)",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::GitDestructive,
        rule_id: "git_reset_hard",
        surface: ScanSurface::BashCommand,
        pattern: r"git\s+reset\s+--hard",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::GitDestructive,
        rule_id: "git_clean_fdx",
        surface: ScanSurface::BashCommand,
        pattern: r"git\s+clean\s+.*-[a-z]*fdx",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::GitDestructive,
        rule_id: "git_push_delete",
        surface: ScanSurface::BashCommand,
        pattern: r"git\s+push\s+.*--delete",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::GitDestructive,
        rule_id: "git_branch_delete_force",
        surface: ScanSurface::BashCommand,
        pattern: r"git\s+branch\s+-D\s+",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::GitDestructive,
        rule_id: "git_reflog_expire",
        surface: ScanSurface::BashCommand,
        pattern: r"git\s+reflog\s+expire\s+--expire=now",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::GitDestructive,
        rule_id: "git_gc_prune",
        surface: ScanSurface::BashCommand,
        pattern: r"git\s+gc\s+--prune=now",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::GitDestructive,
        rule_id: "rm_git_dir",
        surface: ScanSurface::BashCommand,
        pattern: r"rm\s+-rf\s+\.git",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::GitDestructive,
        rule_id: "git_config_unset_all",
        surface: ScanSurface::BashCommand,
        pattern: r"git\s+config\s+--global\s+--unset-all",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    // ========== system_sabotage ==========
    RuleDefinition {
        group: RuleGroup::SystemSabotage,
        rule_id: "rmmod",
        surface: ScanSurface::BashCommand,
        pattern: r"(sudo\s+)?rmmod\s+",
        platforms: PlatformApplicability::LinuxOnly,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::SystemSabotage,
        rule_id: "modprobe_remove",
        surface: ScanSurface::BashCommand,
        pattern: r"(sudo\s+)?modprobe\s+-r\s+",
        platforms: PlatformApplicability::LinuxOnly,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::SystemSabotage,
        rule_id: "kill_all_processes",
        surface: ScanSurface::BashCommand,
        pattern: r"kill\s+-9\s+-1",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::SystemSabotage,
        rule_id: "systemctl_disable",
        surface: ScanSurface::BashCommand,
        pattern: r"(sudo\s+)?systemctl\s+disable\s+--now",
        platforms: PlatformApplicability::LinuxOnly,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::SystemSabotage,
        rule_id: "fork_bomb_bash",
        surface: ScanSurface::BashCommand,
        pattern: r":\(\)\s*\{\s*:\s*\|\s*:\s*&\s*\}\s*;",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::SystemSabotage,
        rule_id: "fork_bomb_perl",
        surface: ScanSurface::BashCommand,
        pattern: r"perl\s+-e\s+.*fork\s+while",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::SystemSabotage,
        rule_id: "sysrq_trigger",
        surface: ScanSurface::BashCommand,
        pattern: r"echo\s+[bcdeghimnpstuvw]\s*>\s*/proc/sysrq-trigger",
        platforms: PlatformApplicability::LinuxOnly,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::SystemSabotage,
        rule_id: "dev_mem_overwrite",
        surface: ScanSurface::BashCommand,
        pattern: r"cat\s+/dev/zero\s*>\s*/dev/mem",
        platforms: PlatformApplicability::LinuxOnly,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::SystemSabotage,
        rule_id: "csrutil_disable",
        surface: ScanSurface::BashCommand,
        pattern: r"(sudo\s+)?csrutil\s+disable",
        platforms: PlatformApplicability::MacOsOnly,
        supports_allow_paths: false,
    },
    // ========== network_sabotage ==========
    RuleDefinition {
        group: RuleGroup::NetworkSabotage,
        rule_id: "iptables_flush",
        surface: ScanSurface::BashCommand,
        pattern: r"(sudo\s+)?iptables\s+-F",
        platforms: PlatformApplicability::LinuxOnly,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::NetworkSabotage,
        rule_id: "ufw_disable",
        surface: ScanSurface::BashCommand,
        pattern: r"(sudo\s+)?ufw\s+disable",
        platforms: PlatformApplicability::LinuxOnly,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::NetworkSabotage,
        rule_id: "ip_link_down",
        surface: ScanSurface::BashCommand,
        pattern: r"(sudo\s+)?ip\s+link\s+set\s+.*\s+down",
        platforms: PlatformApplicability::LinuxOnly,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::NetworkSabotage,
        rule_id: "authorized_keys_wipe",
        surface: ScanSurface::BashCommand,
        pattern: r#"echo\s+["']?["']?\s*>\s*~/.ssh/authorized_keys"#,
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::NetworkSabotage,
        rule_id: "passwd_lock_root",
        surface: ScanSurface::BashCommand,
        pattern: r"(sudo\s+)?passwd\s+-l\s+root",
        platforms: PlatformApplicability::LinuxOnly,
        supports_allow_paths: false,
    },
    // ========== container_cloud ==========
    RuleDefinition {
        group: RuleGroup::ContainerCloud,
        rule_id: "docker_system_prune",
        surface: ScanSurface::BashCommand,
        pattern: r"docker\s+system\s+prune\s+-a",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::ContainerCloud,
        rule_id: "kubectl_delete_namespaces",
        surface: ScanSurface::BashCommand,
        pattern: r"kubectl\s+delete\s+namespaces\s+--all",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::ContainerCloud,
        rule_id: "docker_rm_all",
        surface: ScanSurface::BashCommand,
        pattern: r"docker\s+rm\s+-f\s+\$\(docker\s+ps\s+-aq\)",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::ContainerCloud,
        rule_id: "aws_terminate_instances",
        surface: ScanSurface::BashCommand,
        pattern: r"aws\s+ec2\s+terminate-instances",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::ContainerCloud,
        rule_id: "aws_s3_remove_bucket",
        surface: ScanSurface::BashCommand,
        pattern: r"aws\s+s3\s+rb\s+.*--force",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::ContainerCloud,
        rule_id: "gcloud_projects_delete",
        surface: ScanSurface::BashCommand,
        pattern: r"gcloud\s+projects\s+delete",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    // ========== database_nukes ==========
    RuleDefinition {
        group: RuleGroup::DatabaseNukes,
        rule_id: "mysql_drop_database",
        surface: ScanSurface::BashCommand,
        pattern: r#"mysql\s+.*-e\s+["']DROP\s+DATABASE"#,
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::DatabaseNukes,
        rule_id: "redis_flushall",
        surface: ScanSurface::BashCommand,
        pattern: r"redis-cli\s+flushall",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::DatabaseNukes,
        rule_id: "npm_uninstall_global",
        surface: ScanSurface::BashCommand,
        pattern: r"npm\s+un.*\s+-g",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::DatabaseNukes,
        rule_id: "history_clear",
        surface: ScanSurface::BashCommand,
        pattern: r"history\s+-c\s+&&\s+rm\s+~/\.bash_history",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::DatabaseNukes,
        rule_id: "crontab_remove",
        surface: ScanSurface::BashCommand,
        pattern: r"crontab\s+-r",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::DatabaseNukes,
        rule_id: "userdel_recursive",
        surface: ScanSurface::BashCommand,
        pattern: r"(sudo\s+)?userdel\s+-r\s+",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    // ========== obfuscated_execution ==========
    RuleDefinition {
        group: RuleGroup::ObfuscatedExecution,
        rule_id: "base64_decode_shell",
        surface: ScanSurface::BashCommand,
        pattern: r"echo\s+.*\|\s*base64\s+-d\s*\|\s*(sh|bash)",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::ObfuscatedExecution,
        rule_id: "xxd_decode_shell",
        surface: ScanSurface::BashCommand,
        pattern: r"printf\s+.*\|\s*xxd\s+-r\s+-p\s*\|\s*bash",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::ObfuscatedExecution,
        rule_id: "eval_echo",
        surface: ScanSurface::BashCommand,
        pattern: r"eval\s+\$\(echo\s+",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::ObfuscatedExecution,
        rule_id: "python_exec_decode",
        surface: ScanSurface::BashCommand,
        pattern: r"python.*-c.*exec\(.*(decode|decompress)",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::ObfuscatedExecution,
        rule_id: "alias_trojan",
        surface: ScanSurface::BashCommand,
        pattern: r"alias\s+\w+=.*rm\s+-rf",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    // ========== prompt_injection (MCP) ==========
    RuleDefinition {
        group: RuleGroup::PromptInjection,
        rule_id: "ignore_previous_instructions",
        surface: ScanSurface::McpResponse,
        pattern: r"(?i)(ignore|skip|disregard)\s+(all\s+)?(previous|prior)\s+instructions",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::PromptInjection,
        rule_id: "system_update_injection",
        surface: ScanSurface::McpResponse,
        pattern: r"(?i)(assistant|system)\s+update:\s+the\s+task\s+has\s+changed",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::PromptInjection,
        rule_id: "fake_authorization",
        surface: ScanSurface::McpResponse,
        pattern: r"(?i)user\s+has\s+authorized\s+the\s+following",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::PromptInjection,
        rule_id: "tool_poisoning_delete",
        surface: ScanSurface::McpResponse,
        pattern: r#"(?i)call_tool\(.*["']delete"#,
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::PromptInjection,
        rule_id: "privilege_injection",
        surface: ScanSurface::McpResponse,
        pattern: r#"(?i)["'](admin|root|superuser)["']\s*:\s*true"#,
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::PromptInjection,
        rule_id: "safety_bypass_flags",
        surface: ScanSurface::McpResponse,
        pattern: r"(?i)--no-preserve-root|--force-yes",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::PromptInjection,
        rule_id: "webhook_exfiltration",
        surface: ScanSurface::McpResponse,
        pattern: r"(?i)webhook\.site/[a-zA-Z0-9-]+",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::PromptInjection,
        rule_id: "tag_escape",
        surface: ScanSurface::McpResponse,
        pattern: r"(?i)</system>|\[/INST\]|</s>",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::PromptInjection,
        rule_id: "context_delimiter_fake",
        surface: ScanSurface::McpResponse,
        pattern: r"(?i)---END\s+OF\s+CONTEXT---",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::PromptInjection,
        rule_id: "new_task_injection",
        surface: ScanSurface::McpResponse,
        pattern: r"(?i)\n\nSystem:\s+New\s+Task:",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    // ========== credential_exfiltration ==========
    RuleDefinition {
        group: RuleGroup::CredentialExfiltration,
        rule_id: "grep_credentials",
        surface: ScanSurface::BashCommand,
        pattern: r#"grep\s+-rE\s+.*["']?(PASSWORD|API_KEY|SECRET|TOKEN)"#,
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::CredentialExfiltration,
        rule_id: "tar_network_stream",
        surface: ScanSurface::BashCommand,
        pattern: r"tar\s+-czf\s+-\s+.*\|\s*nc\s+",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::CredentialExfiltration,
        rule_id: "mail_urandom_flood",
        surface: ScanSurface::BashCommand,
        pattern: r"cat\s+/dev/urandom\s*\|\s*mail\s+",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::CredentialExfiltration,
        rule_id: "fetch_url_exfiltrate",
        surface: ScanSurface::McpResponse,
        pattern: r"(?i)fetch_url\(.*exfiltrate.*data=",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    RuleDefinition {
        group: RuleGroup::CredentialExfiltration,
        rule_id: "send_email_attacker",
        surface: ScanSurface::McpResponse,
        pattern: r"(?i)send_email\(.*to:.*attacker@",
        platforms: PlatformApplicability::All,
        supports_allow_paths: false,
    },
    // ========== sensitive_paths ==========
    // NOTE: Sensitive paths are handled separately via prefix matching in path.rs,
    // not via regex. This group exists for configuration purposes but has no
    // catalog entries.
];

/// Returns rules filtered for the given platform.
pub fn rules_for_platform(platform: ProtectPlatform) -> Vec<&'static RuleDefinition> {
    CATALOG
        .iter()
        .filter(|rule| match rule.platforms {
            PlatformApplicability::All => true,
            PlatformApplicability::MacOsOnly => platform == ProtectPlatform::MacOs,
            PlatformApplicability::LinuxOnly => platform == ProtectPlatform::Linux,
        })
        .collect()
}

#[cfg(test)]
mod tests;
