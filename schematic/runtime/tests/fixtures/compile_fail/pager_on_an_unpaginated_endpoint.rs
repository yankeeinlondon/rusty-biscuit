// A request whose definition authors no pagination strategy has no pager.
use schematic_runtime::{Api, ApiDefinition, Client, EndpointSpec, InvalidArgument, Json, NoDeclaredError, RequestParts};

struct Example;

impl Api for Example {
    const NAME: &'static str = "Example";
}

struct GetItem;

impl EndpointSpec for GetItem {
    type Api = Example;
    type Success = String;
    type Failure = NoDeclaredError;
    type Decoder = Json<String>;

    const ENDPOINT_ID: &'static str = "get_item";
    const METHOD: http::Method = http::Method::GET;

    fn into_parts(self) -> Result<RequestParts, InvalidArgument> {
        Ok(RequestParts::new(http::Method::GET, "/item"))
    }
}

fn main() {
    let client = Client::<Example>::new(ApiDefinition::new("https://x.invalid"));
    let _ = client.send(GetItem).pager();
}
