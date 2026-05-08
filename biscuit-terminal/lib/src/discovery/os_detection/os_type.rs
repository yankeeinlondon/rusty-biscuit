use std::env;

use super::types::OsType;

/// Detect the current operating system type.
///
/// Uses `std::env::consts::OS` for reliable compile-time detection,
/// mapping to the appropriate `OsType` variant.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::os_detection::{detect_os_type, OsType};
///
/// let os = detect_os_type();
/// // On macOS:
/// // assert_eq!(os, OsType::MacOS);
/// ```
pub fn detect_os_type() -> OsType {
    match env::consts::OS {
        "windows" => OsType::Windows,
        "linux" => {
            // Check for Android (Linux kernel but different userland)
            if env::var("ANDROID_ROOT").is_ok() || env::var("ANDROID_DATA").is_ok() {
                OsType::Android
            } else {
                OsType::Linux
            }
        }
        "macos" => OsType::MacOS,
        "freebsd" => OsType::FreeBSD,
        "netbsd" => OsType::NetBSD,
        "openbsd" => OsType::OpenBSD,
        "dragonfly" => OsType::DragonFly,
        "illumos" | "solaris" => OsType::Illumos,
        "ios" => OsType::Ios,
        "android" => OsType::Android,
        _ => OsType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_os_type_returns_valid_variant() {
        let os = detect_os_type();
        #[cfg(target_os = "macos")]
        assert_eq!(os, OsType::MacOS);
        #[cfg(target_os = "linux")]
        assert_eq!(os, OsType::Linux);
        #[cfg(target_os = "windows")]
        assert_eq!(os, OsType::Windows);
        let _ = os;
    }
}
