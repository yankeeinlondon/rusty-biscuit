// A generated API struct (given its methods by `api_client!`) accepts only its own
// API's requests through every forwarded surface: the typed send, the raw
// send, the link check, and hook registration. The request names the other
// API's tag, and the two APIs share an endpoint id.
use schematic_runtime::{Api, ApiDefinition, CallOptions, Client, EndpointSpec, InvalidArgument, Json, NoDeclaredError, RequestParts, Url};

pub struct GitHub(Client<GitHubApi>);
pub enum GitHubApi {}
impl Api for GitHubApi {
    const NAME: &'static str = "GitHub";
}
schematic_runtime::api_client!(GitHub, GitHubApi);

pub enum GiteaApi {}
impl Api for GiteaApi {
    const NAME: &'static str = "Gitea";
}

impl GitHub {
    pub fn new() -> Self {
        Self(Client::new(ApiDefinition::new("https://x.invalid")))
    }
}

struct GiteaListRepos;

impl EndpointSpec for GiteaListRepos {
    type Api = GiteaApi;
    type Success = Vec<String>;
    type Failure = NoDeclaredError;
    type Decoder = Json<Vec<String>>;

    const ENDPOINT_ID: &'static str = "list_repos";
    const METHOD: http::Method = http::Method::GET;

    fn into_parts(self) -> Result<RequestParts, InvalidArgument> {
        Ok(RequestParts::new(http::Method::GET, "/repos"))
    }
}

async fn misuse(client: GitHub, link: Url) {
    let _ = client.send(GiteaListRepos, &CallOptions::new()).await;
    let _ = client.send_raw(GiteaListRepos, &CallOptions::new()).await;
    let _ = client.check_link::<GiteaListRepos>(&link);
}

fn main() {
    let _ = misuse;
    let _ = GitHub::new().mutate_response::<GiteaListRepos, _>(|_, _repos| Ok(()));
}
