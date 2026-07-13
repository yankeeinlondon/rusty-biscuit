use std::fmt;

/// A string value that should be treated as sensitive (passwords, tokens, etc.).
///
/// This wrapper provides exactly two guarantees:
/// - [`Debug`] output is redacted to `SensitiveString("***")`, so the value
///   does not leak through logs or formatted diagnostics.
/// - No [`Display`] impl, so it cannot be printed by accident.
///
/// It offers nothing more. There is no comparison protection: `Eq`/`PartialEq`
/// are simply not derived, which only means values cannot be compared at all —
/// this is not a constant-time comparison and does not address timing attacks.
/// There is no zeroize-on-drop; the plaintext lives in the heap `String` until
/// dropped. Both [`SensitiveString::as_str`] and
/// [`SensitiveString::into_inner`] expose the underlying value in the clear.
///
/// ## Examples
///
/// ```
/// use schematic_define::SensitiveString;
///
/// let secret = SensitiveString::from("my-secret-token");
/// assert_eq!(format!("{:?}", secret), "SensitiveString(\"***\")");
/// ```
///
/// Access the inner value:
///
/// ```
/// use schematic_define::SensitiveString;
///
/// let secret = SensitiveString::from("my-secret-token");
/// assert_eq!(secret.as_str(), "my-secret-token");
/// ```
#[derive(Clone, Default)]
pub struct SensitiveString(String);

impl SensitiveString {
    /// Create a new sensitive string from any string-like value.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::SensitiveString;
    ///
    /// let secret = SensitiveString::new("my-token");
    /// assert_eq!(secret.as_str(), "my-token");
    /// ```
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Get the underlying string value as a string slice.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::SensitiveString;
    ///
    /// let secret = SensitiveString::from("my-token");
    /// assert_eq!(secret.as_str(), "my-token");
    /// ```
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the wrapper and return the inner String.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::SensitiveString;
    ///
    /// let secret = SensitiveString::from("my-token");
    /// let inner = secret.into_inner();
    /// assert_eq!(inner, "my-token");
    /// ```
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Debug for SensitiveString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SensitiveString").field(&"***").finish()
    }
}

impl From<String> for SensitiveString {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for SensitiveString {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitive_string_debug_redacts_value() {
        let secret = SensitiveString::from("super-secret-token");
        let debug_output = format!("{:?}", secret);

        assert_eq!(debug_output, "SensitiveString(\"***\")");
        assert!(!debug_output.contains("super-secret-token"));
    }

    #[test]
    fn sensitive_string_as_str_returns_value() {
        let secret = SensitiveString::from("my-token");
        assert_eq!(secret.as_str(), "my-token");
    }

    #[test]
    fn sensitive_string_into_inner_returns_value() {
        let secret = SensitiveString::from("my-token");
        assert_eq!(secret.into_inner(), "my-token");
    }

    #[test]
    fn sensitive_string_default_is_empty() {
        let secret = SensitiveString::default();
        assert_eq!(secret.as_str(), "");
    }

    #[test]
    fn sensitive_string_clone_works() {
        let secret = SensitiveString::from("token");
        let cloned = secret.clone();
        assert_eq!(secret.as_str(), cloned.as_str());
    }

    #[test]
    fn sensitive_string_from_string() {
        let secret = SensitiveString::from(String::from("token"));
        assert_eq!(secret.as_str(), "token");
    }

    #[test]
    fn sensitive_string_from_str() {
        let secret = SensitiveString::from("token");
        assert_eq!(secret.as_str(), "token");
    }

    #[test]
    fn sensitive_string_new_method() {
        let secret = SensitiveString::new("token");
        assert_eq!(secret.as_str(), "token");
    }
}
