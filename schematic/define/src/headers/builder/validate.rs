use super::super::error::HeaderError;

pub(super) fn validate_header_name(name: &str) -> Result<(), HeaderError> {
    if !name.is_ascii() {
        return Err(HeaderError::InvalidHeaderName(name.to_string()));
    }

    for ch in name.chars() {
        if !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_' {
            return Err(HeaderError::InvalidHeaderName(name.to_string()));
        }
    }

    Ok(())
}
