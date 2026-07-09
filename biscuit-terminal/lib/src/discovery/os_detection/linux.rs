//! Linux distribution detection (os-release/lsb-release/system-release parsers + WSL).

use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use super::family::infer_linux_family;
use super::os_type::detect_os_type;
use super::types::{LinuxDistro, LinuxFamily, OsType};

/// Detect Linux distribution details.
///
/// Parses distribution information from standard files:
/// 1. `/etc/os-release` (most reliable, freedesktop.org standard)
/// 2. `/etc/lsb-release` (fallback for older systems)
/// 3. `/etc/system-release` (fallback for older RHEL/CentOS)
///
/// Returns `None` on non-Linux systems or if detection fails.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::os_detection::detect_linux_distro;
///
/// if let Some(distro) = detect_linux_distro() {
///     println!("Distribution: {} ({})", distro.name, distro.id);
///     if let Some(version) = &distro.version {
///         println!("Version: {}", version);
///     }
/// }
/// ```
pub fn detect_linux_distro() -> Option<LinuxDistro> {
    // Only attempt detection on Linux
    if detect_os_type() != OsType::Linux {
        return None;
    }

    if let Some(distro) = parse_os_release("/etc/os-release") {
        return Some(distro);
    }

    if let Some(distro) = parse_lsb_release("/etc/lsb-release") {
        return Some(distro);
    }

    if let Some(distro) = parse_system_release("/etc/system-release") {
        return Some(distro);
    }

    None
}

/// Parse /etc/os-release file format.
fn parse_os_release<P: AsRef<Path>>(path: P) -> Option<LinuxDistro> {
    let content = fs::read_to_string(path).ok()?;
    parse_os_release_content(&content)
}

/// Parse the content of an os-release formatted file.
fn parse_os_release_content(content: &str) -> Option<LinuxDistro> {
    let mut id = None;
    let mut name = None;
    let mut version = None;
    let mut version_codename = None;
    let mut pretty_name = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let value = value.trim_matches('"').trim_matches('\'');

            match key {
                "ID" => id = Some(value.to_lowercase()),
                "NAME" => name = Some(value.to_string()),
                "VERSION_ID" => version = Some(value.to_string()),
                "VERSION_CODENAME" => version_codename = Some(value.to_string()),
                "PRETTY_NAME" => pretty_name = Some(value.to_string()),
                _ => {}
            }
        }
    }

    let id = id?;
    let family = infer_linux_family(&id);

    Some(LinuxDistro {
        id: id.clone(),
        name: pretty_name.or(name).unwrap_or_else(|| id.clone()),
        version,
        codename: version_codename,
        family,
    })
}

/// Parse /etc/lsb-release file format.
fn parse_lsb_release<P: AsRef<Path>>(path: P) -> Option<LinuxDistro> {
    let content = fs::read_to_string(path).ok()?;
    parse_lsb_release_content(&content)
}

/// Parse the content of an lsb-release formatted file.
fn parse_lsb_release_content(content: &str) -> Option<LinuxDistro> {
    let mut id = None;
    let mut description = None;
    let mut release = None;
    let mut codename = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let value = value.trim_matches('"').trim_matches('\'');

            match key {
                "DISTRIB_ID" => id = Some(value.to_lowercase()),
                "DISTRIB_DESCRIPTION" => description = Some(value.to_string()),
                "DISTRIB_RELEASE" => release = Some(value.to_string()),
                "DISTRIB_CODENAME" => codename = Some(value.to_string()),
                _ => {}
            }
        }
    }

    let id = id?;
    let family = infer_linux_family(&id);

    Some(LinuxDistro {
        id: id.clone(),
        name: description.unwrap_or_else(|| id.clone()),
        version: release,
        codename,
        family,
    })
}

/// Parse /etc/system-release file format.
fn parse_system_release<P: AsRef<Path>>(path: P) -> Option<LinuxDistro> {
    let content = fs::read_to_string(path).ok()?;
    parse_system_release_content(&content)
}

/// Parse the content of a system-release file.
fn parse_system_release_content(content: &str) -> Option<LinuxDistro> {
    let line = content.lines().next()?.trim();
    if line.is_empty() {
        return None;
    }

    let lower = line.to_lowercase();

    let id = if lower.contains("centos") {
        "centos"
    } else if lower.contains("red hat") || lower.contains("rhel") {
        "rhel"
    } else if lower.contains("fedora") {
        "fedora"
    } else if lower.contains("oracle") {
        "ol"
    } else {
        return None;
    };

    let version = extract_version_from_release(line);

    let codename = line
        .rfind('(')
        .and_then(|start| line.rfind(')').map(|end| &line[start + 1..end]))
        .map(|s| s.to_string());

    Some(LinuxDistro {
        id: id.to_string(),
        name: line.to_string(),
        version,
        codename,
        family: LinuxFamily::RedHat,
    })
}

