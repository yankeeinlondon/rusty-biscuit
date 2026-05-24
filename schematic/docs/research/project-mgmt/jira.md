# Research into Jira's API Support

## Overview on Product

Jira is Atlassian's flagship project management and issue tracking platform, widely used for software development, IT service management, and business project management. Originally launched in 2002, it has grown into one of the most popular project management tools in the enterprise space.

### Functional Footprint

- **Issue Tracking**: Issues (bugs, tasks, stories, epics, subtasks) with customizable fields, workflows, and issue types
- **Project Management**: Scrum boards, Kanban boards, backlogs, sprint planning, and roadmap views
- **Workflow Engine**: Highly configurable state-machine workflows with conditions, validators, post-functions, and triggers
- **JQL (Jira Query Language)**: A powerful SQL-like query language for searching and filtering issues
- **Dashboards and Reporting**: Customizable dashboards with gadgets, burndown charts, velocity charts, and advanced reporting
- **Jira Service Management**: ITSM/ESM capabilities including service desks, SLAs, queues, and customer portals
- **Jira Software**: Agile development features including Scrum, Kanban, DevOps integration, and code review
- **Automation**: Rule-based automation engine for triggering actions based on events
- **Custom Fields**: Extensive custom field system (text, select, multi-select, date, user, etc.) with per-context configuration
- **Permissions and Security**: Granular permission schemes, issue-level security, and project roles
- **Apps and Integrations**: Extensive marketplace with 5000+ apps; native integrations with Confluence, Bitbucket, GitHub, Slack, and more
- **REST API**: Comprehensive REST API covering nearly all platform functionality
- **Webhooks**: Event-driven notifications for issue changes, project events, and more
- **Atlassian Document Format (ADF)**: Rich text storage format used for descriptions, comments, and multi-line custom fields

### Key URLs

| Resource                               | URL                                                                           |
|----------------------------------------|-------------------------------------------------------------------------------|
| Jira Cloud Platform REST API v3        | https://developer.atlassian.com/cloud/jira/platform/rest/v3/intro/            |
| Jira Cloud Platform REST API v2        | https://developer.atlassian.com/cloud/jira/platform/rest/v2/                  |
| Jira Software Cloud REST API           | https://developer.atlassian.com/cloud/jira/software/rest/intro/               |
| Jira Service Management Cloud REST API | https://developer.atlassian.com/cloud/jira/service-desk/rest/intro/           |
| OpenAPI Spec (v3)                      | https://dac-static.atlassian.com/cloud/jira/platform/swagger-v3.v3.json       |
| Postman Collection                     | https://developer.atlassian.com/cloud/jira/platform/jiracloud.3.postman.json  |
| Developer Documentation                | https://developer.atlassian.com/cloud/jira/platform/                          |
| Atlassian Document Format              | https://developer.atlassian.com/cloud/jira/platform/apis/document/structure/  |
| OAuth 2.0 (3LO) Guide                  | https://developer.atlassian.com/cloud/jira/platform/oauth-2-3lo-apps/         |
| Basic Auth Guide                       | https://developer.atlassian.com/cloud/jira/platform/basic-auth-for-rest-apis/ |
| Webhooks Guide                         | https://developer.atlassian.com/cloud/jira/platform/webhooks/                 |
| API Changelog                          | https://developer.atlassian.com/cloud/jira/platform/changelog/                |
| Developer Console                      | https://developer.atlassian.com/console/myapps/                               |
| Developer Support                      | https://developer.atlassian.com/support                                       |
| Jira Product Page                      | https://www.atlassian.com/software/jira                                       |
| Jira Pricing                           | https://www.atlassian.com/software/jira/pricing                               |
| Community Forum                        | https://community.developer.atlassian.com/                                    |
| Jira Entity Properties                 | https://developer.atlassian.com/cloud/jira/platform/jira-entity-properties/   |
| Jira Expressions                       | https://developer.atlassian.com/cloud/jira/platform/jira-expressions/         |

### Pricing Structure

Jira Cloud pricing is per-user, per-month with monthly and annual billing. Annual billing provides significant discounts.

