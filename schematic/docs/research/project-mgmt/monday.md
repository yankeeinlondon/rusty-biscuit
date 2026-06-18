# Research into Monday.com's API Support

## Overview on Product

Monday.com is a cloud-based Work OS platform that enables teams to create and shape custom workflows, manage projects, track tasks, and collaborate across departments. It positions itself as a flexible work management platform that goes beyond traditional project management tools.

### Functional Footprint

- **Work Management**: Boards (kanban, table, timeline, Gantt, calendar views), items (rows/tasks), groups, columns, automations, and integrations
- **CRM (monday CRM)**: Lead tracking, deal pipeline, contact management
- **Dev (monday Dev)**: Sprint planning, bug tracking, feature roadmaps, git integration
- **Service (monday Service)**: IT service management, ticketing, SLA tracking
- **AI Features**: AI agents, meeting notetaker (Sidekick), AI-powered app builder (Vibe), AI columns, AI workflow builder
- **Docs**: Collaborative documents with live data embedding (Workdocs)
- **Dashboards**: Customizable widgets, charts, and reporting
- **Automations**: Recipe-based workflow automation with 250,000+ actions/month on Enterprise
- **Integrations**: Native integrations with third-party tools (Slack, Teams, Jira, etc.)
- **Marketplace**: Third-party app ecosystem for extending platform capabilities

### Key URLs

| Resource            | URL                                                                                                          |
|---------------------|--------------------------------------------------------------------------------------------------------------|
| Platform API Docs   | https://developer.monday.com/api-reference/docs                                                              |
| API Reference       | https://developer.monday.com/api-reference/reference                                                         |
| Apps Framework Docs | https://developer.monday.com/apps/docs/intro                                                                 |
| API Changelog       | https://developer.monday.com/api-reference/changelog                                                         |
| API Playground      | https://monday.com/developers/v2/try-it-yourself                                                             |
| Developer Community | https://developer-community.monday.com/                                                                      |
| Help Center         | https://support.monday.com/                                                                                  |
| App Marketplace     | https://monday.com/marketplace                                                                               |
| Postman Collection  | https://www.postman.com/matiasdavidson1/my-workspace/collection/dmzv0h4/queries-and-mutations-for-monday-com |
| MCP for Monday      | https://monday.com/w/mcp                                                                                     |
| Vibe Design System  | https://vibe.monday.com                                                                                      |
| GitHub (Vibe)       | https://github.com/mondaycom/monday-ui-react-core                                                            |

### Pricing Structure

Monday.com pricing is per-seat, per-month with annual and monthly billing options. Annual billing provides an 18% discount.

| Plan                   | Price (per seat/month, billed annually) | Key Limits                                                                                                                      |
|------------------------|-----------------------------------------|---------------------------------------------------------------------------------------------------------------------------------|
| **Free**               | $0 (up to 2 seats)                      | 3 boards, 3 docs, 8 column types, 200+ templates                                                                                |
| **Basic**              | $9                                      | 1,000 AI credits/month, unlimited items, 5 GB storage, unlimited free viewers                                                   |
| **Standard**           | $12                                     | 2,000 AI credits/month, 250 automations/month, 250 integrations/month, guest access                                             |
| **Pro** (Most Popular) | $19                                     | 3,000 AI credits/month, 25,000 automations/month, 25,000 integrations/month, private boards, time tracking                      |
| **Enterprise**         | Custom quote                            | 20,000+ AI credits, 250,000 automations/month, portfolio management, resource management, 99.9% uptime SLA, tailored onboarding |

All plans include iOS and Android apps. API access is available on all paid tiers (Free plan has limited API access with 1,000 daily calls). Enterprise adds SCIM provisioning, audit log API, enterprise-grade security, and multi-level permissions.

## API Details

### 1. GraphQL API (Primary API)

The primary and only API for interacting with Monday.com platform data. It is a full GraphQL API supporting both queries (reads) and mutations (writes).

