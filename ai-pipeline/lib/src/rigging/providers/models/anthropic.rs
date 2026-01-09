

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, strum::AsRefStr)]
pub enum ProviderModelAnthropic {
    ClaudeOpus_4_5_20251101,
    ClaudeHaiku_4_5_20251001,
    ClaudeSonnet_4_5_20250929,
    ClaudeOpus_4_1_20250805,
    ClaudeOpus_4_20250514,
    ClaudeSonnet_4_20250514,
    Claude_3_7_Sonnet_20250219,
    Claude_3_5_Haiku_20241022,
    Claude_3_Haiku_20240307,
    Claude_3_Opus_20240229,
    Bespoke(String)
}

impl ProviderModelAnthropic {
  pub fn model_id(&self) -> &str {
    match self {
      Self::ClaudeOpus_4_5_20251101 => "claude-opus-4.5-20251101",
      Self::ClaudeHaiku_4_5_20251001 => "claude-haiku-4.5-20251001",
      Self::ClaudeSonnet_4_5_20250929 => "claude-sonnet-4.5-20250929",
      Self::ClaudeOpus_4_1_20250805 => "claude-opus-4.1-20250805",
      Self::ClaudeOpus_4_20250514 => "claude-opus-4-20250514",
      Self::ClaudeSonnet_4_20250514 => "claude-sonnet-4-20250514",
      Self::Claude_3_7_Sonnet_20250219 => "claude-3.7-sonnet-20250219",
      Self::Claude_3_5_Haiku_20241022 => "claude-3.5-haiku-20241022",
      Self::Claude_3_Haiku_20240307 => "claude-3-haiku-20240307",
      Self::Claude_3_Opus_20240229 => "claude-3-opus-20240229",
      Self::Bespoke(s) => s.as_str()
    }
  }
}
