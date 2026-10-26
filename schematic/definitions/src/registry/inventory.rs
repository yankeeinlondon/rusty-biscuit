//! The registration inventory: the one list of shipped APIs.
//!
//! Every consumer that enumerates APIs (the registry, `schematic-gen`'s
//! selection names and inputs, the REST exports, the WebSocket packaging,
//! and the tests that cover every API) derives its list from
//! [`registrations`]. Adding an API is one entry in it. A consumer that
//! needs only one protocol filters explicitly with
//! [`ApiRegistration::rest`] or [`ApiRegistration::websocket`].

use std::collections::HashMap;

use indexmap::IndexMap;
use schematic_define::{DefinitionError, RestApi, WebSocketApi};

/// Builds every REST API of one generated module from one
/// [`DefinitionContext`](schematic_define::DefinitionContext).
pub type ModuleGroup = fn() -> Result<Vec<RestApi>, DefinitionError>;

/// One registered API.
#[derive(Debug, Clone, Copy)]
pub struct ApiRegistration {
    /// The API's `name` (`RestApi::name` or `WebSocketApi::name`).
    pub name: &'static str,
    /// The generated module the API lives in. REST APIs that share a module
    /// are one module group; the module is also the export file stem.
    pub module: &'static str,
    /// The `schematic_definitions` module that defines the API, as a
    /// `::`-separated path (`unfolded_circle::core_rest`).
    pub definitions_path: &'static str,
    /// How the API is spoken, with what constructs it.
    pub protocol: Protocol,
}

/// The protocol of a registered API and its constructors.
#[derive(Debug, Clone, Copy)]
pub enum Protocol {
    /// An HTTP API built from a module group.
    Rest(RestRegistration),
    /// A WebSocket API.
    WebSocket(WebSocketRegistration),
}

/// How a REST API is selected and constructed.
#[derive(Debug, Clone, Copy)]
pub struct RestRegistration {
    /// The `schematic-gen --api` selection name, which is also the stable
    /// per-API artifact key (see [`registry_key_for`]).
    pub key: &'static str,
    /// The module group that builds this API.
    pub group: ModuleGroup,
    /// This API's index in what `group` returns. Every member of a group
    /// must be registered; [`rest_apis`] panics on an unregistered one.
    pub member: usize,
}

/// How a WebSocket API is constructed and described.
#[derive(Debug, Clone, Copy)]
pub struct WebSocketRegistration {
    /// Builds the definition.
    pub define: fn() -> WebSocketApi,
    /// The name of `define` as re-exported at the `schematic_definitions`
    /// crate root; generated code calls it by this name.
    pub constructor: &'static str,
    /// One line naming the definition, written into generated docs.
    pub summary: &'static str,
}

impl ApiRegistration {
    /// The REST constructors, when this is a REST API.
    #[must_use]
    pub fn rest(&self) -> Option<&RestRegistration> {
        match &self.protocol {
            Protocol::Rest(rest) => Some(rest),
            Protocol::WebSocket(_) => None,
        }
    }

    /// The WebSocket constructors, when this is a WebSocket API.
    #[must_use]
    pub fn websocket(&self) -> Option<&WebSocketRegistration> {
        match &self.protocol {
            Protocol::WebSocket(ws) => Some(ws),
            Protocol::Rest(_) => None,
        }
    }
}

const fn rest(
    name: &'static str,
    key: &'static str,
    module: &'static str,
    definitions_path: &'static str,
    group: ModuleGroup,
    member: usize,
) -> ApiRegistration {
    ApiRegistration {
        name,
        module,
        definitions_path,
        protocol: Protocol::Rest(RestRegistration { key, group, member }),
    }
}

const fn websocket(
    name: &'static str,
    module: &'static str,
    definitions_path: &'static str,
    define: fn() -> WebSocketApi,
    constructor: &'static str,
    summary: &'static str,
) -> ApiRegistration {
    ApiRegistration {
        name,
        module,
        definitions_path,
        protocol: Protocol::WebSocket(WebSocketRegistration { define, constructor, summary }),
    }
}

