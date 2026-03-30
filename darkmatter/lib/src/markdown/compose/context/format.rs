//! Formatting helpers for context values.

/// Joins items with `", "`.
pub(crate) fn format_csv(items: &[impl AsRef<str>]) -> String {
    items
        .iter()
        .map(|i| i.as_ref())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Joins items as a markdown bullet list (`"- item\n"` per entry).
pub(crate) fn format_md_list(items: &[impl AsRef<str>]) -> String {
    items
        .iter()
        .map(|i| format!("- {}\n", i.as_ref()))
        .collect::<String>()
}

/// Formats a byte count as a human-readable string (e.g., "128 GB").
///
/// Uses binary units (GiB thresholds) but labels as GB/MB/KB for
/// readability in prose contexts.
pub(crate) fn format_bytes(bytes: u64) -> String {
    const GB: u64 = 1_073_741_824;
    const MB: u64 = 1_048_576;
    const KB: u64 = 1_024;

    if bytes >= GB {
        let gb = bytes as f64 / GB as f64;
        if gb.fract().abs() < 0.05 {
            format!("{:.0} GB", gb)
        } else {
            format!("{:.1} GB", gb)
        }
    } else if bytes >= MB {
        format!("{:.0} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} bytes")
    }
}

/// Returns the ordinal suffix for a day number ("st", "nd", "rd", "th").
pub(crate) fn ordinal_suffix(n: u32) -> &'static str {
    if (11..=13).contains(&(n % 100)) {
        "th"
    } else {
        match n % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        }
    }
}

/// Determines the meteorological season from month and day.
///
/// Meteorological seasons:
/// - Spring: March 1 - May 31
/// - Summer: June 1 - August 31
/// - Fall: September 1 - November 30
/// - Winter: December 1 - February 28/29
pub(crate) fn determine_season(month: u32, _day: u32) -> &'static str {
    match month {
        3..=5 => "Spring",
        6..=8 => "Summer",
        9..=11 => "Fall",
        _ => "Winter", // 12, 1, 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_csv() {
        assert_eq!(format_csv(&["a", "b", "c"]), "a, b, c");
        let empty: &[&str] = &[];
        assert_eq!(format_csv(empty), "");
        assert_eq!(format_csv(&["only"]), "only");
    }

    #[test]
    fn test_format_md_list() {
        assert_eq!(format_md_list(&["a", "b"]), "- a\n- b\n");
        let empty: &[&str] = &[];
        assert_eq!(format_md_list(empty), "");
    }

    #[test]
    fn test_ordinal_suffix() {
        assert_eq!(ordinal_suffix(1), "st");
        assert_eq!(ordinal_suffix(2), "nd");
        assert_eq!(ordinal_suffix(3), "rd");
        assert_eq!(ordinal_suffix(4), "th");
        assert_eq!(ordinal_suffix(11), "th");
        assert_eq!(ordinal_suffix(12), "th");
        assert_eq!(ordinal_suffix(13), "th");
        assert_eq!(ordinal_suffix(21), "st");
        assert_eq!(ordinal_suffix(22), "nd");
        assert_eq!(ordinal_suffix(23), "rd");
        assert_eq!(ordinal_suffix(31), "st");
    }

    #[test]
    fn test_determine_season() {
        assert_eq!(determine_season(1, 1), "Winter");
        assert_eq!(determine_season(2, 28), "Winter");
        assert_eq!(determine_season(3, 1), "Spring");
        assert_eq!(determine_season(5, 31), "Spring");
        assert_eq!(determine_season(6, 1), "Summer");
        assert_eq!(determine_season(8, 31), "Summer");
        assert_eq!(determine_season(9, 1), "Fall");
        assert_eq!(determine_season(11, 30), "Fall");
        assert_eq!(determine_season(12, 1), "Winter");
    }
}
