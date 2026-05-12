use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::UserSummary;

/// A repository tag from `GET /repos/{owner}/{repo}/tags`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoTag {
    /// Tag name.
    pub name: String,

    /// Commit the tag points to.
    pub commit: TagCommit,

    /// URL to download tarball.
    pub tarball_url: String,

    /// URL to download zipball.
    pub zipball_url: String,

    /// Node ID.
    #[serde(default)]
    pub node_id: Option<String>,
}

/// Commit info embedded in a tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TagCommit {
    /// Commit SHA.
    pub sha: String,

    /// API URL for the commit.
    pub url: String,
}

/// A GitHub release from `GET /repos/{owner}/{repo}/releases`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Release {
    /// Release ID.
    pub id: u64,

    /// Tag name associated with the release.
    pub tag_name: String,

    /// Release name/title.
    #[serde(default)]
    pub name: Option<String>,

    /// Release body (markdown).
    #[serde(default)]
    pub body: Option<String>,

    /// Whether this is a draft.
    #[serde(default)]
    pub draft: bool,

    /// Whether this is a prerelease.
    #[serde(default)]
    pub prerelease: bool,

    /// Whether this release is immutable.
    #[serde(default)]
    pub immutable: Option<bool>,

    /// Creation timestamp.
    pub created_at: String,

    /// Publish timestamp.
    #[serde(default)]
    pub published_at: Option<String>,

    /// HTML URL to the release.
    pub html_url: String,

    /// API URL for the release.
    #[serde(default)]
    pub url: Option<String>,

    /// Author of the release.
    #[serde(default)]
    pub author: Option<UserSummary>,

    /// Release assets.
    #[serde(default)]
    pub assets: Vec<ReleaseAsset>,
}

/// An asset attached to a release.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ReleaseAsset {
    /// Asset ID.
    pub id: u64,

    /// Asset filename.
    pub name: String,

    /// Asset content type.
    #[serde(default)]
    pub content_type: Option<String>,

    /// Asset size in bytes.
    #[serde(default)]
    pub size: Option<u64>,

    /// Download URL.
    #[serde(default)]
    pub browser_download_url: Option<String>,

    /// Download count.
    #[serde(default)]
    pub download_count: Option<u64>,
}

/// A Git reference from `GET /repos/{owner}/{repo}/git/ref/tags/{tag}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GitRef {
    /// Full ref name (e.g., "refs/tags/v1.0.0").
    #[serde(rename = "ref")]
    pub ref_name: String,

    /// Node ID.
    #[serde(default)]
    pub node_id: Option<String>,

    /// API URL.
    pub url: String,

    /// Object the ref points to.
    pub object: GitRefObject,
}

/// Object pointed to by a Git reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GitRefObject {
    /// Object type: "commit" (lightweight tag) or "tag" (annotated tag).
    #[serde(rename = "type")]
    pub object_type: String,

    /// Object SHA.
    pub sha: String,

    /// API URL for the object.
    pub url: String,
}

impl GitRefObject {
    /// Returns `true` if this is an annotated tag.
    pub fn is_annotated_tag(&self) -> bool {
        self.object_type == "tag"
    }

    /// Returns `true` if this is a lightweight tag (points directly to commit).
    pub fn is_lightweight_tag(&self) -> bool {
        self.object_type == "commit"
    }
}

/// An annotated tag object from `GET /repos/{owner}/{repo}/git/tags/{sha}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AnnotatedTagObject {
    /// Tag object SHA.
    pub sha: String,

    /// Tag name.
    pub tag: String,

    /// Tag message.
    pub message: String,

    /// Tagger information.
    #[serde(default)]
    pub tagger: Option<AnnotatedTagger>,

    /// Object the tag points to.
    pub object: TagObjectRef,

    /// GPG signature verification.
    #[serde(default)]
    pub verification: Option<TagVerification>,

    /// Node ID.
    #[serde(default)]
    pub node_id: Option<String>,

    /// API URL.
    #[serde(default)]
    pub url: Option<String>,
}

