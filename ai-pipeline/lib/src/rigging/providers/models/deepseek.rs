use model_id::ModelId;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, ModelId)]
pub enum ProviderModelDeepseek {
    DeepseekChat,
    DeepseekReasoner,
    Bespoke(String)
}

