//! Every API outside the forges and OpenAI.
//!
//! Only ElevenLabs pages anything. The other lists here are unpaginated:
//! recalled but unverified, blocked by a definition gap, or root arrays in an
//! API with no default.

use schematic_define::pagination::Direction::Forward;
use schematic_define::pagination::IndexBase::ZeroBased;
use schematic_define::pagination::ResolutionOrigin::{Exception, Unauthored};
use schematic_define::pagination::{Resolution, resolve};
use schematic_definitions::apis_by_module;

use crate::{Api, Next, Row, Unpaged, page_size, resume, start_page};

pub fn apis() -> Vec<Api> {
    let mut apis = vec![
        anthropic(),
        elevenlabs(),
        huggingface(),
        emqx("EmqxBasic"),
        emqx("EmqxBearer"),
    ];
    apis.extend(
        [
            "ArtificialAnalysisData",
            "ArtificialAnalysisCritPt",
            "LmStudio",
            "OllamaNative",
            "OllamaOpenAI",
            "Eversolo",
            "SamsungSmartTv",
            "UnfoldedCircleCoreRest",
        ]
        .into_iter()
        .map(|name| Api { name, rows: vec![] }),
    );
    apis
}

fn anthropic() -> Api {
    Api {
        name: "Anthropic",
        rows: vec![Row::unpaged(
            "ListModels",
            Exception,
            "data[*]",
            Unpaged::Unverified {
                recalled: "Cursor{after_id <- body last_id; before_id <- body first_id; \
                           Flag(has_more)}, limit 1..1000 default 20",
                source_needed: "Anthropic API reference for GET /v1/models",
            },
        )],
    }
}

fn elevenlabs() -> Api {
    Api {
        name: "ElevenLabs",
        rows: vec![
            Row::paged(
                "GetHistory",
                Exception,
                "history[*]",
                "SpeechHistoryItemResponseModel",
                "cursor: start_after_history_item_id <- body last_history_item_id; \
                 more flag body has_more",
                vec![
                    resume(Forward),
                    page_size("page_size", 1, Some(1000), Some(100)),
                    Next,
                ],
                &[],
            ),
            Row::paged(
                "ListSharedVoices",
                Exception,
                "voices[*]",
                "LibraryVoiceResponseModel",
                "page_number: page page from 0; more flag body has_more",
                vec![
                    start_page(ZeroBased),
                    page_size("page_size", 1, Some(100), Some(30)),
                    Next,
                ],
                &[],
            ),
            Row::unpaged(
                "ListVoices",
                Exception,
                "voices[*]",
                Unpaged::DefinitionGap {
                    gap: "the definition sends the cursor as `page_token`; the research note \
                          names the request input `next_page_token`",
                    recalled: "Cursor{next_page_token <- body next_page_token; Flag(has_more)}, \
                               page_size max 100 default 10, total body total_count",
                    source_needed: "ElevenLabs API reference for GET /v2/voices, confirming \
                                    the cursor's request parameter name",
                },
            ),
            Row::unpaged("ListModels", Unauthored, "[*]", Unpaged::NoPagination),
        ],
    }
}

/// A Hugging Face Hub root array: no API default, so it resolves unpaginated.
fn hub_root(endpoint: &'static str, recalled: &'static str, source_needed: &'static str) -> Row {
    Row::unpaged(
        endpoint,
        Unauthored,
        "[*]",
        Unpaged::Unverified {
            recalled,
            source_needed,
        },
    )
}

const HUB_NEXT_LINK: &str = "NextLink{Link rel=next}, size limit";
const HUB_COMMITS: &str = "PageNumber{p, 0-based}, end on an empty page";
const HUB_UNKNOWN: &str = "unknown; the endpoint itself is unverified";

