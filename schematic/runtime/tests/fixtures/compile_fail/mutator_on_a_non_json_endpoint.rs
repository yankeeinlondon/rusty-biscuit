// A typed mutator runs only on a codec that supports hooks, so registering one
// for a text, binary, or empty endpoint is a compile error rather than a hook
// that silently never runs (R29).
use std::marker::PhantomData;

use schematic_runtime::{
    Api, ApiDefinition, Binary, Client, Decode, Empty, EndpointSpec, InvalidArgument, NoDeclaredError, RequestParts, Text,
};

struct Fixture;

impl Api for Fixture {
    const NAME: &'static str = "Fixture";
}

struct Get<D>(PhantomData<D>);

impl<D: Decode + Send> EndpointSpec for Get<D> {
    type Api = Fixture;
    type Success = D::Output;
    type Failure = NoDeclaredError;
    type Decoder = D;

    const ENDPOINT_ID: &'static str = "get";
    const METHOD: http::Method = http::Method::GET;

    fn into_parts(self) -> Result<RequestParts, InvalidArgument> {
        Ok(RequestParts::new(http::Method::GET, "/"))
    }
}

fn main() {
    let _ = Client::<Fixture>::builder(ApiDefinition::new("https://x.invalid"))
        .mutate_response::<Get<Text>, _>(|_, text| {
            text.push('!');
            Ok(())
        })
        .mutate_response::<Get<Binary>, _>(|_, _bytes| Ok(()))
        .mutate_response::<Get<Empty>, _>(|_, _unit| Ok(()));
}
