//! Static Windows-to-IANA timezone mapping.
//!
//! Mapping table built from the CLDR `windowsZones.xml` territory="001"
//! canonical entries.  Covers the most common Windows timezone IDs;
//! unmapped IDs fall back to the raw Windows string in the caller.

/// Map a Windows timezone ID (as reported by `tzutil /g`) to its canonical
/// IANA equivalent.
///
/// Returns `None` when the ID is not present in the static table, allowing
/// the caller to surface the raw Windows ID as a best-effort fallback.
///
/// Source: CLDR windowsZones.xml, territory="001" mappings.
pub(crate) fn map_windows_timezone_to_iana(id: &str) -> Option<&'static str> {
    match id {
        // US
        "Pacific Standard Time" => Some("America/Los_Angeles"),
        "Mountain Standard Time" => Some("America/Denver"),
        "Mountain Standard Time (Mexico)" => Some("America/Chihuahua"),
        "US Mountain Standard Time" => Some("America/Phoenix"),
        "Central Standard Time" => Some("America/Chicago"),
        "Central Standard Time (Mexico)" => Some("America/Mexico_City"),
        "Canada Central Standard Time" => Some("America/Regina"),
        "Eastern Standard Time" => Some("America/New_York"),
        "US Eastern Standard Time" => Some("America/Indianapolis"),
        "Atlantic Standard Time" => Some("America/Halifax"),
        "Newfoundland Standard Time" => Some("America/St_Johns"),
        "Alaskan Standard Time" => Some("America/Anchorage"),
        "Hawaiian Standard Time" => Some("Pacific/Honolulu"),
        "Samoa Standard Time" => Some("Pacific/Pago_Pago"),
        // South America
        "SA Pacific Standard Time" => Some("America/Bogota"),
        "SA Western Standard Time" => Some("America/La_Paz"),
        "SA Eastern Standard Time" => Some("America/Cayenne"),
        "E. South America Standard Time" => Some("America/Sao_Paulo"),
        "Argentina Standard Time" => Some("America/Buenos_Aires"),
        "Greenland Standard Time" => Some("America/Nuuk"),
        // Europe
        "GMT Standard Time" => Some("Europe/London"),
        "W. Europe Standard Time" => Some("Europe/Berlin"),
        "Romance Standard Time" => Some("Europe/Paris"),
        "Central Europe Standard Time" => Some("Europe/Budapest"),
        "Central European Standard Time" => Some("Europe/Warsaw"),
        "E. Europe Standard Time" => Some("Europe/Chisinau"),
        "GTB Standard Time" => Some("Europe/Bucharest"),
        "FLE Standard Time" => Some("Europe/Kyiv"),
        "Turkey Standard Time" => Some("Europe/Istanbul"),
        "Russian Standard Time" => Some("Europe/Moscow"),
        // Asia / Oceania
        "China Standard Time" => Some("Asia/Shanghai"),
        "Taipei Standard Time" => Some("Asia/Taipei"),
        "Korea Standard Time" => Some("Asia/Seoul"),
        "Tokyo Standard Time" => Some("Asia/Tokyo"),
        "Singapore Standard Time" => Some("Asia/Singapore"),
        "Malay Peninsula Standard Time" => Some("Asia/Kuala_Lumpur"),
        "West Asia Standard Time" => Some("Asia/Tashkent"),
        "Central Asia Standard Time" => Some("Asia/Almaty"),
        "India Standard Time" => Some("Asia/Kolkata"),
        "Sri Lanka Standard Time" => Some("Asia/Colombo"),
        "Nepal Standard Time" => Some("Asia/Kathmandu"),
        "Bangladesh Standard Time" => Some("Asia/Dhaka"),
        "SE Asia Standard Time" => Some("Asia/Bangkok"),
        "W. Australia Standard Time" => Some("Australia/Perth"),
        "AUS Central Standard Time" => Some("Australia/Darwin"),
        "AUS Eastern Standard Time" => Some("Australia/Sydney"),
        "Tasmania Standard Time" => Some("Australia/Hobart"),
        "New Zealand Standard Time" => Some("Pacific/Auckland"),
        // Middle East / Africa
        "Israel Standard Time" => Some("Asia/Jerusalem"),
        "Arab Standard Time" => Some("Asia/Riyadh"),
        "Arabian Standard Time" => Some("Asia/Dubai"),
        "Iran Standard Time" => Some("Asia/Tehran"),
        "Arabic Standard Time" => Some("Asia/Baghdad"),
        "Egypt Standard Time" => Some("Africa/Cairo"),
        "South Africa Standard Time" => Some("Africa/Johannesburg"),
        "W. Central Africa Standard Time" => Some("Africa/Lagos"),
        "E. Africa Standard Time" => Some("Africa/Nairobi"),
        "Morocco Standard Time" => Some("Africa/Casablanca"),
        // Special
        "UTC" => Some("Etc/UTC"),
        "GMT" => Some("Etc/GMT"),
        "Azores Standard Time" => Some("Atlantic/Azores"),
        "Cape Verde Standard Time" => Some("Atlantic/Cape_Verde"),
        "Mid-Atlantic Standard Time" => Some("Atlantic/South_Georgia"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_us_mappings() {
        assert_eq!(
            map_windows_timezone_to_iana("Pacific Standard Time"),
            Some("America/Los_Angeles")
        );
        assert_eq!(
            map_windows_timezone_to_iana("Mountain Standard Time"),
            Some("America/Denver")
        );
        assert_eq!(
            map_windows_timezone_to_iana("Central Standard Time"),
            Some("America/Chicago")
        );
        assert_eq!(
            map_windows_timezone_to_iana("Eastern Standard Time"),
            Some("America/New_York")
        );
    }

    #[test]
    fn test_utc_mapping() {
        assert_eq!(map_windows_timezone_to_iana("UTC"), Some("Etc/UTC"));
    }

    #[test]
    fn test_european_mappings() {
        assert_eq!(
            map_windows_timezone_to_iana("W. Europe Standard Time"),
            Some("Europe/Berlin")
        );
        assert_eq!(
            map_windows_timezone_to_iana("GMT Standard Time"),
            Some("Europe/London")
        );
        assert_eq!(
            map_windows_timezone_to_iana("Romance Standard Time"),
            Some("Europe/Paris")
        );
    }

    #[test]
    fn test_asian_mappings() {
        assert_eq!(
            map_windows_timezone_to_iana("China Standard Time"),
            Some("Asia/Shanghai")
        );
        assert_eq!(
            map_windows_timezone_to_iana("Tokyo Standard Time"),
            Some("Asia/Tokyo")
        );
        assert_eq!(
            map_windows_timezone_to_iana("India Standard Time"),
            Some("Asia/Kolkata")
        );
    }

    #[test]
    fn test_canonical_cldr_names_not_stale_aliases() {
        assert_eq!(
            map_windows_timezone_to_iana("Greenland Standard Time"),
            Some("America/Nuuk"),
            "Should use canonical America/Nuuk, not stale America/Godthab"
        );
        assert_eq!(
            map_windows_timezone_to_iana("FLE Standard Time"),
            Some("Europe/Kyiv"),
            "Should use canonical Europe/Kyiv, not stale Europe/Kiev"
        );
        assert_eq!(
            map_windows_timezone_to_iana("India Standard Time"),
            Some("Asia/Kolkata"),
            "Should use canonical Asia/Kolkata, not stale Asia/Calcutta"
        );
        assert_eq!(
            map_windows_timezone_to_iana("Nepal Standard Time"),
            Some("Asia/Kathmandu"),
            "Should use canonical Asia/Kathmandu, not stale Asia/Katmandu"
        );
    }

    #[test]
    fn test_unknown_id_returns_none() {
        assert_eq!(map_windows_timezone_to_iana("Fake/Timezone"), None);
        assert_eq!(map_windows_timezone_to_iana(""), None);
    }
}
