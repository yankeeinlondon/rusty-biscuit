// A generator bug: the codec decodes a different type than `Success`.
use typed_send_spike::{api::*, runtime::*};

struct Bad;
impl Endpoint for Bad {
    type Success = Repo;
    type Failure = GhError;
    type Decode = Json<Pull>;
    const ENDPOINT_ID: &'static str = "Bad";
}

fn main() {}
