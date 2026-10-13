// `.full()` comes last: a `FullCall` keeps the per-call settings, but the
// request can no longer be changed and `.full()` cannot be applied twice.
use schematic_runtime::{Api, ApiDefinition, Client, EndpointSpec, InvalidArgument, Json, NoDeclaredError, RequestParts};

struct Example;

impl Api for Example {
    const NAME: &'static str = "Example";
}

#[derive(Clone)]
struct ListItems;

impl EndpointSpec for ListItems {
    type Api = Example;
    type Success = Vec<String>;
    type Failure = NoDeclaredError;
    type Decoder = Json<Vec<String>>;

    const ENDPOINT_ID: &'static str = "list_items";
    const METHOD: http::Method = http::Method::GET;

    fn into_parts(self) -> Result<RequestParts, InvalidArgument> {
        Ok(RequestParts::new(http::Method::GET, "/items"))
    }
}

fn main() {
    let client = Client::<Example>::new(ApiDefinition::new("https://x.invalid"));
    let full = client.send(ListItems).anonymous().full().anonymous();
    let _ = full.clone().map_request(|request| request);
    let _ = full.full();
}