| Plan           | Price (per user/month)   | Key Features                                                                              |
|----------------|--------------------------|-------------------------------------------------------------------------------------------|
| **Free**       | $0 (up to 10 users)      | 100K issues, basic automation, community support                                          |
| **Standard**   | ~$8.15/user/mo (annual)  | Unlimited issues, audit log, standard support, 250 automation executions/mo               |
| **Premium**    | ~$16.00/user/mo (annual) | Unlimited automation, advanced roadmaps, sandbox, 99.9% SLA, issue archival, bulk actions |
| **Enterprise** | Custom quote             | Centralized security, multi-site admin, 99.95% SLA, dedicated support, advanced analytics |

Jira Data Center (self-hosted) is also available with perpetual or annual licensing for organizations requiring on-premises deployment. API access is available on all tiers including Free. The Free tier has rate limits but no explicit API call caps. Premium and Enterprise unlock additional API capabilities like bulk issue archival.

## API Details

### 1. REST API v3 (Primary - Cloud)

The primary and most comprehensive API for Jira Cloud. Version 3 is the latest and adds Atlassian Document Format (ADF) support for rich text fields.

- **Base URL**: `https://{site}.atlassian.net/rest/api/3/`
- **OAuth 2.0 (3LO) URL**: `https://api.atlassian.com/ex/jira/{cloudId}/rest/api/3/`
- **Protocol**: HTTPS, JSON request/response
- **API Groups**: 80+ resource groups covering issues, projects, users, workflows, fields, dashboards, permissions, and more
- **Operations**: 500+ individual REST endpoints

#### Key Resource Groups

| Resource Group    | Description                                                    |
|-------------------|----------------------------------------------------------------|
| Issues            | CRUD, bulk operations, assign, transition, archive, changelogs |
| Issue Comments    | CRUD with ADF support                                          |
| Issue Attachments | Upload, download, manage attachments                           |
| Issue Links       | Create/delete issue relationships                              |
| Issue Search      | JQL-based search, paginated results                            |
| Projects          | CRUD, archive, restore, hierarchy                              |
| Users             | CRUD, bulk operations, email lookup                            |
| Workflows         | CRUD, search, capabilities, history                            |
| Workflow Schemes  | Manage workflow-to-issue-type mappings                         |
| Status            | CRUD, bulk operations, search                                  |
| Custom Fields     | Contexts, options, values                                      |
| Dashboards        | CRUD, sharing                                                  |
| Permissions       | Schemes, checks                                                |
| Groups            | CRUD, member management                                        |
| Webhooks          | Register, manage, refresh                                      |
| Time Tracking     | Configuration and worklogs                                     |
| JQL               | Validation, autocomplete, search                               |
| Audit Records     | Read audit logs                                                |

#### Formal Schema

- **OpenAPI 3.0 specification**: Available at `https://dac-static.atlassian.com/cloud/jira/platform/swagger-v3.v3.json` - this is the official machine-readable schema for the entire REST API v3
- **Postman Collection**: Available at `https://developer.atlassian.com/cloud/jira/platform/jiracloud.3.postman.json` - a complete Postman collection for all endpoints
- **Atlassian Document Format (ADF)**: Has a formal JSON schema at `http://go.atlassian.com/adf-json-schema` for rich text content used in issue descriptions, comments, and multi-line custom fields

#### SDKs

Atlassian does not provide official first-party SDKs for Jira Cloud. However, community and Atlassian-maintained libraries exist:

| Language               | Package                           | Notes                                                     |
|------------------------|-----------------------------------|-----------------------------------------------------------|
| **Node.js/TypeScript** | Various community packages        | No official SDK; `jira.js` is popular community option    |
| **Python**             | `jira` (PyPI)                     | Most widely used Python client, community-maintained      |
| **Java**               | `atlassian-jira-rest-java-client` | Atlassian-maintained but primarily for Server/Data Center |
| **Go**                 | Various community packages        | No official SDK                                           |
| **PHP**                | `lesstif/php-jira-rest-client`    | Community-maintained                                      |
| **Ruby**               | `jira-ruby`                       | Community-maintained                                      |
| **Forge (JS/TS)**      | Built-in `requestJira()`          | Official for Forge apps running inside Atlassian platform |
| **Connect (JS/TS)**    | Built-in JWT libraries            | Official for Connect apps                                 |