use crate::{
    anthropic, artificial_analysis, bitbucket, elevenlabs, emqx, eversolo, gitea, github, gitlab,
    huggingface, lmstudio, ollama, openai, samsung_smart_tv, unfolded_circle,
};

/// Every shipped API in catalog order, REST first, then WebSocket.
const REGISTRATIONS: &[ApiRegistration] = &[
    rest("Anthropic", "anthropic", "anthropic", "anthropic", anthropic::module_group, 0),
    rest("ArtificialAnalysisData", "artificial-analysis-data", "artificial_analysis", "artificial_analysis", artificial_analysis::module_group, 0),
    rest("ArtificialAnalysisCritPt", "artificial-analysis-critpt", "artificial_analysis", "artificial_analysis", artificial_analysis::module_group, 1),
    rest("Bitbucket", "bitbucket", "bitbucket", "bitbucket", bitbucket::module_group, 0),
    rest("OpenAI", "openai", "openai", "openai", openai::module_group, 0),
    rest("ElevenLabs", "elevenlabs", "elevenlabs", "elevenlabs", elevenlabs::module_group, 0),
    rest("Gitea", "gitea", "gitea", "gitea", gitea::module_group, 0),
    rest("GitHub", "github", "github", "github", github::module_group, 0),
    rest("GitLab", "gitlab", "gitlab", "gitlab", gitlab::module_group, 0),
    rest("HuggingFaceHub", "huggingface", "huggingface", "huggingface", huggingface::module_group, 0),
    rest("LmStudio", "lmstudio", "lmstudio", "lmstudio", lmstudio::module_group, 0),
    rest("OllamaNative", "ollama-native", "ollama", "ollama", ollama::module_group, 0),
    rest("OllamaOpenAI", "ollama-openai", "ollama", "ollama", ollama::module_group, 1),
    rest("EmqxBasic", "emqx-basic", "emqx", "emqx", emqx::module_group, 0),
    rest("EmqxBearer", "emqx-bearer", "emqx", "emqx", emqx::module_group, 1),
    rest("Eversolo", "eversolo", "eversolo", "eversolo", eversolo::module_group, 0),
    rest("SamsungSmartTv", "samsung-smart-tv", "samsung_smart_tv", "samsung_smart_tv", samsung_smart_tv::module_group, 0),
    rest("UnfoldedCircleCoreRest", "unfolded-circle-core-rest", "unfolded_circle_core_rest", "unfolded_circle::core_rest", unfolded_circle::core_rest::module_group, 0),
    websocket("ElevenLabsTTS", "elevenlabs_ws", "elevenlabs", elevenlabs::define_elevenlabs_websocket_api, "define_elevenlabs_websocket_api", "ElevenLabs Text-to-Speech WebSocket API definition"),
    websocket("UnfoldedCircleCoreWs", "unfolded_circle_core_ws", "unfolded_circle::core_ws", unfolded_circle::define_unfolded_circle_core_ws_api, "define_unfolded_circle_core_ws_api", "Unfolded Circle Core WebSocket API definition"),
    websocket("UnfoldedCircleDockWs", "unfolded_circle_dock_ws", "unfolded_circle::dock_ws", unfolded_circle::define_unfolded_circle_dock_ws_api, "define_unfolded_circle_dock_ws_api", "Unfolded Circle Dock WebSocket API definition"),
    websocket("UnfoldedCircleIntegrationWs", "unfolded_circle_integration_ws", "unfolded_circle::integration_ws", unfolded_circle::define_unfolded_circle_integration_ws_api, "define_unfolded_circle_integration_ws_api", "Unfolded Circle Integration WebSocket API definition"),
    websocket("SamsungSmartTvRemote", "samsung_smart_tv_remote_ws", "samsung_smart_tv::remote_ws", samsung_smart_tv::remote_ws::define_samsung_smart_tv_remote_ws_api, "define_samsung_smart_tv_remote_ws_api", "Samsung Smart TV Remote Control WebSocket API definition"),
];

