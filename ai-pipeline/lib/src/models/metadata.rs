use serde_json::Value;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ModelArchitecture {
    modality: String,
    input_modalities: Vec<String>,
    output_modalities: Vec<String>,
    tokenizer: String,
    instruct_type: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct NumericString(String);

/// A datetime string, typically in ISO 8601 format (e.g., `"2025-06-30T00:00:00Z"`).
///
/// Stored as a raw string since different providers may use varying datetime
/// formats. Parse with `chrono` or `time` crate if structured access is needed.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Datetime(pub String);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ModelPricing {
    prompt: NumericString,
    completion: NumericString,
    request: NumericString,
    image: NumericString,
    web_search: NumericString,
    internal_reasoning: NumericString,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ModelTopProvider {
    context_length: u32,
    max_completion_tokens: u32,
    is_moderated: bool,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ModelDefaultParameters {
    temperature: Option<f32>,
    top_p: Option<f32>,
    frequency_penalty: Option<Value>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ModelPermission {
    created: u32,
    id: String,
    object: String,
    organization: String,
    group: String,
    is_blocking: bool,
}


/// Unlike most primary model providers who are quite
/// sparse on metadata, Mistral provides a bunch of
/// boolean flags to help demonstrate the capabilities
/// each model has.
#[derive(Debug, PartialEq, Clone)]
pub struct MistralCapabilities {
    completion_chat: bool,
    function_calling: bool,
    completion_file: bool,
    fine_tuning: bool,
    vision: bool,
    ocr: bool,
    classification: bool,
    moderation: bool,
    audio: bool
}

/// The shape of a provider's model when returned from the
/// OpenAI API `/models` endpoint.
#[derive(Debug, PartialEq, Clone)]
pub struct ModelDefinition {
    id: String,

    /// only provided on OpenRouter
    canonical_slug: Option<String>,
    /// only provided on OpenRouter
    hugging_face_id: Option<String>,
    /// only provided on OpenRouter
    name: Option<String>,
    /// only provided on OpenRouter and Mistral
    description: Option<String>,
    /// only provided on OpenRouter and Moonshot AI
    context_length: Option<u32>,
    /// only provided on OpenRouter
    architecture: Option<ModelArchitecture>,
    /// only provided on OpenRouter
    pricing: Option<ModelPricing>,
    /// only provided on OpenRouter
    top_provider: Option<ModelTopProvider>,
    /// only provided on OpenRouter
    supported_parameters: Option<Vec<String>>,
    /// only provided on OpenRouter
    default_parameters: Option<ModelDefaultParameters>,

    /// only provided by Mistral
    capabilities: Option<MistralCapabilities>,
    /// only provided by Mistral
    max_context_length: Option<u32>,
    /// only provided by Mistral
    aliases: Option<Vec<String>>,
    /// only provided by Mistral
    default_model_temperature: Option<f32>,
    /// only provided by Mistral
    deprecation: Option<String>,
    /// only provided by Mistral
    deprecation_replacement_model: Option<String>,


    /// only provided on Moonshot AI
    root: Option<String>,
    /// only provided on Moonshot AI
    parent: Option<String>,
    /// only provided on Moonshot AI
    permission: Option<Vec<ModelPermission>>,

    object: String,
    created: u32,
    owned_by: String,
    display_name: Option<String>,
}
