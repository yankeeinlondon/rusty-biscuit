//! GitHub, GitLab, Gitea, and Bitbucket.
//!
//! Each forge has one API default. Root-array lists inherit it; wrapper lists
//! and the few lists that differ have a row in the API's exceptions table.

use schematic_define::pagination::IndexBase::OneBased;
use schematic_define::pagination::Knob;
use schematic_define::pagination::ResolutionOrigin::{ApiDefault, Exception};

use crate::{Api, Next, Prev, Row, Total, Unpaged, page_size, start_page};

const GITHUB: &str = "page_number: page page from 1; more Link rel=next";
const GITHUB_WITH_TOTAL: &str =
    "page_number: page page from 1; more Link rel=next; total body total_count";
const GITLAB: &str =
    "page_number: page page from 1; more present header X-Next-Page; total header X-Total";
const GITEA: &str =
    "page_number: page page from 1; more Link rel=next; total header X-Total-Count";
const BITBUCKET: &str = "next_link: next body next; prev body previous; total body size";

pub fn apis() -> Vec<Api> {
    vec![github(), gitlab(), gitea(), bitbucket()]
}

fn github_knobs() -> Vec<Knob> {
    vec![
        start_page(OneBased),
        page_size("per_page", 1, Some(100), Some(30)),
        Next,
    ]
}

fn github_root(endpoint: &'static str, item_type: &'static str) -> Row {
    Row::paged(
        endpoint,
        ApiDefault,
        "[*]",
        item_type,
        GITHUB,
        github_knobs(),
        &["page", "per_page"],
    )
}

fn github() -> Api {
    let mut workflow_knobs = github_knobs();
    workflow_knobs.push(Total);
    Api {
        name: "GitHub",
        rows: vec![
            github_root("ListPullRequests", "PullRequestSummary"),
            github_root("ListPullRequestFiles", "PullRequestFile"),
            github_root("ListIssues", "IssueSummary"),
            github_root("ListIssueComments", "IssueComment"),
            github_root("ListIssueTimeline", "TimelineEvent"),
            github_root("ListTags", "RepoTag"),
            github_root("ListReleases", "Release"),
            github_root("ListOrgRepos", "RepositoryInfo"),
            Row::paged(
                "ListWorkflowRuns",
                Exception,
                "workflow_runs[*]",
                "WorkflowRun",
                GITHUB_WITH_TOTAL,
                workflow_knobs,
                &["page", "per_page"],
            ),
        ],
    }
}

/// A GitLab list with the default; the keyset alternate adds no knob.
fn gitlab_root(endpoint: &'static str, item_type: &'static str) -> Row {
    Row::paged(
        endpoint,
        ApiDefault,
        "[*]",
        item_type,
        GITLAB,
        vec![
            start_page(OneBased),
            page_size("per_page", 1, Some(100), Some(20)),
            Next,
            Total,
        ],
        &["page", "per_page"],
    )
}

fn gitlab() -> Api {
    Api {
        name: "GitLab",
        rows: vec![
            gitlab_root("ListRepositoryTree", "TreeItem"),
            gitlab_root("ListGroupProjects", "Project"),
            gitlab_root("ListMergeRequests", "MergeRequest"),
            gitlab_root("ListMergeRequestCommits", "Commit"),
            gitlab_root("ListIssues", "Issue"),
            gitlab_root("ListIssueNotes", "Note"),
            gitlab_root("ListTags", "Tag"),
            gitlab_root("ListReleases", "Release"),
            gitlab_root("ListProjectPipelines", "Pipeline"),
            Row::unpaged(
                "ListIssueParticipants",
                Exception,
                "[*]",
                Unpaged::Unverified {
                    recalled: "the API default: PageNumber{page, per_page}, \
                               more present header X-Next-Page, total header X-Total",
                    source_needed: "GitLab REST API reference for \
                                    GET /projects/:id/issues/:issue_iid/participants",
                },
            ),
        ],
    }
}

fn gitea_root(endpoint: &'static str, item_type: &'static str) -> Row {
    Row::paged(
        endpoint,
        ApiDefault,
        "[*]",
        item_type,
        GITEA,
        vec![
            start_page(OneBased),
            page_size("limit", 1, Some(50), Some(30)),
            Next,
            Total,
        ],
        &["page", "limit"],
    )
}

fn gitea_tree(endpoint: &'static str) -> Row {
    Row::unpaged(
        endpoint,
        Exception,
        "tree[*]",
        Unpaged::Unverified {
            recalled: "PageNumber{page, per_page, Flag(truncated)}, total body total_count",
            source_needed: "Gitea API reference (swagger) for \
                            GET /repos/{owner}/{repo}/git/trees/{sha}",
        },
    )
}

fn gitea() -> Api {
    Api {
        name: "Gitea",
        rows: vec![
            gitea_root("ListPullRequests", "PullRequestSummary"),
            gitea_root("ListPullRequestFiles", "PullRequestFile"),
            gitea_root("ListIssues", "IssueSummary"),
            gitea_root("ListIssueComments", "IssueComment"),
            gitea_root("ListIssueTimeline", "TimelineEvent"),
            gitea_root("ListTags", "RepoTag"),
            gitea_root("ListReleases", "Release"),
            gitea_root("ListOrgRepos", "RepositoryInfo"),
            gitea_tree("GetGitTree"),
            Row::unpaged(
                "GetTagReference",
                Exception,
                "[*]",
                Unpaged::NotPaged("a ref-prefix lookup returning every matching ref"),
            ),
        ],
    }
}

/// A `PaginatedResponse<T>` list; `page` is not adopted because a next link
/// has no page input.
fn bitbucket_values(endpoint: &'static str, item_type: &'static str) -> Row {
    Row::paged(
        endpoint,
        ApiDefault,
        "values[*]",
        item_type,
        BITBUCKET,
        vec![page_size("pagelen", 1, Some(100), Some(10)), Next, Prev, Total],
        &["pagelen"],
    )
}

fn bitbucket() -> Api {
    Api {
        name: "Bitbucket",
        rows: vec![
            bitbucket_values("ListPullRequests", "PullRequest"),
            bitbucket_values("ListPullRequestComments", "PullRequestComment"),
            bitbucket_values("ListIssues", "Issue"),
            bitbucket_values("ListIssueComments", "IssueComment"),
            bitbucket_values("ListIssueChanges", "IssueChange"),
            bitbucket_values("ListTags", "Tag"),
            bitbucket_values("ListDownloads", "Download"),
            bitbucket_values("ListWorkspaceRepos", "Repository"),
        ],
    }
}