/// Every shipped API, in catalog order.
///
/// ## Examples
///
/// ```
/// use schematic_definitions::registry::registrations;
///
/// let rest = registrations().iter().filter(|entry| entry.rest().is_some()).count();
/// let websocket = registrations().iter().filter(|entry| entry.websocket().is_some()).count();
/// assert_eq!((rest, websocket), (18, 5));
/// ```
#[must_use]
pub fn registrations() -> &'static [ApiRegistration] {
    REGISTRATIONS
}

/// The REST registration selected by `key`, if any.
#[must_use]
pub fn find_rest<'a>(registrations: &'a [ApiRegistration], key: &str) -> Option<&'a ApiRegistration> {
    registrations.iter().find(|entry| entry.rest().is_some_and(|rest| rest.key == key))
}

/// Builds the REST APIs of `registrations`, in their order, each assembled
/// with its module group's schema components.
///
/// Each module group is built once.
///
/// ## Panics
///
/// When a module group fails to build, returns an API whose name differs
/// from its registration, or builds an API that no registration names. A
/// registration error is a defect in the inventory, never a runtime input.
#[must_use]
pub fn rest_apis(registrations: &[ApiRegistration]) -> Vec<RestApi> {
    let mut groups: HashMap<&str, Vec<Option<RestApi>>> = HashMap::new();
    let mut apis = Vec::new();
    for entry in registrations {
        let Some(rest) = entry.rest() else { continue };
        let members = groups.entry(entry.module).or_insert_with(|| {
            (rest.group)()
                .unwrap_or_else(|error| panic!("module group `{}` does not build: {error}", entry.module))
                .into_iter()
                .map(Some)
                .collect()
        });
        let api = members
            .get_mut(rest.member)
            .and_then(Option::take)
            .unwrap_or_else(|| panic!("`{}` names member {} of module group `{}`, which has no such unclaimed member", entry.name, rest.member, entry.module));
        assert_eq!(api.name, entry.name, "member {} of module group `{}`", rest.member, entry.module);
        apis.push(api);
    }
    for (module, members) in groups {
        if let Some(api) = members.into_iter().flatten().next() {
            panic!("module group `{module}` builds `{}`, which has no registration", api.name);
        }
    }
    apis
}

/// [`rest_apis`] grouped by registered module, in first-appearance order.
#[must_use]
pub fn group_by_module(registrations: &[ApiRegistration]) -> IndexMap<String, Vec<RestApi>> {
    let modules = registrations.iter().filter(|entry| entry.rest().is_some()).map(|entry| entry.module);
    let mut grouped: IndexMap<String, Vec<RestApi>> = IndexMap::new();
    for (module, api) in modules.zip(rest_apis(registrations)) {
        grouped.entry(module.to_owned()).or_default().push(api);
    }
    grouped
}

/// Builds the WebSocket APIs of `registrations`, in their order.
#[must_use]
pub fn websocket_apis(registrations: &[ApiRegistration]) -> Vec<WebSocketApi> {
    registrations.iter().filter_map(ApiRegistration::websocket).map(|ws| (ws.define)()).collect()
}

/// Returns the stable kebab-case key for a shipped API name.
///
/// `RestApi::name` uses the API's struct-style name (e.g. `"OllamaNative"`);
/// per-API artifacts are keyed by kebab-case identifiers
/// (e.g. `"ollama-native"`), which are the registrations'
/// [`RestRegistration::key`]s.
///
/// ## Returns
///
/// The registered key when `api_name` is a shipped REST API; otherwise the
/// lowercased input as a best-effort fallback.
///
/// ## Examples
///
/// ```
/// use schematic_definitions::registry::registry_key_for;
///
/// assert_eq!(registry_key_for("OllamaNative"), "ollama-native");
/// assert_eq!(registry_key_for("HuggingFaceHub"), "huggingface");
/// ```
#[must_use]
pub fn registry_key_for(api_name: &str) -> String {
    REGISTRATIONS
        .iter()
        .filter(|entry| entry.name == api_name)
        .find_map(ApiRegistration::rest)
        .map_or_else(|| api_name.to_lowercase(), |rest| rest.key.to_owned())
}
