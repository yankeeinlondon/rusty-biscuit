use typed_send_spike::{api::*, client::DemoClient};

async fn f(c: DemoClient) {
    let pulls: Vec<Pull> = c.send(GetRepoRequest::new("o", "r")).await.unwrap().into_result().unwrap();
}

fn main() {}
