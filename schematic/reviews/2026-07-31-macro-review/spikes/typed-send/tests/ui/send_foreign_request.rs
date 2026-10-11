// A request type that is not part of this API's enum cannot be sent.
use typed_send_spike::{api::*, client::DemoClient, runtime::*};

struct Foreign;
impl Endpoint for Foreign {
    type Success = Repo;
    type Failure = GhError;
    type Decode = Json<Repo>;
    const ENDPOINT_ID: &'static str = "Foreign";
}

async fn f(c: DemoClient) {
    let _ = c.send(Foreign).await;
}

fn main() {}
