use model_id::ModelId;

#[derive(ModelId, Debug, Clone, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum ProviderOpenAi {
    Gpt_4o,
    Gpt_4o__Mini,
    Gpt_3_5__Turbo,
    O1__Preview,
    Bespoke(String),
}

fn main() {
    // Test model_id()
    assert_eq!(ProviderOpenAi::Gpt_4o.model_id(), "gpt.4o");
    assert_eq!(ProviderOpenAi::Gpt_4o__Mini.model_id(), "gpt.4o-mini");
    assert_eq!(ProviderOpenAi::Gpt_3_5__Turbo.model_id(), "gpt.3.5-turbo");
    assert_eq!(ProviderOpenAi::O1__Preview.model_id(), "o1-preview");

    // Test Bespoke
    let custom = ProviderOpenAi::Bespoke("custom-model".to_string());
    assert_eq!(custom.model_id(), "custom-model");

    // Test FromStr
    let parsed: ProviderOpenAi = "gpt.4o".parse().unwrap();
    assert_eq!(parsed, ProviderOpenAi::Gpt_4o);

    // Test FromStr with unknown model falls back to Bespoke
    let unknown: ProviderOpenAi = "unknown-model".parse().unwrap();
    assert_eq!(unknown, ProviderOpenAi::Bespoke("unknown-model".to_string()));

    // Test ALL constant
    assert_eq!(ProviderOpenAi::ALL.len(), 4);
    assert!(ProviderOpenAi::ALL.contains(&ProviderOpenAi::Gpt_4o));
}
