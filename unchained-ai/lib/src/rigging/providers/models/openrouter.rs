//! Auto-generated provider model enum
//!
//! Generated: 2026-07-06T22:44:36.054955+00:00
//! Generator: gen-models v0.1.0
//! Provider: OpenRouter
//!
//! Do not edit manually.

use model_id::ModelId;

/// Models provided by [OpenRouter](<https://openrouter.ai>).
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, ModelId)]
#[model_id_metadata(
    lookup = "super::metadata_generated::MODEL_METADATA",
    returns = "crate::models::model_metadata::ModelMetadata"
)]
pub enum ProviderModelOpenRouter {
    /// Model: `ai21/jamba-large-1.7`
    Ai21___Jamba__Large__1_7,
    /// Model: `aion-labs/aion-1.0`
    Aion__Labs___Aion__1_0,
    /// Model: `aion-labs/aion-1.0-mini`
    Aion__Labs___Aion__1_0__Mini,
    /// Model: `aion-labs/aion-2.0`
    Aion__Labs___Aion__2_0,
    /// Model: `aion-labs/aion-rp-llama-3.1-8b`
    Aion__Labs___Aion__Rp__Llama__3_1__8b,
    /// Model: `allenai/olmo-3-32b-think`
    Allenai___Olmo__3__32b__Think,
    /// Model: `amazon/nova-2-lite-v1`
    Amazon___Nova__2__Lite__V1,
    /// Model: `amazon/nova-lite-v1`
    Amazon___Nova__Lite__V1,
    /// Model: `amazon/nova-micro-v1`
    Amazon___Nova__Micro__V1,
    /// Model: `amazon/nova-premier-v1`
    Amazon___Nova__Premier__V1,
    /// Model: `amazon/nova-pro-v1`
    Amazon___Nova__Pro__V1,
    /// Model: `anthracite-org/magnum-v4-72b`
    Anthracite__Org___Magnum__V4__72b,
    /// Model: `anthropic/claude-3-haiku`
    Anthropic___Claude__3__Haiku,
    /// Model: `anthropic/claude-fable-5`
    Anthropic___Claude__Fable__5,
    /// Model: `anthropic/claude-haiku-4.5`
    Anthropic___Claude__Haiku__4_5,
    /// Model: `anthropic/claude-opus-4`
    Anthropic___Claude__Opus__4,
    /// Model: `anthropic/claude-opus-4.1`
    Anthropic___Claude__Opus__4_1,
    /// Model: `anthropic/claude-opus-4.5`
    Anthropic___Claude__Opus__4_5,
    /// Model: `anthropic/claude-opus-4.6`
    Anthropic___Claude__Opus__4_6,
    /// Model: `anthropic/claude-opus-4.7`
    Anthropic___Claude__Opus__4_7,
    /// Model: `anthropic/claude-opus-4.7-fast`
    Anthropic___Claude__Opus__4_7__Fast,
    /// Model: `anthropic/claude-opus-4.8`
    Anthropic___Claude__Opus__4_8,
    /// Model: `anthropic/claude-opus-4.8-fast`
    Anthropic___Claude__Opus__4_8__Fast,
    /// Model: `anthropic/claude-sonnet-4`
    Anthropic___Claude__Sonnet__4,
    /// Model: `anthropic/claude-sonnet-4.5`
    Anthropic___Claude__Sonnet__4_5,
    /// Model: `anthropic/claude-sonnet-4.6`
    Anthropic___Claude__Sonnet__4_6,
    /// Model: `anthropic/claude-sonnet-5`
    Anthropic___Claude__Sonnet__5,
    /// Model: `arcee-ai/coder-large`
    Arcee__Ai___Coder__Large,
    /// Model: `arcee-ai/trinity-large-thinking`
    Arcee__Ai___Trinity__Large__Thinking,
    /// Model: `arcee-ai/trinity-mini`
    Arcee__Ai___Trinity__Mini,
    /// Model: `arcee-ai/virtuoso-large`
    Arcee__Ai___Virtuoso__Large,
    /// Model: `baidu/ernie-4.5-vl-424b-a47b`
    Baidu___Ernie__4_5__Vl__424b__A47b,
    /// Model: `bytedance-seed/seed-1.6`
    Bytedance__Seed___Seed__1_6,
    /// Model: `bytedance-seed/seed-1.6-flash`
    Bytedance__Seed___Seed__1_6__Flash,
    /// Model: `bytedance-seed/seed-2.0-lite`
    Bytedance__Seed___Seed__2_0__Lite,
    /// Model: `bytedance-seed/seed-2.0-mini`
    Bytedance__Seed___Seed__2_0__Mini,
    /// Model: `bytedance/ui-tars-1.5-7b`
    Bytedance___Ui__Tars__1_5__7b,
    /// Model: `cognitivecomputations/dolphin-mistral-24b-venice-edition:free`
    Cognitivecomputations___Dolphin__Mistral__24b__Venice__Edition__Free,
    /// Model: `cohere/command-a`
    Cohere___Command__A,
    /// Model: `cohere/command-r-08-2024`
    Cohere___Command__R__08__2024,
    /// Model: `cohere/command-r-plus-08-2024`
    Cohere___Command__R__Plus__08__2024,
    /// Model: `cohere/command-r7b-12-2024`
    Cohere___Command__R7b__12__2024,
    /// Model: `cohere/north-mini-code:free`
    Cohere___North__Mini__Code__Free,
    /// Model: `deepcogito/cogito-v2.1-671b`
    Deepcogito___Cogito__V2_1__671b,
    /// Model: `deepseek/deepseek-chat`
    Deepseek___Deepseek__Chat,
    /// Model: `deepseek/deepseek-chat-v3-0324`
    Deepseek___Deepseek__Chat__V3__0324,
    /// Model: `deepseek/deepseek-chat-v3.1`
    Deepseek___Deepseek__Chat__V3_1,
    /// Model: `deepseek/deepseek-r1`
    Deepseek___Deepseek__R1,
    /// Model: `deepseek/deepseek-r1-0528`
    Deepseek___Deepseek__R1__0528,
    /// Model: `deepseek/deepseek-r1-distill-llama-70b`
    Deepseek___Deepseek__R1__Distill__Llama__70b,
    /// Model: `deepseek/deepseek-v3.1-terminus`
    Deepseek___Deepseek__V3_1__Terminus,
    /// Model: `deepseek/deepseek-v3.2`
    Deepseek___Deepseek__V3_2,
    /// Model: `deepseek/deepseek-v3.2-exp`
    Deepseek___Deepseek__V3_2__Exp,
    /// Model: `deepseek/deepseek-v4-flash`
    Deepseek___Deepseek__V4__Flash,
    /// Model: `deepseek/deepseek-v4-pro`
    Deepseek___Deepseek__V4__Pro,
    /// Model: `google/gemini-2.5-flash`
    Google___Gemini__2_5__Flash,
    /// Model: `google/gemini-2.5-flash-image`
    Google___Gemini__2_5__Flash__Image,
    /// Model: `google/gemini-2.5-flash-lite`
    Google___Gemini__2_5__Flash__Lite,
    /// Model: `google/gemini-2.5-flash-lite-preview-09-2025`
    Google___Gemini__2_5__Flash__Lite__Preview__09__2025,
    /// Model: `google/gemini-2.5-pro`
    Google___Gemini__2_5__Pro,
    /// Model: `google/gemini-2.5-pro-preview`
    Google___Gemini__2_5__Pro__Preview,
    /// Model: `google/gemini-2.5-pro-preview-05-06`
    Google___Gemini__2_5__Pro__Preview__05__06,
    /// Model: `google/gemini-3-flash-preview`
    Google___Gemini__3__Flash__Preview,
    /// Model: `google/gemini-3-pro-image`
    Google___Gemini__3__Pro__Image,
    /// Model: `google/gemini-3-pro-image-preview`
    Google___Gemini__3__Pro__Image__Preview,
    /// Model: `google/gemini-3.1-flash-image`
    Google___Gemini__3_1__Flash__Image,
    /// Model: `google/gemini-3.1-flash-image-preview`
    Google___Gemini__3_1__Flash__Image__Preview,
    /// Model: `google/gemini-3.1-flash-lite`
    Google___Gemini__3_1__Flash__Lite,
    /// Model: `google/gemini-3.1-flash-lite-image`
    Google___Gemini__3_1__Flash__Lite__Image,
    /// Model: `google/gemini-3.1-flash-lite-preview`
    Google___Gemini__3_1__Flash__Lite__Preview,
    /// Model: `google/gemini-3.1-pro-preview`
    Google___Gemini__3_1__Pro__Preview,
    /// Model: `google/gemini-3.1-pro-preview-customtools`
    Google___Gemini__3_1__Pro__Preview__Customtools,
    /// Model: `google/gemini-3.5-flash`
    Google___Gemini__3_5__Flash,
    /// Model: `google/gemma-2-27b-it`
    Google___Gemma__2__27b__It,
    /// Model: `google/gemma-3-12b-it`
    Google___Gemma__3__12b__It,
    /// Model: `google/gemma-3-27b-it`
    Google___Gemma__3__27b__It,
    /// Model: `google/gemma-3-4b-it`
    Google___Gemma__3__4b__It,
    /// Model: `google/gemma-3n-e4b-it`
    Google___Gemma__3n__E4b__It,
    /// Model: `google/gemma-4-26b-a4b-it`
    Google___Gemma__4__26b__A4b__It,
    /// Model: `google/gemma-4-26b-a4b-it:free`
    Google___Gemma__4__26b__A4b__It__Free,
    /// Model: `google/gemma-4-31b-it`
    Google___Gemma__4__31b__It,
    /// Model: `google/gemma-4-31b-it:free`
    Google___Gemma__4__31b__It__Free,
    /// Model: `google/lyria-3-clip-preview`
    Google___Lyria__3__Clip__Preview,
    /// Model: `google/lyria-3-pro-preview`
    Google___Lyria__3__Pro__Preview,
    /// Model: `gryphe/mythomax-l2-13b`
    Gryphe___Mythomax__L2__13b,
    /// Model: `ibm-granite/granite-4.0-h-micro`
    Ibm__Granite___Granite__4_0__H__Micro,
    /// Model: `ibm-granite/granite-4.1-8b`
    Ibm__Granite___Granite__4_1__8b,
    /// Model: `inception/mercury-2`
    Inception___Mercury__2,
    /// Model: `inclusionai/ling-2.6-1t`
    Inclusionai___Ling__2_6__1t,
    /// Model: `inclusionai/ling-2.6-flash`
    Inclusionai___Ling__2_6__Flash,
    /// Model: `inclusionai/ring-2.6-1t`
    Inclusionai___Ring__2_6__1t,
    /// Model: `inflection/inflection-3-pi`
    Inflection___Inflection__3__Pi,
    /// Model: `inflection/inflection-3-productivity`
    Inflection___Inflection__3__Productivity,
    /// Model: `kwaipilot/kat-coder-pro-v2`
    Kwaipilot___Kat__Coder__Pro__V2,
    /// Model: `liquid/lfm-2-24b-a2b`
    Liquid___Lfm__2__24b__A2b,
    /// Model: `liquid/lfm-2.5-1.2b-instruct:free`
    Liquid___Lfm__2_5__1_2b__Instruct__Free,
    /// Model: `liquid/lfm-2.5-1.2b-thinking:free`
    Liquid___Lfm__2_5__1_2b__Thinking__Free,
    /// Model: `mancer/weaver`
    Mancer___Weaver,
    /// Model: `meta-llama/llama-3-8b-instruct`
    Meta__Llama___Llama__3__8b__Instruct,
    /// Model: `meta-llama/llama-3.1-70b-instruct`
    Meta__Llama___Llama__3_1__70b__Instruct,
    /// Model: `meta-llama/llama-3.1-8b-instruct`
    Meta__Llama___Llama__3_1__8b__Instruct,
    /// Model: `meta-llama/llama-3.2-11b-vision-instruct`
    Meta__Llama___Llama__3_2__11b__Vision__Instruct,
    /// Model: `meta-llama/llama-3.2-1b-instruct`
    Meta__Llama___Llama__3_2__1b__Instruct,
    /// Model: `meta-llama/llama-3.2-3b-instruct`
    Meta__Llama___Llama__3_2__3b__Instruct,
    /// Model: `meta-llama/llama-3.2-3b-instruct:free`
    Meta__Llama___Llama__3_2__3b__Instruct__Free,
    /// Model: `meta-llama/llama-3.3-70b-instruct`
    Meta__Llama___Llama__3_3__70b__Instruct,
    /// Model: `meta-llama/llama-3.3-70b-instruct:free`
    Meta__Llama___Llama__3_3__70b__Instruct__Free,
    /// Model: `meta-llama/llama-4-maverick`
    Meta__Llama___Llama__4__Maverick,
    /// Model: `meta-llama/llama-4-scout`
    Meta__Llama___Llama__4__Scout,
    /// Model: `meta-llama/llama-guard-4-12b`
    Meta__Llama___Llama__Guard__4__12b,
    /// Model: `microsoft/phi-4`
    Microsoft___Phi__4,
    /// Model: `microsoft/wizardlm-2-8x22b`
    Microsoft___Wizardlm__2__8x22b,
    /// Model: `minimax/minimax-01`
    Minimax___Minimax__01,
    /// Model: `minimax/minimax-m1`
    Minimax___Minimax__M1,
    /// Model: `minimax/minimax-m2`
    Minimax___Minimax__M2,
    /// Model: `minimax/minimax-m2-her`
    Minimax___Minimax__M2__Her,
    /// Model: `minimax/minimax-m2.1`
    Minimax___Minimax__M2_1,
    /// Model: `minimax/minimax-m2.5`
    Minimax___Minimax__M2_5,
    /// Model: `minimax/minimax-m2.7`
    Minimax___Minimax__M2_7,
    /// Model: `minimax/minimax-m3`
    Minimax___Minimax__M3,
    /// Model: `mistralai/codestral-2508`
    Mistralai___Codestral__2508,
    /// Model: `mistralai/devstral-2512`
    Mistralai___Devstral__2512,
    /// Model: `mistralai/ministral-14b-2512`
    Mistralai___Ministral__14b__2512,
    /// Model: `mistralai/ministral-3b-2512`
    Mistralai___Ministral__3b__2512,
    /// Model: `mistralai/ministral-8b-2512`
    Mistralai___Ministral__8b__2512,
    /// Model: `mistralai/mistral-large`
    Mistralai___Mistral__Large,
    /// Model: `mistralai/mistral-large-2407`
    Mistralai___Mistral__Large__2407,
    /// Model: `mistralai/mistral-large-2512`
    Mistralai___Mistral__Large__2512,
    /// Model: `mistralai/mistral-medium-3`
    Mistralai___Mistral__Medium__3,
    /// Model: `mistralai/mistral-medium-3-5`
    Mistralai___Mistral__Medium__3__5,
    /// Model: `mistralai/mistral-medium-3.1`
    Mistralai___Mistral__Medium__3_1,
    /// Model: `mistralai/mistral-nemo`
    Mistralai___Mistral__Nemo,
    /// Model: `mistralai/mistral-saba`
    Mistralai___Mistral__Saba,
    /// Model: `mistralai/mistral-small-24b-instruct-2501`
    Mistralai___Mistral__Small__24b__Instruct__2501,
    /// Model: `mistralai/mistral-small-2603`
    Mistralai___Mistral__Small__2603,
    /// Model: `mistralai/mistral-small-3.1-24b-instruct`
    Mistralai___Mistral__Small__3_1__24b__Instruct,
    /// Model: `mistralai/mistral-small-3.2-24b-instruct`
    Mistralai___Mistral__Small__3_2__24b__Instruct,
    /// Model: `mistralai/mixtral-8x22b-instruct`
    Mistralai___Mixtral__8x22b__Instruct,
    /// Model: `mistralai/voxtral-small-24b-2507`
    Mistralai___Voxtral__Small__24b__2507,
    /// Model: `moonshotai/kimi-k2`
    Moonshotai___Kimi__K2,
    /// Model: `moonshotai/kimi-k2-0905`
    Moonshotai___Kimi__K2__0905,
    /// Model: `moonshotai/kimi-k2-thinking`
    Moonshotai___Kimi__K2__Thinking,
    /// Model: `moonshotai/kimi-k2.5`
    Moonshotai___Kimi__K2_5,
    /// Model: `moonshotai/kimi-k2.6`
    Moonshotai___Kimi__K2_6,
    /// Model: `moonshotai/kimi-k2.7-code`
    Moonshotai___Kimi__K2_7__Code,
    /// Model: `morph/morph-v3-fast`
    Morph___Morph__V3__Fast,
    /// Model: `morph/morph-v3-large`
    Morph___Morph__V3__Large,
    /// Model: `nex-agi/nex-n2-mini`
    Nex__Agi___Nex__N2__Mini,
    /// Model: `nex-agi/nex-n2-pro`
    Nex__Agi___Nex__N2__Pro,
    /// Model: `nousresearch/hermes-3-llama-3.1-405b`
    Nousresearch___Hermes__3__Llama__3_1__405b,
    /// Model: `nousresearch/hermes-3-llama-3.1-405b:free`
    Nousresearch___Hermes__3__Llama__3_1__405b__Free,
    /// Model: `nousresearch/hermes-3-llama-3.1-70b`
    Nousresearch___Hermes__3__Llama__3_1__70b,
    /// Model: `nousresearch/hermes-4-405b`
    Nousresearch___Hermes__4__405b,
    /// Model: `nousresearch/hermes-4-70b`
    Nousresearch___Hermes__4__70b,
    /// Model: `nvidia/llama-3.3-nemotron-super-49b-v1.5`
    Nvidia___Llama__3_3__Nemotron__Super__49b__V1_5,
    /// Model: `nvidia/nemotron-3-nano-30b-a3b`
    Nvidia___Nemotron__3__Nano__30b__A3b,
    /// Model: `nvidia/nemotron-3-nano-30b-a3b:free`
    Nvidia___Nemotron__3__Nano__30b__A3b__Free,
    /// Model: `nvidia/nemotron-3-nano-omni-30b-a3b-reasoning:free`
    Nvidia___Nemotron__3__Nano__Omni__30b__A3b__Reasoning__Free,
    /// Model: `nvidia/nemotron-3-super-120b-a12b`
    Nvidia___Nemotron__3__Super__120b__A12b,
    /// Model: `nvidia/nemotron-3-super-120b-a12b:free`
    Nvidia___Nemotron__3__Super__120b__A12b__Free,
    /// Model: `nvidia/nemotron-3-ultra-550b-a55b`
    Nvidia___Nemotron__3__Ultra__550b__A55b,
    /// Model: `nvidia/nemotron-3-ultra-550b-a55b:free`
    Nvidia___Nemotron__3__Ultra__550b__A55b__Free,
    /// Model: `nvidia/nemotron-3.5-content-safety:free`
    Nvidia___Nemotron__3_5__Content__Safety__Free,
    /// Model: `nvidia/nemotron-nano-12b-v2-vl:free`
    Nvidia___Nemotron__Nano__12b__V2__Vl__Free,
    /// Model: `nvidia/nemotron-nano-9b-v2:free`
    Nvidia___Nemotron__Nano__9b__V2__Free,
    /// Model: `openai/gpt-3.5-turbo`
    Openai___Gpt__3_5__Turbo,
    /// Model: `openai/gpt-3.5-turbo-0613`
    Openai___Gpt__3_5__Turbo__0613,
    /// Model: `openai/gpt-3.5-turbo-16k`
    Openai___Gpt__3_5__Turbo__16k,
    /// Model: `openai/gpt-3.5-turbo-instruct`
    Openai___Gpt__3_5__Turbo__Instruct,
    /// Model: `openai/gpt-4`
    Openai___Gpt__4,
    /// Model: `openai/gpt-4-turbo`
    Openai___Gpt__4__Turbo,
    /// Model: `openai/gpt-4-turbo-preview`
    Openai___Gpt__4__Turbo__Preview,
    /// Model: `openai/gpt-4.1`
    Openai___Gpt__4_1,
    /// Model: `openai/gpt-4.1-mini`
    Openai___Gpt__4_1__Mini,
    /// Model: `openai/gpt-4.1-nano`
    Openai___Gpt__4_1__Nano,
    /// Model: `openai/gpt-4o`
    Openai___Gpt__4o,
    /// Model: `openai/gpt-4o-2024-05-13`
    Openai___Gpt__4o__2024__05__13,
    /// Model: `openai/gpt-4o-2024-08-06`
    Openai___Gpt__4o__2024__08__06,
    /// Model: `openai/gpt-4o-2024-11-20`
    Openai___Gpt__4o__2024__11__20,
    /// Model: `openai/gpt-4o-mini`
    Openai___Gpt__4o__Mini,
    /// Model: `openai/gpt-4o-mini-2024-07-18`
    Openai___Gpt__4o__Mini__2024__07__18,
    /// Model: `openai/gpt-4o-mini-search-preview`
    Openai___Gpt__4o__Mini__Search__Preview,
    /// Model: `openai/gpt-4o-search-preview`
    Openai___Gpt__4o__Search__Preview,
    /// Model: `openai/gpt-5`
    Openai___Gpt__5,
    /// Model: `openai/gpt-5-chat`
    Openai___Gpt__5__Chat,
    /// Model: `openai/gpt-5-codex`
    Openai___Gpt__5__Codex,
    /// Model: `openai/gpt-5-image`
    Openai___Gpt__5__Image,
    /// Model: `openai/gpt-5-image-mini`
    Openai___Gpt__5__Image__Mini,
    /// Model: `openai/gpt-5-mini`
    Openai___Gpt__5__Mini,
    /// Model: `openai/gpt-5-nano`
    Openai___Gpt__5__Nano,
    /// Model: `openai/gpt-5-pro`
    Openai___Gpt__5__Pro,
    /// Model: `openai/gpt-5.1`
    Openai___Gpt__5_1,
    /// Model: `openai/gpt-5.1-chat`
    Openai___Gpt__5_1__Chat,
    /// Model: `openai/gpt-5.1-codex`
    Openai___Gpt__5_1__Codex,
    /// Model: `openai/gpt-5.1-codex-max`
    Openai___Gpt__5_1__Codex__Max,
    /// Model: `openai/gpt-5.1-codex-mini`
    Openai___Gpt__5_1__Codex__Mini,
    /// Model: `openai/gpt-5.2`
    Openai___Gpt__5_2,
    /// Model: `openai/gpt-5.2-chat`
    Openai___Gpt__5_2__Chat,
    /// Model: `openai/gpt-5.2-codex`
    Openai___Gpt__5_2__Codex,
    /// Model: `openai/gpt-5.2-pro`
    Openai___Gpt__5_2__Pro,
    /// Model: `openai/gpt-5.3-chat`
    Openai___Gpt__5_3__Chat,
    /// Model: `openai/gpt-5.3-codex`
    Openai___Gpt__5_3__Codex,
    /// Model: `openai/gpt-5.4`
    Openai___Gpt__5_4,
    /// Model: `openai/gpt-5.4-image-2`
    Openai___Gpt__5_4__Image__2,
    /// Model: `openai/gpt-5.4-mini`
    Openai___Gpt__5_4__Mini,
    /// Model: `openai/gpt-5.4-nano`
    Openai___Gpt__5_4__Nano,
    /// Model: `openai/gpt-5.4-pro`
    Openai___Gpt__5_4__Pro,
    /// Model: `openai/gpt-5.5`
    Openai___Gpt__5_5,
    /// Model: `openai/gpt-5.5-pro`
    Openai___Gpt__5_5__Pro,
    /// Model: `openai/gpt-audio`
    Openai___Gpt__Audio,
    /// Model: `openai/gpt-audio-mini`
    Openai___Gpt__Audio__Mini,
    /// Model: `openai/gpt-chat-latest`
    Openai___Gpt__Chat__Latest,
    /// Model: `openai/gpt-oss-120b`
    Openai___Gpt__Oss__120b,
    /// Model: `openai/gpt-oss-120b:free`
    Openai___Gpt__Oss__120b__Free,
    /// Model: `openai/gpt-oss-20b`
    Openai___Gpt__Oss__20b,
    /// Model: `openai/gpt-oss-20b:free`
    Openai___Gpt__Oss__20b__Free,
    /// Model: `openai/gpt-oss-safeguard-20b`
    Openai___Gpt__Oss__Safeguard__20b,
    /// Model: `openai/o1`
    Openai___O1,
    /// Model: `openai/o1-pro`
    Openai___O1__Pro,
    /// Model: `openai/o3`
    Openai___O3,
    /// Model: `openai/o3-deep-research`
    Openai___O3__Deep__Research,
    /// Model: `openai/o3-mini`
    Openai___O3__Mini,
    /// Model: `openai/o3-mini-high`
    Openai___O3__Mini__High,
    /// Model: `openai/o3-pro`
    Openai___O3__Pro,
    /// Model: `openai/o4-mini`
    Openai___O4__Mini,
    /// Model: `openai/o4-mini-deep-research`
    Openai___O4__Mini__Deep__Research,
    /// Model: `openai/o4-mini-high`
    Openai___O4__Mini__High,
    /// Model: `openrouter/auto`
    Openrouter___Auto,
    /// Model: `openrouter/bodybuilder`
    Openrouter___Bodybuilder,
    /// Model: `openrouter/free`
    Openrouter___Free,
    /// Model: `openrouter/fusion`
    Openrouter___Fusion,
    /// Model: `openrouter/pareto-code`
    Openrouter___Pareto__Code,
    /// Model: `perceptron/perceptron-mk1`
    Perceptron___Perceptron__Mk1,
    /// Model: `perplexity/sonar`
    Perplexity___Sonar,
    /// Model: `perplexity/sonar-deep-research`
    Perplexity___Sonar__Deep__Research,
    /// Model: `perplexity/sonar-pro`
    Perplexity___Sonar__Pro,
    /// Model: `perplexity/sonar-pro-search`
    Perplexity___Sonar__Pro__Search,
    /// Model: `perplexity/sonar-reasoning-pro`
    Perplexity___Sonar__Reasoning__Pro,
    /// Model: `poolside/laguna-m.1`
    Poolside___Laguna__M_1,
    /// Model: `poolside/laguna-m.1:free`
    Poolside___Laguna__M_1__Free,
    /// Model: `poolside/laguna-xs-2.1`
    Poolside___Laguna__Xs__2_1,
    /// Model: `poolside/laguna-xs-2.1:free`
    Poolside___Laguna__Xs__2_1__Free,
    /// Model: `poolside/laguna-xs.2`
    Poolside___Laguna__Xs_2,
    /// Model: `poolside/laguna-xs.2:free`
    Poolside___Laguna__Xs_2__Free,
    /// Model: `qwen/qwen-2.5-72b-instruct`
    Qwen___Qwen__2_5__72b__Instruct,
    /// Model: `qwen/qwen-2.5-7b-instruct`
    Qwen___Qwen__2_5__7b__Instruct,
    /// Model: `qwen/qwen-2.5-coder-32b-instruct`
    Qwen___Qwen__2_5__Coder__32b__Instruct,
    /// Model: `qwen/qwen-plus`
    Qwen___Qwen__Plus,
    /// Model: `qwen/qwen-plus-2025-07-28`
    Qwen___Qwen__Plus__2025__07__28,
    /// Model: `qwen/qwen-plus-2025-07-28:thinking`
    Qwen___Qwen__Plus__2025__07__28__Thinking,
    /// Model: `qwen/qwen2.5-vl-72b-instruct`
    Qwen___Qwen2_5__Vl__72b__Instruct,
    /// Model: `qwen/qwen3-14b`
    Qwen___Qwen3__14b,
    /// Model: `qwen/qwen3-235b-a22b`
    Qwen___Qwen3__235b__A22b,
    /// Model: `qwen/qwen3-235b-a22b-2507`
    Qwen___Qwen3__235b__A22b__2507,
    /// Model: `qwen/qwen3-235b-a22b-thinking-2507`
    Qwen___Qwen3__235b__A22b__Thinking__2507,
    /// Model: `qwen/qwen3-30b-a3b`
    Qwen___Qwen3__30b__A3b,
    /// Model: `qwen/qwen3-30b-a3b-instruct-2507`
    Qwen___Qwen3__30b__A3b__Instruct__2507,
    /// Model: `qwen/qwen3-30b-a3b-thinking-2507`
    Qwen___Qwen3__30b__A3b__Thinking__2507,
    /// Model: `qwen/qwen3-32b`
    Qwen___Qwen3__32b,
    /// Model: `qwen/qwen3-8b`
    Qwen___Qwen3__8b,
    /// Model: `qwen/qwen3-coder`
    Qwen___Qwen3__Coder,
    /// Model: `qwen/qwen3-coder-30b-a3b-instruct`
    Qwen___Qwen3__Coder__30b__A3b__Instruct,
    /// Model: `qwen/qwen3-coder-flash`
    Qwen___Qwen3__Coder__Flash,
    /// Model: `qwen/qwen3-coder-next`
    Qwen___Qwen3__Coder__Next,
    /// Model: `qwen/qwen3-coder-plus`
    Qwen___Qwen3__Coder__Plus,
    /// Model: `qwen/qwen3-coder:free`
    Qwen___Qwen3__Coder__Free,
    /// Model: `qwen/qwen3-max`
    Qwen___Qwen3__Max,
    /// Model: `qwen/qwen3-max-thinking`
    Qwen___Qwen3__Max__Thinking,
    /// Model: `qwen/qwen3-next-80b-a3b-instruct`
    Qwen___Qwen3__Next__80b__A3b__Instruct,
    /// Model: `qwen/qwen3-next-80b-a3b-instruct:free`
    Qwen___Qwen3__Next__80b__A3b__Instruct__Free,
    /// Model: `qwen/qwen3-next-80b-a3b-thinking`
    Qwen___Qwen3__Next__80b__A3b__Thinking,
    /// Model: `qwen/qwen3-vl-235b-a22b-instruct`
    Qwen___Qwen3__Vl__235b__A22b__Instruct,
    /// Model: `qwen/qwen3-vl-235b-a22b-thinking`
    Qwen___Qwen3__Vl__235b__A22b__Thinking,
    /// Model: `qwen/qwen3-vl-30b-a3b-instruct`
    Qwen___Qwen3__Vl__30b__A3b__Instruct,
    /// Model: `qwen/qwen3-vl-30b-a3b-thinking`
    Qwen___Qwen3__Vl__30b__A3b__Thinking,
    /// Model: `qwen/qwen3-vl-32b-instruct`
    Qwen___Qwen3__Vl__32b__Instruct,
    /// Model: `qwen/qwen3-vl-8b-instruct`
    Qwen___Qwen3__Vl__8b__Instruct,
    /// Model: `qwen/qwen3-vl-8b-thinking`
    Qwen___Qwen3__Vl__8b__Thinking,
    /// Model: `qwen/qwen3.5-122b-a10b`
    Qwen___Qwen3_5__122b__A10b,
    /// Model: `qwen/qwen3.5-27b`
    Qwen___Qwen3_5__27b,
    /// Model: `qwen/qwen3.5-35b-a3b`
    Qwen___Qwen3_5__35b__A3b,
    /// Model: `qwen/qwen3.5-397b-a17b`
    Qwen___Qwen3_5__397b__A17b,
    /// Model: `qwen/qwen3.5-9b`
    Qwen___Qwen3_5__9b,
    /// Model: `qwen/qwen3.5-flash-02-23`
    Qwen___Qwen3_5__Flash__02__23,
    /// Model: `qwen/qwen3.5-plus-02-15`
    Qwen___Qwen3_5__Plus__02__15,
    /// Model: `qwen/qwen3.5-plus-20260420`
    Qwen___Qwen3_5__Plus__20260420,
    /// Model: `qwen/qwen3.6-27b`
    Qwen___Qwen3_6__27b,
    /// Model: `qwen/qwen3.6-35b-a3b`
    Qwen___Qwen3_6__35b__A3b,
    /// Model: `qwen/qwen3.6-flash`
    Qwen___Qwen3_6__Flash,
    /// Model: `qwen/qwen3.6-max-preview`
    Qwen___Qwen3_6__Max__Preview,
    /// Model: `qwen/qwen3.6-plus`
    Qwen___Qwen3_6__Plus,
    /// Model: `qwen/qwen3.7-max`
    Qwen___Qwen3_7__Max,
    /// Model: `qwen/qwen3.7-plus`
    Qwen___Qwen3_7__Plus,
    /// Model: `rekaai/reka-edge`
    Rekaai___Reka__Edge,
    /// Model: `rekaai/reka-flash-3`
    Rekaai___Reka__Flash__3,
    /// Model: `relace/relace-apply-3`
    Relace___Relace__Apply__3,
    /// Model: `relace/relace-search`
    Relace___Relace__Search,
    /// Model: `sakana/fugu-ultra`
    Sakana___Fugu__Ultra,
    /// Model: `sao10k/l3-lunaris-8b`
    Sao10k___L3__Lunaris__8b,
    /// Model: `sao10k/l3.1-70b-hanami-x1`
    Sao10k___L3_1__70b__Hanami__X1,
    /// Model: `sao10k/l3.1-euryale-70b`
    Sao10k___L3_1__Euryale__70b,
    /// Model: `sao10k/l3.3-euryale-70b`
    Sao10k___L3_3__Euryale__70b,
    /// Model: `stepfun/step-3.5-flash`
    Stepfun___Step__3_5__Flash,
    /// Model: `stepfun/step-3.7-flash`
    Stepfun___Step__3_7__Flash,
    /// Model: `switchpoint/router`
    Switchpoint___Router,
    /// Model: `tencent/hunyuan-a13b-instruct`
    Tencent___Hunyuan__A13b__Instruct,
    /// Model: `tencent/hy3`
    Tencent___Hy3,
    /// Model: `tencent/hy3-preview`
    Tencent___Hy3__Preview,
    /// Model: `tencent/hy3:free`
    Tencent___Hy3__Free,
    /// Model: `thedrummer/cydonia-24b-v4.1`
    Thedrummer___Cydonia__24b__V4_1,
    /// Model: `thedrummer/rocinante-12b`
    Thedrummer___Rocinante__12b,
    /// Model: `thedrummer/skyfall-36b-v2`
    Thedrummer___Skyfall__36b__V2,
    /// Model: `thedrummer/unslopnemo-12b`
    Thedrummer___Unslopnemo__12b,
    /// Model: `undi95/remm-slerp-l2-13b`
    Undi95___Remm__Slerp__L2__13b,
    /// Model: `upstage/solar-pro-3`
    Upstage___Solar__Pro__3,
    /// Model: `writer/palmyra-x5`
    Writer___Palmyra__X5,
    /// Model: `x-ai/grok-4.20`
    X__Ai___Grok__4_20,
    /// Model: `x-ai/grok-4.20-multi-agent`
    X__Ai___Grok__4_20__Multi__Agent,
    /// Model: `x-ai/grok-4.3`
    X__Ai___Grok__4_3,
    /// Model: `x-ai/grok-build-0.1`
    X__Ai___Grok__Build__0_1,
    /// Model: `xiaomi/mimo-v2.5`
    Xiaomi___Mimo__V2_5,
    /// Model: `xiaomi/mimo-v2.5-pro`
    Xiaomi___Mimo__V2_5__Pro,
    /// Model: `z-ai/glm-4.5`
    Z__Ai___Glm__4_5,
    /// Model: `z-ai/glm-4.5-air`
    Z__Ai___Glm__4_5__Air,
    /// Model: `z-ai/glm-4.5v`
    Z__Ai___Glm__4_5v,
    /// Model: `z-ai/glm-4.6`
    Z__Ai___Glm__4_6,
    /// Model: `z-ai/glm-4.6v`
    Z__Ai___Glm__4_6v,
    /// Model: `z-ai/glm-4.7`
    Z__Ai___Glm__4_7,
    /// Model: `z-ai/glm-4.7-flash`
    Z__Ai___Glm__4_7__Flash,
    /// Model: `z-ai/glm-5`
    Z__Ai___Glm__5,
    /// Model: `z-ai/glm-5-turbo`
    Z__Ai___Glm__5__Turbo,
    /// Model: `z-ai/glm-5.1`
    Z__Ai___Glm__5_1,
    /// Model: `z-ai/glm-5.2`
    Z__Ai___Glm__5_2,
    /// Model: `z-ai/glm-5v-turbo`
    Z__Ai___Glm__5v__Turbo,
    /// Model: `~anthropic/claude-fable-latest`
    __Anthropic___Claude__Fable__Latest,
    /// Model: `~anthropic/claude-haiku-latest`
    __Anthropic___Claude__Haiku__Latest,
    /// Model: `~anthropic/claude-opus-latest`
    __Anthropic___Claude__Opus__Latest,
    /// Model: `~anthropic/claude-sonnet-latest`
    __Anthropic___Claude__Sonnet__Latest,
    /// Model: `~google/gemini-flash-latest`
    __Google___Gemini__Flash__Latest,
    /// Model: `~google/gemini-pro-latest`
    __Google___Gemini__Pro__Latest,
    /// Model: `~moonshotai/kimi-latest`
    __Moonshotai___Kimi__Latest,
    /// Model: `~openai/gpt-latest`
    __Openai___Gpt__Latest,
    /// Model: `~openai/gpt-mini-latest`
    __Openai___Gpt__Mini__Latest,
    /// Custom model ID not in the predefined list.
    Bespoke(String),
}
