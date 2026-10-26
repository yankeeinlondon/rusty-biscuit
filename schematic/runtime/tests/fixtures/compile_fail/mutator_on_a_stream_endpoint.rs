// The stream codecs do not support hooks, so registering a typed mutator
// for an SSE, NDJSON, or raw byte stream endpoint is a compile error rather
// than a hook that silently never runs (R12).
use std::marker::PhantomData;

use schematic_runtime::{
    Api, ApiDefinition, ByteStream, Client, Decode, EndpointSpec, InvalidArgument, Ndjson, NoDeclaredError, RequestParts, Sse,
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
    let _ = Client::<Fixture>::new(ApiDefinition::new("https://x.invalid"))
        .mutate_response::<Get<Sse<serde_json::Value>>, _>(|_, _items| Ok(()))
        .mutate_response::<Get<Ndjson<serde_json::Value>>, _>(|_, _items| Ok(()))
        .mutate_response::<Get<ByteStream>, _>(|_, _bytes| Ok(()));
}
