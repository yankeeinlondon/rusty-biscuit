// A client accepts only its own API's requests, through every public surface:
// the typed send, the raw send, and hook registration. The two APIs share an
// endpoint id, so the id alone could not tell them apart.
use schematic_runtime::{Api, ApiDefinition, CallOptions, Client, EndpointSpec, InvalidArgument, Json, NoDeclaredError, RequestParts};

struct Github;

impl Api for Github {
    const NAME: &'static str = "Github";
}

struct Gitea;

impl Api for Gitea {
    const NAME: &'static str = "Gitea";
}

struct GiteaListRepos;

impl EndpointSpec for GiteaListRepos {
    type Api = Gitea;
    type Success = Vec<String>;
    type Failure = NoDeclaredError;
    type Decoder = Json<Vec<String>>;

    const ENDPOINT_ID: &'static str = "list_repos";
    const METHOD: http::Method = http::Method::GET;

    fn into_parts(self) -> Result<RequestParts, InvalidArgument> {
        Ok(RequestParts::new(http::Method::GET, "/repos"))
    }
}

async fn misuse(client: Client<Github>) {
    let _ = client.send(GiteaListRepos).await;
    let _ = client.send_raw(GiteaListRepos, &CallOptions::new()).await;
}

fn main() {
    let _ = misuse;
    let _ = Client::<Github>::new(ApiDefinition::new("https://x.invalid"))
        .mutate_response::<GiteaListRepos, _>(|_, _repos| Ok(()));
}
