use super::types::LinuxFamily;

/// Infer the Linux distribution family from its ID.
///
/// Maps distribution IDs to their family for package manager detection
/// and system-specific behavior.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::os_detection::{infer_linux_family, LinuxFamily};
///
/// assert_eq!(infer_linux_family("ubuntu"), LinuxFamily::Debian);
/// assert_eq!(infer_linux_family("fedora"), LinuxFamily::RedHat);
/// assert_eq!(infer_linux_family("arch"), LinuxFamily::Arch);
/// assert_eq!(infer_linux_family("alpine"), LinuxFamily::Alpine);
/// ```
pub fn infer_linux_family(id: &str) -> LinuxFamily {
    let id_lower = id.to_lowercase();

    // Debian family (apt/dpkg)
    if matches!(
        id_lower.as_str(),
        "debian"
            | "ubuntu"
            | "linuxmint"
            | "mint"
            | "pop"
            | "pop_os"
            | "elementary"
            | "elementaryos"
            | "zorin"
            | "zorinos"
            | "kali"
            | "parrot"
            | "raspbian"
            | "pureos"
            | "deepin"
            | "mx"
            | "mxlinux"
            | "lmde"
            | "bunsenlabs"
            | "antix"
            | "sparky"
            | "devuan"
            | "tails"
    ) {
        return LinuxFamily::Debian;
    }

    // Red Hat family (dnf/yum/rpm)
    if matches!(
        id_lower.as_str(),
        "fedora"
            | "rhel"
            | "centos"
            | "rocky"
            | "rockylinux"
            | "almalinux"
            | "alma"
            | "ol"
            | "oracle"
            | "oraclelinux"
            | "scientific"
            | "springdale"
            | "clearos"
            | "amazon"
            | "amzn"
            | "mageia"
            | "openmandriva"
            | "nobara"
    ) {
        return LinuxFamily::RedHat;
    }

    // Arch family (pacman)
    if matches!(
        id_lower.as_str(),
        "arch"
            | "archlinux"
            | "manjaro"
            | "endeavouros"
            | "endeavour"
            | "garuda"
            | "garudalinux"
            | "artix"
            | "arcolinux"
            | "blackarch"
            | "archcraft"
            | "rebornos"
            | "bluestar"
            | "cachyos"
    ) {
        return LinuxFamily::Arch;
    }

    // SUSE family (zypper)
    if matches!(
        id_lower.as_str(),
        "opensuse"
            | "opensuse-leap"
            | "opensuse-tumbleweed"
            | "suse"
            | "sles"
            | "sled"
            | "opensuse-microos"
            | "gecko"
    ) {
        return LinuxFamily::SUSE;
    }

    // Alpine (apk)
    if id_lower == "alpine" {
        return LinuxFamily::Alpine;
    }

    // Gentoo family (emerge/portage)
    if matches!(
        id_lower.as_str(),
        "gentoo" | "calculate" | "funtoo" | "sabayon" | "redcore"
    ) {
        return LinuxFamily::Gentoo;
    }

    // Void Linux (xbps)
    if id_lower == "void" || id_lower == "voidlinux" {
        return LinuxFamily::Void;
    }

    // NixOS (nix)
    if id_lower == "nixos" {
        return LinuxFamily::NixOS;
    }

    // Slackware family
    if matches!(
        id_lower.as_str(),
        "slackware" | "salix" | "slackel" | "zenwalk" | "porteus"
    ) {
        return LinuxFamily::Slackware;
    }

    // Independent distributions
    if matches!(
        id_lower.as_str(),
        "solus" | "clear-linux-os" | "clearlinux" | "guix" | "chimera" | "kiss"
    ) {
        return LinuxFamily::Independent;
    }

    LinuxFamily::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_linux_family_debian() {
        assert_eq!(infer_linux_family("ubuntu"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("debian"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("linuxmint"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("pop"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("elementary"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("kali"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("raspbian"), LinuxFamily::Debian);
    }

    #[test]
    fn test_infer_linux_family_redhat() {
        assert_eq!(infer_linux_family("fedora"), LinuxFamily::RedHat);
        assert_eq!(infer_linux_family("rhel"), LinuxFamily::RedHat);
        assert_eq!(infer_linux_family("centos"), LinuxFamily::RedHat);
        assert_eq!(infer_linux_family("rocky"), LinuxFamily::RedHat);
        assert_eq!(infer_linux_family("almalinux"), LinuxFamily::RedHat);
        assert_eq!(infer_linux_family("ol"), LinuxFamily::RedHat);
    }

    #[test]
    fn test_infer_linux_family_arch() {
        assert_eq!(infer_linux_family("arch"), LinuxFamily::Arch);
        assert_eq!(infer_linux_family("manjaro"), LinuxFamily::Arch);
        assert_eq!(infer_linux_family("endeavouros"), LinuxFamily::Arch);
        assert_eq!(infer_linux_family("garuda"), LinuxFamily::Arch);
    }

    #[test]
    fn test_infer_linux_family_suse() {
        assert_eq!(infer_linux_family("opensuse"), LinuxFamily::SUSE);
        assert_eq!(infer_linux_family("opensuse-leap"), LinuxFamily::SUSE);
        assert_eq!(infer_linux_family("opensuse-tumbleweed"), LinuxFamily::SUSE);
        assert_eq!(infer_linux_family("sles"), LinuxFamily::SUSE);
    }

    #[test]
    fn test_infer_linux_family_others() {
        assert_eq!(infer_linux_family("alpine"), LinuxFamily::Alpine);
        assert_eq!(infer_linux_family("gentoo"), LinuxFamily::Gentoo);
        assert_eq!(infer_linux_family("void"), LinuxFamily::Void);
        assert_eq!(infer_linux_family("nixos"), LinuxFamily::NixOS);
        assert_eq!(infer_linux_family("slackware"), LinuxFamily::Slackware);
        assert_eq!(infer_linux_family("solus"), LinuxFamily::Independent);
    }

    #[test]
    fn test_infer_linux_family_unknown() {
        assert_eq!(infer_linux_family("unknown_distro"), LinuxFamily::Unknown);
        assert_eq!(infer_linux_family(""), LinuxFamily::Unknown);
        assert_eq!(infer_linux_family("myowndistro"), LinuxFamily::Unknown);
    }

    #[test]
    fn test_infer_linux_family_case_insensitive() {
        assert_eq!(infer_linux_family("Ubuntu"), LinuxFamily::Debian);
        assert_eq!(infer_linux_family("FEDORA"), LinuxFamily::RedHat);
        assert_eq!(infer_linux_family("ARCH"), LinuxFamily::Arch);
    }

    #[test]
    fn test_infer_linux_family_comprehensive() {
        // Test all documented debian family distros
        let debian_ids = [
            "debian",
            "ubuntu",
            "linuxmint",
            "mint",
            "pop",
            "pop_os",
            "elementary",
            "elementaryos",
            "zorin",
            "zorinos",
            "kali",
            "parrot",
            "raspbian",
            "pureos",
            "deepin",
            "mx",
            "mxlinux",
            "lmde",
            "bunsenlabs",
            "antix",
            "sparky",
            "devuan",
            "tails",
        ];
        for id in debian_ids {
            assert_eq!(
                infer_linux_family(id),
                LinuxFamily::Debian,
                "Failed for {}",
                id
            );
        }

        let redhat_ids = [
            "fedora",
            "rhel",
            "centos",
            "rocky",
            "rockylinux",
            "almalinux",
            "alma",
            "ol",
            "oracle",
            "oraclelinux",
            "scientific",
            "springdale",
            "clearos",
            "amazon",
            "amzn",
            "mageia",
            "openmandriva",
            "nobara",
        ];
        for id in redhat_ids {
            assert_eq!(
                infer_linux_family(id),
                LinuxFamily::RedHat,
                "Failed for {}",
                id
            );
        }

        let arch_ids = [
            "arch",
            "archlinux",
            "manjaro",
            "endeavouros",
            "endeavour",
            "garuda",
            "garudalinux",
            "artix",
            "arcolinux",
            "blackarch",
            "archcraft",
            "rebornos",
            "bluestar",
            "cachyos",
        ];
        for id in arch_ids {
            assert_eq!(
                infer_linux_family(id),
                LinuxFamily::Arch,
                "Failed for {}",
                id
            );
        }

        let suse_ids = [
            "opensuse",
            "opensuse-leap",
            "opensuse-tumbleweed",
            "suse",
            "sles",
            "sled",
            "opensuse-microos",
            "gecko",
        ];
        for id in suse_ids {
            assert_eq!(
                infer_linux_family(id),
                LinuxFamily::SUSE,
                "Failed for {}",
                id
            );
        }
    }
}
