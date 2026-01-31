//! Auto-generated provider model enum
//!
//! Generated: 2026-01-11T20:35:18.410405+00:00
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
    /// Model: `aqa`
    Aqa,
    /// Model: `deep-research-pro-preview-12-2025`
    Deep__Research__Pro__Preview__12__2025,
    /// Model: `embedding-001`
    Embedding__001,
    /// Model: `embedding-gecko-001`
    Embedding__Gecko__001,
    /// Model: `gemini-2.0-flash`
    Gemini__2_0__Flash,
    /// Model: `gemini-2.0-flash-001`
    Gemini__2_0__Flash__001,
    /// Model: `gemini-2.0-flash-exp`
    Gemini__2_0__Flash__Exp,
    /// Model: `gemini-2.0-flash-exp-image-generation`
    Gemini__2_0__Flash__Exp__Image__Generation,
    /// Model: `gemini-2.0-flash-lite`
    Gemini__2_0__Flash__Lite,
    /// Model: `gemini-2.0-flash-lite-001`
    Gemini__2_0__Flash__Lite__001,
    /// Model: `gemini-2.0-flash-lite-preview`
    Gemini__2_0__Flash__Lite__Preview,
    /// Model: `gemini-2.0-flash-lite-preview-02-05`
    Gemini__2_0__Flash__Lite__Preview__02__05,
    /// Model: `gemini-2.5-computer-use-preview-10-2025`
    Gemini__2_5__Computer__Use__Preview__10__2025,
    /// Model: `gemini-2.5-flash`
    Gemini__2_5__Flash,
    /// Model: `gemini-2.5-flash-image`
    Gemini__2_5__Flash__Image,
    /// Model: `gemini-2.5-flash-image-preview`
    Gemini__2_5__Flash__Image__Preview,
    /// Model: `gemini-2.5-flash-lite`
    Gemini__2_5__Flash__Lite,
    /// Model: `gemini-2.5-flash-lite-preview-09-2025`
    Gemini__2_5__Flash__Lite__Preview__09__2025,
    /// Model: `gemini-2.5-flash-preview-09-2025`
    Gemini__2_5__Flash__Preview__09__2025,
    /// Model: `gemini-2.5-flash-preview-tts`
    Gemini__2_5__Flash__Preview__Tts,
    /// Model: `gemini-2.5-pro`
    Gemini__2_5__Pro,
    /// Model: `gemini-2.5-pro-preview-tts`
    Gemini__2_5__Pro__Preview__Tts,
    /// Model: `gemini-3-flash-preview`
    Gemini__3__Flash__Preview,
    /// Model: `gemini-3-pro-image-preview`
    Gemini__3__Pro__Image__Preview,
    /// Model: `gemini-3-pro-preview`
    Gemini__3__Pro__Preview,
    /// Model: `gemini-embedding-001`
    Gemini__Embedding__001,
    /// Model: `gemini-embedding-exp`
    Gemini__Embedding__Exp,
    /// Model: `gemini-embedding-exp-03-07`
    Gemini__Embedding__Exp__03__07,
    /// Model: `gemini-exp-1206`
    Gemini__Exp__1206,
    /// Model: `gemini-flash-latest`
    Gemini__Flash__Latest,
    /// Model: `gemini-flash-lite-latest`
    Gemini__Flash__Lite__Latest,
    /// Model: `gemini-pro-latest`
    Gemini__Pro__Latest,
    /// Model: `gemini-robotics-er-1.5-preview`
    Gemini__Robotics__Er__1_5__Preview,
    /// Model: `gemma-3-12b-it`
    Gemma__3__12b__It,
    /// Model: `gemma-3-1b-it`
    Gemma__3__1b__It,
    /// Model: `gemma-3-27b-it`
    Gemma__3__27b__It,
    /// Model: `gemma-3-4b-it`
    Gemma__3__4b__It,
    /// Model: `gemma-3n-e2b-it`
    Gemma__3n__E2b__It,
    /// Model: `gemma-3n-e4b-it`
    Gemma__3n__E4b__It,
    /// Model: `imagen-4.0-fast-generate-001`
    Imagen__4_0__Fast__Generate__001,
    /// Model: `imagen-4.0-generate-001`
    Imagen__4_0__Generate__001,
    /// Model: `imagen-4.0-generate-preview-06-06`
    Imagen__4_0__Generate__Preview__06__06,
    /// Model: `imagen-4.0-ultra-generate-001`
    Imagen__4_0__Ultra__Generate__001,
    /// Model: `imagen-4.0-ultra-generate-preview-06-06`
    Imagen__4_0__Ultra__Generate__Preview__06__06,
    /// Model: `nano-banana-pro-preview`
    Nano__Banana__Pro__Preview,
    /// Model: `text-embedding-004`
    Text__Embedding__004,
    /// Model: `veo-2.0-generate-001`
    Veo__2_0__Generate__001,
    /// Model: `veo-3.0-fast-generate-001`
    Veo__3_0__Fast__Generate__001,
    /// Model: `veo-3.0-generate-001`
    Veo__3_0__Generate__001,
    /// Model: `veo-3.1-generate-preview`
    Veo__3_1__Generate__Preview,
    /// Custom model ID not in the predefined list.
    Bespoke(String),
}
