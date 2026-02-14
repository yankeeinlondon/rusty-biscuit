use model_id::ModelId;

#[derive(ModelId, Debug, Clone, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum ProviderOpenRouter {
    OpenAi___Gpt_4o,
    OpenAi___Gpt_4o__Mini,
    Anthropic___Claude_3__Opus,
    Bespoke(String),
}

fn main() {
    // Test aggregator model_id() with provider prefix
    assert_eq!(ProviderOpenRouter::OpenAi___Gpt_4o.model_id(), "openai/gpt.4o");
    assert_eq!(ProviderOpenRouter::OpenAi___Gpt_4o__Mini.model_id(), "openai/gpt.4o-mini");
    assert_eq!(ProviderOpenRouter::Anthropic___Claude_3__Opus.model_id(), "anthropic/claude.3-opus");

    // Test Bespoke
    let custom = ProviderOpenRouter::Bespoke("google/gemini-pro".to_string());
    assert_eq!(custom.model_id(), "google/gemini-pro");

    // Test FromStr
    let parsed: ProviderOpenRouter = "openai/gpt.4o".parse().unwrap();
    assert_eq!(parsed, ProviderOpenRouter::OpenAi___Gpt_4o);

    // Test ALL constant
    assert_eq!(ProviderOpenRouter::ALL.len(), 3);
}
