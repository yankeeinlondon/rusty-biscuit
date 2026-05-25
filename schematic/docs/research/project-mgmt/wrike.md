# Research into Wrike's API Support

## Overview on Product

Wrike is a cloud-based work management and project collaboration platform used by 20,000+ organizations worldwide. It provides task management, project planning (with Gantt charts, Kanban boards, table views), resource and capacity planning, reporting, time tracking, custom workflows, and AI-powered features (agents, copilot). Wrike organizes work into **Spaces > Folders/Projects > Tasks**, with a rich permission model (selective sharing, role-based access). The product supports integrations with 150+ apps via Wrike Integrate and offers AI features through MCP (Model Context Protocol) server connections.

### Key URLs

| Resource                    | URL                                                                 |
|-----------------------------|---------------------------------------------------------------------|
| Developer Portal            | https://developers.wrike.com                                        |
| API Reference (V4)          | https://developers.wrike.com/reference                              |
| OAuth 2.0 Docs              | https://developers.wrike.com/docs/oauth-20-authorization            |
| API Changelog               | https://developers.wrike.com/changelog                              |
| App Console (register apps) | https://www.wrike.com/appconsole.htm?#/api                          |
| Postman Collection          | https://cdn.wrike.com/CSO/Official_Wrike_Collection.zip             |
| API Community Forum         | https://community.wrike.com/categories/200428765-API-and-Developers |
| MCP Server Docs             | https://developers.wrike.com/docs/wrike-mcp-server-overview         |
| DataHub API Docs            | https://developers.wrike.com/docs/datahub-overview                  |
| BI Export Docs              | https://developers.wrike.com/docs/export-data                       |
| Cloud Content Connector     | https://developers.wrike.com/docs/overview-cloud-content-connector  |
| Help Center                 | https://help.wrike.com                                              |
| Product Pricing             | https://www.wrike.com/price/                                        |

### Pricing Structure

Wrike uses a per-user/month pricing model (billed annually):

| Plan         | Price (per user/month) | Key Capabilities                                                      |
|--------------|------------------------|-----------------------------------------------------------------------|
| **Free**     | $0                     | Basic task management, board/table views, web/desktop/mobile apps     |
| **Team**     | $10                    | 2-15 users, AI Essentials, shareable dashboards, Gantt charts         |
| **Business** | $25                    | 5-200 users, AI Elite starter, space templates, standard integrations |
| **Pinnacle** | Contact us             | Advanced resource/capacity planning, budgeting, advanced reporting/BI |
| **Apex**     | Contact us             | Full AI Elite, unlimited whiteboards, Wrike Integrate, Wrike Sync     |

Add-ons (included with Apex): Wrike Whiteboard ($15/user/month), Wrike Integrate (custom), Wrike Two-Way Sync (custom), Wrike DataHub (custom), Wrike Lock (custom), AI Elite action packs (custom).

API access is **free** for all subscription tiers, including the Free plan. However, access to certain API features (e.g., Custom Fields) depends on the subscription level.

---

## API Details

### 1. REST API (V4)

Wrike's primary API is a RESTful JSON API at version 4.

- **Base URL**: `https://{host}/api/v4` where `host` varies by data center (e.g., `www.wrike.com` for US, `app-eu.wrike.com` for EU)
- **Methods**: GET, POST, PUT, DELETE (with fallback `method` parameter for clients that only support GET)
- **Content Type**: JSON responses; `kind` field indicates entity type, `data` field contains entity array
- **Rate Limit**: ~400 requests per minute (per-second basis); 429 response on excess
- **Pagination**: `nextPageToken`-based; GET /tasks returns max 1,000 results per call

**Key endpoint groups**: Accounts, Spaces, Folders/Projects, Tasks, Contacts/Users, Comments, Attachments, Timelogs, Custom Fields, Workflows, Webhooks, Dependencies, Invitations, Groups, Roles

#### Formal Schema

- **No OpenAPI/Swagger spec published.** Wrike does not provide a machine-readable schema definition (no OpenAPI YAML/JSON, no GraphQL schema).
- The API reference on Readme.io provides human-readable documentation with example request/response payloads.
- A Postman collection is available for testing.
- The BI Export API does provide a tabular schema reference in the docs.

#### SDKs

Wrike **does not provide official SDKs**. Their FAQ explicitly states they have no official language-specific wrappers. However, community-maintained libraries exist:

| Language           | Library                                         |
|--------------------|-------------------------------------------------|
| C#                 | Community-maintained (referenced in API forums) |
| PHP                | Community-maintained (referenced in API forums) |
| Python             | `wrike-python` (community, unofficial)          |
| Go                 | Various community clients                       |
| Node.js/TypeScript | Community clients exist                         |

No official Rust SDK exists.

#### Authentication Mechanisms

