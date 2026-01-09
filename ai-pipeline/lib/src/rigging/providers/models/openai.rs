#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, strum::AsRefStr)]
pub enum ProviderModelOpenAi {
    Gpt_4_0613,
    Gpt_4,
    Gpt_3_5_Turbo,
    Chatgpt_Image_Latest,
    Gpt_4o_Mini_Tts_2025_03_20,
    Gpt_4o_Mini_Tts_2025_12_15,
    Gpt_Realtime_Mini_2025_12_15,
    Gpt_Audio_Mini_2025_12_15,
    Davinci_002,
    Babbage_002,
    Gpt_3_5_Turbo_Instruct,
    Gpt_3_5_Turbo_Instruct_0914,
    Dall_E_3,
    Dall_E_2,
    Gpt_4_1106_Preview,
    Gpt_3_5_Turbo_1106,
    Tts_1_Hd,
    Tts_1_1106,
    Tts_1_Hd_1106,
    Text_Embedding_3_Small,
    Text_Embedding_3_Large,
    Gpt_4_0125_Preview,
    Gpt_4_Turbo_Preview,
    Gpt_3_5_Turbo_0125,
    Gpt_4_Turbo,
    Gpt_4_Turbo_2024_04_09,
    Gpt_4o_2024_05_13,
    Gpt_4o_Mini_2024_07_18,
    Gpt_4o_2024_08_06,
    Chatgpt_4o_Latest,
    Gpt_4o_Audio_Preview,
    Gpt_4o_Realtime_Preview,
    Omni_Moderation_Latest,
    Omni_Moderation_2024_09_26,
    Gpt_4o_Realtime_Preview_2024_12_17,
    Gpt_4o_Audio_Preview_2024_12_17,
    Gpt_4o_Mini_Realtime_Preview_2024_12_17,
    Gpt_4o_Mini_Audio_Preview_2024_12_17,
    O1_2024_12_17,
    Gpt_4o_Mini_Realtime_Preview,
    Gpt_4o_Mini_Audio_Preview,
    O3_Mini,
    O3_Mini_2025_01_31,
    Gpt_4o_2024_11_20,
    Gpt_4o_Search_Preview_2025_03_11,
    Gpt_4o_Search_Preview,
    Gpt_4o_Mini_Search_Preview_2025_03_11,
    Gpt_4o_Mini_Search_Preview,
    Gpt_4o_Transcribe,
    Gpt_4o_Mini_Transcribe,
    O1_Pro_2025_03_19,
    O1_Pro,
    Gpt_4o_Mini_Tts,
    O3_2025_04_16,
    O4_Mini_2025_04_16,
    O3,
    O4_Mini,
    Gpt_4_1_2025_04_14,
    Gpt_4_1,
    Gpt_4_1_Mini_2025_04_14,
    Gpt_4_1_Mini,
    Gpt_4_1_Nano_2025_04_14,
    Gpt_4_1_Nano,
    Gpt_Image_1,
    Gpt_4o_Realtime_Preview_2025_06_03,
    Gpt_4o_Audio_Preview_2025_06_03,
    Gpt_4o_Transcribe_Diarize,
    Gpt_5_Chat_Latest,
    Gpt_5_2025_08_07,
    Gpt_5,
    Gpt_5_Mini_2025_08_07,
    Gpt_5_Mini,
    Gpt_5_Nano_2025_08_07,
    Gpt_5_Nano,
    Gpt_Audio_2025_08_28,
    Gpt_Realtime,
    Gpt_Realtime_2025_08_28,
    Gpt_Audio,
    Gpt_5_Codex,
    Gpt_Image_1_Mini,
    Gpt_5_Pro_2025_10_06,
    Gpt_5_Pro,
    Gpt_Audio_Mini,
    Gpt_Audio_Mini_2025_10_06,
    Gpt_5_Search_Api,
    Gpt_Realtime_Mini,
    Gpt_Realtime_Mini_2025_10_06,
    Sora_2,
    Sora_2_Pro,
    Gpt_5_Search_Api_2025_10_14,
    Gpt_5_1_Chat_Latest,
    Gpt_5_1_2025_11_13,
    Gpt_5_1,
    Gpt_5_1_Codex,
    Gpt_5_1_Codex_Mini,
    Gpt_5_1_Codex_Max,
    Gpt_Image_1_5,
    Gpt_5_2_2025_12_11,
    Gpt_5_2,
    Gpt_5_2_Pro_2025_12_11,
    Gpt_5_2_Pro,
    Gpt_5_2_Chat_Latest,
    Gpt_4o_Mini_Transcribe_2025_12_15,
    Gpt_4o_Mini_Transcribe_2025_03_20,
    Gpt_3_5_Turbo_16k,
    Tts_1,
    Whisper_1,
    Text_Embedding_Ada_002,
    Bespoke(String)
}

impl ProviderModelOpenAi {

