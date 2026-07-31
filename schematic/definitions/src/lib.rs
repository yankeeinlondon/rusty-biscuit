//! Schematic API Definitions
//!
//! This crate contains actual REST API definitions that use the primitives
//! from `schematic-define`. Each API is organized in its own module.
//!
//! ## Available APIs
//!
//! - [`anthropic`] - Anthropic Messages API for Claude AI and agent tool use
//! - [`bitbucket`] - Bitbucket Cloud REST API for repositories, PRs, issues, and tags
//! - [`elevenlabs`] - ElevenLabs TTS and voice management API definition
//! - [`emqx`] - EMQX Broker REST API (Basic Auth + Bearer Token variants)
//! - [`eversolo`] - Eversolo DMP-A8 local HTTP control API (device, playback, I/O, display)
//! - [`gitea`] - Gitea REST API for self-hosted Git forge instances
//! - [`github`] - GitHub REST API for repositories, PRs, issues, and releases
//! - [`gitlab`] - GitLab REST API for repositories, MRs, issues, and releases
//! - [`huggingface`] - Hugging Face Hub API for model/dataset discovery
//! - [`lmstudio`] - LM Studio local LLM inference (v1 native API)
//! - [`ollama`] - Ollama local LLM inference (native + OpenAI-compatible APIs)
//! - [`openai`] - OpenAI Models API definition
//! - [`samsung_smart_tv`] - Samsung Smart TV LAN REST + Remote WebSocket API definitions
//! - [`unfolded_circle`] - Unfolded Circle Core REST + WebSocket API definitions
//!
//! ## Examples
//!
//! ```
//! use schematic_definitions::anthropic::define_anthropic_api;
//!
//! let api = define_anthropic_api();
//! assert_eq!(api.name, "Anthropic");
//! assert_eq!(api.endpoints.len(), 4);
//! ```
//!
//! ```
//! use schematic_definitions::openai::define_openai_api;
//!
//! let api = define_openai_api();
//! assert_eq!(api.name, "OpenAI");
//! // Imported from the vendored spec; the count tracks OpenAI's surface.
//! assert!(api.endpoints.len() > 200);
//! ```
//!
//! ```
//! use schematic_definitions::elevenlabs::{define_elevenlabs_rest_api, define_elevenlabs_websocket_api};
//!
//! let rest_api = define_elevenlabs_rest_api();
//! assert_eq!(rest_api.name, "ElevenLabs");
//! assert!(rest_api.endpoints.len() >= 35);
//!
//! let ws_api = define_elevenlabs_websocket_api();
//! assert_eq!(ws_api.name, "ElevenLabsTTS");
//! ```
//!
//! ```
//! use schematic_definitions::huggingface::define_huggingface_hub_api;
//!
//! let api = define_huggingface_hub_api();
//! assert_eq!(api.name, "HuggingFaceHub");
//! assert!(api.endpoints.len() >= 26);
//! ```
//!
//! ```
//! use schematic_definitions::ollama::{define_ollama_native_api, define_ollama_openai_api};
//!
//! let native_api = define_ollama_native_api();
//! assert_eq!(native_api.name, "OllamaNative");
//! assert_eq!(native_api.endpoints.len(), 11);
//!
//! let openai_api = define_ollama_openai_api();
//! assert_eq!(openai_api.name, "OllamaOpenAI");
//! assert_eq!(openai_api.endpoints.len(), 4);
//! ```
//!
//! ```
//! use schematic_definitions::lmstudio::define_lmstudio_api;
//!
//! let api = define_lmstudio_api();
//! assert_eq!(api.name, "LmStudio");
//! assert_eq!(api.endpoints.len(), 6);
//! ```
//!
//! ```
//! use schematic_definitions::artificial_analysis::{
//!     define_artificial_analysis_data_api,
//!     define_artificial_analysis_critpt_api,
//! };
//!
//! let data = define_artificial_analysis_data_api();
//! assert_eq!(data.name, "ArtificialAnalysisData");
//! assert_eq!(data.endpoints.len(), 6);
//!
//! let critpt = define_artificial_analysis_critpt_api();
//! assert_eq!(critpt.name, "ArtificialAnalysisCritPt");
//! assert_eq!(critpt.endpoints.len(), 1);
//! ```
//!
//! ```
//! use schematic_definitions::emqx::{define_emqx_basic_api, define_emqx_bearer_api};
//!
//! let basic_api = define_emqx_basic_api();
//! assert_eq!(basic_api.name, "EmqxBasic");
//! assert!(basic_api.endpoints.len() >= 30);
//!
//! let bearer_api = define_emqx_bearer_api();
//! assert_eq!(bearer_api.name, "EmqxBearer");
//! // Bearer API has login/logout plus all common endpoints
//! assert!(bearer_api.endpoints.len() > basic_api.endpoints.len());
//! ```
//!
//! ```
//! use schematic_definitions::eversolo::define_eversolo_api;
//!
//! let api = define_eversolo_api();
//! assert_eq!(api.name, "Eversolo");
//! assert_eq!(api.endpoints.len(), 24);
//! ```
//!
//! ```
//! use schematic_definitions::github::define_github_api;
//!
//! let api = define_github_api();
//! assert_eq!(api.name, "GitHub");
//! assert_eq!(api.endpoints.len(), 16);
//! ```
//!
//! ```
//! use schematic_definitions::gitea::define_gitea_api;
//!
//! let api = define_gitea_api();
//! assert_eq!(api.name, "Gitea");
//! assert_eq!(api.endpoints.len(), 15);
//! ```
//!
//! ```
//! use schematic_definitions::gitlab::define_gitlab_api;
//!
//! let api = define_gitlab_api();
//! assert_eq!(api.name, "GitLab");
//! assert_eq!(api.endpoints.len(), 18);
//! ```
//!
//! ```
//! use schematic_definitions::bitbucket::define_bitbucket_api;
//!
//! let api = define_bitbucket_api();
//! assert_eq!(api.name, "Bitbucket");
//! assert_eq!(api.endpoints.len(), 15);
//! ```
//!
//! ```
//! use schematic_definitions::samsung_smart_tv::{
//!     define_samsung_smart_tv_api,
//!     remote_ws::define_samsung_smart_tv_remote_ws_api,
//! };
//!
//! let rest_api = define_samsung_smart_tv_api();
//! assert_eq!(rest_api.name, "SamsungSmartTv");
//! assert_eq!(rest_api.endpoints.len(), 7);
//!
//! let ws_api = define_samsung_smart_tv_remote_ws_api();
//! assert_eq!(ws_api.name, "SamsungSmartTvRemote");
//! assert_eq!(ws_api.endpoints.len(), 2);
//! ```