| Mechanism                          | Description                                                                                                                                                                                                                               |
|------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **OAuth 2.0 (Authorization Code)** | The primary and recommended method. Only the authorization code flow is supported (no implicit, client credentials, or password flows). Access tokens expire after 1 hour; refresh tokens are provided. Requires redirect URI with HTTPS. |
| **Permanent Access Token**         | For testing or single-user integrations. Never expires. Generated from the App Console. Grants full access to the user's account. Revoked on password reset, user deactivation, or admin "log in as user" actions.                        |

OAuth scopes: `Default`, `wsReadOnly`, `wsReadWrite`, `amReadOnlyWorkflow`, `amReadWriteWorkflow`, `amReadOnlyInvitation`, `amReadWriteInvitation`, `amReadOnlyGroup`, `amReadWriteGroup`, `amReadOnlyUser`, `amReadWriteUser`

#### Signup Process

1. Register for a Wrike account (free tier works) at https://www.wrike.com
2. Go to App Console at https://www.wrike.com/appconsole.htm?#/api
3. Create a new API application (provides Client ID and Client Secret)
4. For OAuth: configure redirect URIs (HTTPS required, `https://localhost` for dev)
5. For testing: click "Get token" for a Permanent Access Token
6. Use the token or OAuth flow to authenticate API calls

---

### 2. Webhooks

Wrike provides webhook support for event-driven notifications.

- **Scopes**: Folder webhooks (single folder/project, optional recursive), Account webhooks (all tasks shared with user), Space webhooks (single space)
- **Delivery**: HTTP POST to specified `hookUrl` (must be port 80 or 443, publicly accessible)
- **Security**: Optional HMAC-SHA256 signing via shared `secret`
- **Retries**: Up to 3 retries for retryable HTTP errors; immediate suspension for 4xx errors (except 408/429)
- **Event filtering**: `parameterisedEvents` with field-level filtering (e.g., specific custom fields, recurrent tasks)
- **Custom payload fields**: Can request `title`, `status`, `dates`, `responsibleIds`, `customFields`, etc. inline
- **Custom Item Type filtering**: `customItemTypes` parameter restricts delivery to specific CITs
- **Not supported**: Mass action events, import events
- **Idempotency**: `Idempotency-Key` header provided for deduplication (at-least-once delivery only)
- **Description changes**: ~5 minute delay for `TaskDescriptionChanged` events

**Supported event types** (30+): `TaskCreated`, `TaskDeleted`, `TaskTitleChanged`, `TaskStatusChanged`, `TaskDatesChanged`, `TaskImportanceChanged`, `TaskParentsAdded/Removed`, `TaskResponsiblesAdded/Removed`, `TaskSharedsAdded/Removed`, `TaskDescriptionChanged`, `TaskCustomFieldChanged`, `CommentAdded/Deleted`, `AttachmentAdded/Deleted`, `TimelogChanged`, `FolderCreated/Deleted`, `FolderTitleChanged`, `FolderDescriptionChanged`, `FolderParentsAdded/Removed`, `FolderSharedsAdded/Removed`, `FolderCommentAdded/Deleted`, `FolderAttachmentAdded/Deleted`, `FolderCustomFieldChanged`, `ProjectDatesChanged`, `ProjectOwnersAdded/Removed`, `ProjectStatusChanged`, `TaskApprovalStatusChanged`, `TaskApprovalDecisionChanged/Reset`, `FolderApprovalStatusChanged/DecisionChanged/Reset`, `WorkItemTypeChanged`, `CreateFromBlueprintCompleted`, `ImportFromFileCompleted`

---

### 3. DataHub Public API

A separate REST API for accessing Wrike's structured data (databases, records, fields) within the DataHub feature.

- **Base URL**: `https://{host}/api/v4` (same host routing as main API)
- **Authentication**: Same OAuth 2.0 / Permanent Token as main API
- **Entities**: Databases, Records, Fields, Folders (DataHub-specific, not Wrike Folders)
- **Features**: CRUD operations, batch editing, pagination, filtering with rich operators, formula fields, idempotency via `requestId`
- **Pagination**: `nextPageToken` with configurable `limit` (default 100, max 1000)
- **Session binding**: Page tokens are session-bound (up to 3 hours)

---

### 4. BI Export API

For exporting Wrike account data in analytics-friendly CSV format.

- **Purpose**: Bulk data export for BI tools (Tableau, Power BI, Looker)
- **Output**: Set of CSV files
- **Contains**: Some data not available in REST API (e.g., history of task statuses and custom field changes)
- **Schema reference**: Available in docs at https://developers.wrike.com/docs/schema-reference-bi-export

---

### 5. Cloud Content Connector

Provides access to digital assets (files/attachments) stored in Wrike.

- **Purpose**: Find, retrieve, and manage file assets
- **Reference**: https://developers.wrike.com/docs/overview-cloud-content-connector

