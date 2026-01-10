use std::fmt;

/// A validated Endpoint Identifier.
/// Rules:
/// 1. Must start with an alphabetic character.
/// 2. Remaining characters must be alphanumeric or `_`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EndpointId(String);

#[derive(Debug, PartialEq, Eq)]
pub enum EndpointIdError {
    Empty,
    InvalidStartCharacter,
    InvalidCharacter(char),
}

impl fmt::Display for EndpointIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "EndpointId cannot be empty"),
            Self::InvalidStartCharacter => write!(f, "EndpointId must start with an alphabetic character"),
            Self::InvalidCharacter(c) => write!(f, "EndpointId contains invalid character: '{}'", c),
        }
    }
}

impl std::error::Error for EndpointIdError {}

impl EndpointId {
    /// Creates a new EndpointId from any type that can turn into a String.
    /// Returns a Result.
    pub fn new<S: Into<String>>(id: S) -> Result<Self, EndpointIdError> {
        let s = id.into();
        Self::validate(&s)?;
        Ok(Self(s))
    }

    /// Internal validation logic
    fn validate(s: &str) -> Result<(), EndpointIdError> {
        let mut chars = s.chars();

        // Check first character
        match chars.next() {
            Some(c) if !c.is_alphabetic() => return Err(EndpointIdError::InvalidStartCharacter),
            None => return Err(EndpointIdError::Empty),
            _ => {} // First char is valid
        }

        // Check remaining characters
        for c in chars {
            if !c.is_alphanumeric() && c != '_' {
                return Err(EndpointIdError::InvalidCharacter(c));
            }
        }

        Ok(())
    }

    /// Returns a string slice reference
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// --- Trait Implementations ---

// Implements Into<String> for EndpointId (via From)
impl From<EndpointId> for String {
    fn from(id: EndpointId) -> Self {
        id.0
    }
}

// Implements TryFrom for String
impl TryFrom<String> for EndpointId {
    type Error = EndpointIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

// Implements TryFrom for &str
impl TryFrom<&str> for EndpointId {
    type Error = EndpointIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

// --- Unit Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_creation() {
        assert!(EndpointId::new("User_123").is_ok());
        assert!(EndpointId::new("api_route").is_ok());
        assert!(EndpointId::new("A").is_ok());
        assert!(EndpointId::new(String::from("Test")).is_ok());
    }

    #[test]
    fn test_invalid_start() {
        assert_eq!(EndpointId::new("_id"), Err(EndpointIdError::InvalidStartCharacter));
        assert_eq!(EndpointId::new("1st"), Err(EndpointIdError::InvalidStartCharacter));
    }

    #[test]
    fn test_invalid_characters() {
        assert!(matches!(EndpointId::new("user-id"), Err(EndpointIdError::InvalidCharacter('-'))));
        assert!(matches!(EndpointId::new("user id"), Err(EndpointIdError::InvalidCharacter(' '))));
        assert!(matches!(EndpointId::new("user@home"), Err(EndpointIdError::InvalidCharacter('@'))));
    }

    #[test]
    fn test_empty() {
        assert_eq!(EndpointId::new(""), Err(EndpointIdError::Empty));
    }

    #[test]
    fn test_traits() {
        // Test TryFrom
        let id: EndpointId = "Valid_Id".try_into().unwrap();

        // Test Into<String>
        let s: String = id.into();
        assert_eq!(s, "Valid_Id");
    }
}
