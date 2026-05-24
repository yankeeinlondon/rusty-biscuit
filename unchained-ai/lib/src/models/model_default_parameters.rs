//! Default inference parameters for LLM models.
//!
//! Captures the default sampling and generation parameters that providers
//! expose per model (e.g., default temperature, top_p). All fields are optional
//! since not all providers expose defaults for every parameter.

/// Default generation parameters for a model.
///
/// These represent the provider-recommended defaults for sampling parameters.
/// They are useful as starting points when constructing inference requests.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModelDefaultParameters {
    /// Default sampling temperature. Higher values produce more random output.
    pub temperature: Option<f32>,

    /// Default nucleus sampling probability mass.
    pub top_p: Option<f32>,

    /// Default top-k sampling limit.
    pub top_k: Option<u32>,

    /// Default frequency penalty. Positive values reduce repetition of tokens.
    pub frequency_penalty: Option<f32>,

    /// Default presence penalty. Positive values encourage discussing new topics.
    pub presence_penalty: Option<f32>,
}

impl ModelDefaultParameters {
    /// Creates a new empty parameters instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_is_default() {
        assert_eq!(
            ModelDefaultParameters::new(),
            ModelDefaultParameters::default()
        );
    }

    #[test]
    fn test_default_all_none() {
        let params = ModelDefaultParameters::default();
        assert_eq!(params.temperature, None);
        assert_eq!(params.top_p, None);
        assert_eq!(params.top_k, None);
        assert_eq!(params.frequency_penalty, None);
        assert_eq!(params.presence_penalty, None);
    }

    #[test]
    fn test_with_values() {
        let params = ModelDefaultParameters {
            temperature: Some(0.7),
            top_p: Some(0.9),
            top_k: Some(50),
            frequency_penalty: Some(0.0),
            presence_penalty: Some(0.0),
        };
        assert_eq!(params.temperature, Some(0.7));
        assert_eq!(params.top_p, Some(0.9));
        assert_eq!(params.top_k, Some(50));
    }
}
