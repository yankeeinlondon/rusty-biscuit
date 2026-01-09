

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, strum::AsRefStr)]
pub enum ProviderModelGemini {
    Gemini_3_Pro_Image_Preview,
    Gemini_3_Pro_Preview,
    Gemini_2_5_Flash_Image,
    Gemini_2_5_Flash_Preview_09_2025,
    Gemini_2_5_Flash_Lite_Preview_09_2025,
    Gemini_2_5_Flash_Image_Preview,
    Gemini_2_5_Flash_Lite,
    Gemma_3n_E2b_Itfree,
    Gemini_2_5_Flash,
    Gemini_2_5_Pro,
    Gemini_2_5_Pro_Preview,
    Gemma_3n_E4b_Itfree,
    Gemma_3n_E4b_It,
    Gemini_2_5_Pro_Preview_05_06,
    Gemma_3_4b_Itfree,
    Gemma_3_4b_It,
    Gemma_3_12b_Itfree,
    Gemma_3_12b_It,
    Gemma_3_27b_Itfree,
    Gemma_3_27b_It,
    Gemini_2_0_Flash_Lite_001,
    Gemini_2_0_Flash_001,
    Gemini_2_0_Flash_Expfree,
    Gemma_2_27b_It,
    Gemma_2_9b_It,
    Gemini_2_0_Flash,
    Gemini_3_Flash_Preview_Free,

    Bespoke(String)
}

impl ProviderModelGemini {
    /// Canonical Gemini model id to send over the wire.
    ///
    /// - Most variants return a static string literal.
    /// - `Bespoke(String)` returns a borrow of the stored string.
    pub fn model_id(&self) -> &str {
        match self {
            // Gemini 3
            Self::Gemini_3_Pro_Image_Preview => "gemini-3-pro-image-preview",
            Self::Gemini_3_Pro_Preview => "gemini-3-pro-preview",
            Self::Gemini_3_Flash_Preview_Free => "gemini-3-flash-preview-free",

            // Gemini 2.5
            Self::Gemini_2_5_Flash_Image => "gemini-2.5-flash-image",
            Self::Gemini_2_5_Flash_Preview_09_2025 => "gemini-2.5-flash-preview-09-2025",
            Self::Gemini_2_5_Flash_Lite_Preview_09_2025 => "gemini-2.5-flash-lite-preview-09-2025",
            Self::Gemini_2_5_Flash_Image_Preview => "gemini-2.5-flash-image-preview",
            Self::Gemini_2_5_Flash_Lite => "gemini-2.5-flash-lite",
            Self::Gemini_2_5_Flash => "gemini-2.5-flash",
            Self::Gemini_2_5_Pro => "gemini-2.5-pro",
            Self::Gemini_2_5_Pro_Preview => "gemini-2.5-pro-preview",
            Self::Gemini_2_5_Pro_Preview_05_06 => "gemini-2.5-pro-preview-05-06",

            // Gemini 2.0
            Self::Gemini_2_0_Flash_Lite_001 => "gemini-2.0-flash-lite-001",
            Self::Gemini_2_0_Flash_001 => "gemini-2.0-flash-001",
            Self::Gemini_2_0_Flash_Expfree => "gemini-2.0-flash-expfree",
            Self::Gemini_2_0_Flash => "gemini-2.0-flash",

            // Gemma 3n
            Self::Gemma_3n_E2b_Itfree => "gemma-3n-e2b-itfree",
            Self::Gemma_3n_E4b_Itfree => "gemma-3n-e4b-itfree",
            Self::Gemma_3n_E4b_It => "gemma-3n-e4b-it",

            // Gemma 3
            Self::Gemma_3_4b_Itfree => "gemma-3-4b-itfree",
            Self::Gemma_3_4b_It => "gemma-3-4b-it",
            Self::Gemma_3_12b_Itfree => "gemma-3-12b-itfree",
            Self::Gemma_3_12b_It => "gemma-3-12b-it",
            Self::Gemma_3_27b_Itfree => "gemma-3-27b-itfree",
            Self::Gemma_3_27b_It => "gemma-3-27b-it",

            // Gemma 2
            Self::Gemma_2_27b_It => "gemma-2-27b-it",
            Self::Gemma_2_9b_It => "gemma-2-9b-it",

            // Custom
            Self::Bespoke(s) => s.as_str(),
        }
    }
}
