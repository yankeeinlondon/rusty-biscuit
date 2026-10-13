// A typed mutator receives the endpoint's declared success type and nothing
// else (R29).
use std::marker::PhantomData;

use schematic_runtime::{Api, ApiDefinition, Client, EndpointSpec, InvalidArgument, Json, NoDeclaredError, RequestParts};

struct Fixture;

impl Api for Fixture {
    const NAME: &'static str = "Fixture";
}

struct ListIds(PhantomData<()>);

impl EndpointSpec for ListIds {
    type Api = Fixture;
    type Success = Vec<u64>;
    type Failure = NoDeclaredError;
    type Decoder = Json<Vec<u64>>;

    const ENDPOINT_ID: &'static str = "list_ids";
    const METHOD: http::Method = http::Method::GET;

    fn into_parts(self) -> Result<RequestParts, InvalidArgument> {
        Ok(RequestParts::new(http::Method::GET, "/ids"))
    }
}

fn main() {
    let _ = Client::<Fixture>::new(ApiDefinition::new("https://x.invalid"))
        .mutate_response::<ListIds, _>(|_, ids: &mut String| {
            ids.clear();
            Ok(())
        });
}