/// Extract version number from a release string.
fn extract_version_from_release(text: &str) -> Option<String> {
    let mut chars = text.chars().peekable();
    let mut version = String::new();
    let mut found_digit = false;

    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            found_digit = true;
            version.push(c);
            while let Some(&next) = chars.peek() {
                if next.is_ascii_digit() || next == '.' {
                    version.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            if !version.is_empty() {
                return Some(version.trim_end_matches('.').to_string());
            }
        }
    }

    if found_digit && !version.is_empty() {
        Some(version.trim_end_matches('.').to_string())
    } else {
        None
    }
}

/// Check if the process is running inside WSL 1 (Windows Subsystem for Linux, version 1).
///
/// WSL 1 exposes the Linux termios API but the underlying Windows console does not honor
/// canonical-mode / `VMIN` / `VTIME` flags. Programs that issue terminal probes (OSC 10/11/12,
/// DSR cursor position, CSI 14 t window size) will block on `read()` until the user presses
/// Enter, because `tcsetattr` silently fails to leave cooked mode. WSL 2 uses a real Linux
/// kernel and does not have this problem.
///
/// The result is cached per-process via `OnceLock`.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::os_detection::is_wsl1;
///
/// if is_wsl1() {
///     println!("WSL 1 detected — skipping terminal probes");
/// }
/// ```
pub fn is_wsl1() -> bool {
    static CACHED: OnceLock<bool> = OnceLock::new();
    *CACHED.get_or_init(|| {
        if detect_os_type() != OsType::Linux {
            return false;
        }
        let Ok(osrelease) = fs::read_to_string("/proc/sys/kernel/osrelease") else {
            return false;
        };
        osrelease.contains("Microsoft") && !osrelease.contains("WSL2")
    })
}

