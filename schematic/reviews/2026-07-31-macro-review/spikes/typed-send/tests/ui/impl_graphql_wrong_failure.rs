// GraphQL's codec can only pair with GraphQlFailure<D>.
use typed_send_spike::{api::*, runtime::*};

struct Bad;
impl Endpoint for Bad {
    type Success = ViewerReposData;
    type Failure = GhError;
    type Decode = GraphQl<ViewerReposData>;
    const ENDPOINT_ID: &'static str = "Bad";
}

fn main() {}
