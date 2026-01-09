
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, strum::AsRefStr)]
pub enum ProviderModelZai {
    Kimi_K2_Thinking,
    Kimi_K2_0905,
    Kimi_K2_0905exacto,
    Kimi_K2free,
    Kimi_K2,
    Kimi_Dev_72b,
    Kimi_K2_0711,
    Kimi_K2_Thinking_Turbo,

    Bespoke(String)
}

impl ProviderModelZai {
    /// Canonical ZAI model id to send over the wire.
    ///
    /// - Known variants return a static string literal.
    /// - `Bespoke(String)` returns a borrow of the stored string.
    pub fn model_id(&self) -> &str {
        match self {
            Self::Kimi_K2_Thinking => "kimi-k2-thinking",
            Self::Kimi_K2_0905 => "kimi-k2-0905",
            Self::Kimi_K2_0905exacto => "kimi-k2-0905exacto",
            Self::Kimi_K2free => "kimi-k2-free",
            Self::Kimi_K2 => "kimi-k2",
            Self::Kimi_Dev_72b => "kimi-dev-72b",
            Self::Kimi_K2_0711 => "kimi-k2-0711",
            Self::Kimi_K2_Thinking_Turbo => "kimi-k2-thinking-turbo",

            Self::Bespoke(s) => s.as_str(),
        }
    }
}