/// Check if the process is running inside WSL 2 (Windows Subsystem for Linux, version 2).
///
/// Unlike WSL 1, WSL 2 runs a genuine Linux kernel inside a lightweight VM, so it does not
/// suffer the cooked-mode terminal-probe hazard that motivates [`is_wsl1`]. This detector
/// exists for *config-location* resolution: under WSL the relevant terminal config often
/// lives on the Windows filesystem (reached via `/mnt/c`), so callers need to distinguish
/// WSL 2 from native Linux, and WSL 1 from WSL 2 (their IPC/interop surfaces differ).
///
/// ## Detection contract
///
/// This is a best-effort heuristic, not a guarantee. A result of `true` is treated as
/// "very likely WSL 2"; `false` means "no WSL 2 signature found", which on an exotic setup
/// could be a false negative. Detection is Linux-only and returns `false` when the WSL 1
/// signature is present (WSL 1 takes precedence). It looks for:
///
/// 1. `microsoft` (or `wsl2`) in `/proc/version`, the marker Microsoft ships in the WSL 2
///    kernel build string.
/// 2. Failing that, the `WSL_INTEROP` or `WSL_DISTRO_NAME` environment variables, which the
///    WSL 2 runtime exports into the distro.
///
/// The result is cached per-process via `OnceLock`.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::os_detection::is_wsl2;
///
/// if is_wsl2() {
///     println!("WSL 2 detected — config may live on the Windows filesystem");
/// }
/// ```
pub fn is_wsl2() -> bool {
    static CACHED: OnceLock<bool> = OnceLock::new();
    *CACHED.get_or_init(|| {
        if detect_os_type() != OsType::Linux {
            return false;
        }
        // WSL 1 and WSL 2 are mutually exclusive; WSL 1's signature wins.
        if is_wsl1() {
            return false;
        }
        if let Ok(version) = fs::read_to_string("/proc/version") {
            let lower = version.to_lowercase();
            if lower.contains("microsoft") || lower.contains("wsl2") {
                return true;
            }
        }
        std::env::var("WSL_INTEROP").is_ok() || std::env::var("WSL_DISTRO_NAME").is_ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_os_release_content_ubuntu() {
        let content = r#"
NAME="Ubuntu"
VERSION="24.04.1 LTS (Noble Numbat)"
ID=ubuntu
ID_LIKE=debian
PRETTY_NAME="Ubuntu 24.04.1 LTS"
VERSION_ID="24.04"
VERSION_CODENAME=noble
HOME_URL="https://www.ubuntu.com/"
"#;
        let distro = parse_os_release_content(content).unwrap();
        assert_eq!(distro.id, "ubuntu");
        assert_eq!(distro.name, "Ubuntu 24.04.1 LTS");
        assert_eq!(distro.version, Some("24.04".to_string()));
        assert_eq!(distro.codename, Some("noble".to_string()));
        assert_eq!(distro.family, LinuxFamily::Debian);
    }

    #[test]
    fn test_parse_os_release_content_fedora() {
        let content = r#"
NAME="Fedora Linux"
VERSION="40 (Workstation Edition)"
ID=fedora
VERSION_ID=40
PRETTY_NAME="Fedora Linux 40 (Workstation Edition)"
"#;
        let distro = parse_os_release_content(content).unwrap();
        assert_eq!(distro.id, "fedora");
        assert_eq!(distro.name, "Fedora Linux 40 (Workstation Edition)");
        assert_eq!(distro.version, Some("40".to_string()));
        assert_eq!(distro.family, LinuxFamily::RedHat);
    }

    #[test]
    fn test_parse_os_release_content_arch() {
        let content = r#"
NAME="Arch Linux"
PRETTY_NAME="Arch Linux"
ID=arch
BUILD_ID=rolling
"#;
        let distro = parse_os_release_content(content).unwrap();
        assert_eq!(distro.id, "arch");
        assert_eq!(distro.name, "Arch Linux");
        assert_eq!(distro.version, None);
        assert_eq!(distro.family, LinuxFamily::Arch);
    }

    #[test]
    fn test_parse_os_release_content_alpine() {
        let content = r#"
NAME="Alpine Linux"
ID=alpine
VERSION_ID=3.19.1
PRETTY_NAME="Alpine Linux v3.19"
"#;
        let distro = parse_os_release_content(content).unwrap();
        assert_eq!(distro.id, "alpine");
        assert_eq!(distro.version, Some("3.19.1".to_string()));
        assert_eq!(distro.family, LinuxFamily::Alpine);
    }

    #[test]
    fn test_parse_os_release_content_missing_id() {
        let content = r#"
NAME="Some Distro"
VERSION="1.0"
"#;
        assert!(parse_os_release_content(content).is_none());
    }

    #[test]
    fn test_parse_lsb_release_content() {
        let content = r#"
DISTRIB_ID=Ubuntu
DISTRIB_RELEASE=22.04
DISTRIB_CODENAME=jammy
DISTRIB_DESCRIPTION="Ubuntu 22.04.3 LTS"
"#;
        let distro = parse_lsb_release_content(content).unwrap();
        assert_eq!(distro.id, "ubuntu");
        assert_eq!(distro.version, Some("22.04".to_string()));
        assert_eq!(distro.codename, Some("jammy".to_string()));
        assert_eq!(distro.family, LinuxFamily::Debian);
    }

    #[test]
    fn test_parse_system_release_content_centos() {
        let content = "CentOS Linux release 7.9.2009 (Core)";
        let distro = parse_system_release_content(content).unwrap();
        assert_eq!(distro.id, "centos");
        assert_eq!(distro.version, Some("7.9.2009".to_string()));
        assert_eq!(distro.codename, Some("Core".to_string()));
        assert_eq!(distro.family, LinuxFamily::RedHat);
    }

    #[test]
    fn test_parse_system_release_content_rhel() {
        let content = "Red Hat Enterprise Linux Server release 7.9 (Maipo)";
        let distro = parse_system_release_content(content).unwrap();
        assert_eq!(distro.id, "rhel");
        assert_eq!(distro.version, Some("7.9".to_string()));
        assert_eq!(distro.codename, Some("Maipo".to_string()));
        assert_eq!(distro.family, LinuxFamily::RedHat);
    }

    #[test]
    fn test_extract_version_from_release() {
        assert_eq!(
            extract_version_from_release("release 7.9.2009"),
            Some("7.9.2009".to_string())
        );
        assert_eq!(
            extract_version_from_release("version 24.04"),
            Some("24.04".to_string())
        );
        assert_eq!(extract_version_from_release("no version here"), None);
    }

    #[test]
    fn test_detect_linux_distro_on_non_linux() {
        #[cfg(not(target_os = "linux"))]
        assert!(detect_linux_distro().is_none());
    }

    #[test]
    fn test_parse_os_release_content_empty_file() {
        let content = "";
        let distro = parse_os_release_content(content);
        assert!(distro.is_none(), "Empty file should return None");
    }

    #[test]
    fn test_parse_os_release_content_malformed() {
        let content = "NOT_A_VALID_KEY";
        let distro = parse_os_release_content(content);
        assert!(
            distro.is_none(),
            "Malformed content without = should return None"
        );
    }

    #[test]
    fn test_parse_os_release_content_only_comments() {
        let content = r#"
# This is a comment
# Another comment
# ID=fake
"#;
        let distro = parse_os_release_content(content);
        assert!(
            distro.is_none(),
            "File with only comments should return None"
        );
    }

    #[test]
    fn test_parse_os_release_content_whitespace_only() {
        let content = "   \n\t\n   \n";
        let distro = parse_os_release_content(content);
        assert!(distro.is_none(), "Whitespace-only file should return None");
    }

    #[test]
    fn test_parse_os_release_content_quoted_values() {
        let content = r#"ID="ubuntu"
NAME="Ubuntu 24.04.1 LTS"
VERSION_ID="24.04"
VERSION_CODENAME="noble"
"#;
        let distro = parse_os_release_content(content).unwrap();
        assert_eq!(distro.id, "ubuntu");
        assert_eq!(distro.name, "Ubuntu 24.04.1 LTS");
        assert_eq!(distro.version, Some("24.04".to_string()));
        assert_eq!(distro.codename, Some("noble".to_string()));
    }

    #[test]
    fn test_parse_os_release_content_single_quoted_values() {
        let content = r#"ID='ubuntu'
NAME='Ubuntu 24.04.1 LTS'
VERSION_ID='24.04'
"#;
        let distro = parse_os_release_content(content).unwrap();
        assert_eq!(distro.id, "ubuntu");
        assert_eq!(distro.name, "Ubuntu 24.04.1 LTS");
    }

    #[test]
    fn test_parse_os_release_content_mixed_quotes() {
        let content = r#"ID=ubuntu
NAME="Ubuntu 24.04.1 LTS"
VERSION_ID='24.04'
PRETTY_NAME="Ubuntu 24.04.1 LTS"
"#;
        let distro = parse_os_release_content(content).unwrap();
        assert_eq!(distro.id, "ubuntu");
        assert_eq!(distro.version, Some("24.04".to_string()));
    }

    #[test]
    fn test_parse_os_release_content_special_characters() {
        let content = r#"ID=pop
NAME="Pop!_OS"
PRETTY_NAME="Pop!_OS 22.04 LTS"
"#;
        let distro = parse_os_release_content(content).unwrap();
        assert_eq!(distro.id, "pop");
        assert_eq!(distro.name, "Pop!_OS 22.04 LTS");
    }

    #[test]
    fn test_parse_lsb_release_content_empty() {
        let content = "";
        assert!(parse_lsb_release_content(content).is_none());
    }

    #[test]
    fn test_parse_lsb_release_content_missing_id() {
        let content = r#"
DISTRIB_DESCRIPTION="Ubuntu 22.04"
DISTRIB_RELEASE=22.04
"#;
        assert!(parse_lsb_release_content(content).is_none());
    }

    #[test]
    fn test_parse_system_release_content_empty() {
        let content = "";
        assert!(parse_system_release_content(content).is_none());
    }

    #[test]
    fn test_parse_system_release_content_whitespace_only() {
        let content = "   \n";
        assert!(parse_system_release_content(content).is_none());
    }

    #[test]
    fn test_parse_system_release_content_unknown_distro() {
        let content = "Some Unknown Distro release 1.0";
        assert!(parse_system_release_content(content).is_none());
    }

    #[test]
    fn test_parse_system_release_content_oracle() {
        let content = "Oracle Linux Server release 8.6";
        let distro = parse_system_release_content(content).unwrap();
        assert_eq!(distro.id, "ol");
        assert_eq!(distro.version, Some("8.6".to_string()));
        assert_eq!(distro.family, LinuxFamily::RedHat);
    }

    #[test]
    fn test_extract_version_from_release_edge_cases() {
        assert_eq!(
            extract_version_from_release("7.9 release"),
            Some("7.9".to_string())
        );
        assert_eq!(
            extract_version_from_release("version 1.2.3.4.5"),
            Some("1.2.3.4.5".to_string())
        );
        assert_eq!(
            extract_version_from_release("release 7.9."),
            Some("7.9".to_string())
        );
        assert_eq!(
            extract_version_from_release("version 8"),
            Some("8".to_string())
        );
    }

    #[test]
    fn test_is_wsl1_does_not_panic_and_is_cached() {
        let first = is_wsl1();
        let second = is_wsl1();
        assert_eq!(first, second, "is_wsl1 result should be cached");

        #[cfg(not(target_os = "linux"))]
        assert!(!first, "is_wsl1 must be false on non-Linux platforms");
    }

    #[test]
    fn test_is_wsl2_does_not_panic_and_is_cached() {
        let first = is_wsl2();
        let second = is_wsl2();
        assert_eq!(first, second, "is_wsl2 result should be cached");

        #[cfg(not(target_os = "linux"))]
        assert!(!first, "is_wsl2 must be false on non-Linux platforms");
    }

    #[test]
    fn test_wsl1_and_wsl2_are_mutually_exclusive() {
        assert!(
            !(is_wsl1() && is_wsl2()),
            "a host cannot be both WSL 1 and WSL 2"
        );
    }
}
