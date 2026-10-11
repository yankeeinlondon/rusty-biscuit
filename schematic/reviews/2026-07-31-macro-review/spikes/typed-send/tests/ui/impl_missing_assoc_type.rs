// Associated type defaults are unstable: every impl must name all three.
use typed_send_spike::{api::*, runtime::*};

struct Bad;
impl Endpoint for Bad {
    type Success = Repo;
    type Decode = Json<Repo>;
    const ENDPOINT_ID: &'static str = "Bad";
}

fn main() {}
