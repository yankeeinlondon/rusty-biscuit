// A multi-shape codec supports a typed mutator only when every alternative
// is JSON (`JsonAlternatives`); an enum that mixes JSON with a stream does
// not compile with one (R12).
use std::marker::PhantomData;

use schematic_runtime::{
    Alternatives, Api, ApiDefinition, Client, Decode, DecodeFailure, EndpointSpec, InvalidArgument, ItemStream, Json,
    LiveBody, NoDeclaredError, RequestParts, Route, ShapeKind, Shapes, Sse,
};

struct Fixture;

impl Api for Fixture {
    const NAME: &'static str = "Fixture";
}

enum Mixed {
    Record(serde_json::Value),
    Events(ItemStream<serde_json::Value>),
}

impl Alternatives for Mixed {
    const ROUTES: &'static [Route] = &[
        Route::new(ShapeKind::Json).media("application/json"),
        Route::new(ShapeKind::Stream).media("text/event-stream"),
    ];

    async fn decode_alternative(index: usize, body: LiveBody) -> Result<Self, DecodeFailure> {
        match index {
            0 => Json::<serde_json::Value>::decode(body).await.map(Self::Record),
            _ => Sse::<serde_json::Value>::decode(body).await.map(Self::Events),
        }
    }
}

struct Get(PhantomData<Mixed>);

impl EndpointSpec for Get {
    type Api = Fixture;
    type Success = Mixed;
    type Failure = NoDeclaredError;
    type Decoder = Shapes<Mixed>;

    const ENDPOINT_ID: &'static str = "get";
    const METHOD: http::Method = http::Method::GET;

    fn into_parts(self) -> Result<RequestParts, InvalidArgument> {
        Ok(RequestParts::new(http::Method::GET, "/"))
    }
}

fn main() {
    let _ = Client::<Fixture>::new(ApiDefinition::new("https://x.invalid")).mutate_response::<Get, _>(|_, _value| Ok(()));
}
