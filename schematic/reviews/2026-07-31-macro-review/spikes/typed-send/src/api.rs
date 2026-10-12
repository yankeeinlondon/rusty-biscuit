//! Hand-written stand-in for one generated API module (`requests.rs` +
//! `types.rs`). Everything here is what the generator would emit; nothing
//! in `client.rs` is per-endpoint.

use serde::{Deserialize, Serialize};

use crate::runtime::{
    ApiError, Binary, Decode, DecodeCtx, DecodeError, Empty, Endpoint, ErrorBody, GraphQl,
    GraphQlFailure, Json, Ndjson, NoDeclaredError, Sse, Text, decode_json_error,
};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Repo {
    pub id: u64,
    pub full_name: String,
    pub default_branch: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pull {
    pub number: u64,
    pub title: String,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeekActivity {
    pub week: u64,
    pub total: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatChunk {
    pub delta: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PullProgress {
    pub status: String,
    #[serde(default)]
    pub completed: Option<u64>,
}

/// Declared error body (GitHub style).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GhError {
    pub message: String,
    #[serde(default)]
    pub documentation_url: Option<String>,
}

impl ErrorBody for GhError {
    fn decode(body: &[u8]) -> Result<Self, DecodeError> {
        decode_json_error(body)
    }
}

// ---------------------------------------------------------------------------
// Request plumbing (same role as today's `into_parts`)
// ---------------------------------------------------------------------------

pub enum RequestBody {
    Empty,
    Json(Vec<u8>),
}

pub struct RequestParts {
    pub method: &'static str,
    pub path: String,
    /// Separated from the path (today it is formatted into it) so the
    /// envelope can report and redact query parameters.
    pub query: Vec<(String, String)>,
    pub body: RequestBody,
    pub headers: Vec<(String, String)>,
}

fn enc(s: &str) -> String {
    crate::client::percent_encode(s)
}

fn json_body<T: Serialize>(value: &T) -> Result<RequestBody, String> {
    serde_json::to_vec(value)
        .map(RequestBody::Json)
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// 1. Plain JSON success, declared JSON error
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct GetRepoRequest {
    pub owner: String,
    pub repo: String,
}

impl GetRepoRequest {
    pub fn new(owner: impl Into<String>, repo: impl Into<String>) -> Self {
        Self {
            owner: owner.into(),
            repo: repo.into(),
        }
    }
    fn into_parts(self) -> Result<RequestParts, String> {
        Ok(RequestParts {
            method: "GET",
            path: format!("/repos/{}/{}", enc(&self.owner), enc(&self.repo)),
            query: vec![],
            body: RequestBody::Empty,
            headers: vec![],
        })
    }
}

impl Endpoint for GetRepoRequest {
    type Success = Repo;
    type Failure = GhError;
    type Decode = Json<Repo>;
    const ENDPOINT_ID: &'static str = "GetRepo";
}

// ---------------------------------------------------------------------------
// 2. Text
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct GetReadmeRawRequest {
    pub owner: String,
    pub repo: String,
}

impl GetReadmeRawRequest {
    pub fn new(owner: impl Into<String>, repo: impl Into<String>) -> Self {
        Self {
            owner: owner.into(),
            repo: repo.into(),
        }
    }
    fn into_parts(self) -> Result<RequestParts, String> {
        Ok(RequestParts {
            method: "GET",
            path: format!("/repos/{}/{}/readme", enc(&self.owner), enc(&self.repo)),
            query: vec![],
            body: RequestBody::Empty,
            headers: vec![("accept".into(), "application/vnd.github.raw".into())],
        })
    }
}

impl Endpoint for GetReadmeRawRequest {
    type Success = String;
    type Failure = NoDeclaredError;
    type Decode = Text;
    const ENDPOINT_ID: &'static str = "GetReadmeRaw";
}

// ---------------------------------------------------------------------------
// 3. Bytes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct DownloadArchiveRequest {
    pub owner: String,
    pub repo: String,
}

impl DownloadArchiveRequest {
    fn into_parts(self) -> Result<RequestParts, String> {
        Ok(RequestParts {
            method: "GET",
            path: format!("/repos/{}/{}/tarball", enc(&self.owner), enc(&self.repo)),
            query: vec![],
            body: RequestBody::Empty,
            headers: vec![],
        })
    }
}

impl Endpoint for DownloadArchiveRequest {
    type Success = bytes::Bytes;
    type Failure = NoDeclaredError;
    type Decode = Binary;
    const ENDPOINT_ID: &'static str = "DownloadArchive";
}

// ---------------------------------------------------------------------------
// 4. Empty (204)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct DeleteBranchRequest {
    pub owner: String,
    pub repo: String,
    pub branch: String,
}

impl DeleteBranchRequest {
    fn into_parts(self) -> Result<RequestParts, String> {
        Ok(RequestParts {
            method: "DELETE",
            path: format!(
                "/repos/{}/{}/git/refs/heads/{}",
                enc(&self.owner),
                enc(&self.repo),
                enc(&self.branch)
            ),
            query: vec![],
            body: RequestBody::Empty,
            headers: vec![],
        })
    }
}

impl Endpoint for DeleteBranchRequest {
    type Success = ();
    type Failure = GhError;
    type Decode = Empty;
    const ENDPOINT_ID: &'static str = "DeleteBranch";
    /// Only 404 and 422 are declared; e.g. a 500 is `Undeclared`.
    const DECLARED_ERROR_STATUSES: &'static [(u16, u16)] = &[(404, 404), (422, 422)];
}

// ---------------------------------------------------------------------------
// 5. Multi-shape success: 200 JSON vs 202 empty ("still computing")
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum CommitActivitySuccess {
    Ready(Vec<WeekActivity>),
    Computing,
}

/// Generated once per multi-shape endpoint: dispatch on status, delegate to
/// the stock codecs.
impl<E: Send + 'static> Decode<E> for CommitActivitySuccess {
    type Output = Self;
    async fn decode(response: reqwest::Response, ctx: DecodeCtx<'_>) -> Result<Self, ApiError<E>> {
        match ctx.response.status {
            202 => <Empty as Decode<E>>::decode(response, ctx)
                .await
                .map(|()| Self::Computing),
            _ => <Json<Vec<WeekActivity>> as Decode<E>>::decode(response, ctx)
                .await
                .map(Self::Ready),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct GetCommitActivityRequest {
    pub owner: String,
    pub repo: String,
}

impl GetCommitActivityRequest {
    fn into_parts(self) -> Result<RequestParts, String> {
        Ok(RequestParts {
            method: "GET",
            path: format!("/repos/{}/{}/stats/commit_activity", enc(&self.owner), enc(&self.repo)),
            query: vec![],
            body: RequestBody::Empty,
            headers: vec![],
        })
    }
}

impl Endpoint for GetCommitActivityRequest {
    type Success = CommitActivitySuccess;
    type Failure = GhError;
    type Decode = CommitActivitySuccess;
    const ENDPOINT_ID: &'static str = "GetCommitActivity";
}

// ---------------------------------------------------------------------------
// 6. SSE stream
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize)]
pub struct StreamChatRequest {
    pub prompt: String,
}

impl StreamChatRequest {
    fn into_parts(self) -> Result<RequestParts, String> {
        Ok(RequestParts {
            method: "POST",
            path: "/v1/chat".into(),
            query: vec![],
            body: json_body(&self)?,
            headers: vec![],
        })
    }
}

impl Endpoint for StreamChatRequest {
    type Success = crate::runtime::ItemStream<ChatChunk>;
    type Failure = GhError;
    type Decode = Sse<ChatChunk>;
    const ENDPOINT_ID: &'static str = "StreamChat";
}

// ---------------------------------------------------------------------------
// 7. NDJSON stream
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize)]
pub struct PullModelRequest {
    pub model: String,
}

