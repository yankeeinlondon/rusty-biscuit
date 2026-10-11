use typed_send_spike::{api::*, client::DemoClient};

async fn f(c: DemoClient) {
    let pulls = c.request::<Vec<Pull>>(GetRepoRequest::new("o", "r")).await;
}

fn main() {}
