//! Auto-generated provider model enum
//!
//! Generated: 2026-05-07T02:07:03.569579+00:00
//! Generator: gen-models v0.1.0
//! Provider: Mistral
//!
//! Do not edit manually.

use model_id::ModelId;

/// Models provided by [Mistral AI](<https://mistral.ai>).
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, ModelId)]
#[model_id_metadata(
    lookup = "super::metadata_generated::MODEL_METADATA",
    returns = "crate::models::model_metadata::ModelMetadata"
)]
pub enum ProviderModelMistral {
    /// Model: `codestral-2508`
    Codestral__2508,
    /// Model: `codestral-embed`
    Codestral__Embed,
    /// Model: `codestral-embed-2505`
    Codestral__Embed__2505,
    /// Model: `codestral-latest`
    Codestral__Latest,
    /// Model: `devstral-2512`
    Devstral__2512,
    /// Model: `devstral-latest`
    Devstral__Latest,
    /// Model: `devstral-medium-2507`
    Devstral__Medium__2507,
    /// Model: `devstral-medium-latest`
    Devstral__Medium__Latest,
    /// Model: `devstral-small-2507`
    Devstral__Small__2507,
    /// Model: `labs-leanstral-2603`
    Labs__Leanstral__2603,
    /// Model: `magistral-medium-2509`
    Magistral__Medium__2509,
    /// Model: `magistral-medium-latest`
    Magistral__Medium__Latest,
    /// Model: `magistral-small-2509`
    Magistral__Small__2509,
    /// Model: `magistral-small-latest`
    Magistral__Small__Latest,
    /// Model: `ministral-14b-2512`
    Ministral__14b__2512,
    /// Model: `ministral-14b-latest`
    Ministral__14b__Latest,
    /// Model: `ministral-3b-2512`
    Ministral__3b__2512,
    /// Model: `ministral-3b-latest`
    Ministral__3b__Latest,
    /// Model: `ministral-8b-2512`
    Ministral__8b__2512,
    /// Model: `ministral-8b-latest`
    Ministral__8b__Latest,
    /// Model: `mistral-embed`
    Mistral__Embed,
    /// Model: `mistral-embed-2312`
    Mistral__Embed__2312,
    /// Model: `mistral-large-2411`
    Mistral__Large__2411,
    /// Model: `mistral-large-2512`
    Mistral__Large__2512,
    /// Model: `mistral-large-latest`
    Mistral__Large__Latest,
    /// Model: `mistral-large-pixtral-2411`
    Mistral__Large__Pixtral__2411,
    /// Model: `mistral-medium`
    Mistral__Medium,
    /// Model: `mistral-medium-2505`
    Mistral__Medium__2505,
    /// Model: `mistral-medium-2508`
    Mistral__Medium__2508,
    /// Model: `mistral-medium-2604`
    Mistral__Medium__2604,
    /// Model: `mistral-medium-3`
    Mistral__Medium__3,
    /// Model: `mistral-medium-3-5`
    Mistral__Medium__3__5,
    /// Model: `mistral-medium-3.5`
    Mistral__Medium__3_5,
    /// Model: `mistral-medium-c21211-r0-75`
    Mistral__Medium__C21211__R0__75,
    /// Model: `mistral-medium-latest`
    Mistral__Medium__Latest,
    /// Model: `mistral-moderation-2411`
    Mistral__Moderation__2411,
    /// Model: `mistral-moderation-2603`
    Mistral__Moderation__2603,
    /// Model: `mistral-moderation-latest`
    Mistral__Moderation__Latest,
    /// Model: `mistral-ocr-2505`
    Mistral__Ocr__2505,
    /// Model: `mistral-ocr-2512`
    Mistral__Ocr__2512,
    /// Model: `mistral-ocr-latest`
    Mistral__Ocr__Latest,
    /// Model: `mistral-small-2506`
    Mistral__Small__2506,
    /// Model: `mistral-small-2603`
    Mistral__Small__2603,
    /// Model: `mistral-small-latest`
    Mistral__Small__Latest,
    /// Model: `mistral-tiny-2407`
    Mistral__Tiny__2407,
    /// Model: `mistral-tiny-latest`
    Mistral__Tiny__Latest,
    /// Model: `mistral-vibe-cli-fast`
    Mistral__Vibe__Cli__Fast,
    /// Model: `mistral-vibe-cli-latest`
    Mistral__Vibe__Cli__Latest,
    /// Model: `mistral-vibe-cli-with-tools`
    Mistral__Vibe__Cli__With__Tools,
    /// Model: `open-mistral-nemo`
    Open__Mistral__Nemo,
    /// Model: `open-mistral-nemo-2407`
    Open__Mistral__Nemo__2407,
    /// Model: `pixtral-large-2411`
    Pixtral__Large__2411,
    /// Model: `pixtral-large-latest`
    Pixtral__Large__Latest,
    /// Model: `voxtral-mini-2507`
    Voxtral__Mini__2507,
    /// Model: `voxtral-mini-2602`
    Voxtral__Mini__2602,
    /// Model: `voxtral-mini-latest`
    Voxtral__Mini__Latest,
    /// Model: `voxtral-mini-realtime-2602`
    Voxtral__Mini__Realtime__2602,
    /// Model: `voxtral-mini-realtime-latest`
    Voxtral__Mini__Realtime__Latest,
    /// Model: `voxtral-mini-transcribe-2507`
    Voxtral__Mini__Transcribe__2507,
    /// Model: `voxtral-mini-transcribe-realtime-2602`
    Voxtral__Mini__Transcribe__Realtime__2602,
    /// Model: `voxtral-mini-tts-2603`
    Voxtral__Mini__Tts__2603,
    /// Model: `voxtral-mini-tts-latest`
    Voxtral__Mini__Tts__Latest,
    /// Model: `voxtral-small-2507`
    Voxtral__Small__2507,
    /// Model: `voxtral-small-latest`
    Voxtral__Small__Latest,
    /// Custom model ID not in the predefined list.
    Bespoke(String),
}
