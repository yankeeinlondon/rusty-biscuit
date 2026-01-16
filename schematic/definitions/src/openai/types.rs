//! OpenAI response types.
//!
//! This module contains the data types used in OpenAI API responses.

use serde::{Deserialize, Serialize};

/// An OpenAI model object.
///
/// Describes a model available through the OpenAI API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    /// The model identifier (e.g., "gpt-4").
    pub id: String,
    /// The object type, always "model".
    pub object: String,
    /// Unix timestamp of when the model was created.
    pub created: i64,
    /// The organization that owns the model.
    pub owned_by: String,
}

/// Response from the List Models endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListModelsResponse {
    /// The object type, always "list".
    pub object: String,
    /// List of model objects.
    pub data: Vec<Model>,
}

/// Response from the Delete Model endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteModelResponse {
    /// The model identifier that was deleted.
    pub id: String,
    /// The object type, always "model".
    pub object: String,
    /// Whether the deletion was successful.
    pub deleted: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_schema_serialization() {
        let model = Model {
            id: "gpt-4".to_string(),
            object: "model".to_string(),
            created: 1687882411,
            owned_by: "openai".to_string(),
        };

        let json = serde_json::to_string(&model).unwrap();
        let parsed: Model = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, model.id);
        assert_eq!(parsed.owned_by, model.owned_by);
    }

    #[test]
    fn list_models_response_serialization() {
        let response = ListModelsResponse {
            object: "list".to_string(),
            data: vec![Model {
                id: "gpt-4".to_string(),
                object: "model".to_string(),
                created: 1687882411,
                owned_by: "openai".to_string(),
            }],
        };

        let json = serde_json::to_string(&response).unwrap();
        let parsed: ListModelsResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.object, "list");
        assert_eq!(parsed.data.len(), 1);
    }

    #[test]
    fn delete_model_response_serialization() {
        let response = DeleteModelResponse {
            id: "ft:gpt-4:my-org".to_string(),
            object: "model".to_string(),
            deleted: true,
        };

        let json = serde_json::to_string(&response).unwrap();
        let parsed: DeleteModelResponse = serde_json::from_str(&json).unwrap();

        assert!(parsed.deleted);
        assert_eq!(parsed.id, "ft:gpt-4:my-org");
    }
}
