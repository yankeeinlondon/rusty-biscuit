

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ModelArchitecture {
    modality: String,
    input_modalities: Vec<String>,
    output_modalities: Vec<String>,
    tokenizer: String,
    instruct_type: Option<String>
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct NumericString(String);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ModelPricing {
    prompt: NumericString,
    completion: NumericString,
    request: NumericString,
    image: NumericString,
    web_search: NumericString,
    internal_reasoning: NumericString
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ModelTopProvider {
    context_length: u32,
    max_completion_tokens: u32,
    is_moderated: bool
}

#[derive(Debug, PartialEq, Clone)]
pub struct ModelDefaultParameters {
    temperature: Option<f32>,
    top_p: Option<f32>,
    frequency_penalty: Option<Value>
}

#[derive(Debug, PartialEq, Clone)]
pub struct ModelPermission {
    created: u32,
    id: String,
    object: String,
    organization: String,
    group: String,
    is_blocking: bool
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
    /// only provided on OpenRouter
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

    /// only provided on Moonshot AI
    root: Option<String>,
    /// only provided on Moonshot AI
    parent: Option<String>,
    /// only provided on Moonshot AI
    permission: Option<Vec<ModelPermission>>,

    object: String,
    created: u32,
    owned_by: String,
    display_name: Option<String>
}