---

### 6. MCP (Model Context Protocol) Server

Wrike offers an MCP server for AI assistant integration.

- **Compatible with**: Claude Desktop, Claude Code, ChatGPT, Microsoft Copilot Studio, Cursor, and any MCP-compatible client
- **Authentication**: OAuth 2.0 (preferred) or Permanent Access Token
- **Capabilities**: Query projects, manage tasks, navigate folders, search, create work structures
- **URL**: `https://mcp.wrike.com/app/mcp` (streamable HTTP) or `https://mcp.wrike.com/app/mcp/sse` (SSE fallback)

---

### Summary: No WebSocket, No JSON-RPC

Wrike does **not** provide WebSocket or JSON-RPC APIs. All real-time needs are served by webhooks. The API surface is entirely REST + Webhooks + MCP.

---

## Schemas

The following schemas are derived from the official API documentation's example responses and field descriptions. Confidence is **high** for Task and Contact schemas (well-documented with multiple examples). Confidence is **medium** for Account and Workflow schemas (derived from response examples and partial documentation). The ID format uses Wrike's proprietary short alphanumeric strings (e.g., `KUAJ25LC`, `IEAGIITR`).

### Task (Todo / Action)

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskDates {
    #[serde(rename = "type")]
    pub date_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_on_weekends: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Task {
    pub id: String,
    pub account_id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brief_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub super_parent_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_status_id: Option<String>,
    pub importance: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dates: Option<TaskDates>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub responsible_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub responsible_placeholder_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependency_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_fields: Option<Vec<CustomFieldValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permalink: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_item_type_id: Option<String>,
    pub created_date: String,
    pub updated_date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurrence: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_attachments: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}
```

**Source**: API reference response examples and OAuth docs contact endpoint examples. The Task entity is the most thoroughly documented entity in Wrike's API. The `status` field uses string values like `"Active"`, `"Completed"`, `"Deferred"`, `"Cancelled"`. Custom statuses are referenced by `custom_status_id`. Description content is HTML with a restricted tag set.

### Person / Contact

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContactProfile {
    pub account_id: String,
    pub email: String,
    pub role: String,
    pub external: bool,
    pub admin: bool,
    pub owner: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Contact {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    #[serde(rename = "type")]
    pub contact_type: String,
    pub profiles: Vec<ContactProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub me: Option<bool>,
}
```

**Source**: OAuth docs example response (contacts endpoint). Confidence is **high**. The `type` field distinguishes `"Person"` from group contacts. Each contact can belong to multiple accounts via `profiles`. The `role` field contains values like `"User"`, `"Collaborator"`, `"Viewer"`.

### Company / Organization (Account)

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Account {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_day_of_week: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_days: Option<Vec<String>>,
    pub root_folder_id: String,
    pub recycle_bin_id: String,
    pub created_date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub joined_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Vec<serde_json::Value>>,
}
```

**Source**: Webhooks guide example (account query). Confidence is **medium-high**. Wrike uses "Account" to represent a company/organization. Each user can belong to multiple accounts. The `metadata` field can contain custom account-level metadata.

### Workflow (Custom Status)

Wrike models workflows as a collection of custom statuses. A workflow defines the valid status transitions for tasks and projects.

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CustomStatus {
    pub id: String,
    pub name: String,
    pub color: String,
    #[serde(rename = "type")]
    pub status_type: String,
    pub group: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standard: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub custom_statuses: Vec<CustomStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_workflow: Option<bool>,
}
```

**Source**: API reference documentation (workflows endpoint). Confidence is **medium**. The `status_type` maps to standard statuses (Active, Completed, Deferred, Cancelled). The `group` field categorizes statuses for UI rendering. Workflows can be applied at the project or account level.

### Status

Status is represented within the Workflow/CustomStatus model above and also inline in Task entities:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskStatus {
    pub status: String,
    pub custom_status_id: String,
}
```

The `status` field contains standard values (`"Active"`, `"Completed"`, `"Deferred"`, `"Cancelled"`) and is always present on tasks. The `custom_status_id` maps to a CustomStatus within the account's workflow, allowing custom labels/colors while preserving the underlying standard status grouping.

### Supporting Types

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CustomFieldValue {
    pub id: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Folder {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space_id: Option<String>,
    pub scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<Project>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_fields: Option<Vec<CustomFieldValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_item_type_id: Option<String>,
    pub created_date: String,
    pub updated_date: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Project {
    pub author_id: String,
    pub owner_ids: Vec<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_status_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_date: Option<String>,
}
```

**Source**: Webhook payloads and API reference. Confidence is **medium-high**. In Wrike, a "Project" is a Folder with an attached `project` metadata object. Folders and Projects share the same ID namespace and many of the same API endpoints.

---

## Gotchas

### 1. Data Center Routing (Host Parameter)

**Issue**: Wrike stores customer data in multiple data centers (US, US2, EU). The base URL for API calls varies by data center. Using the wrong host returns a generic `not_authorized` error that is indistinguishable from an actual auth error.

**Workaround**: Always use the `host` parameter returned from the `/oauth2/token` endpoint to construct the base URL (`https://{host}/api/v4`). For permanent tokens, inspect the browser URL when logged into Wrike to determine the host.

### 2. Rate Limiting is Aggressive

**Issue**: ~400 requests per minute with no official burst allowance. The rate limit is enforced on a per-second basis, not per-minute. Exceeding it returns a 429 response. Internal service overload protection can also trigger 429 responses even under the nominal limit.

**Workaround**: Implement exponential backoff with jitter on 429 responses. Use webhooks instead of polling where possible. Batch operations where the API supports them.

### 3. Pagination with nextPageToken Only

**Issue**: The GET /tasks endpoint returns a maximum of 1,000 results per call. Pagination uses opaque `nextPageToken` values (not offset-based), so you cannot skip pages or estimate total counts.

**Workaround**: Iterate through all pages sequentially. There is no way to get a total count without fetching all pages.

### 4. No Official SDKs or Type Definitions

**Issue**: Wrike provides no official SDKs, no OpenAPI spec, and no TypeScript type definitions. All client code must be built from scratch using the documentation.

**Workaround**: Use the Postman collection as a reference. Community libraries exist for some languages. For Rust, you must hand-roll the client.

### 5. Authorization Code Expires in 10 Minutes

**Issue**: The OAuth authorization code is valid for only 10 minutes. If the exchange for tokens is not completed within this window, the entire flow must be restarted.

**Workaround**: Ensure your token exchange endpoint is fast and automated. For background apps, use permanent tokens or store refresh tokens securely.

### 6. Mass Actions Don't Trigger Webhooks

**Issue**: Changes made via mass actions (bulk edit, import) do not trigger webhook notifications. This is a documented limitation.

**Workaround**: After performing mass operations, manually poll for changes or use the BI Export API for reconciliation.

### 7. TaskDescriptionChanged Has ~5 Minute Delay

**Issue**: Webhook notifications for task description changes are fired with approximately a 5-minute delay, unlike other events which are near-real-time.

**Workaround**: If real-time description monitoring is critical, poll the task endpoint instead of relying on webhooks.

### 8. Webhook URL Restrictions

**Issue**: Webhook URLs must use port 80 or 443 only. No private/intranet addresses are supported. This makes local development testing difficult.

**Workaround**: Use a tunneling service like ngrok or webhook.site for development. Production endpoints must be publicly accessible on standard ports.

### 9. Webhooks Are At-Least-Once Delivery

**Issue**: Wrike cannot guarantee exactly-once delivery. Duplicate events are possible due to network issues or retries.

**Workaround**: Use the `Idempotency-Key` header to detect and discard duplicate deliveries.

### 10. API V3 Is Deprecated

**Issue**: API v3 was sunset on June 30, 2019. The current version is v4. V3 is not compatible with v4 and the main difference is that v4 can access data from only one account at a time.

**Workaround**: All new development should use v4 exclusively.

### 11. Token Revocation on Admin Actions

**Issue**: API tokens (both permanent and OAuth) are revoked when: user manually revokes, user resets password, user is deactivated, admin signs in as the user with "Log in as this user", or admin changes password strength policy.

**Workaround**: Implement token refresh/re-authentication logic. For service integrations, use a dedicated "technical" user account whose password is never reset.

### 12. Custom Field Values Are JSON-Stringified

**Issue**: Custom field values in webhook payloads are represented as JSON strings (double-quoted). For example, a text custom field value appears as `"\"some text\""` rather than `"some text"`.

**Workaround**: Parse custom field values with an extra JSON deserialization step.

### 13. HTML-Only Task Descriptions

**Issue**: Task descriptions and comments use a restricted HTML subset. There is no Markdown support. Comments can optionally be retrieved as plain text via `plainText=true`.

**Workaround**: Build HTML for creating/updating descriptions. Use the documented allowed tag set. For reading, strip HTML tags or use the `plainText` parameter where available.

### 14. No GraphQL or Advanced Query Language

**Issue**: Wrike's API does not support GraphQL or any advanced query language. Filtering is done via URL parameters with limited operators.

**Workaround**: Use the available query parameters for filtering. For complex analytical queries, use the DataHub API or BI Export.

### 15. Access Tied to User Permissions

**Issue**: All API requests execute on behalf of the authenticated user. The API returns only data that the user can see according to Wrike's sharing model. There is no admin/superuser API mode.

**Workaround**: For integrations that need broad access, create a dedicated technical user with appropriate permissions and folder sharing.
