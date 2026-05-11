use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A GitHub user summary (common across many responses).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UserSummary {
    /// The user's login handle.
    pub login: String,

    /// The user's unique ID.
    #[serde(default)]
    pub id: Option<u64>,

    /// URL to the user's avatar image.
    #[serde(default)]
    pub avatar_url: Option<String>,

    /// API URL for the user resource.
    #[serde(default)]
    pub url: Option<String>,

    /// HTML URL to the user's profile.
    #[serde(default)]
    pub html_url: Option<String>,

    /// User type (e.g., "User", "Organization", "Bot").
    #[serde(rename = "type", default)]
    pub user_type: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_summary_deserialization() {
        let json = r#"{
            "login": "octocat",
            "id": 1,
            "avatar_url": "https://github.com/images/error/octocat_happy.gif",
            "type": "User"
        }"#;

        let user: UserSummary = serde_json::from_str(json).unwrap();
        assert_eq!(user.login, "octocat");
        assert_eq!(user.id, Some(1));
        assert_eq!(user.user_type, Some("User".to_string()));
    }
}