The Forge platform provides `@forge/bridge` with `requestJira()` for apps running inside Atlassian's hosted environment, which is the recommended approach for new app development.

#### Authentication Mechanisms

| Method              | Use Case                       | Description                                                                                                                                                                                                       |
|---------------------|--------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **OAuth 2.0 (3LO)** | External integrations          | Authorization code grant flow via `auth.atlassian.com`. Supports refresh tokens (rotating, 90-day inactivity expiry). Scopes are granular (classic and granular). Requires app registration in developer console. |
| **Basic Auth**      | Personal scripts, ad-hoc calls | Email + API token (not password) sent as `Authorization: Basic {base64}` header. API tokens created at `https://id.atlassian.com/manage-profile/security/api-tokens`.                                             |
| **Connect JWT**     | Connect apps                   | Built-in JWT-based authentication for Atlassian Connect framework apps. Auto-handled by Connect libraries.                                                                                                        |
| **Forge Scopes**    | Forge apps                     | Scope-based authentication built into the Forge platform. No manual token management needed.                                                                                                                      |

#### Signup Process for API Access

1. **Create an Atlassian account** at `https://id.atlassian.com`
2. **Create a Jira Cloud site** (free tier available for up to 10 users)
3. **For OAuth 2.0 apps**: Register in the [developer console](https://developer.atlassian.com/console/myapps/), configure callback URL, add API permissions, obtain client ID and secret
4. **For Basic Auth**: Generate an API token at `https://id.atlassian.com/manage-profile/security/api-tokens`
5. **For Forge apps**: Install the Forge CLI (`npm install -g @forge/cli`), create an app, deploy to Atlassian infrastructure
6. **For Connect apps**: Create an app descriptor, register on the Atlassian Marketplace (note: Connect is being sunset in favor of Forge)

### 2. REST API v2 (Cloud - Legacy)

A legacy version of the REST API that uses plain text strings instead of ADF for rich text fields.

- **Base URL**: `https://{site}.atlassian.net/rest/api/2/`
- **Same operation set** as v3 but without ADF support
- Still functional but v3 is recommended for new integrations

### 3. Jira Software Cloud REST API

Additional endpoints specific to Jira Software (agile/development features).

- **Base URL**: `https://{site}.atlassian.net/rest/agile/1.0/`
- **Resources**: Boards, sprints, backlog, epic management, development information
- **Authentication**: Same mechanisms as the platform REST API

### 4. Jira Service Management REST API

Endpoints specific to Jira Service Management (ITSM).

- **Base URL**: `https://{site}.atlassian.net/rest/servicedeskapi/`
- **Resources**: Service desks, request types, customer requests, queues, SLAs, organizations, knowledge base
- **Authentication**: Same mechanisms as the platform REST API

### 5. Webhooks (Event-Driven)

Jira supports webhooks as an event-driven notification mechanism. This is NOT WebSocket-based but rather traditional HTTP callback webhooks.

- **Registration**: Via Jira Administration UI, REST API (`POST /rest/api/3/webhook`), or Connect app descriptor
- **Protocol**: HTTPS POST with JSON payload to a user-defined URL
- **Supported Events**: 40+ event types including:

    - Issue: created, updated, deleted
    - Comment: created, updated, deleted
    - Attachment: created, deleted
    - Project: created, updated, deleted, archived, restored
    - Sprint: created, started, closed, deleted
    - Board: created, updated, deleted
    - User: created, updated, deleted
    - Version: created, released, unreleased, deleted
    - Worklog: created, updated, deleted
    - Issue link: created, deleted
    - Issue property: set, deleted

- **Filtering**: JQL-based filtering for issue events; field-level and property-level filtering
- **Retry Policy**: Up to 5 retries with 5-15 minute randomized back-off
- **Concurrency Limits**: 20 concurrent requests per tenant + webhook URL host (Primary), 10 for Secondary
- **Expiration**: REST-registered webhooks expire after 30 days; must be refreshed via `PUT /rest/api/3/webhook/refresh`
- **Security**: HMAC-SHA256 signature via `X-Hub-Signature` header for admin webhooks; JWT for Connect app webhooks; Bearer token for OAuth 2.0 app webhooks
- **Payload Size Limit**: 25MB maximum

### 6. Bulk Operations API

Jira provides specialized endpoints for bulk operations on issues.

- **Bulk Create**: `POST /rest/api/3/issue/bulk` - up to 50 issues per request
- **Bulk Fetch**: `POST /rest/api/3/issue/bulkfetch` - up to 100 issues per request
- **Bulk Changelog**: `POST /rest/api/3/changelog/bulkfetch` - up to 1000 issues
- **Bulk Archive**: `PUT /rest/api/3/issue/archive` - up to 1000 issues; JQL-based archive supports up to 100,000 issues (async)

### 7. No WebSocket or JSON-RPC Support

Jira does NOT provide WebSocket, JSON-RPC, gRPC, or GraphQL APIs. All programmatic access is via REST API endpoints and webhook callbacks.

## Schemas

### Task / Issue (Jira "Issue")

Jira's core entity is the "Issue" - a highly configurable work item that can represent bugs, tasks, stories, epics, subtasks, or any custom issue type. The schema is dynamic: fields vary per project and issue type configuration. The following represents the canonical structure as returned by the API.

**Source**: Jira Cloud REST API v3 - `GET /rest/api/3/issue/{issueIdOrKey}` response schema and OpenAPI spec. **Confidence: High** - directly from the official OpenAPI specification and API documentation.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraIssue {
    pub expand: Option<String>,
    pub id: String,
    pub self: String,
    pub key: String,
    pub fields: JiraIssueFields,
    pub rendered_fields: Option<serde_json::Value>,
    pub names: Option<serde_json::Map<String, serde_json::Value>>,
    pub changelog: Option<Changelog>,
    pub operations: Option<serde_json::Value>,
    pub properties: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraIssueFields {
    pub summary: String,
    pub description: Option<AdfDocument>,
    pub issuetype: IssueType,
    pub project: ProjectRef,
    pub status: StatusRef,
    pub priority: Option<Priority>,
    pub assignee: Option<UserRef>,
    pub reporter: Option<UserRef>,
    pub created: String,
    pub updated: Option<String>,
    pub duedate: Option<String>,
    pub resolution: Option<Resolution>,
    pub resolutiondate: Option<String>,
    pub labels: Option<Vec<String>>,
    pub components: Option<Vec<ComponentRef>>,
    pub versions: Option<Vec<VersionRef>>,
    pub fix_versions: Option<Vec<VersionRef>>,
    pub timetracking: Option<TimeTracking>,
    pub issuelinks: Option<Vec<IssueLink>>,
    pub subtasks: Option<Vec<JiraSubtask>>,
    pub attachment: Option<Vec<Attachment>>,
    pub comment: Option<CommentPage>,
    pub worklog: Option<WorklogPage>,
    pub environment: Option<AdfDocument>,
    pub parent: Option<ParentRef>,
    pub security: Option<SecurityLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdfDocument {
    #[serde(rename = "type")]
    pub doc_type: String,
    pub version: u32,
    pub content: Vec<AdfNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdfNode {
    #[serde(rename = "type")]
    pub node_type: String,
    pub content: Option<Vec<AdfNode>>,
    pub text: Option<String>,
    pub marks: Option<Vec<AdfMark>>,
    pub attrs: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdfMark {
    #[serde(rename = "type")]
    pub mark_type: String,
    pub attrs: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueType {
    pub id: String,
    pub name: String,
    pub subtask: bool,
    pub description: Option<String>,
    pub avatar_id: Option<u64>,
    pub hierarchy_level: Option<i32>,
    pub icon_url: Option<String>,
    pub scope: Option<Scope>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Priority {
    pub id: String,
    pub name: String,
    pub self_url: Option<String>,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusRef {
    pub id: Option<String>,
    pub name: String,
    pub self_url: Option<String>,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub status_category: Option<StatusCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusCategory {
    pub id: Option<u64>,
    pub key: String,
    pub color_name: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeTracking {
    pub original_estimate: Option<String>,
    pub remaining_estimate: Option<String>,
    pub time_spent: Option<String>,
    pub original_estimate_seconds: Option<u64>,
    pub remaining_estimate_seconds: Option<u64>,
    pub time_spent_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Changelog {
    pub start_at: Option<u64>,
    pub max_results: Option<u64>,
    pub total: Option<u64>,
    pub histories: Option<Vec<ChangeHistory>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeHistory {
    pub id: String,
    pub author: Option<UserRef>,
    pub created: String,
    pub items: Vec<ChangeItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeItem {
    pub field: String,
    pub fieldtype: String,
    pub from: Option<String>,
    pub from_string: Option<String>,
    pub to: Option<String>,
    pub to_string: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueLink {
    pub id: Option<String>,
    pub link_type: Option<IssueLinkType>,
    pub inward_issue: Option<IssueRef>,
    pub outward_issue: Option<IssueRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueLinkType {
    pub id: String,
    pub name: String,
    pub inward: String,
    pub outward: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueRef {
    pub id: String,
    pub key: String,
    pub self_url: Option<String>,
    pub fields: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParentRef {
    pub id: String,
    pub key: String,
    pub self_url: Option<String>,
    pub fields: Option<ParentFields>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParentFields {
    pub summary: Option<String>,
    pub status: Option<StatusRef>,
    pub priority: Option<Priority>,
    pub issuetype: Option<IssueType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraSubtask {
    pub id: String,
    pub key: String,
    pub self_url: Option<String>,
    pub fields: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub self_url: String,
    pub filename: String,
    pub content: String,
    pub size: Option<u64>,
    pub mime_type: Option<String>,
    pub author: Option<UserRef>,
    pub created: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentPage {
    pub start_at: Option<u64>,
    pub max_results: Option<u64>,
    pub total: Option<u64>,
    pub comments: Option<Vec<Comment>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub self_url: String,
    pub author: Option<UserRef>,
    pub body: Option<AdfDocument>,
    pub created: String,
    pub updated: Option<String>,
    pub update_author: Option<UserRef>,
    pub jsd_public: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorklogPage {
    pub start_at: Option<u64>,
    pub max_results: Option<u64>,
    pub total: Option<u64>,
    pub worklogs: Option<Vec<Worklog>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worklog {
    pub id: String,
    pub self_url: String,
    pub author: Option<UserRef>,
    pub time_spent: Option<String>,
    pub time_spent_seconds: Option<u64>,
    pub started: Option<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
    pub comment: Option<AdfDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityLevel {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}
```

### Person / Contact (Jira "User")

Jira uses an Atlassian Account-based identity system. Users are identified by `accountId` (UUID-like string). User data is subject to privacy controls - email addresses and other PII may be hidden depending on user profile visibility settings.

**Source**: `GET /rest/api/3/user` response schema from OpenAPI spec and API documentation. **Confidence: High** - directly from official documentation with response examples.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraUser {
    pub account_id: String,
    pub account_type: Option<String>,
    pub active: Option<bool>,
    pub display_name: String,
    pub email_address: Option<String>,
    pub self_url: String,
    pub avatar_urls: Option<AvatarUrls>,
    pub time_zone: Option<String>,
    pub locale: Option<String>,
    pub groups: Option<UserGroupPage>,
    pub application_roles: Option<ApplicationRolePage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRef {
    pub account_id: String,
    pub account_type: Option<String>,
    pub active: Option<bool>,
    pub display_name: String,
    pub self_url: Option<String>,
    pub avatar_urls: Option<AvatarUrls>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarUrls {
    #[serde(rename = "16x16")]
    pub size_16: Option<String>,
    #[serde(rename = "24x24")]
    pub size_24: Option<String>,
    #[serde(rename = "32x32")]
    pub size_32: Option<String>,
    #[serde(rename = "48x48")]
    pub size_48: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserGroupPage {
    pub size: Option<u64>,
    pub items: Option<Vec<UserGroup>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserGroup {
    pub group_id: String,
    pub name: String,
    pub self_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationRolePage {
    pub size: Option<u64>,
    pub items: Option<Vec<ApplicationRole>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationRole {
    pub key: Option<String>,
    pub groups: Option<Vec<String>>,
    pub default_groups: Option<Vec<String>>,
    pub name: Option<String>,
    pub selected_by_default: Option<bool>,
}
```

### Company / Organization (Jira "Project")

Jira does not have a dedicated "Organization" or "Company" entity in the standard platform. The closest equivalent is the "Project", which serves as a container for issues and can represent a team, product, or organizational unit. Jira Service Management has a separate "Organization" concept for customer-facing service desks.

**Source**: `GET /rest/api/3/project/{projectIdOrKey}` response schema from OpenAPI spec and API documentation. **Confidence: High** - directly from official documentation.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraProject {
    pub id: String,
    pub key: String,
    pub name: String,
    pub self_url: String,
    pub description: Option<String>,
    pub lead: Option<UserRef>,
    pub assignee_type: Option<String>,
    pub avatar_urls: Option<AvatarUrls>,
    pub simplified: Option<bool>,
    pub style: Option<String>,
    pub url: Option<String>,
    pub email: Option<String>,
    pub project_category: Option<ProjectCategory>,
    pub project_type_key: Option<String>,
    pub components: Option<Vec<Component>>,
    pub issue_types: Option<Vec<IssueType>>,
    pub versions: Option<Vec<VersionRef>>,
    pub roles: Option<serde_json::Map<String, String>>,
    pub insight: Option<ProjectInsight>,
    pub properties: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRef {
    pub id: String,
    pub key: String,
    pub name: Option<String>,
    pub self_url: Option<String>,
    pub project_type_key: Option<String>,
    pub avatar_urls: Option<AvatarUrls>,
    pub project_category: Option<ProjectCategory>,
    pub simplified: Option<bool>,
    pub style: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectCategory {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub self_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub project: Option<String>,
    pub project_id: Option<u64>,
    pub assignee: Option<UserRef>,
    pub assignee_type: Option<String>,
    pub lead: Option<UserRef>,
    pub real_assignee: Option<UserRef>,
    pub real_assignee_type: Option<String>,
    pub is_assignee_type_valid: Option<bool>,
    pub self_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentRef {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionRef {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub archived: Option<bool>,
    pub released: Option<bool>,
    pub release_date: Option<String>,
    pub start_date: Option<String>,
    pub self_url: Option<String>,
    pub project_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInsight {
    pub total_issue_count: Option<u64>,
    pub last_issue_update_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    #[serde(rename = "type")]
    pub scope_type: String,
    pub project: Option<ScopeProject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeProject {
    pub id: String,
}
```

### Workflow

Jira workflows are state machines that define the lifecycle of issues: which statuses an issue can transition between, and what rules (conditions, validators, post-functions) apply to each transition. Workflows are associated with projects via Workflow Schemes, which map issue types to workflows.

**Source**: `POST /rest/api/3/workflows` (Bulk get workflows) response schema and API documentation. Also from the deprecated `GET /rest/api/3/workflow/search` endpoint. **Confidence: High** - directly from official API documentation with response examples.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraWorkflow {
    pub id: Option<WorkflowId>,
    pub name: String,
    pub description: Option<String>,
    pub scope: Option<Scope>,
    pub statuses: Vec<WorkflowStatus>,
    pub transitions: Vec<WorkflowTransition>,
    pub is_default: Option<bool>,
    pub is_editable: Option<bool>,
    pub schemes: Option<Vec<WorkflowSchemeRef>>,
    pub projects: Option<Vec<ProjectRef>>,
    pub has_draft_workflow: Option<bool>,
    pub operations: Option<WorkflowOperations>,
    pub created: Option<String>,
    pub updated: Option<String>,
    pub version: Option<WorkflowVersion>,
    pub start_point_layout: Option<LayoutPosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowId {
    pub name: String,
    pub entity_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStatus {
    pub id: Option<String>,
    pub name: Option<String>,
    pub properties: Option<serde_json::Map<String, serde_json::Value>>,
    pub status_reference: Option<String>,
    pub deprecated: Option<bool>,
    pub layout: Option<LayoutPosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTransition {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub from: Option<Vec<String>>,
    pub to: Option<String>,
    #[serde(rename = "type")]
    pub transition_type: Option<String>,
    pub screen: Option<TransitionScreen>,
    pub rules: Option<TransitionRules>,
    pub properties: Option<serde_json::Map<String, serde_json::Value>>,
    pub to_status_reference: Option<String>,
    pub links: Option<Vec<TransitionLink>>,
    pub actions: Option<Vec<serde_json::Value>>,
    pub triggers: Option<Vec<serde_json::Value>>,
    pub validators: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionScreen {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionRules {
    pub conditions_tree: Option<ConditionNode>,
    pub validators: Option<Vec<Validator>>,
    pub post_functions: Option<Vec<PostFunction>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "nodeType")]
pub enum ConditionNode {
    #[serde(rename = "compound")]
    Compound {
        operator: String,
        conditions: Vec<ConditionNode>,
    },
    #[serde(rename = "simple")]
    Simple {
        #[serde(rename = "type")]
        condition_type: String,
        configuration: Option<serde_json::Map<String, serde_json::Value>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Validator {
    #[serde(rename = "type")]
    pub validator_type: String,
    pub configuration: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostFunction {
    #[serde(rename = "type")]
    pub function_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutPosition {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowVersion {
    pub id: String,
    pub version_number: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionLink {
    pub from_port: Option<u32>,
    pub from_status_reference: Option<String>,
    pub to_port: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowOperations {
    pub can_edit: Option<bool>,
    pub can_delete: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSchemeRef {
    pub id: String,
    pub name: Option<String>,
}
```

### Status

Statuses represent the states an issue can be in within a workflow. They are categorized into three status categories: TO DO, IN PROGRESS, and DONE. Statuses can be scoped globally or to specific projects.

**Source**: `GET /rest/api/3/statuses` (Bulk get statuses) response schema from API documentation. **Confidence: High** - directly from official API documentation with response examples.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraStatus {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status_category: Option<StatusCategoryEnum>,
    pub scope: Option<StatusScope>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StatusCategoryEnum {
    Todo,
    InProgress,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusScope {
    #[serde(rename = "type")]
    pub scope_type: String,
    pub project: Option<ScopeProject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusPage {
    pub start_at: Option<u64>,
    pub max_results: Option<u64>,
    pub total: Option<u64>,
    pub is_last: Option<bool>,
    pub next_page: Option<String>,
    pub self_url: Option<String>,
    pub values: Vec<JiraStatus>,
}
```

## Gotchas

### 1. Atlassian Document Format (ADF) Complexity

**Problem**: The v3 REST API uses ADF (a complex ProseMirror-based JSON document format) for issue descriptions, comments, and multi-line text custom fields instead of plain strings. This makes creating and updating issues significantly more complex than v2.

**Workaround**: Use v2 API endpoints (`/rest/api/2/`) where rich text fields accept plain strings. Alternatively, build ADF documents programmatically - the minimum valid ADF for a simple text string is:

```json
{
  "type": "doc",
  "version": 1,
  "content": [
    {
      "type": "paragraph",
      "content": [
        { "type": "text", "text": "Your text here" }
      ]
    }
  ]
}
```

### 2. Custom Fields Are Dynamic and Opaque

**Problem**: Custom fields have IDs like `customfield_10000` that vary per Jira instance. The same "Epic Link" field might be `customfield_10008` on one instance and `customfield_10014` on another. There is no universal field name mapping.

**Workaround**: Use `GET /rest/api/3/field` to discover all fields, or `GET /rest/api/3/issue/createmeta` to get the fields available for a specific project/issue type combination. Cache field mappings per instance.

### 3. Rate Limiting Is Undocumented

**Problem**: Atlassian does not publish specific rate limits for the Jira Cloud REST API. Rate limits vary by tenant and can change without notice. When limits are exceeded, Jira returns HTTP 429 responses.

**Workaround**: Implement exponential backoff with jitter. Monitor for 429 status codes and respect the `Retry-After` header if present. Avoid polling - use webhooks where possible.

### 4. Pagination Inconsistency

**Problem**: Not all paginated endpoints return the `total` count or the `isLast` flag. The `maxResults` parameter has different limits per endpoint and these limits can change without notice. Setting `maxResults` to a large number and checking the returned value is the only way to discover the actual maximum.

**Workaround**: Always paginate until an empty page is returned. Do not rely on `total` being present. Check the returned `maxResults` against your requested value to detect the real limit.

### 5. User Privacy Controls Hide Data

**Problem**: User profile visibility settings can cause `emailAddress` and other fields to be hidden in API responses. A user who has restricted their profile will return `null` for email even if your app has the `read:email-address:jira` scope.

**Workaround**: Use the dedicated `GET /rest/api/3/user/email` endpoint which bypasses profile visibility restrictions, but requires the special `ACCESS_EMAIL_ADDRESSES` Connect scope or `read:email-address:jira` OAuth scope. For general user info, design your integration to gracefully handle missing email fields.

### 6. Username/Key Deprecation

**Problem**: The `username` and `key` fields on User objects have been deprecated in favor of `accountId`. Many older integrations still rely on these fields, which now return empty strings.

**Workaround**: Always use `accountId` as the primary user identifier. If you need to migrate from old username-based systems, use `GET /rest/api/3/user/bulk/migration` to map usernames to account IDs.

### 7. Webhook Reliability Gaps

**Problem**: Webhooks have several known reliability issues:

- Project deletion cascades do NOT send `issue_deleted` webhooks
- Attachments added during issue creation trigger `jira:issue_created` but NOT `attachment_created`
- Webhooks larger than 25MB are silently dropped
- REST-registered webhooks expire after 30 days if not refreshed
- Post-function webhooks don't fire on the Create Issue transition

**Workaround**: Don't rely solely on webhooks for critical synchronization. Implement periodic reconciliation polling as a fallback. Use `GET /rest/api/3/issue/{key}/changelog` to detect missed changes. Set up automated webhook refresh cron jobs.

### 8. JQL Complexity Limits

**Problem**: JQL queries have complexity limits that are not clearly documented. Complex queries involving many custom fields, sub-queries, or large IN clauses can be rejected or time out. JQL performance degrades significantly with certain custom field types.

**Workaround**: Keep JQL queries simple. Use indexed fields where possible. Break complex queries into multiple simpler queries and combine results. Test query performance with `GET /rest/api/3/search` using small page sizes first.

### 9. API Version Differences (v2 vs v3)

**Problem**: v2 and v3 APIs have the same endpoints but differ in how they handle rich text. Using v2 endpoints with v3-style ADF payloads (or vice versa) causes errors. Some newer features are only available in v3.

**Workaround**: Choose one API version and stick with it consistently across your integration. Use v3 for new integrations; use v2 only if you need simple string-based text handling.

### 10. Connect Framework Sunset

**Problem**: Atlassian is ending support for the Connect app framework. New Connect apps can no longer be published on the Atlassian Marketplace. The recommended path is Forge, which has a different execution model (serverless functions on Atlassian infrastructure vs. your own hosting).

**Workaround**: New integrations should use OAuth 2.0 (3LO) for external services or Forge for apps running inside Atlassian. If you have an existing Connect app, plan migration to Forge using Atlassian's incremental migration guide.

### 11. Issue Creation Requires Metadata Discovery

**Problem**: Creating an issue requires knowing which fields are available and required for the target project and issue type. This metadata is not static - it varies per project/issue type combination and can be changed by Jira administrators at any time.

**Workaround**: Always call `GET /rest/api/3/issue/createmeta` before creating issues to discover required fields. For editing, use `GET /rest/api/3/issue/{key}/editmeta` to see which fields can be modified.

### 12. Bulk Operation Limits

**Problem**: Bulk operations have strict but poorly documented limits. Bulk create is capped at 50 issues per request. Bulk fetch at 100. Bulk archive at 1000 (by ID) or 100,000 (by JQL, but this is async). Exceeding limits results in opaque errors.

**Workaround**: Batch your operations within documented limits. For large-scale migrations, use the async archive endpoint with JQL and poll the task URL returned in the response.
