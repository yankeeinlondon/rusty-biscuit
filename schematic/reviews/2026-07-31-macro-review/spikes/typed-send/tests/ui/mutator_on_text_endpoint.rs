use typed_send_spike::{api::*, client::DemoClient};

fn main() {
    DemoClient::builder("http://x")
        .mutate_response::<GetReadmeRawRequest>(|_ctx, s| { s.push('!'); Ok(()) });
}
