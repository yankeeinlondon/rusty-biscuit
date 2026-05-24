mod collections;
mod common;
mod datasets;
mod discussions;
mod filters;
mod inference;
pub mod models;
mod repos;
mod spaces;
mod stubs;
mod users;
mod webhooks;

pub use collections::*;
pub use common::*;
pub use datasets::*;
pub use discussions::*;
pub use filters::*;
pub use inference::*;
pub use models::*;
pub use repos::*;
pub use spaces::*;
pub use stubs::*;
pub use users::*;
pub use webhooks::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_type_serialization() {
        assert_eq!(
            serde_json::to_string(&RepoType::Model).unwrap(),
            "\"model\""
        );
        assert_eq!(
            serde_json::to_string(&RepoType::Dataset).unwrap(),
            "\"dataset\""
        );
        assert_eq!(
            serde_json::to_string(&RepoType::Space).unwrap(),
            "\"space\""
        );
    }

    #[test]
    fn repo_type_deserialization() {
        let model: RepoType = serde_json::from_str("\"model\"").unwrap();
        assert_eq!(model, RepoType::Model);

        let dataset: RepoType = serde_json::from_str("\"dataset\"").unwrap();
        assert_eq!(dataset, RepoType::Dataset);
    }

    #[test]
    fn pipeline_serialization() {
        assert_eq!(
            serde_json::to_string(&Pipeline::TextGeneration).unwrap(),
            "\"text-generation\""
        );
        assert_eq!(
            serde_json::to_string(&Pipeline::ImageClassification).unwrap(),
            "\"image-classification\""
        );
    }

    #[test]
    fn library_serialization() {
        assert_eq!(
            serde_json::to_string(&Library::Transformers).unwrap(),
            "\"transformers\""
        );
        assert_eq!(
            serde_json::to_string(&Library::SentenceTransformers).unwrap(),
            "\"sentence-transformers\""
        );
        assert_eq!(serde_json::to_string(&Library::Gguf).unwrap(), "\"gguf\"");
    }

    #[test]
    fn sort_direction_serialization() {
        assert_eq!(
            serde_json::to_string(&SortDirection::Ascending).unwrap(),
            "\"1\""
        );
        assert_eq!(
            serde_json::to_string(&SortDirection::Descending).unwrap(),
            "\"-1\""
        );
    }

    #[test]
    fn gated_status_serialization() {
        assert_eq!(
            serde_json::to_string(&GatedStatus::False).unwrap(),
            "\"false\""
        );
        assert_eq!(
            serde_json::to_string(&GatedStatus::Manual).unwrap(),
            "\"manual\""
        );
    }

    #[test]
    fn space_hardware_serialization() {
        assert_eq!(
            serde_json::to_string(&SpaceHardware::CpuBasic).unwrap(),
            "\"cpu-basic\""
        );
        assert_eq!(
            serde_json::to_string(&SpaceHardware::A10gSmall).unwrap(),
            "\"a10g-small\""
        );
    }

    #[test]
    fn model_info_deserialization() {
        let json = r#"{
            "modelId": "bert-base-uncased",
            "sha": "abc123",
            "author": "google",
            "downloads": 1000000,
            "likes": 5000,
            "pipeline_tag": "fill-mask",
            "library_name": "transformers",
            "tags": ["pytorch", "bert"],
            "private": false
        }"#;

        let model: ModelInfo = serde_json::from_str(json).unwrap();
        assert_eq!(model.model_id, "bert-base-uncased");
        assert_eq!(model.author, Some("google".to_string()));
        assert_eq!(model.downloads, 1000000);
        assert_eq!(model.tags, vec!["pytorch", "bert"]);
    }

    #[test]
    fn model_info_with_id_alias() {
        let json = r#"{
            "id": "org/model-name",
            "downloads": 500
        }"#;

        let model: ModelInfo = serde_json::from_str(json).unwrap();
        assert_eq!(model.model_id, "org/model-name");
    }

    #[test]
    fn repo_file_deserialization() {
        let json = r#"{
            "rfilename": "model.safetensors",
            "size": 1073741824,
            "blobId": "abc123"
        }"#;

        let file: RepoFile = serde_json::from_str(json).unwrap();
        assert_eq!(file.rfilename, "model.safetensors");
        assert_eq!(file.size, Some(1073741824));
    }

    #[test]
    fn safetensors_info_deserialization() {
        let json = r#"{
            "total": 2147483648,
            "parameters": {
                "F16": 1073741824,
                "BF16": 536870912
            },
            "sharded": true
        }"#;

        let info: SafetensorsInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.total, Some(2147483648));
        assert_eq!(info.sharded, Some(true));
    }

    #[test]
    fn inference_parameters_serialization() {
        let params = InferenceParameters {
            max_new_tokens: Some(100),
            temperature: Some(0.7),
            top_p: Some(0.95),
            do_sample: Some(true),
            ..Default::default()
        };

        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("\"max_new_tokens\":100"));
        assert!(json.contains("\"temperature\":0.7"));
    }

    #[test]
    fn classification_result_deserialization() {
        let json = r#"{"label": "POSITIVE", "score": 0.9998}"#;

        let result: ClassificationResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.label, "POSITIVE");
        assert!(result.score > 0.99);
    }

    #[test]
    fn who_am_i_deserialization() {
        let json = r#"{
            "type": "user",
            "name": "testuser",
            "fullname": "Test User",
            "emailVerified": true,
            "canPay": false,
            "isPro": true,
            "orgs": [{"name": "test-org", "isPro": false}]
        }"#;

        let response: WhoAmIResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.account_type, "user");
        assert_eq!(response.name, "testuser");
        assert!(response.is_pro);
        assert_eq!(response.orgs.len(), 1);
    }

    #[test]
    fn space_runtime_deserialization() {
        let json = r#"{
            "stage": "RUNNING",
            "hardware": "cpu-basic",
            "gcTimeout": 3600
        }"#;

        let runtime: SpaceRuntime = serde_json::from_str(json).unwrap();
        assert_eq!(runtime.stage, Some(SpaceStage::Running));
        assert_eq!(runtime.hardware, Some(SpaceHardware::CpuBasic));
        assert_eq!(runtime.gc_timeout, Some(3600));
    }

    #[test]
    fn api_error_deserialization() {
        let json = r#"{
            "error": "Model too busy",
            "estimated_time": 20.5
        }"#;

        let error: ApiError = serde_json::from_str(json).unwrap();
        assert_eq!(error.error, "Model too busy");
        assert_eq!(error.estimated_time, Some(20.5));
    }

    #[test]
    fn create_repo_request_serialization() {
        let request = CreateRepoBody {
            name: "my-model".to_string(),
            repo_type: Some(RepoType::Model),
            private: true,
            ..Default::default()
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"name\":\"my-model\""));
        assert!(json.contains("\"type\":\"model\""));
        assert!(json.contains("\"private\":true"));
    }

    #[test]
    fn default_values() {
        assert_eq!(RepoType::default(), RepoType::Model);
        assert_eq!(SortField::default(), SortField::LastModified);
        assert_eq!(SortDirection::default(), SortDirection::Descending);
        assert_eq!(GatedStatus::default(), GatedStatus::False);
        assert_eq!(SpaceHardware::default(), SpaceHardware::CpuBasic);
    }
}