impl PullModelRequest {
    fn into_parts(self) -> Result<RequestParts, String> {
        Ok(RequestParts {
            method: "POST",
            path: "/api/pull".into(),
            query: vec![],
            body: json_body(&self)?,
            headers: vec![],
        })
    }
}

impl Endpoint for PullModelRequest {
    type Success = crate::runtime::ItemStream<PullProgress>;
    type Failure = NoDeclaredError;
    type Decode = Ndjson<PullProgress>;
    const ENDPOINT_ID: &'static str = "PullModel";
}

// ---------------------------------------------------------------------------
// 8. GraphQL: string operation, typed variables, typed data (D17)
// ---------------------------------------------------------------------------

/// Generated once per operation authored in the definition.
pub trait GraphQlOperation: Send + 'static {
    const ID: &'static str;
    /// Sent exactly as written; never parsed or validated.
    const DOCUMENT: &'static str;
    type Variables: Serialize + Send + 'static;
    type Data: serde::de::DeserializeOwned + Send + 'static;
}

pub struct GraphQlRequest<Op: GraphQlOperation> {
    pub variables: Op::Variables,
}

impl<Op: GraphQlOperation> GraphQlRequest<Op> {
    pub fn new(variables: Op::Variables) -> Self {
        Self { variables }
    }
}