- **Endpoint**: `https://api.monday.com/v2`
- **Protocol**: HTTPS POST with JSON body containing a `query` field
- **Versioning**: Quarterly releases in `yyyy-mm` format (January, April, July, October). Versions move through Release Candidate -> Current -> Maintenance -> Deprecated lifecycle. Each version is stable for at least 6 months.
- **Current versions** (as of research date):

    - `2026-01` (Maintenance)
    - `2026-04` (Current)
    - `2026-07` (Release Candidate)

#### Formal Schema

- **GraphQL introspection**: The API supports full GraphQL introspection, meaning the schema is self-describing. You can query `__schema` and `__type` to discover all types, fields, queries, and mutations.
- **No OpenAPI/Swagger**: Monday does not publish a formal OpenAPI or REST-style schema. The GraphQL schema is the canonical schema definition.
- **Schema is downloadable**: The MCP tool `get_graphql_schema` and the JS SDK's `fetch-schema.sh` script allow downloading the full schema for offline code generation.
- **Schema changes** are tracked in the API changelog and release notes.

#### SDKs

| Language                   | Package                      | Version | Notes                                                                                                                                    |
|----------------------------|------------------------------|---------|------------------------------------------------------------------------------------------------------------------------------------------|
| **JavaScript/TypeScript**  | `@mondaydotcomorg/api`       | 14.0.0  | Full TypeScript types generated from schema, wraps `graphql-request`, pre-built operations, file upload support, Node.js 18+ and browser |
| **JavaScript (companion)** | `@mondaydotcomorg/setup-api` | 2.1.1   | GraphQL codegen scaffolding CLI for custom typed queries                                                                                 |
| **Python**                 | `monday-api-python-sdk`      | 1.6.5   | Module-based client (boards, items, updates, etc.), auto-deserialization via `dacite`, built-in retry logic, Python 3.7+                 |

The JS SDK also provides a `SeamlessApiClient` for use inside monday.com app iframes (no token needed, uses `postMessage` bridge).

#### Authentication

| Method                 | Description                                                                                                                                                                                                          |
|------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **Personal API Token** | V2 tokens accessible from Developer Center or Admin tab. Mirrors UI permissions. Admin and member users can access tokens.                                                                                           |
| **OAuth 2.0**          | Standard OAuth flow for apps. Authorization URL: `https://auth.monday.com/oauth2/authorize`. Token URL: `https://auth.monday.com/oauth2/token`. Tokens do not expire (valid until app uninstall). No refresh tokens. |
| **Short-Lived Token**  | Available for guest users who cannot access personal API tokens.                                                                                                                                                     |
| **Session Token**      | For apps running inside monday.com iframes (via `SeamlessApiClient`).                                                                                                                                                |

OAuth scopes include: `account:read`, `assets:read`, `boards:read`, `boards:write`, `departments:read`, `departments:write`, `docs:read`, `docs:write`, `me:read`, `notifications:write`, `tags:read`, `teams:read`, `teams:write`, `updates:read`, `updates:write`, `users:read`, `users:write`, `webhooks:read`, `webhooks:write`, `workspaces:read`, `workspaces:write`.

#### Signup Process for API Access

1. Create a monday.com account (free tier or trial)
2. Access the Developer Center from the profile menu
3. For personal tokens: Navigate to API Token > Show
4. For OAuth apps: Create an app in the Developer Center, configure OAuth scopes and redirect URLs
5. No separate developer account or API gateway signup is required

#### Rate Limits

| Limit Type                                 | Free/Basic/Standard  | Pro                  | Enterprise           |
|--------------------------------------------|----------------------|----------------------|----------------------|
| **Daily calls**                            | 1,000                | 10,000 (soft)        | 25,000 (soft)        |
| **Queries per minute**                     | 1,000                | 2,500                | 5,000                |
| **Max concurrent requests**                | 40                   | 100                  | 250                  |
| **Complexity per query**                   | 5,000,000            | 5,000,000            | 5,000,000            |
| **Complexity per minute (personal token)** | 1,000,000            | 10,000,000           | 10,000,000           |
| **IP limit**                               | 5,000 per 10 seconds | 5,000 per 10 seconds | 5,000 per 10 seconds |

Daily limits reset at midnight UTC. Rate-limited responses include `retry_in_seconds` and `Retry-After` headers.

### 2. Webhooks

