/// Check if a string looks like an `owner/repo` shorthand.
///
/// Returns `true` when the string contains exactly one `/` splitting it into
/// two non-empty parts.
pub fn is_owner_repo_shorthand(value: &str) -> bool {
    let parts: Vec<&str> = value.split('/').collect();
    parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty()
}

/// Check whether a version update is a major semver bump.
///
/// Returns `true` when both versions parse as `major.minor.patch` and either:
/// - The actual major is 0 and the latest has a larger minor version, or
/// - The latest has a larger major version.
///
/// Returns `false` for non-semver versions or patch/minor-only bumps.
pub fn is_major_update(actual: &str, latest: &str) -> bool {
    let parse = |v: &str| -> Option<(u64, u64)> {
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() < 3 {
            return None;
        }
        let major = parts[0].parse::<u64>().ok()?;
        let minor = parts[1].parse::<u64>().ok()?;
        parts[2].split(|c: char| !c.is_ascii_digit()).next()?.parse::<u64>().ok()?;
        Some((major, minor))
    };

    let Some((actual_major, actual_minor)) = parse(actual) else {
        return false;
    };
    let Some((latest_major, latest_minor)) = parse(latest) else {
        return false;
    };

    if actual_major == 0 {
        latest_major > 0 || latest_minor > actual_minor
    } else {
        latest_major > actual_major
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn non_semver_returns_false() {
        assert!(!is_major_update("abc", "def"));
        assert!(!is_major_update("1.0", "2.0"));
    }

    #[test]
    fn major_update_examples() {
        assert!(!is_major_update("1.0.0", "1.0.1"));
        assert!(!is_major_update("1.0.0", "1.1.0"));
        assert!(is_major_update("1.0.0", "2.0.0"));
        assert!(is_major_update("0.1.0", "0.2.0"));
        assert!(!is_major_update("0.1.0", "0.1.1"));
        assert!(is_major_update("0.9.0", "1.0.0"));
    }

    proptest! {
        #[test]
        fn stable_major_changes_depend_on_major_component(
            actual_major in 1u64..50,
            actual_minor in 0u64..50,
            actual_patch in 0u64..50,
            latest_major in 1u64..50,
            latest_minor in 0u64..50,
            latest_patch in 0u64..50
        ) {
            let actual = format!("{actual_major}.{actual_minor}.{actual_patch}");
            let latest = format!("{latest_major}.{latest_minor}.{latest_patch}");
            prop_assert_eq!(is_major_update(&actual, &latest), latest_major > actual_major);
        }

        #[test]
        fn pre_one_zero_minor_bump_is_major(
            actual_minor in 0u64..50,
            actual_patch in 0u64..50,
            latest_minor in 0u64..50,
            latest_patch in 0u64..50
        ) {
            let actual = format!("0.{actual_minor}.{actual_patch}");
            let latest = format!("0.{latest_minor}.{latest_patch}");
            prop_assert_eq!(is_major_update(&actual, &latest), latest_minor > actual_minor);
        }

        #[test]
        fn pre_one_zero_to_one_or_more_is_always_major(
            actual_minor in 0u64..50,
            actual_patch in 0u64..50,
            latest_major in 1u64..10,
            latest_minor in 0u64..50,
            latest_patch in 0u64..50
        ) {
            let actual = format!("0.{actual_minor}.{actual_patch}");
            let latest = format!("{latest_major}.{latest_minor}.{latest_patch}");
            prop_assert!(is_major_update(&actual, &latest));
        }
    }

    #[test]
    fn shorthand_detection_examples() {
        assert!(is_owner_repo_shorthand("owner/repo"));
        assert!(is_owner_repo_shorthand("yankeeinlondon/gotcha"));
        assert!(!is_owner_repo_shorthand("origin"));
        assert!(!is_owner_repo_shorthand("owner/"));
        assert!(!is_owner_repo_shorthand("/repo"));
        assert!(!is_owner_repo_shorthand("a/b/c"));
    }
}
