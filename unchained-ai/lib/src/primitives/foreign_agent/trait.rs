use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum ForeignAgentLocality {
    LocalCli,
    CloudApi,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum ConcurrencyCap {
    Cap(u32),
    Unlimited,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum UsageCap {
    Daily(u32),
    Weekly(u32),
    Monthly(u32),
    Other,
    None,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum AuthMethod {
    None,
    BearerToken,
    ApiKey(String),
    QueryParams(String),
    OAuth,
}

/// Describes the types of plans an Agentic
/// solution offers.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum AgenticPlanType {
    /// include URL for pricing
    Subscription(url),
    /// include URL for pricing
    PerUse(url),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ForeignAgentSuccess {
    frontmatter: Frontmatter,
    markdown: String,
    /// if the tool use of the agent platform
    /// was available then we should provide
    /// this back as metadata.
    tool_use: Option<Vec<String>>,

}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ForeignAgentFailure {
    /// a Foreign Agent may optionally provide
    /// a classification of the error which occurred
    kind: Option<String>,
    /// A Foreign Agent may optionally provide
    /// a unique identifier to help lookup the
    /// log entry on the Foreign Agent.
    log_id: Option<String>,
    /// The error message
    message: String
}

pub enum ForeignAgentPayload {
  Success(),
  Failure()
}

pub struct ForeignAgentResponse {
    /// The foreign agent's unique ID
    id: String,
    success: bool,
    payload: ForeignAgentPayload
}

/// A **ForeignAgent** is a trait that allows us to interact
/// with external agentic software in a consistent fashion.
///
/// This could include local CLI software like Claude Code
/// and Opencode, but it can also refer to Cloud based Agentic
/// software like the solution from [Firecrawl](https://www.firecrawl.dev/agent)
pub trait ForeignAgent {
    const id: String;

    /// if a caller has provided an API key then expose it in a consistent fashion
    const api_key: Option<String>;
    /// provide links to the foreign agent's pricing structure(s)
    const plans_offered: &'static Vec<AgenticPlanType>;

    /// The authentication method the ForeignAgent expects
    const auth_method: &'static AuthMethod;

    /// if a caller has successfully called `validate()`
    const authorized: Option<AgenticPlanType>;

    /// validate that the user is able to use this ForeignAgent
    /// with the credentials they've provided
    fn validate() -> bool;

    fn prompt<T: Into<String>>(prompt: T) ->
}