Monday.com supports webhooks for receiving real-time event notifications. Webhooks can be created via the API (`webhooks:write` scope) and fire on board-level events.

- **Setup**: Via API mutation or board settings in the UI
- **Events supported**: Item created, item updated, column value changed, status changed, etc.
- **Payload format**: JSON POST to a configured URL
- **Limitations**: No doc-specific webhook events, webhook numeric limits are not formally documented, integration action limits apply
- **Workaround for doc changes**: Use the version history queries for docs

### 3. Platform MCP (Model Context Protocol)

Monday.com provides a Platform MCP server that exposes 50+ tools for interacting with the platform via the MCP standard. This enables AI agents and tools to interact with monday.com.

- **URL**: https://monday.com/w/mcp
- **Capabilities**: Board CRUD, item management, workspace operations, doc management, dashboard creation, form management, user/team listing, search, sprint management, and direct GraphQL execution
- **Includes UI tools**: Show Table, Show Chart, Show Battery, Show Assign

### 4. External APIs (Enterprise Only)

| API                       | Description                                                                                      |
|---------------------------|--------------------------------------------------------------------------------------------------|
| **SCIM Provisioning API** | User and team lifecycle management (provision, de-provision, update). Follows SCIM 2.0 standard. |
| **Audit Log API**         | Access to audit log events for compliance and security monitoring.                               |

### 5. Apps Framework API (for building monday apps)

- **monday.api**: Server-side API calls from app backend
- **monday.listen**: Listen for frontend events
- **monday.get**: Get contextual data from the monday UI
- **monday.execute**: Execute actions in the monday UI
- **monday.storage**: Key-value storage for app data
- **monday.set**: Set contextual data in the monday UI

## Schemas

Monday.com uses a highly flexible, board-centric data model rather than traditional fixed entity types. There is no native "Task", "Contact", or "Company" entity -- instead, these concepts are modeled as items (rows) on boards with custom column configurations. The structure below is inferred from the GraphQL schema types, SDK type exports, and API reference documentation.

### Entity: Item (Task / Action / Todo)

Items are the rows on a monday board and represent individual work items. They are the closest analog to a "task" or "todo".