fn huggingface() -> Api {
    Api {
        name: "HuggingFaceHub",
        rows: vec![
            hub_root(
                "ListModels",
                HUB_NEXT_LINK,
                "Hugging Face Hub API reference for GET /api/models",
            ),
            hub_root(
                "ListModelFiles",
                HUB_NEXT_LINK,
                "Hugging Face Hub API reference for GET /api/models/{repo_id}/tree/{revision}",
            ),
            hub_root(
                "ListModelCommits",
                HUB_COMMITS,
                "Hugging Face Hub API reference for GET /api/models/{repo_id}/commits/{revision}",
            ),
            Row::unpaged(
                "ListModelDiscussions",
                Exception,
                "discussions[*]",
                Unpaged::Unverified {
                    recalled: "PageNumber{p, 0-based}, end at body count",
                    source_needed: "Hugging Face Hub API reference for \
                                    GET /api/models/{repo_id}/discussions",
                },
            ),
            hub_root(
                "ListDatasets",
                HUB_NEXT_LINK,
                "Hugging Face Hub API reference for GET /api/datasets",
            ),
            hub_root(
                "ListDatasetFiles",
                HUB_NEXT_LINK,
                "Hugging Face Hub API reference for GET /api/datasets/{repo_id}/tree/{revision}",
            ),
            hub_root(
                "ListDatasetCommits",
                HUB_COMMITS,
                "Hugging Face Hub API reference for GET /api/datasets/{repo_id}/commits/{revision}",
            ),
            hub_root(
                "ListSpaces",
                HUB_NEXT_LINK,
                "Hugging Face Hub API reference for GET /api/spaces",
            ),
            hub_root(
                "ListSpaceFiles",
                HUB_NEXT_LINK,
                "Hugging Face Hub API reference for GET /api/spaces/{repo_id}/tree/{revision}",
            ),
            hub_root(
                "ListUserRepos",
                HUB_UNKNOWN,
                "Hugging Face Hub API reference for GET /api/users/{username}/repos",
            ),
            hub_root(
                "GetUserCollections",
                HUB_UNKNOWN,
                "Hugging Face Hub API reference for GET /api/users/{username}/collections",
            ),
        ],
    }
}

const EMQX_RECALLED: &str = "PageNumber{page, limit, Flag(meta.hasnext)}, total meta.count";

/// An EMQX `{data, meta}` list, left unpaginated.
fn emqx_data(endpoint: &'static str, source_needed: &'static str) -> Row {
    Row::unpaged(
        endpoint,
        Exception,
        "data[*]",
        Unpaged::Unverified {
            recalled: EMQX_RECALLED,
            source_needed,
        },
    )
}

/// EMQX basic and bearer share their endpoints, so their rows are identical.
fn emqx(name: &'static str) -> Api {
    Api {
        name,
        rows: vec![
            emqx_data("ListAlarms", "EMQX 5 REST API reference for GET /api/v5/alarms"),
            emqx_data("ListClients", "EMQX 5 REST API reference for GET /api/v5/clients"),
            emqx_data(
                "ListSubscriptions",
                "EMQX 5 REST API reference for GET /api/v5/subscriptions",
            ),
            emqx_data("ListTopics", "EMQX 5 REST API reference for GET /api/v5/topics"),
            emqx_data(
                "ListRetained",
                "EMQX 5 REST API reference for GET /api/v5/retainer/messages",
            ),
            emqx_data("ListRules", "EMQX 5 REST API reference for GET /api/v5/rules"),
            emqx_data("ListBanned", "EMQX 5 REST API reference for GET /api/v5/banned"),
            Row::unpaged(
                "ListAuthUsers",
                Unauthored,
                "[*]",
                Unpaged::DefinitionGap {
                    gap: "the definition declares a root `Vec<AuthUser>`; the server is \
                          believed to reply `{data, meta}`",
                    recalled: EMQX_RECALLED,
                    source_needed: "EMQX 5 REST API reference for \
                                    GET /api/v5/authentication/{id}/users",
                },
            ),
        ],
    }
}

/// Writes that return an array alias are not lists: only a GET whose response
/// is literally `Vec<T>` is promoted without a row.
#[test]
fn unfolded_circle_writes_returning_arrays_are_not_lists() {
    let api = apis_by_module()
        .into_values()
        .flatten()
        .find(|api| api.name == "UnfoldedCircleCoreRest")
        .expect("the Unfolded Circle Core REST API ships");
    for id in ["RestoreBackup", "UploadResource"] {
        let endpoint = api
            .endpoints
            .iter()
            .find(|endpoint| endpoint.id == id)
            .unwrap_or_else(|| panic!("{id} is an Unfolded Circle endpoint"));
        assert_eq!(resolve(&api, endpoint), Resolution::NotAList, "{id}");
    }
}