/// Object reference from an annotated tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TagObjectRef {
    /// Object type (usually "commit").
    #[serde(rename = "type")]
    pub object_type: String,

    /// Object SHA.
    pub sha: String,

    /// API URL.
    pub url: String,
}

/// Tagger information for annotated tags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AnnotatedTagger {
    /// Tagger's name.
    pub name: String,

    /// Tagger's email.
    pub email: String,

    /// Tagging timestamp (ISO 8601).
    pub date: String,
}

/// GPG signature verification for tags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TagVerification {
    /// Whether the signature is verified.
    pub verified: bool,

    /// Verification reason/status.
    pub reason: String,

    /// Signature data.
    #[serde(default)]
    pub signature: Option<String>,

    /// Payload that was signed.
    #[serde(default)]
    pub payload: Option<String>,

    /// Verification timestamp.
    #[serde(default)]
    pub verified_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_deserialization() {
        let json = r#"{
            "id": 1,
            "tag_name": "v1.0.0",
            "name": "Version 1.0.0",
            "draft": false,
            "prerelease": false,
            "created_at": "2013-02-27T19:35:32Z",
            "html_url": "https://github.com/octocat/Hello-World/releases/v1.0.0",
            "assets": []
        }"#;

        let release: Release = serde_json::from_str(json).unwrap();
        assert_eq!(release.tag_name, "v1.0.0");
        assert!(!release.draft);
        assert!(!release.prerelease);
    }

    #[test]
    fn git_ref_lightweight_tag() {
        let json = r#"{
            "ref": "refs/tags/v1.0.0",
            "url": "https://api.github.com/repos/octocat/Hello-World/git/ref/tags/v1.0.0",
            "object": {
                "type": "commit",
                "sha": "abc123",
                "url": "https://api.github.com/repos/octocat/Hello-World/git/commits/abc123"
            }
        }"#;

        let git_ref: GitRef = serde_json::from_str(json).unwrap();
        assert!(git_ref.object.is_lightweight_tag());
        assert!(!git_ref.object.is_annotated_tag());
    }

    #[test]
    fn git_ref_annotated_tag() {
        let json = r#"{
            "ref": "refs/tags/v1.0.0",
            "url": "https://api.github.com/repos/octocat/Hello-World/git/ref/tags/v1.0.0",
            "object": {
                "type": "tag",
                "sha": "abc123",
                "url": "https://api.github.com/repos/octocat/Hello-World/git/tags/abc123"
            }
        }"#;

        let git_ref: GitRef = serde_json::from_str(json).unwrap();
        assert!(git_ref.object.is_annotated_tag());
        assert!(!git_ref.object.is_lightweight_tag());
    }

    #[test]
    fn annotated_tag_object_deserialization() {
        let json = r#"{
            "sha": "abc123",
            "tag": "v1.0.0",
            "message": "Release v1.0.0\n\nInitial release.",
            "tagger": {
                "name": "Octocat",
                "email": "octocat@github.com",
                "date": "2014-11-07T22:01:45Z"
            },
            "object": {
                "type": "commit",
                "sha": "def456",
                "url": "https://api.github.com/repos/octocat/Hello-World/git/commits/def456"
            }
        }"#;

        let tag: AnnotatedTagObject = serde_json::from_str(json).unwrap();
        assert_eq!(tag.tag, "v1.0.0");
        assert!(tag.message.contains("Initial release"));
        assert!(tag.tagger.is_some());
    }

    #[test]
    fn repo_tag_deserialization() {
        let json = r#"{
            "name": "v1.0.0",
            "commit": {
                "sha": "abc123",
                "url": "https://api.github.com/repos/octocat/Hello-World/commits/abc123"
            },
            "tarball_url": "https://api.github.com/repos/octocat/Hello-World/tarball/v1.0.0",
            "zipball_url": "https://api.github.com/repos/octocat/Hello-World/zipball/v1.0.0"
        }"#;

        let tag: RepoTag = serde_json::from_str(json).unwrap();
        assert_eq!(tag.name, "v1.0.0");
        assert_eq!(tag.commit.sha, "abc123");
    }
}
