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
pub(crate) fn map_windows_timezone_to_iana(id: &str) -> Option<String> {
    let iana = match id {
        // US
        "Pacific Standard Time" => "America/Los_Angeles",
        "Mountain Standard Time" => "America/Denver",
        "Mountain Standard Time (Mexico)" => "America/Chihuahua",
        "US Mountain Standard Time" => "America/Phoenix",
        "Central Standard Time" => "America/Chicago",
        "Central Standard Time (Mexico)" => "America/Mexico_City",
        "Canada Central Standard Time" => "America/Regina",
        "Eastern Standard Time" => "America/New_York",
        "US Eastern Standard Time" => "America/Indianapolis",
        "Atlantic Standard Time" => "America/Halifax",
        "Newfoundland Standard Time" => "America/St_Johns",
        "Alaskan Standard Time" => "America/Anchorage",
        "Hawaiian Standard Time" => "Pacific/Honolulu",
        "Samoa Standard Time" => "Pacific/Pago_Pago",
        // South America
        "SA Pacific Standard Time" => "America/Bogota",
        "SA Western Standard Time" => "America/La_Paz",
        "SA Eastern Standard Time" => "America/Cayenne",
        "E. South America Standard Time" => "America/Sao_Paulo",
        "Argentina Standard Time" => "America/Buenos_Aires",
        "Greenland Standard Time" => "America/Godthab",
        // Europe
        "GMT Standard Time" => "Europe/London",
        "W. Europe Standard Time" => "Europe/Berlin",
        "Romance Standard Time" => "Europe/Paris",
        "Central Europe Standard Time" => "Europe/Budapest",
        "Central European Standard Time" => "Europe/Warsaw",
        "E. Europe Standard Time" => "Europe/Chisinau",
        "GTB Standard Time" => "Europe/Bucharest",
        "FLE Standard Time" => "Europe/Kiev",
        "Turkey Standard Time" => "Europe/Istanbul",
        "Russian Standard Time" => "Europe/Moscow",
        // Asia / Oceania
        "China Standard Time" => "Asia/Shanghai",
        "Taipei Standard Time" => "Asia/Taipei",
        "Korea Standard Time" => "Asia/Seoul",
        "Tokyo Standard Time" => "Asia/Tokyo",
        "Singapore Standard Time" => "Asia/Singapore",
        "Malay Peninsula Standard Time" => "Asia/Kuala_Lumpur",
        "West Asia Standard Time" => "Asia/Tashkent",
        "Central Asia Standard Time" => "Asia/Almaty",
        "India Standard Time" => "Asia/Calcutta",
        "Sri Lanka Standard Time" => "Asia/Colombo",
        "Nepal Standard Time" => "Asia/Katmandu",
        "Bangladesh Standard Time" => "Asia/Dhaka",
        "SE Asia Standard Time" => "Asia/Bangkok",
        "W. Australia Standard Time" => "Australia/Perth",
        "AUS Central Standard Time" => "Australia/Darwin",
        "AUS Eastern Standard Time" => "Australia/Sydney",
        "Tasmania Standard Time" => "Australia/Hobart",
        "New Zealand Standard Time" => "Pacific/Auckland",
        // Middle East / Africa
        "Israel Standard Time" => "Asia/Jerusalem",
        "Arab Standard Time" => "Asia/Riyadh",
        "Arabian Standard Time" => "Asia/Dubai",
        "Iran Standard Time" => "Asia/Tehran",
        "Arabic Standard Time" => "Asia/Baghdad",
        "Egypt Standard Time" => "Africa/Cairo",
        "South Africa Standard Time" => "Africa/Johannesburg",
        "W. Central Africa Standard Time" => "Africa/Lagos",
        "E. Africa Standard Time" => "Africa/Nairobi",
        "Morocco Standard Time" => "Africa/Casablanca",
        // Special
        "UTC" => "Etc/UTC",
        "GMT" => "Etc/GMT",
        "Azores Standard Time" => "Atlantic/Azores",
        "Cape Verde Standard Time" => "Atlantic/Cape_Verde",
        "Mid-Atlantic Standard Time" => "Atlantic/South_Georgia",
        _ => return None,
    };
    Some(iana.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_us_mappings() {
        assert_eq!(
            map_windows_timezone_to_iana("Pacific Standard Time"),
            Some("America/Los_Angeles".to_string())
        );
        assert_eq!(
            map_windows_timezone_to_iana("Mountain Standard Time"),
            Some("America/Denver".to_string())
        );
        assert_eq!(
            map_windows_timezone_to_iana("Central Standard Time"),
            Some("America/Chicago".to_string())
        );
        assert_eq!(
            map_windows_timezone_to_iana("Eastern Standard Time"),
            Some("America/New_York".to_string())
        );
    }

    #[test]
    fn test_utc_mapping() {
        assert_eq!(
            map_windows_timezone_to_iana("UTC"),
            Some("Etc/UTC".to_string())
        );
    }

    #[test]
    fn test_european_mappings() {
        assert_eq!(
            map_windows_timezone_to_iana("W. Europe Standard Time"),
            Some("Europe/Berlin".to_string())
        );
        assert_eq!(
            map_windows_timezone_to_iana("GMT Standard Time"),
            Some("Europe/London".to_string())
        );
        assert_eq!(
            map_windows_timezone_to_iana("Romance Standard Time"),
            Some("Europe/Paris".to_string())
        );
    }

    #[test]
    fn test_asian_mappings() {
        assert_eq!(
            map_windows_timezone_to_iana("China Standard Time"),
            Some("Asia/Shanghai".to_string())
        );
        assert_eq!(
            map_windows_timezone_to_iana("Tokyo Standard Time"),
            Some("Asia/Tokyo".to_string())
        );
        assert_eq!(
            map_windows_timezone_to_iana("India Standard Time"),
            Some("Asia/Calcutta".to_string())
        );
    }

    #[test]
    fn test_unknown_id_returns_none() {
        assert_eq!(map_windows_timezone_to_iana("Fake/Timezone"), None);
        assert_eq!(map_windows_timezone_to_iana(""), None);
    }
}