**Source**: GraphQL schema `Item` type, JS SDK `Item` type, Python SDK `Item` dataclass. High confidence -- this is the core entity and is well-documented.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MondayItem {
    pub id: String,
    pub name: String,
    pub board_id: Option<String>,
    pub group_id: Option<String>,
    pub state: Option<ItemState>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub column_values: Vec<ColumnValue>,
    pub subitems: Option<Vec<MondayItem>>,
    pub parent_item: Option<Box<MondayItem>>,
    pub updates: Option<Vec<Update>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemState {
    Active,
    Archived,
    Deleted,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnValue {
    pub id: String,
    pub text: Option<String>,
    pub value: Option<String>,
    pub display_value: Option<String>,
    #[serde(rename = "type")]
    pub column_type: Option<String>,
    pub column: Option<ColumnRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnRef {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub column_type: String,
}
```

### Entity: User (Person / Contact)

Users are the people on a monday.com account. Contacts are not a distinct entity -- they are modeled as items on CRM boards.

**Source**: GraphQL schema `User` type, OAuth scopes. High confidence for the user model; low confidence for contact modeling (which is board-config-dependent).

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MondayUser {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub enabled: Option<bool>,
    pub is_guest: Option<bool>,
    pub is_pending: Option<bool>,
    pub is_admin: Option<bool>,
    pub photo_small: Option<String>,
    pub photo_original: Option<String>,
    pub phone: Option<String>,
    pub mobile_phone: Option<String>,
    pub title: Option<String>,
    pub birthday: Option<String>,
    pub location: Option<String>,
    pub timezone: Option<String>,
    pub url: Option<String>,
    pub account: Option<AccountRef>,
    pub teams: Option<Vec<TeamRef>>,
    pub created_at: Option<String>,
    pub joined_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountRef {
    pub id: String,
    pub name: Option<String>,
    pub slug: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamRef {
    pub id: String,
    pub name: Option<String>,
}
```

### Entity: Board (Workspace / Project Container)

Boards are the primary organizing structure in monday.com. They contain items organized into groups with customizable columns. Companies/organizations are not a distinct entity -- they would be modeled as boards or items on CRM boards.

**Source**: GraphQL schema `Board` type. High confidence.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MondayBoard {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub board_kind: Option<BoardKind>,
    pub state: Option<BoardState>,
    pub workspace_id: Option<String>,
    pub columns: Option<Vec<Column>>,
    pub groups: Option<Vec<Group>>,
    pub items_page: Option<ItemsPage>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub owner: Option<UserRef>,
    pub subscribers: Option<Vec<UserRef>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoardKind {
    Public,
    Private,
    Share,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoardState {
    Active,
    Archived,
    Deleted,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub column_type: String,
    pub description: Option<String>,
    pub settings_str: Option<String>,
    pub archived: Option<bool>,
    pub width: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub title: String,
    pub color: Option<String>,
    pub position: Option<String>,
    pub archived: Option<bool>,
    pub deleted: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemsPage {
    pub items: Vec<MondayItem>,
    pub cursor: Option<String>,
}
```

### Entity: Workflow

Monday.com does not have a formal "Workflow" entity in its API. Workflows are represented through **automations** (recipe-based) and **integrations**. Automation management (create, delete, list) is not currently exposed via the API -- only manageable through the UI.

**Source**: API coverage gaps documentation explicitly lists automation management as unsupported. Low confidence -- this is a known gap.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MondayAutomation {
    pub id: String,
    pub board_id: Option<String>,
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub trigger_type: Option<String>,
    pub action_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MondayWorkspace {
    pub id: String,
    pub name: String,
    pub kind: Option<WorkspaceKind>,
    pub description: Option<String>,
    pub boards: Option<Vec<MondayBoard>>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceKind {
    Open,
    Closed,
}
```

### Entity: Status

Status in monday.com is not a standalone entity -- it is a column type on boards. The status column has configurable labels and colors. The column value JSON format determines the current status.

**Source**: Column values reference documentation, GraphQL `StatusValue` fragment type. High confidence on the value format; medium confidence on the full label configuration (retrievable via `settings_str` on Column).

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusColumnValue {
    pub index: Option<i32>,
    pub label: Option<String>,
    pub changed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusColumnSettings {
    pub labels: Option<Vec<StatusLabel>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusLabel {
    pub index: i32,
    pub name: String,
}
```

### Entity: Update (Comment)

Updates are comments/notes attached to items. They are first-class entities in the API.

**Source**: GraphQL schema `Update` type. High confidence.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MondayUpdate {
    pub id: String,
    pub body: Option<String>,
    pub text_body: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub creator: Option<UserRef>,
    pub replies: Option<Vec<MondayUpdate>>,
    pub assets: Option<Vec<Asset>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRef {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub name: Option<String>,
    pub url: Option<String>,
    pub public_url: Option<String>,
    pub file_size: Option<i64>,
    pub file_extension: Option<String>,
}
```

## Gotchas

### 1. GraphQL-Only API (No REST)

Monday.com offers only a GraphQL API. There is no REST API. Developers accustomed to RESTful patterns must learn GraphQL query syntax, mutations, and introspection. This also means standard REST tooling (e.g., OpenAPI generators, REST clients) does not apply directly.

**Workaround**: Use the official SDKs (JS/Python) which abstract away much of the GraphQL complexity. The MCP server also provides a higher-level interface.

### 2. Complexity-Based Rate Limiting

Rate limiting is not purely request-count-based. Each query has a computed "complexity" cost that depends on the number of fields, nesting depth, and data volume. Deeply nested queries or queries requesting large data sets can exhaust your complexity budget very quickly, even with few requests.

**Workaround**: Always include the `complexity` field in queries to monitor cost. Use pagination (`page`, `limit`, cursor-based), request only needed fields, and avoid deeply nested queries.

### 3. Daily Call Limits Are Surprisingly Low on Lower Tiers

Free/Basic/Standard plans are limited to 1,000 API calls per day. For a Pro plan, 10,000 is a "soft limit" but exceeding it requires requesting an increase. Even Enterprise is capped at 25,000 (soft). Failed requests and rate-limited responses also count toward the daily limit.

**Workaround**: Implement aggressive caching, use webhooks instead of polling, batch operations with `change_multiple_column_values`, and monitor usage via the API analytics dashboard (Enterprise) or `platform_api` query.

### 4. Column Values Are Dynamically Typed JSON

Column values are returned as JSON strings (`value` field) with a `text` convenience field. Each column type (status, date, people, etc.) has a different JSON structure. There are 40+ column types, each with its own expected JSON format for reads and writes.

**Workaround**: Use the SDKs which handle column value formatting. Refer to the column values reference documentation for the exact JSON structure each column type expects. Use GraphQL fragments for type-specific value parsing.

### 5. No Automation Management via API

Creating, listing, updating, or deleting board automations/recipes is not supported through the API. This is a commonly requested feature and is listed as a known coverage gap. Automations can only be managed through the UI.

**Workaround**: Use custom webhook-based integrations or the apps framework's integration recipes for programmatic workflow automation.

### 6. No Bulk Import/Update API

There is no file-based bulk import or update endpoint (e.g., CSV upload). Each item must be created or updated individually via `create_item` and `change_multiple_column_values` mutations.

**Workaround**: Use parallel async requests with pagination, or leverage the Python SDK's `fetch_all_items_by_board_id` for reads. Batch column updates using `change_multiple_column_values`. A bulk import API is listed as upcoming.

### 7. Board Kind (Visibility) Cannot Be Changed After Creation

The `board_kind` (public/private/share) is set when creating a board and cannot be changed afterward via the API. The `update_board` mutation only supports `name`, `description`, `communication`, and `item_nickname`.

**Workaround**: Create boards with the correct visibility from the start, or use the UI to change visibility after creation.

### 8. Classic Boards Cannot Be Converted to Multi-Level

There is no API or UI migration path to convert existing classic boards to multi-level boards (which support sub-items natively).

**Workaround**: Create new multi-level boards and migrate items programmatically.

### 9. OAuth Tokens Do Not Expire

OAuth access tokens are valid until the user uninstalls the app. There are no refresh tokens. While this simplifies token management, it means there is no way to periodically rotate tokens without user interaction.

**Workaround**: Store tokens securely. Monitor for 401 errors indicating the app was uninstalled. Implement re-authorization flows.

### 10. Mirror and Formula Columns Cannot Be Filtered Server-Side

The `items_page` query with server-side filtering does not support `mirror` or `formula` column types. This means you cannot use these columns in API queries for filtering.

**Workaround**: Fetch all items and filter client-side, or restructure your board to use filterable column types for the data you need to query.

### 11. Webhook Limits Are Undocumented

While webhooks are supported, specific limits (max webhooks per board, per account, delivery rate caps) are not formally documented.

**Workaround**: Test incrementally and monitor for errors. Contact monday support for high-volume webhook use cases.

### 12. No PDF Export for Docs

The API supports markdown export for Workdocs but not PDF export. Image upload in docs requires hosting images externally and passing URLs.

**Workaround**: Export to markdown and convert to PDF using external tools. Host images on your own infrastructure.

### 13. Versioning Requires Explicit Headers

If you do not pass the `API-Version` header, the API defaults to the "Current" version. This means your queries could break when a new version becomes Current if you rely on fields that are deprecated.

**Workaround**: Always explicitly pass the `API-Version` header with your desired version. The SDKs handle this automatically but allow per-request overrides.

### 14. Free/Basic Accounts Have Severely Limited API Access

Trial, NGO, and free accounts have a complexity budget of only 1,000,000 points per minute (vs. 10,000,000 for paid accounts). Daily call limits of 1,000 are also very restrictive for anything beyond light automation.

**Workaround**: Upgrade to at least Standard plan for production API usage. Pro or Enterprise is recommended for serious integrations.

### 15. Viewer Users Cannot Access the API

Users with the "Viewer" role, deactivated/disabled users, users with unconfirmed emails, and student accounts cannot access the API at all. Only Admin, Member, and Guest (with limitations) users can use the API.

**Workaround**: Ensure API tokens are generated by Admin or Member users. For guest access, use OAuth or short-lived tokens.
