// An `EndpointSpec` must name its success type and codec; neither can be left
// for a caller to choose.
use schematic_runtime::{Api, EndpointSpec, InvalidArgument, NoDeclaredError, RequestParts};

struct Fixture;

impl Api for Fixture {
    const NAME: &'static str = "Fixture";
}

struct ListItems;

impl EndpointSpec for ListItems {
    type Api = Fixture;
    type Failure = NoDeclaredError;

    const ENDPOINT_ID: &'static str = "list_items";
    const METHOD: http::Method = http::Method::GET;

    fn into_parts(self) -> Result<RequestParts, InvalidArgument> {
        Ok(RequestParts::new(http::Method::GET, "/items"))
    }
}

fn main() {}