    /// Canonical OpenAI model id to send over the wire.
    ///
    /// - Most variants return a static string literal.
    /// - `Bespoke(String)` returns a borrow of the stored string.
    pub fn model_id(&self) -> &str {
        match self {
            Self::Gpt_4_0613 => "gpt-4-0613",
            Self::Gpt_4 => "gpt-4",
            Self::Gpt_3_5_Turbo => "gpt-3.5-turbo",
            Self::Chatgpt_Image_Latest => "chatgpt-image-latest",

            Self::Gpt_4o_Mini_Tts_2025_03_20 => "gpt-4o-mini-tts-2025-03-20",
            Self::Gpt_4o_Mini_Tts_2025_12_15 => "gpt-4o-mini-tts-2025-12-15",
            Self::Gpt_Realtime_Mini_2025_12_15 => "gpt-realtime-mini-2025-12-15",
            Self::Gpt_Audio_Mini_2025_12_15 => "gpt-audio-mini-2025-12-15",

            Self::Davinci_002 => "davinci-002",
            Self::Babbage_002 => "babbage-002",

            Self::Gpt_3_5_Turbo_Instruct => "gpt-3.5-turbo-instruct",
            Self::Gpt_3_5_Turbo_Instruct_0914 => "gpt-3.5-turbo-instruct-0914",

            Self::Dall_E_3 => "dall-e-3",
            Self::Dall_E_2 => "dall-e-2",

            Self::Gpt_4_1106_Preview => "gpt-4-1106-preview",
            Self::Gpt_3_5_Turbo_1106 => "gpt-3.5-turbo-1106",
            Self::Gpt_4_0125_Preview => "gpt-4-0125-preview",
            Self::Gpt_4_Turbo_Preview => "gpt-4-turbo-preview",
            Self::Gpt_3_5_Turbo_0125 => "gpt-3.5-turbo-0125",

            Self::Gpt_4_Turbo => "gpt-4-turbo",
            Self::Gpt_4_Turbo_2024_04_09 => "gpt-4-turbo-2024-04-09",

            Self::Gpt_4o_2024_05_13 => "gpt-4o-2024-05-13",
            Self::Gpt_4o_Mini_2024_07_18 => "gpt-4o-mini-2024-07-18",
            Self::Gpt_4o_2024_08_06 => "gpt-4o-2024-08-06",
            Self::Chatgpt_4o_Latest => "chatgpt-4o-latest",

            Self::Gpt_4o_Audio_Preview => "gpt-4o-audio-preview",
            Self::Gpt_4o_Realtime_Preview => "gpt-4o-realtime-preview",

            Self::Omni_Moderation_Latest => "omni-moderation-latest",
            Self::Omni_Moderation_2024_09_26 => "omni-moderation-2024-09-26",

            Self::Gpt_4o_Realtime_Preview_2024_12_17 => "gpt-4o-realtime-preview-2024-12-17",
            Self::Gpt_4o_Audio_Preview_2024_12_17 => "gpt-4o-audio-preview-2024-12-17",
            Self::Gpt_4o_Mini_Realtime_Preview_2024_12_17 => "gpt-4o-mini-realtime-preview-2024-12-17",
            Self::Gpt_4o_Mini_Audio_Preview_2024_12_17 => "gpt-4o-mini-audio-preview-2024-12-17",

            Self::O1_2024_12_17 => "o1-2024-12-17",
            Self::Gpt_4o_Mini_Realtime_Preview => "gpt-4o-mini-realtime-preview",
            Self::Gpt_4o_Mini_Audio_Preview => "gpt-4o-mini-audio-preview",

            Self::O3_Mini => "o3-mini",
            Self::O3_Mini_2025_01_31 => "o3-mini-2025-01-31",

            Self::Gpt_4o_2024_11_20 => "gpt-4o-2024-11-20",
            Self::Gpt_4o_Search_Preview_2025_03_11 => "gpt-4o-search-preview-2025-03-11",
            Self::Gpt_4o_Search_Preview => "gpt-4o-search-preview",
            Self::Gpt_4o_Mini_Search_Preview_2025_03_11 => "gpt-4o-mini-search-preview-2025-03-11",
            Self::Gpt_4o_Mini_Search_Preview => "gpt-4o-mini-search-preview",

            Self::Gpt_4o_Transcribe => "gpt-4o-transcribe",
            Self::Gpt_4o_Mini_Transcribe => "gpt-4o-mini-transcribe",

            Self::O1_Pro_2025_03_19 => "o1-pro-2025-03-19",
            Self::O1_Pro => "o1-pro",

            Self::Gpt_4o_Mini_Tts => "gpt-4o-mini-tts",

            Self::O3_2025_04_16 => "o3-2025-04-16",
            Self::O4_Mini_2025_04_16 => "o4-mini-2025-04-16",
            Self::O3 => "o3",
            Self::O4_Mini => "o4-mini",

            Self::Gpt_4_1_2025_04_14 => "gpt-4.1-2025-04-14",
            Self::Gpt_4_1 => "gpt-4.1",
            Self::Gpt_4_1_Mini_2025_04_14 => "gpt-4.1-mini-2025-04-14",
            Self::Gpt_4_1_Mini => "gpt-4.1-mini",
            Self::Gpt_4_1_Nano_2025_04_14 => "gpt-4.1-nano-2025-04-14",
            Self::Gpt_4_1_Nano => "gpt-4.1-nano",

            Self::Gpt_Image_1 => "gpt-image-1",

            Self::Gpt_4o_Realtime_Preview_2025_06_03 => "gpt-4o-realtime-preview-2025-06-03",
            Self::Gpt_4o_Audio_Preview_2025_06_03 => "gpt-4o-audio-preview-2025-06-03",

            Self::Gpt_4o_Transcribe_Diarize => "gpt-4o-transcribe-diarize",

            Self::Gpt_5_Chat_Latest => "gpt-5-chat-latest",
            Self::Gpt_5_2025_08_07 => "gpt-5-2025-08-07",
            Self::Gpt_5 => "gpt-5",
            Self::Gpt_5_Mini_2025_08_07 => "gpt-5-mini-2025-08-07",
            Self::Gpt_5_Mini => "gpt-5-mini",
            Self::Gpt_5_Nano_2025_08_07 => "gpt-5-nano-2025-08-07",
            Self::Gpt_5_Nano => "gpt-5-nano",

            Self::Gpt_Audio_2025_08_28 => "gpt-audio-2025-08-28",
            Self::Gpt_Realtime => "gpt-realtime",
            Self::Gpt_Realtime_2025_08_28 => "gpt-realtime-2025-08-28",
            Self::Gpt_Audio => "gpt-audio",

            Self::Gpt_5_Codex => "gpt-5-codex",
            Self::Gpt_Image_1_Mini => "gpt-image-1-mini",

            Self::Gpt_5_Pro_2025_10_06 => "gpt-5-pro-2025-10-06",
            Self::Gpt_5_Pro => "gpt-5-pro",

            Self::Gpt_Audio_Mini => "gpt-audio-mini",
            Self::Gpt_Audio_Mini_2025_10_06 => "gpt-audio-mini-2025-10-06",

            Self::Gpt_5_Search_Api => "gpt-5-search-api",
            Self::Gpt_5_Search_Api_2025_10_14 => "gpt-5-search-api-2025-10-14",

            Self::Gpt_Realtime_Mini => "gpt-realtime-mini",
            Self::Gpt_Realtime_Mini_2025_10_06 => "gpt-realtime-mini-2025-10-06",
            Self::Gpt_Realtime_Mini_2025_12_15 => "gpt-realtime-mini-2025-12-15",

            Self::Gpt_Audio_Mini => "gpt-audio-mini",
            Self::Gpt_Audio_Mini_2025_10_06 => "gpt-audio-mini-2025-10-06",
            Self::Gpt_Audio_Mini_2025_12_15 => "gpt-audio-mini-2025-12-15",

            Self::Sora_2 => "sora-2",
            Self::Sora_2_Pro => "sora-2-pro",

            Self::Gpt_5_1_Chat_Latest => "gpt-5.1-chat-latest",
            Self::Gpt_5_1_2025_11_13 => "gpt-5.1-2025-11-13",
            Self::Gpt_5_1 => "gpt-5.1",
            Self::Gpt_5_1_Codex => "gpt-5.1-codex",
            Self::Gpt_5_1_Codex_Mini => "gpt-5.1-codex-mini",
            Self::Gpt_5_1_Codex_Max => "gpt-5.1-codex-max",

            Self::Gpt_Image_1_5 => "gpt-image-1.5",

            Self::Gpt_5_2_2025_12_11 => "gpt-5.2-2025-12-11",
            Self::Gpt_5_2 => "gpt-5.2",
            Self::Gpt_5_2_Pro_2025_12_11 => "gpt-5.2-pro-2025-12-11",
            Self::Gpt_5_2_Pro => "gpt-5.2-pro",
            Self::Gpt_5_2_Chat_Latest => "gpt-5.2-chat-latest",

            Self::Gpt_4o_Mini_Transcribe_2025_12_15 => "gpt-4o-mini-transcribe-2025-12-15",
            Self::Gpt_4o_Mini_Transcribe_2025_03_20 => "gpt-4o-mini-transcribe-2025-03-20",

            Self::Gpt_3_5_Turbo_16k => "gpt-3.5-turbo-16k",

            Self::Tts_1 => "tts-1",
            Self::Tts_1_Hd => "tts-1-hd",
            Self::Tts_1_1106 => "tts-1-1106",
            Self::Tts_1_Hd_1106 => "tts-1-hd-1106",

            Self::Whisper_1 => "whisper-1",

            Self::Text_Embedding_Ada_002 => "text-embedding-ada-002",
            Self::Text_Embedding_3_Small => "text-embedding-3-small",
            Self::Text_Embedding_3_Large => "text-embedding-3-large",

            Self::Bespoke(s) => s.as_str(),

    }
  }
}
