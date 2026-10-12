use typed_send_spike::{api::*, client::DemoClient};

fn main() {
    DemoClient::builder("http://x")
        .mutate_response::<GetRepoRequest>(|_ctx, pulls: &mut Vec<Pull>| Ok(()));
}