pub mod anthropic;
pub mod artificial_analysis;
pub mod bitbucket;
pub mod elevenlabs;
pub mod emqx;
pub mod eversolo;
pub mod gitea;
pub mod github;
pub mod gitlab;
pub mod huggingface;
pub mod lmstudio;
pub mod ollama;
pub mod openai;
pub mod prelude;
pub mod registry;
pub mod samsung_smart_tv;
pub mod unfolded_circle;

// Re-export API definition functions for convenience
pub use anthropic::define_anthropic_api;
pub use artificial_analysis::{
    define_artificial_analysis_critpt_api, define_artificial_analysis_data_api,
};
pub use bitbucket::define_bitbucket_api;
pub use elevenlabs::{define_elevenlabs_rest_api, define_elevenlabs_websocket_api};
pub use emqx::{define_emqx_basic_api, define_emqx_bearer_api};
pub use eversolo::define_eversolo_api;
pub use gitea::define_gitea_api;
pub use github::define_github_api;
pub use gitlab::define_gitlab_api;
pub use huggingface::define_huggingface_hub_api;
pub use lmstudio::define_lmstudio_api;
pub use ollama::{define_ollama_native_api, define_ollama_openai_api};
pub use openai::define_openai_api;
pub use samsung_smart_tv::define_samsung_smart_tv_api;
pub use samsung_smart_tv::remote_ws::define_samsung_smart_tv_remote_ws_api;
pub use unfolded_circle::{
    define_unfolded_circle_core_rest_api, define_unfolded_circle_core_ws_api,
    define_unfolded_circle_dock_ws_api, define_unfolded_circle_integration_ws_api,
};

use indexmap::IndexMap;
use schematic_define::RestApi;

/// Returns all RestApi definitions grouped by resolved module name.
///
/// APIs are grouped by their `module_path` field if set, otherwise by their
/// lowercased name. This enables grouped export for APIs that share a module.
///
/// ## Examples
///
/// ```
/// use schematic_definitions::apis_by_module;
///
/// let grouped = apis_by_module();
/// assert!(grouped.contains_key("ollama")); // OllamaNative + OllamaOpenAI
/// assert!(grouped.contains_key("emqx")); // EmqxBasic + EmqxBearer
/// assert_eq!(grouped["ollama"].len(), 2);
/// assert_eq!(grouped["emqx"].len(), 2);
/// ```
pub fn apis_by_module() -> IndexMap<String, Vec<RestApi>> {
    let all_apis = vec![
        define_anthropic_api(),
        define_artificial_analysis_data_api(),
        define_artificial_analysis_critpt_api(),
        define_bitbucket_api(),
        define_openai_api(),
        define_elevenlabs_rest_api(),
        define_gitea_api(),
        define_github_api(),
        define_gitlab_api(),
        define_huggingface_hub_api(),
        define_lmstudio_api(),
        define_ollama_native_api(),
        define_ollama_openai_api(),
        define_emqx_basic_api(),
        define_emqx_bearer_api(),
        define_eversolo_api(),
        define_samsung_smart_tv_api(),
        define_unfolded_circle_core_rest_api(),
    ];

    let mut grouped: IndexMap<String, Vec<RestApi>> = IndexMap::new();

    for api in all_apis {
        let module_name = api
            .module_path
            .as_deref()
            .unwrap_or(&api.name.to_lowercase())
            .to_string();

        grouped.entry(module_name).or_default().push(api);
    }

    grouped
}
