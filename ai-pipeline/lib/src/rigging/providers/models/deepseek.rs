

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, strum::AsRefStr)]
pub enum ProviderModelDeepseek {
    DeepseekChat,
    DeepseekReasoner,
    Bespoke(String)
}

impl ProviderModelDeepseek {
  pub fn model_id(&self) -> &str {
    match self {
      Self::DeepseekChat => "deepseek-chat",
      Self::DeepseekReasoner => "deepseek-reasoner",

      Self::Bespoke(s) => s.as_str()
    }
  }
}
