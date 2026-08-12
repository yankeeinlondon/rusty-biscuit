//! Auto-generated provider model enum
//!
//! Generated: 2026-07-06T22:44:33.958972+00:00
//! Generator: gen-models v0.1.0
//! Provider: Gemini
//!
//! Do not edit manually.

use model_id::ModelId;

/// Models provided by [Google Gemini](<https://ai.google.dev>).
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, ModelId)]
#[model_id_metadata(
    lookup = "super::metadata_generated::MODEL_METADATA",
    returns = "crate::models::model_metadata::ModelMetadata"
)]
pub enum ProviderModelGemini {
    /// Model: `antigravity-preview-05-2026`
    Antigravity__Preview__05__2026,
    /// Model: `aqa`
    Aqa,
    /// Model: `deep-research-max-preview-04-2026`
    Deep__Research__Max__Preview__04__2026,
    /// Model: `deep-research-preview-04-2026`
    Deep__Research__Preview__04__2026,
    /// Model: `deep-research-pro-preview-12-2025`
    Deep__Research__Pro__Preview__12__2025,
    /// Model: `gemini-2.0-flash`
    Gemini__2_0__Flash,
    /// Model: `gemini-2.0-flash-001`
    Gemini__2_0__Flash__001,
    /// Model: `gemini-2.0-flash-lite`
    Gemini__2_0__Flash__Lite,
    /// Model: `gemini-2.0-flash-lite-001`
    Gemini__2_0__Flash__Lite__001,
    /// Model: `gemini-2.5-computer-use-preview-10-2025`
    Gemini__2_5__Computer__Use__Preview__10__2025,
    /// Model: `gemini-2.5-flash`
    Gemini__2_5__Flash,
    /// Model: `gemini-2.5-flash-image`
    Gemini__2_5__Flash__Image,
    /// Model: `gemini-2.5-flash-lite`
    Gemini__2_5__Flash__Lite,
    /// Model: `gemini-2.5-flash-native-audio-latest`
    Gemini__2_5__Flash__Native__Audio__Latest,
    /// Model: `gemini-2.5-flash-preview-tts`
    Gemini__2_5__Flash__Preview__Tts,
    /// Model: `gemini-2.5-pro`
    Gemini__2_5__Pro,
    /// Model: `gemini-2.5-pro-preview-tts`
    Gemini__2_5__Pro__Preview__Tts,
    /// Model: `gemini-3-flash-preview`
    Gemini__3__Flash__Preview,
    /// Model: `gemini-3-pro-image`
    Gemini__3__Pro__Image,
    /// Model: `gemini-3-pro-image-preview`
    Gemini__3__Pro__Image__Preview,
    /// Model: `gemini-3-pro-preview`
    Gemini__3__Pro__Preview,
    /// Model: `gemini-3.1-flash-image`
    Gemini__3_1__Flash__Image,
    /// Model: `gemini-3.1-flash-image-preview`
    Gemini__3_1__Flash__Image__Preview,
    /// Model: `gemini-3.1-flash-lite`
    Gemini__3_1__Flash__Lite,
    /// Model: `gemini-3.1-flash-lite-image`
    Gemini__3_1__Flash__Lite__Image,
    /// Model: `gemini-3.1-flash-lite-preview`
    Gemini__3_1__Flash__Lite__Preview,
    /// Model: `gemini-3.1-flash-tts-preview`
    Gemini__3_1__Flash__Tts__Preview,
    /// Model: `gemini-3.1-pro-preview`
    Gemini__3_1__Pro__Preview,
    /// Model: `gemini-3.1-pro-preview-customtools`
    Gemini__3_1__Pro__Preview__Customtools,
    /// Model: `gemini-3.5-flash`
    Gemini__3_5__Flash,
    /// Model: `gemini-embedding-001`
    Gemini__Embedding__001,
    /// Model: `gemini-embedding-2`
    Gemini__Embedding__2,
    /// Model: `gemini-embedding-2-preview`
    Gemini__Embedding__2__Preview,
    /// Model: `gemini-flash-latest`
    Gemini__Flash__Latest,
    /// Model: `gemini-flash-lite-latest`
    Gemini__Flash__Lite__Latest,
    /// Model: `gemini-omni-flash-preview`
    Gemini__Omni__Flash__Preview,
    /// Model: `gemini-pro-latest`
    Gemini__Pro__Latest,
    /// Model: `gemini-robotics-er-1.5-preview`
    Gemini__Robotics__Er__1_5__Preview,
    /// Model: `gemini-robotics-er-1.6-preview`
    Gemini__Robotics__Er__1_6__Preview,
    /// Model: `gemma-4-26b-a4b-it`
    Gemma__4__26b__A4b__It,
    /// Model: `gemma-4-31b-it`
    Gemma__4__31b__It,
    /// Model: `imagen-4.0-fast-generate-001`
    Imagen__4_0__Fast__Generate__001,
    /// Model: `imagen-4.0-generate-001`
    Imagen__4_0__Generate__001,
    /// Model: `imagen-4.0-ultra-generate-001`
    Imagen__4_0__Ultra__Generate__001,
    /// Model: `lyria-3-clip-preview`
    Lyria__3__Clip__Preview,
    /// Model: `lyria-3-pro-preview`
    Lyria__3__Pro__Preview,
    /// Model: `nano-banana-pro-preview`
    Nano__Banana__Pro__Preview,
    /// Model: `veo-3.1-fast-generate-preview`
    Veo__3_1__Fast__Generate__Preview,
    /// Model: `veo-3.1-generate-preview`
    Veo__3_1__Generate__Preview,
    /// Model: `veo-3.1-lite-generate-preview`
    Veo__3_1__Lite__Generate__Preview,
    /// Custom model ID not in the predefined list.
    Bespoke(String),
}
