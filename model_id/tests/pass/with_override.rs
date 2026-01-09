use model_id::ModelId;

#[derive(ModelId, Debug, Clone, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum ProviderWithOverrides {
    // Normal encoding
    Gpt_4o,

    // Override with explicit model ID (doesn't follow naming convention)
    #[model_id("gpt-4-turbo-preview")]
    Gpt4TurboPreview,

    // Override with complex ID
    #[model_id("claude-3-opus-20240229")]
    Claude3Opus,

    Bespoke(String),
}

fn main() {
    // Test normal encoding
    assert_eq!(ProviderWithOverrides::Gpt_4o.model_id(), "gpt.4o");

    // Test override
    assert_eq!(ProviderWithOverrides::Gpt4TurboPreview.model_id(), "gpt-4-turbo-preview");
    assert_eq!(ProviderWithOverrides::Claude3Opus.model_id(), "claude-3-opus-20240229");

    // Test FromStr with overridden IDs
    let parsed: ProviderWithOverrides = "gpt-4-turbo-preview".parse().unwrap();
    assert_eq!(parsed, ProviderWithOverrides::Gpt4TurboPreview);

    let parsed2: ProviderWithOverrides = "claude-3-opus-20240229".parse().unwrap();
    assert_eq!(parsed2, ProviderWithOverrides::Claude3Opus);

    // Test ALL includes overridden variants
    assert_eq!(ProviderWithOverrides::ALL.len(), 3);
}
