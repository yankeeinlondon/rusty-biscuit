//! Auto-generated provider model enum
//!
//! Generated: 2026-01-11T20:35:19.535745+00:00
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
    /// Model: `ai21/jamba-mini-1.7`
    Ai21___Jamba__Mini__1_7,
    /// Model: `aion-labs/aion-1.0`
    Aion__Labs___Aion__1_0,
    /// Model: `aion-labs/aion-1.0-mini`
    Aion__Labs___Aion__1_0__Mini,
    /// Model: `aion-labs/aion-rp-llama-3.1-8b`
    Aion__Labs___Aion__Rp__Llama__3_1__8b,
    /// Model: `alfredpros/codellama-7b-instruct-solidity`
    Alfredpros___Codellama__7b__Instruct__Solidity,
    /// Model: `alibaba/tongyi-deepresearch-30b-a3b`
    Alibaba___Tongyi__Deepresearch__30b__A3b,
    /// Model: `allenai/molmo-2-8b:free`
    Allenai___Molmo__2__8b__Free,
    /// Model: `allenai/olmo-2-0325-32b-instruct`
    Allenai___Olmo__2__0325__32b__Instruct,
    /// Model: `allenai/olmo-3-32b-think`
    Allenai___Olmo__3__32b__Think,
    /// Model: `allenai/olmo-3-7b-instruct`
    Allenai___Olmo__3__7b__Instruct,
    /// Model: `allenai/olmo-3-7b-think`
    Allenai___Olmo__3__7b__Think,
    /// Model: `allenai/olmo-3.1-32b-instruct`
    Allenai___Olmo__3_1__32b__Instruct,
    /// Model: `allenai/olmo-3.1-32b-think`
    Allenai___Olmo__3_1__32b__Think,
    /// Model: `alpindale/goliath-120b`
    Alpindale___Goliath__120b,
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
    /// Model: `anthropic/claude-3.5-haiku`
    Anthropic___Claude__3_5__Haiku,
    /// Model: `anthropic/claude-3.5-haiku-20241022`
    Anthropic___Claude__3_5__Haiku__20241022,
    /// Model: `anthropic/claude-3.5-sonnet`
    Anthropic___Claude__3_5__Sonnet,
    /// Model: `anthropic/claude-3.7-sonnet`
    Anthropic___Claude__3_7__Sonnet,
    /// Model: `anthropic/claude-3.7-sonnet:thinking`
    Anthropic___Claude__3_7__Sonnet__Thinking,
    /// Model: `anthropic/claude-haiku-4.5`
    Anthropic___Claude__Haiku__4_5,
    /// Model: `anthropic/claude-opus-4`
    Anthropic___Claude__Opus__4,
    /// Model: `anthropic/claude-opus-4.1`
    Anthropic___Claude__Opus__4_1,
    /// Model: `anthropic/claude-opus-4.5`
    Anthropic___Claude__Opus__4_5,
    /// Model: `anthropic/claude-sonnet-4`
    Anthropic___Claude__Sonnet__4,
    /// Model: `anthropic/claude-sonnet-4.5`
    Anthropic___Claude__Sonnet__4_5,
    /// Model: `arcee-ai/coder-large`
    Arcee__Ai___Coder__Large,
    /// Model: `arcee-ai/maestro-reasoning`
    Arcee__Ai___Maestro__Reasoning,
    /// Model: `arcee-ai/spotlight`
    Arcee__Ai___Spotlight,
    /// Model: `arcee-ai/trinity-mini`
    Arcee__Ai___Trinity__Mini,
    /// Model: `arcee-ai/trinity-mini:free`
    Arcee__Ai___Trinity__Mini__Free,
    /// Model: `arcee-ai/virtuoso-large`
    Arcee__Ai___Virtuoso__Large,
    /// Model: `baidu/ernie-4.5-21b-a3b`
    Baidu___Ernie__4_5__21b__A3b,
    /// Model: `baidu/ernie-4.5-21b-a3b-thinking`
    Baidu___Ernie__4_5__21b__A3b__Thinking,
    /// Model: `baidu/ernie-4.5-300b-a47b`
    Baidu___Ernie__4_5__300b__A47b,
    /// Model: `baidu/ernie-4.5-vl-28b-a3b`
    Baidu___Ernie__4_5__Vl__28b__A3b,
    /// Model: `baidu/ernie-4.5-vl-424b-a47b`
    Baidu___Ernie__4_5__Vl__424b__A47b,
    /// Model: `bytedance-seed/seed-1.6`
    Bytedance__Seed___Seed__1_6,
    /// Model: `bytedance-seed/seed-1.6-flash`
    Bytedance__Seed___Seed__1_6__Flash,
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
    /// Model: `deepcogito/cogito-v2-preview-llama-109b-moe`
    Deepcogito___Cogito__V2__Preview__Llama__109b__Moe,
    /// Model: `deepcogito/cogito-v2-preview-llama-405b`
    Deepcogito___Cogito__V2__Preview__Llama__405b,
    /// Model: `deepcogito/cogito-v2-preview-llama-70b`
    Deepcogito___Cogito__V2__Preview__Llama__70b,
    /// Model: `deepcogito/cogito-v2.1-671b`
    Deepcogito___Cogito__V2_1__671b,
    /// Model: `deepseek/deepseek-chat`
    Deepseek___Deepseek__Chat,
    /// Model: `deepseek/deepseek-chat-v3-0324`
    Deepseek___Deepseek__Chat__V3__0324,
    /// Model: `deepseek/deepseek-chat-v3.1`
    Deepseek___Deepseek__Chat__V3_1,
    /// Model: `deepseek/deepseek-prover-v2`
    Deepseek___Deepseek__Prover__V2,
    /// Model: `deepseek/deepseek-r1`
    Deepseek___Deepseek__R1,
    /// Model: `deepseek/deepseek-r1-0528`
    Deepseek___Deepseek__R1__0528,
    /// Model: `deepseek/deepseek-r1-0528-qwen3-8b`
    Deepseek___Deepseek__R1__0528__Qwen3__8b,
    /// Model: `deepseek/deepseek-r1-0528:free`
    Deepseek___Deepseek__R1__0528__Free,
    /// Model: `deepseek/deepseek-r1-distill-llama-70b`
    Deepseek___Deepseek__R1__Distill__Llama__70b,
    /// Model: `deepseek/deepseek-r1-distill-qwen-14b`
    Deepseek___Deepseek__R1__Distill__Qwen__14b,
    /// Model: `deepseek/deepseek-r1-distill-qwen-32b`
    Deepseek___Deepseek__R1__Distill__Qwen__32b,
    /// Model: `deepseek/deepseek-v3.1-terminus`
    Deepseek___Deepseek__V3_1__Terminus,
    /// Model: `deepseek/deepseek-v3.1-terminus:exacto`
    Deepseek___Deepseek__V3_1__Terminus__Exacto,
    /// Model: `deepseek/deepseek-v3.2`
    Deepseek___Deepseek__V3_2,
    /// Model: `deepseek/deepseek-v3.2-exp`
    Deepseek___Deepseek__V3_2__Exp,
    /// Model: `deepseek/deepseek-v3.2-speciale`
    Deepseek___Deepseek__V3_2__Speciale,
    /// Model: `eleutherai/llemma_7b`
    Eleutherai___Llemma__7b,
    /// Model: `essentialai/rnj-1-instruct`
    Essentialai___Rnj__1__Instruct,
    /// Model: `google/gemini-2.0-flash-001`
    Google___Gemini__2_0__Flash__001,
    /// Model: `google/gemini-2.0-flash-exp:free`
    Google___Gemini__2_0__Flash__Exp__Free,
    /// Model: `google/gemini-2.0-flash-lite-001`
    Google___Gemini__2_0__Flash__Lite__001,
    /// Model: `google/gemini-2.5-flash`
    Google___Gemini__2_5__Flash,
    /// Model: `google/gemini-2.5-flash-image`
    Google___Gemini__2_5__Flash__Image,
    /// Model: `google/gemini-2.5-flash-image-preview`
    Google___Gemini__2_5__Flash__Image__Preview,
    /// Model: `google/gemini-2.5-flash-lite`
    Google___Gemini__2_5__Flash__Lite,
    /// Model: `google/gemini-2.5-flash-lite-preview-09-2025`
    Google___Gemini__2_5__Flash__Lite__Preview__09__2025,
    /// Model: `google/gemini-2.5-flash-preview-09-2025`
    Google___Gemini__2_5__Flash__Preview__09__2025,
    /// Model: `google/gemini-2.5-pro`
    Google___Gemini__2_5__Pro,
    /// Model: `google/gemini-2.5-pro-preview`
    Google___Gemini__2_5__Pro__Preview,
    /// Model: `google/gemini-2.5-pro-preview-05-06`
    Google___Gemini__2_5__Pro__Preview__05__06,
    /// Model: `google/gemini-3-flash-preview`
    Google___Gemini__3__Flash__Preview,
    /// Model: `google/gemini-3-pro-image-preview`
    Google___Gemini__3__Pro__Image__Preview,
    /// Model: `google/gemini-3-pro-preview`
    Google___Gemini__3__Pro__Preview,
    /// Model: `google/gemma-2-27b-it`
    Google___Gemma__2__27b__It,
    /// Model: `google/gemma-2-9b-it`
    Google___Gemma__2__9b__It,
    /// Model: `google/gemma-3-12b-it`
    Google___Gemma__3__12b__It,
    /// Model: `google/gemma-3-12b-it:free`
    Google___Gemma__3__12b__It__Free,
    /// Model: `google/gemma-3-27b-it`
    Google___Gemma__3__27b__It,
    /// Model: `google/gemma-3-27b-it:free`
    Google___Gemma__3__27b__It__Free,
    /// Model: `google/gemma-3-4b-it`
    Google___Gemma__3__4b__It,
    /// Model: `google/gemma-3-4b-it:free`
    Google___Gemma__3__4b__It__Free,
    /// Model: `google/gemma-3n-e2b-it:free`
    Google___Gemma__3n__E2b__It__Free,
    /// Model: `google/gemma-3n-e4b-it`
    Google___Gemma__3n__E4b__It,
    /// Model: `google/gemma-3n-e4b-it:free`
    Google___Gemma__3n__E4b__It__Free,
    /// Model: `gryphe/mythomax-l2-13b`
    Gryphe___Mythomax__L2__13b,
    /// Model: `ibm-granite/granite-4.0-h-micro`
    Ibm__Granite___Granite__4_0__H__Micro,
    /// Model: `inception/mercury`
    Inception___Mercury,
    /// Model: `inception/mercury-coder`
    Inception___Mercury__Coder,
    /// Model: `inflection/inflection-3-pi`
    Inflection___Inflection__3__Pi,
    /// Model: `inflection/inflection-3-productivity`
    Inflection___Inflection__3__Productivity,
    /// Model: `kwaipilot/kat-coder-pro`
    Kwaipilot___Kat__Coder__Pro,
    /// Model: `kwaipilot/kat-coder-pro:free`
    Kwaipilot___Kat__Coder__Pro__Free,
    /// Model: `liquid/lfm-2.2-6b`
    Liquid___Lfm__2_2__6b,
    /// Model: `liquid/lfm2-8b-a1b`
    Liquid___Lfm2__8b__A1b,
    /// Model: `mancer/weaver`
    Mancer___Weaver,
    /// Model: `meituan/longcat-flash-chat`
    Meituan___Longcat__Flash__Chat,
    /// Model: `meta-llama/llama-3-70b-instruct`
    Meta__Llama___Llama__3__70b__Instruct,
    /// Model: `meta-llama/llama-3-8b-instruct`
    Meta__Llama___Llama__3__8b__Instruct,
    /// Model: `meta-llama/llama-3.1-405b`
    Meta__Llama___Llama__3_1__405b,
    /// Model: `meta-llama/llama-3.1-405b-instruct`
    Meta__Llama___Llama__3_1__405b__Instruct,
    /// Model: `meta-llama/llama-3.1-405b-instruct:free`
    Meta__Llama___Llama__3_1__405b__Instruct__Free,
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
    /// Model: `meta-llama/llama-3.2-90b-vision-instruct`
    Meta__Llama___Llama__3_2__90b__Vision__Instruct,
    /// Model: `meta-llama/llama-3.3-70b-instruct`
    Meta__Llama___Llama__3_3__70b__Instruct,
    /// Model: `meta-llama/llama-3.3-70b-instruct:free`
    Meta__Llama___Llama__3_3__70b__Instruct__Free,
    /// Model: `meta-llama/llama-4-maverick`
    Meta__Llama___Llama__4__Maverick,
    /// Model: `meta-llama/llama-4-scout`
    Meta__Llama___Llama__4__Scout,
    /// Model: `meta-llama/llama-guard-2-8b`
    Meta__Llama___Llama__Guard__2__8b,
    /// Model: `meta-llama/llama-guard-3-8b`
    Meta__Llama___Llama__Guard__3__8b,
    /// Model: `meta-llama/llama-guard-4-12b`
    Meta__Llama___Llama__Guard__4__12b,
    /// Model: `microsoft/phi-4`
    Microsoft___Phi__4,
    /// Model: `microsoft/phi-4-multimodal-instruct`
    Microsoft___Phi__4__Multimodal__Instruct,
    /// Model: `microsoft/phi-4-reasoning-plus`
    Microsoft___Phi__4__Reasoning__Plus,
    /// Model: `microsoft/wizardlm-2-8x22b`
    Microsoft___Wizardlm__2__8x22b,
    /// Model: `minimax/minimax-01`
    Minimax___Minimax__01,
    /// Model: `minimax/minimax-m1`
    Minimax___Minimax__M1,
    /// Model: `minimax/minimax-m2`
    Minimax___Minimax__M2,
    /// Model: `minimax/minimax-m2.1`
    Minimax___Minimax__M2_1,
    /// Model: `mistralai/codestral-2508`
    Mistralai___Codestral__2508,
    /// Model: `mistralai/devstral-2512`
    Mistralai___Devstral__2512,
    /// Model: `mistralai/devstral-2512:free`
    Mistralai___Devstral__2512__Free,
    /// Model: `mistralai/devstral-medium`
    Mistralai___Devstral__Medium,
    /// Model: `mistralai/devstral-small`
    Mistralai___Devstral__Small,
    /// Model: `mistralai/devstral-small-2505`
    Mistralai___Devstral__Small__2505,
    /// Model: `mistralai/ministral-14b-2512`
    Mistralai___Ministral__14b__2512,
    /// Model: `mistralai/ministral-3b`
    Mistralai___Ministral__3b,
    /// Model: `mistralai/ministral-3b-2512`
    Mistralai___Ministral__3b__2512,
    /// Model: `mistralai/ministral-8b`
    Mistralai___Ministral__8b,
    /// Model: `mistralai/ministral-8b-2512`
    Mistralai___Ministral__8b__2512,
    /// Model: `mistralai/mistral-7b-instruct`
    Mistralai___Mistral__7b__Instruct,
    /// Model: `mistralai/mistral-7b-instruct-v0.1`
    Mistralai___Mistral__7b__Instruct__V0_1,
    /// Model: `mistralai/mistral-7b-instruct-v0.2`
    Mistralai___Mistral__7b__Instruct__V0_2,
    /// Model: `mistralai/mistral-7b-instruct-v0.3`
    Mistralai___Mistral__7b__Instruct__V0_3,
    /// Model: `mistralai/mistral-7b-instruct:free`
    Mistralai___Mistral__7b__Instruct__Free,
    /// Model: `mistralai/mistral-large`
    Mistralai___Mistral__Large,
    /// Model: `mistralai/mistral-large-2407`
    Mistralai___Mistral__Large__2407,
    /// Model: `mistralai/mistral-large-2411`
    Mistralai___Mistral__Large__2411,
    /// Model: `mistralai/mistral-large-2512`
    Mistralai___Mistral__Large__2512,
    /// Model: `mistralai/mistral-medium-3`
    Mistralai___Mistral__Medium__3,
    /// Model: `mistralai/mistral-medium-3.1`
    Mistralai___Mistral__Medium__3_1,
    /// Model: `mistralai/mistral-nemo`
    Mistralai___Mistral__Nemo,
    /// Model: `mistralai/mistral-saba`
    Mistralai___Mistral__Saba,
    /// Model: `mistralai/mistral-small-24b-instruct-2501`
    Mistralai___Mistral__Small__24b__Instruct__2501,
    /// Model: `mistralai/mistral-small-3.1-24b-instruct`
    Mistralai___Mistral__Small__3_1__24b__Instruct,
    /// Model: `mistralai/mistral-small-3.1-24b-instruct:free`
    Mistralai___Mistral__Small__3_1__24b__Instruct__Free,
    /// Model: `mistralai/mistral-small-3.2-24b-instruct`
    Mistralai___Mistral__Small__3_2__24b__Instruct,
    /// Model: `mistralai/mistral-small-creative`
    Mistralai___Mistral__Small__Creative,
    /// Model: `mistralai/mistral-tiny`
    Mistralai___Mistral__Tiny,
    /// Model: `mistralai/mixtral-8x22b-instruct`
    Mistralai___Mixtral__8x22b__Instruct,
    /// Model: `mistralai/mixtral-8x7b-instruct`
    Mistralai___Mixtral__8x7b__Instruct,
    /// Model: `mistralai/pixtral-12b`
    Mistralai___Pixtral__12b,
    /// Model: `mistralai/pixtral-large-2411`
    Mistralai___Pixtral__Large__2411,
    /// Model: `mistralai/voxtral-small-24b-2507`
    Mistralai___Voxtral__Small__24b__2507,
    /// Model: `moonshotai/kimi-dev-72b`
    Moonshotai___Kimi__Dev__72b,
    /// Model: `moonshotai/kimi-k2`
    Moonshotai___Kimi__K2,
    /// Model: `moonshotai/kimi-k2-0905`
    Moonshotai___Kimi__K2__0905,
    /// Model: `moonshotai/kimi-k2-0905:exacto`
    Moonshotai___Kimi__K2__0905__Exacto,
    /// Model: `moonshotai/kimi-k2-thinking`
    Moonshotai___Kimi__K2__Thinking,
    /// Model: `moonshotai/kimi-k2:free`
    Moonshotai___Kimi__K2__Free,
    /// Model: `morph/morph-v3-fast`
    Morph___Morph__V3__Fast,
    /// Model: `morph/morph-v3-large`
    Morph___Morph__V3__Large,
    /// Model: `neversleep/llama-3.1-lumimaid-8b`
    Neversleep___Llama__3_1__Lumimaid__8b,
    /// Model: `neversleep/noromaid-20b`
    Neversleep___Noromaid__20b,
    /// Model: `nex-agi/deepseek-v3.1-nex-n1`
    Nex__Agi___Deepseek__V3_1__Nex__N1,
    /// Model: `nousresearch/deephermes-3-mistral-24b-preview`
    Nousresearch___Deephermes__3__Mistral__24b__Preview,
    /// Model: `nousresearch/hermes-2-pro-llama-3-8b`
    Nousresearch___Hermes__2__Pro__Llama__3__8b,
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
    /// Model: `nvidia/llama-3.1-nemotron-70b-instruct`
    Nvidia___Llama__3_1__Nemotron__70b__Instruct,
    /// Model: `nvidia/llama-3.1-nemotron-ultra-253b-v1`
    Nvidia___Llama__3_1__Nemotron__Ultra__253b__V1,
    /// Model: `nvidia/llama-3.3-nemotron-super-49b-v1.5`
    Nvidia___Llama__3_3__Nemotron__Super__49b__V1_5,
    /// Model: `nvidia/nemotron-3-nano-30b-a3b`
    Nvidia___Nemotron__3__Nano__30b__A3b,
    /// Model: `nvidia/nemotron-3-nano-30b-a3b:free`
    Nvidia___Nemotron__3__Nano__30b__A3b__Free,
    /// Model: `nvidia/nemotron-nano-12b-v2-vl`
    Nvidia___Nemotron__Nano__12b__V2__Vl,
    /// Model: `nvidia/nemotron-nano-12b-v2-vl:free`
    Nvidia___Nemotron__Nano__12b__V2__Vl__Free,
    /// Model: `nvidia/nemotron-nano-9b-v2`
    Nvidia___Nemotron__Nano__9b__V2,
    /// Model: `nvidia/nemotron-nano-9b-v2:free`
    Nvidia___Nemotron__Nano__9b__V2__Free,
    /// Model: `openai/chatgpt-4o-latest`
    Openai___Chatgpt__4o__Latest,
    /// Model: `openai/codex-mini`
    Openai___Codex__Mini,
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
    /// Model: `openai/gpt-4-0314`
    Openai___Gpt__4__0314,
    /// Model: `openai/gpt-4-1106-preview`
    Openai___Gpt__4__1106__Preview,
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
    /// Model: `openai/gpt-4o-audio-preview`
    Openai___Gpt__4o__Audio__Preview,
    /// Model: `openai/gpt-4o-mini`
    Openai___Gpt__4o__Mini,
    /// Model: `openai/gpt-4o-mini-2024-07-18`
    Openai___Gpt__4o__Mini__2024__07__18,
    /// Model: `openai/gpt-4o-mini-search-preview`
    Openai___Gpt__4o__Mini__Search__Preview,
    /// Model: `openai/gpt-4o-search-preview`
    Openai___Gpt__4o__Search__Preview,
    /// Model: `openai/gpt-4o:extended`
    Openai___Gpt__4o__Extended,
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
    /// Model: `openai/gpt-5.2-pro`
    Openai___Gpt__5_2__Pro,
    /// Model: `openai/gpt-oss-120b`
    Openai___Gpt__Oss__120b,
    /// Model: `openai/gpt-oss-120b:exacto`
    Openai___Gpt__Oss__120b__Exacto,
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
    /// Model: `opengvlab/internvl3-78b`
    Opengvlab___Internvl3__78b,
    /// Model: `openrouter/auto`
    Openrouter___Auto,
    /// Model: `openrouter/bodybuilder`
    Openrouter___Bodybuilder,
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
    /// Model: `prime-intellect/intellect-3`
    Prime__Intellect___Intellect__3,
    /// Model: `qwen/qwen-2.5-72b-instruct`
    Qwen___Qwen__2_5__72b__Instruct,
    /// Model: `qwen/qwen-2.5-7b-instruct`
    Qwen___Qwen__2_5__7b__Instruct,
    /// Model: `qwen/qwen-2.5-coder-32b-instruct`
    Qwen___Qwen__2_5__Coder__32b__Instruct,
    /// Model: `qwen/qwen-2.5-vl-7b-instruct`
    Qwen___Qwen__2_5__Vl__7b__Instruct,
    /// Model: `qwen/qwen-2.5-vl-7b-instruct:free`
    Qwen___Qwen__2_5__Vl__7b__Instruct__Free,
    /// Model: `qwen/qwen-max`
    Qwen___Qwen__Max,
    /// Model: `qwen/qwen-plus`
    Qwen___Qwen__Plus,
    /// Model: `qwen/qwen-plus-2025-07-28`
    Qwen___Qwen__Plus__2025__07__28,
    /// Model: `qwen/qwen-plus-2025-07-28:thinking`
    Qwen___Qwen__Plus__2025__07__28__Thinking,
    /// Model: `qwen/qwen-turbo`
    Qwen___Qwen__Turbo,
    /// Model: `qwen/qwen-vl-max`
    Qwen___Qwen__Vl__Max,
    /// Model: `qwen/qwen-vl-plus`
    Qwen___Qwen__Vl__Plus,
    /// Model: `qwen/qwen2.5-coder-7b-instruct`
    Qwen___Qwen2_5__Coder__7b__Instruct,
    /// Model: `qwen/qwen2.5-vl-32b-instruct`
    Qwen___Qwen2_5__Vl__32b__Instruct,
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
    /// Model: `qwen/qwen3-4b:free`
    Qwen___Qwen3__4b__Free,
    /// Model: `qwen/qwen3-8b`
    Qwen___Qwen3__8b,
    /// Model: `qwen/qwen3-coder`
    Qwen___Qwen3__Coder,
    /// Model: `qwen/qwen3-coder-30b-a3b-instruct`
    Qwen___Qwen3__Coder__30b__A3b__Instruct,
    /// Model: `qwen/qwen3-coder-flash`
    Qwen___Qwen3__Coder__Flash,
    /// Model: `qwen/qwen3-coder-plus`
    Qwen___Qwen3__Coder__Plus,
    /// Model: `qwen/qwen3-coder:exacto`
    Qwen___Qwen3__Coder__Exacto,
    /// Model: `qwen/qwen3-coder:free`
    Qwen___Qwen3__Coder__Free,
    /// Model: `qwen/qwen3-max`
    Qwen___Qwen3__Max,
    /// Model: `qwen/qwen3-next-80b-a3b-instruct`
    Qwen___Qwen3__Next__80b__A3b__Instruct,
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
    /// Model: `qwen/qwq-32b`
    Qwen___Qwq__32b,
    /// Model: `raifle/sorcererlm-8x22b`
    Raifle___Sorcererlm__8x22b,
    /// Model: `relace/relace-apply-3`
    Relace___Relace__Apply__3,
    /// Model: `relace/relace-search`
    Relace___Relace__Search,
    /// Model: `sao10k/l3-euryale-70b`
    Sao10k___L3__Euryale__70b,
    /// Model: `sao10k/l3-lunaris-8b`
    Sao10k___L3__Lunaris__8b,
    /// Model: `sao10k/l3.1-70b-hanami-x1`
    Sao10k___L3_1__70b__Hanami__X1,
    /// Model: `sao10k/l3.1-euryale-70b`
    Sao10k___L3_1__Euryale__70b,
    /// Model: `sao10k/l3.3-euryale-70b`
    Sao10k___L3_3__Euryale__70b,
    /// Model: `stepfun-ai/step3`
    Stepfun__Ai___Step3,
    /// Model: `switchpoint/router`
    Switchpoint___Router,
    /// Model: `tencent/hunyuan-a13b-instruct`
    Tencent___Hunyuan__A13b__Instruct,
    /// Model: `thedrummer/cydonia-24b-v4.1`
    Thedrummer___Cydonia__24b__V4_1,
    /// Model: `thedrummer/rocinante-12b`
    Thedrummer___Rocinante__12b,
    /// Model: `thedrummer/skyfall-36b-v2`
    Thedrummer___Skyfall__36b__V2,
    /// Model: `thedrummer/unslopnemo-12b`
    Thedrummer___Unslopnemo__12b,
    /// Model: `thudm/glm-4.1v-9b-thinking`
    Thudm___Glm__4_1v__9b__Thinking,
    /// Model: `tngtech/deepseek-r1t-chimera`
    Tngtech___Deepseek__R1t__Chimera,
    /// Model: `tngtech/deepseek-r1t-chimera:free`
    Tngtech___Deepseek__R1t__Chimera__Free,
    /// Model: `tngtech/deepseek-r1t2-chimera`
    Tngtech___Deepseek__R1t2__Chimera,
    /// Model: `tngtech/deepseek-r1t2-chimera:free`
    Tngtech___Deepseek__R1t2__Chimera__Free,
    /// Model: `tngtech/tng-r1t-chimera`
    Tngtech___Tng__R1t__Chimera,
    /// Model: `tngtech/tng-r1t-chimera:free`
    Tngtech___Tng__R1t__Chimera__Free,
    /// Model: `undi95/remm-slerp-l2-13b`
    Undi95___Remm__Slerp__L2__13b,
    /// Model: `x-ai/grok-3`
    X__Ai___Grok__3,
    /// Model: `x-ai/grok-3-beta`
    X__Ai___Grok__3__Beta,
    /// Model: `x-ai/grok-3-mini`
    X__Ai___Grok__3__Mini,
    /// Model: `x-ai/grok-3-mini-beta`
    X__Ai___Grok__3__Mini__Beta,
    /// Model: `x-ai/grok-4`
    X__Ai___Grok__4,
    /// Model: `x-ai/grok-4-fast`
    X__Ai___Grok__4__Fast,
    /// Model: `x-ai/grok-4.1-fast`
    X__Ai___Grok__4_1__Fast,
    /// Model: `x-ai/grok-code-fast-1`
    X__Ai___Grok__Code__Fast__1,
    /// Model: `xiaomi/mimo-v2-flash:free`
    Xiaomi___Mimo__V2__Flash__Free,
    /// Model: `z-ai/glm-4-32b`
    Z__Ai___Glm__4__32b,
    /// Model: `z-ai/glm-4.5`
    Z__Ai___Glm__4_5,
    /// Model: `z-ai/glm-4.5-air`
    Z__Ai___Glm__4_5__Air,
    /// Model: `z-ai/glm-4.5-air:free`
    Z__Ai___Glm__4_5__Air__Free,
    /// Model: `z-ai/glm-4.5v`
    Z__Ai___Glm__4_5v,
    /// Model: `z-ai/glm-4.6`
    Z__Ai___Glm__4_6,
    /// Model: `z-ai/glm-4.6:exacto`
    Z__Ai___Glm__4_6__Exacto,
    /// Model: `z-ai/glm-4.6v`
    Z__Ai___Glm__4_6v,
    /// Model: `z-ai/glm-4.7`
    Z__Ai___Glm__4_7,
    /// Custom model ID not in the predefined list.
    Bespoke(String),
}
