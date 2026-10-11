//! Q6: five short usage snippets, each compiled and run.

use serde_json::json;
use typed_send_spike::api::*;
use typed_send_spike::client::DemoClient;
use typed_send_spike::runtime::*;
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn server() -> MockServer {
    let s = MockServer::start().await;
    Mock::given(path("/repos/o/r"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("etag", "\"v1\"")
                .set_body_json(json!({"id": 1, "full_name": "o/r", "default_branch": "main"})),
        )
        .mount(&s)
        .await;
    Mock::given(path("/repos/o/gone"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"message": "Not Found"})))
        .mount(&s)
        .await;
    s
}

// 1. "Just give me the value": two `?`, caller error = CallError<E>.
async fn snippet_1(c: &DemoClient) -> Result<String, CallError<GhError>> {
    let repo = c.send(GetRepoRequest::new("o", "r")).await?.into_result()?;
    Ok(repo.default_branch)
}

// 1b. Same thing through the extension trait: one `?`.
async fn snippet_1b(c: &DemoClient) -> Result<String, CallError<GhError>> {
    let repo = c.send(GetRepoRequest::new("o", "r")).await.into_data()?;
    Ok(repo.default_branch)
}

// 2. Value AND headers on success (pagination / caching).
async fn snippet_2(c: &DemoClient) -> Result<(Repo, Option<String>), CallError<GhError>> {
    let resp = c.send(GetRepoRequest::new("o", "r")).await?;
    let etag = resp.header("etag").map(str::to_owned);
    Ok((resp.into_result()?, etag))
}

// 3. Branch on the outcome without consuming the envelope.
async fn snippet_3(c: &DemoClient) -> Result<&'static str, TransportError> {
    let resp = c.send(GetRepoRequest::new("o", "gone")).await?;
    Ok(match (resp.data(), resp.error()) {
        (Some(_), _) => "found",
        (None, Some(e)) if resp.status() == 404 => {
            assert_eq!(e.declared().unwrap().message, "Not Found");
            "missing"
        }
        _ => "other",
    })
}

// 4. Exhaustive match on the kind of failure.
async fn snippet_4(c: &DemoClient) -> Result<String, TransportError> {
    let resp = c.send(GetRepoRequest::new("o", "gone")).await?;
    let status = resp.status();
    Ok(match resp.outcome {
        Outcome::Success(repo) => repo.full_name,
        Outcome::Failure(ApiError { kind, body }) => match kind {
            ErrorKind::Declared(e) => format!("{status}: {}", e.message),
            ErrorKind::Undeclared | ErrorKind::MalformedError(_) => {
                format!("{status}: {}", String::from_utf8_lossy(&body))
            }
            other => format!("{status}: {other:?}"),
        },
    })
}

// 5. Into an application error (color-eyre/anyhow style): Box<dyn Error>.
async fn snippet_5(c: &DemoClient) -> Result<Repo, Box<dyn std::error::Error + Send + Sync>> {
    Ok(c.send(GetRepoRequest::new("o", "gone")).await?.into_result()?)
}

#[tokio::test]
async fn snippets_run() {
    let s = server().await;
    let c = DemoClient::builder(s.uri()).build();
    assert_eq!(snippet_1(&c).await.unwrap(), "main");
    assert_eq!(snippet_1b(&c).await.unwrap(), "main");
    assert_eq!(snippet_2(&c).await.unwrap().1.as_deref(), Some("\"v1\""));
    assert_eq!(snippet_3(&c).await.unwrap(), "missing");
    assert_eq!(snippet_4(&c).await.unwrap(), "404: Not Found");
    let e = snippet_5(&c).await.unwrap_err();
    assert!(e.to_string().starts_with("HTTP 404 from GET "), "{e}");
}
