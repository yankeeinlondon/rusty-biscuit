use thiserror::Error;

/// Errors that can occur when working with headers.
///
/// ## Examples
///
/// ```
/// use schematic_define::HeaderError;
///
/// let error = HeaderError::InvalidHeaderName("Inválid-Héader".to_string());
/// assert!(error.to_string().contains("Invalid header name"));
/// ```
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum HeaderError {
    #[error("Invalid header name: {0}")]
    InvalidHeaderName(String),

    #[error("Invalid header value for header '{0}'")]
    InvalidHeaderValue(String),

    #[error("Environment variable not set: {0}")]
    MissingEnv(String),

    #[error("Missing credential: none of the environment variables were set: {0:?}")]
    MissingCredential(Vec<String>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_error_invalid_header_name() {
        let error = HeaderError::InvalidHeaderName("Inválid-Héader".to_string());
        let msg = error.to_string();

        assert!(msg.contains("Invalid header name"));
        assert!(msg.contains("Inválid-Héader"));
    }

    #[test]
    fn header_error_invalid_header_value() {
        let error = HeaderError::InvalidHeaderValue("X-Custom-Header".to_string());
        let msg = error.to_string();

        assert!(msg.contains("Invalid header value"));
        assert!(msg.contains("X-Custom-Header"));
    }

    #[test]
    fn header_error_missing_env() {
        let error = HeaderError::MissingEnv("API_KEY".to_string());
        let msg = error.to_string();

        assert!(msg.contains("Environment variable not set"));
        assert!(msg.contains("API_KEY"));
    }

    #[test]
    fn header_error_missing_credential() {
        let error = HeaderError::MissingCredential(vec![
            "OPENAI_API_KEY".to_string(),
            "OPENAI_KEY".to_string(),
        ]);
        let msg = error.to_string();

        assert!(msg.contains("Missing credential"));
        assert!(msg.contains("OPENAI_API_KEY"));
        assert!(msg.contains("OPENAI_KEY"));
    }

    #[test]
    fn header_error_clone() {
        let error = HeaderError::MissingEnv("KEY".to_string());
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }

    #[test]
    fn header_error_debug() {
        let error = HeaderError::InvalidHeaderName("Test".to_string());
        let debug = format!("{:?}", error);
        assert!(debug.contains("InvalidHeaderName"));
        assert!(debug.contains("Test"));
    }

    #[test]
    fn header_error_eq() {
        let e1 = HeaderError::MissingEnv("KEY".to_string());
        let e2 = HeaderError::MissingEnv("KEY".to_string());
        let e3 = HeaderError::MissingEnv("OTHER".to_string());

        assert_eq!(e1, e2);
        assert_ne!(e1, e3);
    }
}