/// One blanket impl covers every operation.
impl<Op: GraphQlOperation> Endpoint for GraphQlRequest<Op> {
    type Success = Op::Data;
    type Failure = GraphQlFailure<Op::Data>;
    type Decode = GraphQl<Op::Data>;
    const ENDPOINT_ID: &'static str = Op::ID;
    /// A non-2xx GraphQL body is decoded only if it carries `errors`;
    /// otherwise `MalformedError` with the raw body.
    const DECLARED_ERROR_STATUSES: &'static [(u16, u16)] = &[(400, 599)];
}

pub struct ErasedGraphQl {
    pub endpoint_id: &'static str,
    pub body: Result<Vec<u8>, String>,
}

impl<Op: GraphQlOperation> From<GraphQlRequest<Op>> for DemoRequest {
    fn from(r: GraphQlRequest<Op>) -> Self {
        #[derive(Serialize)]
        struct Wire<'a, V> {
            query: &'a str,
            variables: &'a V,
        }
        let body = serde_json::to_vec(&Wire {
            query: Op::DOCUMENT,
            variables: &r.variables,
        })
        .map_err(|e| e.to_string());
        DemoRequest::GraphQl(ErasedGraphQl {
            endpoint_id: Op::ID,
            body,
        })
    }
}

pub struct ViewerRepos;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ViewerReposVars {
    pub login: String,
    pub first: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ViewerReposData {
    pub user: Option<GqlUser>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GqlUser {
    pub login: String,
    pub repositories: GqlRepoConnection,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GqlRepoConnection {
    #[serde(rename = "totalCount")]
    pub total_count: u32,
}

impl GraphQlOperation for ViewerRepos {
    const ID: &'static str = "ViewerRepos";
    const DOCUMENT: &'static str =
        "query ViewerRepos($login: String!, $first: Int!) { user(login: $login) { login repositories(first: $first) { totalCount } } }";
    type Variables = ViewerReposVars;
    type Data = ViewerReposData;
}

// ---------------------------------------------------------------------------
// 9/10. Used by the sniff-style port (Link header, 404, Retry-After)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct ListPullsRequest {
    pub owner: String,
    pub repo: String,
    pub state: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

impl ListPullsRequest {
    pub fn new(owner: impl Into<String>, repo: impl Into<String>) -> Self {
        Self {
            owner: owner.into(),
            repo: repo.into(),
            ..Self::default()
        }
    }
    pub fn with_page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }
    fn into_parts(self) -> Result<RequestParts, String> {
        let mut query = vec![];
        if let Some(s) = self.state {
            query.push(("state".into(), s));
        }
        if let Some(p) = self.page {
            query.push(("page".into(), p.to_string()));
        }
        if let Some(p) = self.per_page {
            query.push(("per_page".into(), p.to_string()));
        }
        Ok(RequestParts {
            method: "GET",
            path: format!("/repos/{}/{}/pulls", enc(&self.owner), enc(&self.repo)),
            query,
            body: RequestBody::Empty,
            headers: vec![],
        })
    }
}

impl Endpoint for ListPullsRequest {
    type Success = Vec<Pull>;
    type Failure = GhError;
    type Decode = Json<Vec<Pull>>;
    const ENDPOINT_ID: &'static str = "ListPulls";
}

#[derive(Debug, Clone, Default)]
pub struct GetPullRequest {
    pub owner: String,
    pub repo: String,
    pub number: u64,
}

impl GetPullRequest {
    pub fn new(owner: impl Into<String>, repo: impl Into<String>, number: u64) -> Self {
        Self {
            owner: owner.into(),
            repo: repo.into(),
            number,
        }
    }
    fn into_parts(self) -> Result<RequestParts, String> {
        Ok(RequestParts {
            method: "GET",
            path: format!("/repos/{}/{}/pulls/{}", enc(&self.owner), enc(&self.repo), self.number),
            query: vec![],
            body: RequestBody::Empty,
            headers: vec![],
        })
    }
}

impl Endpoint for GetPullRequest {
    type Success = Pull;
    type Failure = GhError;
    type Decode = Json<Pull>;
    const ENDPOINT_ID: &'static str = "GetPull";
}

// ---------------------------------------------------------------------------
// The erasing request enum (as generated today)
// ---------------------------------------------------------------------------

pub enum DemoRequest {
    GetRepo(GetRepoRequest),
    GetReadmeRaw(GetReadmeRawRequest),
    DownloadArchive(DownloadArchiveRequest),
    DeleteBranch(DeleteBranchRequest),
    GetCommitActivity(GetCommitActivityRequest),
    StreamChat(StreamChatRequest),
    PullModel(PullModelRequest),
    ListPulls(ListPullsRequest),
    GetPull(GetPullRequest),
    GraphQl(ErasedGraphQl),
}

macro_rules! from_variant {
    ($($v:ident($t:ty)),* $(,)?) => {$(
        impl From<$t> for DemoRequest {
            fn from(r: $t) -> Self { DemoRequest::$v(r) }
        }
    )*};
}

from_variant!(
    GetRepo(GetRepoRequest),
    GetReadmeRaw(GetReadmeRawRequest),
    DownloadArchive(DownloadArchiveRequest),
    DeleteBranch(DeleteBranchRequest),
    GetCommitActivity(GetCommitActivityRequest),
    StreamChat(StreamChatRequest),
    PullModel(PullModelRequest),
    ListPulls(ListPullsRequest),
    GetPull(GetPullRequest),
);

impl DemoRequest {
    pub fn endpoint_id(&self) -> &'static str {
        match self {
            Self::GetRepo(_) => GetRepoRequest::ENDPOINT_ID,
            Self::GetReadmeRaw(_) => GetReadmeRawRequest::ENDPOINT_ID,
            Self::DownloadArchive(_) => DownloadArchiveRequest::ENDPOINT_ID,
            Self::DeleteBranch(_) => DeleteBranchRequest::ENDPOINT_ID,
            Self::GetCommitActivity(_) => GetCommitActivityRequest::ENDPOINT_ID,
            Self::StreamChat(_) => StreamChatRequest::ENDPOINT_ID,
            Self::PullModel(_) => PullModelRequest::ENDPOINT_ID,
            Self::ListPulls(_) => ListPullsRequest::ENDPOINT_ID,
            Self::GetPull(_) => GetPullRequest::ENDPOINT_ID,
            Self::GraphQl(g) => g.endpoint_id,
        }
    }

    pub fn into_parts(self) -> Result<RequestParts, String> {
        match self {
            Self::GetRepo(r) => r.into_parts(),
            Self::GetReadmeRaw(r) => r.into_parts(),
            Self::DownloadArchive(r) => r.into_parts(),
            Self::DeleteBranch(r) => r.into_parts(),
            Self::GetCommitActivity(r) => r.into_parts(),
            Self::StreamChat(r) => r.into_parts(),
            Self::PullModel(r) => r.into_parts(),
            Self::ListPulls(r) => r.into_parts(),
            Self::GetPull(r) => r.into_parts(),
            Self::GraphQl(g) => Ok(RequestParts {
                method: "POST",
                path: "/graphql".into(),
                query: vec![],
                body: RequestBody::Json(g.body?),
                headers: vec![],
            }),
        }
    }
}
