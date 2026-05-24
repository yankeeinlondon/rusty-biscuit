# Research into ClickUp's API Support

## Overview on Product

ClickUp is an all-in-one productivity and project management platform that positions itself as "one app to replace them all." Launched in 2017, it has rapidly grown to serve over 2 million teams with a broad feature set that spans task management, documentation, goal tracking, time tracking, whiteboards, and AI-powered workflows.

### Functional Footprint

- **Task Management**: Tasks with custom statuses, priorities, assignees, due dates, start dates, tags, checklists, dependencies, and task links. Supports subtasks and custom task types.
- **Hierarchical Organization**: Workspaces > Spaces > Folders > Lists > Tasks. Spaces can be configured with features (due dates, time tracking, etc.) and multiple statuses.
- **Views**: Kanban boards, list views, Gantt charts, calendar, timeline, workload, table, activity, and map views.
- **Docs and Wikis**: Rich document editing with nested pages, real-time collaboration, and linking to tasks.
- **Whiteboards**: Collaborative whiteboards with drawing, shapes, connectors, and task creation from board elements.
- **Goals**: Goals with measurable key results, roll-up progress from tasks, lists, and manual updates.
- **Time Tracking**: Native time tracking with global timer, time estimates, billable time, and timesheet views.
- **Dashboards**: Customizable dashboards with 50+ card types for reporting on workload, time, status, and more.
- **Automations**: If-this-then-that automation engine with 100+ automation recipes (Business plan and above). Supports triggers, conditions, and actions.
- **Custom Fields**: Extensive custom field system (text, dropdown, labels, date, number, currency, checkbox, URL, users, email, phone, formula, relationship, rollup, AI-powered fields).
- **Forms**: Custom forms that create tasks on submission, with conditional logic.
- **Chat**: Built-in chat with channels and direct messages.
- **Brain AI**: AI-powered assistant for writing, summarizing, generating tasks, auto-assigning, prioritizing, and "super agents" for complex workflows.
- **Integrations**: Native integrations with Slack, GitHub, GitLab, Figma, Google Drive, Dropbox, Zoom, and 1000+ more via Zapier/Make.
- **Webhooks**: Outbound webhook notifications for workspace events (Business plan and above).
- **MCP Server**: Model Context Protocol server for AI assistant integration (Claude, ChatGPT, etc.).
- **Reporting**: Sprint reporting, burndown charts, velocity tracking, portfolio management, and resource management.

### Key URLs

| Resource                   | URL                                                                         |
|----------------------------|-----------------------------------------------------------------------------|
| API Documentation (Guides) | https://clickup.com/api/docs                                                |
| API Reference              | https://clickup.com/api/reference                                           |
| OpenAPI Spec Download      | https://clickup.com/api/docs/open-api-spec                                  |
| Webhooks Guide             | https://clickup.com/api/docs/webhooks                                       |
| OAuth Guide                | https://clickup.com/api/docs/authentication                                 |
| MCP Server Setup           | https://clickup.com/api/docs/connect-an-ai-assistant-to-clickups-mcp-server |
| MCP Tools Reference        | https://clickup.com/api/docs/mcp-tools                                      |
| Developer Feedback         | https://feedback.clickup.com/public-api/                                    |
| ClickUp Support            | https://help.clickup.com/hc/en-us/                                          |
| ClickUp Product Page       | https://clickup.com/                                                        |
| ClickUp Pricing            | https://clickup.com/pricing                                                 |
| ClickUp API Status         | https://status.clickup.com/                                                 |
| Community (on GitHub SDKs) | Various community SDKs on GitHub                                            |

### Pricing Structure

ClickUp pricing is per-user, per-month with monthly and annual billing. Annual billing provides discounts.

| Plan             | Price (annual) | Key API-Related Features                                                                               |
|------------------|----------------|--------------------------------------------------------------------------------------------------------|
| **Free Forever** | $0/user/month  | Unlimited tasks, basic custom fields, 60MB storage, API access included                                |
| **Unlimited**    | $7/user/month  | Unlimited storage, unlimited integrations, unlimited custom fields, native time tracking               |
| **Business**     | $12/user/month | Webhooks, 5K automations/month, unlimited dashboards, Google SSO                                       |
| **Enterprise**   | Custom pricing | Enterprise API (higher rate limits), SAML SSO, SCIM, audit log, 250K automations/month, data residency |

**AI Add-ons** (separate pricing):

- **Brain AI**: $9/user/month - AI assistant, AI writing, enterprise search
- **Everything AI**: $28/user/month - Full agentic suite with AI notetaker, image generation, AI fields, automations

**Notable**: API access is available on all plans including Free Forever. Webhooks require the Business plan or above. Enterprise customers get a dedicated "Enterprise API" with higher rate limits and additional endpoints.

## API Details

### REST API (v2)

ClickUp provides a RESTful JSON API at `https://api.clickup.com/api/v2/`. This is the primary and only official API surface.

**Base URL**: `https://api.clickup.com/api/v2/`

**Supported Operations** (non-exhaustive):

| Resource           | Endpoints                                                                     |
|--------------------|-------------------------------------------------------------------------------|
| Authorization      | Get access token (OAuth), Get authorized user                                 |
| Teams (Workspaces) | Get teams, Get team by ID                                                     |
| Spaces             | CRUD operations on spaces                                                     |
| Folders            | CRUD operations on folders                                                    |
| Lists              | CRUD operations on lists, folderless lists                                    |
| Tasks              | Get/create/update/delete tasks, get filtered tasks, bulk update time estimate |
| Task Comments      | CRUD on comments, threaded comments                                           |
| Task Checklists    | CRUD on checklists and checklist items                                        |
| Task Attachments   | Upload attachments                                                            |
| Task Dependencies  | Add/remove dependencies                                                       |
| Task Links         | Add/remove task links                                                         |
| Tags               | CRUD on space tags, tag/untag tasks                                           |
| Custom Fields      | Get fields, set/remove values on tasks                                        |
| Custom Task Types  | Get custom task types for workspace                                           |
| Goals              | CRUD on goals and key results                                                 |
| Time Tracking      | Get time entries, start/stop timer, get single entry                          |
| Members            | Get task members, get list members                                            |
| Roles              | Get custom roles (Enterprise)                                                 |
| Webhooks           | Create/list/update/delete webhooks                                            |
| Views              | Get views for team, space, folder, list                                       |
| Guest              | Invite/update/remove guests                                                   |

#### Formal Schema

ClickUp provides a downloadable OpenAPI specification from their documentation site at https://clickup.com/api/docs/open-api-spec. The specification is maintained alongside their documentation on Readme.io.

**Confidence in schema accuracy**: Medium. Multiple community SDK authors note that the actual API responses sometimes differ from the documented types. The Kendew Agency SDK explicitly warns: "The actual data returned by ClickUp's API may sometimes differ from their official documentation."

#### SDKs

ClickUp does **not** provide official SDKs. However, community-maintained SDKs exist:

| Language               | Package/Library                    | Notes                                                                             |
|------------------------|------------------------------------|-----------------------------------------------------------------------------------|
| **Python**             | `clickupython` (pip)               | Most mature community SDK (65 stars), covers main endpoints. Actively maintained. |
| **TypeScript/Node.js** | `@kendew-agency/clickup-sdk` (npm) | Type-safe SDK with full TypeScript definitions. WIP.                              |
| **TypeScript/Node.js** | `node-clickup` (npm)               | TypeScript SDK by twelvearrays.                                                   |
| **Go**                 | `clickup-client-go`                | Community Go client.                                                              |
| **Go**                 | `clickup-go-client`                | Simple Go client.                                                                 |
| **C#/.NET**            | `clickup-sdk` (modelingevolution)  | Strongly-typed C# SDK.                                                            |
| **C#/.NET**            | `ClickUpApiMcpLib`                 | .NET 9 SDK with MCP integration.                                                  |

#### Authentication Mechanisms

ClickUp supports two authentication approaches:

1. **Personal API Token**

    - Generated from ClickUp settings page (Settings > Apps > API Token)
    - Sent as `Authorization: <token>` header (no "Bearer" prefix required, though Bearer is also accepted)
    - Suitable for personal scripts and integrations acting on behalf of a single user
    - Token format: `pk_XXXXXXXXXX_XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX`

2. **OAuth 2.0 Authorization Code Flow**

    - For building apps that act on behalf of other users
    - Flow: Register app -> Redirect user to ClickUp authorization -> Receive code -> Exchange for access token
    - Token endpoint: `https://api.clickup.com/api/v2/oauth/token`
    - Authorization URL: `https://app.clickup.com/api?client_id=<CLIENT_ID>&redirect_uri=<REDIRECT_URI>`
    - Access tokens expire and can be refreshed
    - Requires registering an app in ClickUp developer settings to obtain `client_id` and `client_secret`

#### Signup Process for Developers

1. Create a free ClickUp account at https://clickup.com/
2. For personal API token: Navigate to Settings > Apps > Generate API Token
3. For OAuth apps: Create an app in ClickUp's developer settings to obtain client credentials
4. No separate developer account or registration is required - API access is available immediately
5. Rate limits apply; Enterprise plans receive higher rate limits

### Webhooks

ClickUp provides outbound webhooks for receiving real-time event notifications.

- **Availability**: Business plan and above
- **Events supported**: Task created, updated, deleted; list created, updated, deleted; folder created, updated, deleted; space updated; and more
- **Configuration**: Per-workspace via API (`POST /team/{team_id}/webhook`)
- **Security**: Webhook signatures for verification
- **Documentation**: https://clickup.com/api/docs/webhooks

### MCP (Model Context Protocol) Server

ClickUp provides an MCP server for AI assistant integration.

- Allows AI tools (Claude, ChatGPT, etc.) to interact with ClickUp workspaces
- Supports reading/writing tasks, managing projects, and more
- Setup instructions: https://clickup.com/api/docs/connect-an-ai-assistant-to-clickups-mcp-server
- MCP tools reference: https://clickup.com/api/docs/mcp-tools

### No WebSocket or JSON-RPC Support

ClickUp does not provide WebSocket or JSON-RPC APIs. Real-time updates are handled via webhooks only.

## Schemas

ClickUp's data model is hierarchical. The following sections describe the key entities and provide representative Rust structs.

### Task

The Task is the central entity in ClickUp. Tasks live inside Lists, which live inside Folders (optional), which live inside Spaces, which live inside Teams/Workspaces.

**Source**: ClickUp OpenAPI spec and community SDK types. **Confidence**: Medium-high -- the overall shape is well-documented, but the OpenAPI spec has been noted to occasionally diverge from actual responses, particularly around custom fields and nested objects.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub custom_id: Option<String>,
    pub name: String,
    pub status: TaskStatus,
    pub orderindex: Option<String>,
    pub date_created: Option<String>,
    pub date_updated: Option<String>,
    pub date_closed: Option<String>,
    pub archived: Option<bool>,
    pub creator: Option<UserBrief>,
    pub assignees: Option<Vec<UserBrief>>,
    pub watchers: Option<Vec<UserBrief>>,
    pub checklists: Option<Vec<Checklist>>,
    pub tags: Option<Vec<Tag>>,
    pub parent: Option<String>,
    pub priority: Option<PriorityValue>,
    pub due_date: Option<String>,
    pub due_date_time: Option<bool>,
    pub start_date: Option<String>,
    pub start_date_time: Option<bool>,
    pub points: Option<String>,
    pub time_estimate: Option<String>,
    pub time_spent: Option<i64>,
    pub custom_fields: Option<Vec<CustomField>>,
    pub list: Option<TaskListRef>,
    pub folder: Option<TaskFolderRef>,
    pub space: Option<TaskSpaceRef>,
    pub url: Option<String>,
    pub description: Option<String>,
    pub dependencies: Option<Vec<Dependency>>,
    pub links: Option<Vec<TaskLink>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatus {
    pub status: String,
    #[serde(rename = "type")]
    pub status_type: String,
    pub color: String,
    pub orderindex: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PriorityValue {
    Integer(i32),
    String(String),
    Null,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBrief {
    pub id: i64,
    pub username: Option<String>,
    pub color: Option<String>,
    pub profile_picture: Option<String>,
    pub initials: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub tag_fg: Option<String>,
    pub tag_bg: Option<String>,
    pub creator: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checklist {
    pub id: String,
    pub name: String,
    pub orderindex: Option<i32>,
    pub resolved: Option<i32>,
    pub unresolved: Option<i32>,
    pub items: Option<Vec<ChecklistItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItem {
    pub id: String,
    pub name: String,
    pub orderindex: Option<i32>,
    pub assignee: Option<UserBrief>,
    pub resolved: Option<bool>,
    pub parent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomField {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(rename = "type_config")]
    pub type_config: Option<serde_json::Value>,
    pub date_created: Option<String>,
    pub hide_from_guests: Option<bool>,
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub task_id: Option<String>,
    pub depends_on: Option<String>,
    pub dependency_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskLink {
    pub task_id: Option<String>,
    pub links_to: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskListRef {
    pub id: String,
    pub name: Option<String>,
    pub access: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFolderRef {
    pub id: String,
    pub name: Option<String>,
    pub hidden: Option<bool>,
    pub access: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSpaceRef {
    pub id: String,
    pub name: Option<String>,
    pub access: Option<bool>,
}
```

### Person / User

ClickUp represents users in a simplified form when embedded in other entities (like task assignees). The full user profile is available through authorization endpoints.

**Source**: OpenAPI spec and community SDK types. **Confidence**: Medium -- the `UserBrief` struct shown above (embedded in tasks) is well-documented. The full authorized user response is less thoroughly documented.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizedUser {
    pub user: UserDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDetail {
    pub id: i64,
    pub username: Option<String>,
    pub email: Option<String>,
    pub color: Option<String>,
    pub profile_picture: Option<String>,
    pub initials: Option<String>,
    pub week_start_day: Option<i32>,
    pub timezone: Option<String>,
}
```

### Company / Organization / Workspace

ClickUp uses the term "Team" for what most other platforms call a workspace or organization. A user can belong to multiple teams.

**Source**: OpenAPI spec. **Confidence**: High -- the Team model is straightforward and consistently documented.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub avatar: Option<String>,
    pub members: Option<Vec<TeamMember>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub user: UserBrief,
    pub invited_by: Option<String>,
    pub userid: Option<String>,
    pub email: Option<String>,
    pub role: Option<String>,
}
```

### Space

Spaces are the top-level organizational unit within a Team. Each Space can have its own features and statuses enabled/disabled.

**Source**: OpenAPI spec and community SDK types. **Confidence**: High.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Space {
    pub id: String,
    pub name: String,
    pub access: Option<bool>,
    pub features: Option<SpaceFeatures>,
    pub statuses: Option<Vec<TaskStatus>>,
    pub color: Option<String>,
    pub orderindex: Option<i32>,
    pub archived: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceFeatures {
    pub due_dates: Option<FeatureConfig>,
    pub time_tracking: Option<FeatureConfig>,
    pub points: Option<FeatureConfig>,
    pub custom_fields: Option<FeatureConfig>,
    pub priorities: Option<FeatureConfig>,
    pub tags: Option<FeatureConfig>,
    pub checklists: Option<FeatureConfig>,
    pub assignments: Option<FeatureConfig>,
    pub comments: Option<FeatureConfig>,
    pub time_estimates: Option<FeatureConfig>,
    pub milestones: Option<FeatureConfig>,
    pub multiple_assignees: Option<FeatureConfig>,
    #[serde(rename = "start_date")]
    pub start_date: Option<FeatureConfig>,
    pub remap_due_dates: Option<FeatureConfig>,
    pub dependency_warning: Option<FeatureConfig>,
    pub folio: Option<FeatureConfig>,
    pub gantt: Option<FeatureConfig>,
    pub rolling_priors: Option<FeatureConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureConfig {
    pub enabled: bool,
}
```

### Workflow

ClickUp does not have a formal "Workflow" entity exposed via the API. Instead, workflow behavior is configured through:

1. **Statuses**: Each Space defines its own set of statuses with types (`open`, `in progress`, `closed`, etc.)
2. **Automations**: Rule-based automation engine (Business plan+) that triggers actions based on status changes, assignments, due dates, etc. Automations are configured via the UI, not the API.
3. **Task Dependencies**: Wait-for and blocking relationships between tasks

**Source**: No API-accessible workflow model. **Confidence**: High that no formal Workflow API entity exists.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Automation {
    pub id: String,
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub trigger: Option<serde_json::Value>,
    pub conditions: Option<Vec<serde_json::Value>>,
    pub actions: Option<Vec<serde_json::Value>>,
}
```

**Note**: The above `Automation` struct is speculative. ClickUp does not expose a public API for managing automations. They can only be created and managed through the UI.

### Status

Statuses in ClickUp are defined at the Space level and control the lifecycle of tasks within that Space.

**Source**: OpenAPI spec and embedded in Task responses. **Confidence**: High.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub status: String,
    #[serde(rename = "type")]
    pub status_type: StatusType,
    pub color: String,
    pub orderindex: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatusType {
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "in progress")]
    InProgress,
    #[serde(rename = "closed")]
    Closed,
    #[serde(rename = "custom")]
    Custom,
}
```

## Gotchas

### Priority Field Type Inconsistency

The `priority` field on tasks is documented as an integer but may be returned as a string or null depending on the context. This is a known issue reported by developers (see Stack Overflow: "Priority is not integer in ClickUp API"). Workaround: deserialize as an enum that handles both `i32` and `String` representations.

### Pagination Limits and Inconsistencies

- Task listings return a maximum of 100 tasks per page
- Comments are limited to 25 per page
- The pagination model uses `page` for tasks and `start`/`start_id` for comments, making it inconsistent across endpoints
- Getting "all" tasks requires iterating through pages; there is no bulk export endpoint
- Tasks with subtasks require explicitly passing `subtasks=true` to include them

### Status Filtering Quirks

Filtering tasks by status is a common source of frustration. The `statuses[]` query parameter must match the exact status name (which is case-sensitive and Space-specific). Closed tasks are excluded by default and require `include_closed=true`. See Stack Overflow question "ClickUp get tasks by status in list" for a detailed example of this issue.

### No Recurring Task Support via API

ClickUp supports recurring tasks in the UI but there is no API endpoint to create or manage recurring task configurations. The recurring task fields are not exposed in the API at all. This is a frequently requested feature in developer feedback.

### Webhooks Require Business Plan

Webhooks are only available on the Business plan ($12/user/month) and above. This is a common surprise for developers building on the Free Forever or Unlimited plans who expect webhook support. Workaround: Use polling with the REST API, though this is subject to rate limiting.

### Rate Limiting (Undocumented Specifics)

ClickUp applies rate limits but does not publish specific rate limit numbers or headers in their documentation. Enterprise customers receive higher limits. Developers report hitting rate limits when performing bulk operations (creating/updating many tasks in sequence). Workaround: Implement exponential backoff and batch operations where possible.

### Custom Fields Complexity

Custom fields have a complex type system with `type_config` objects that vary by field type. The OpenAPI spec does not fully capture all type configurations. Additionally:

- Setting custom field values uses a different endpoint than updating the task itself
- The `value` field in responses can be any JSON type depending on the custom field type
- Relationship and rollup fields have particularly complex value structures

### OAuth Token Refresh Not Documented Well

While OAuth 2.0 is supported, the token refresh flow is not well-documented. Developers report confusion about token expiration times and the refresh token flow. The authorization header accepts both `Authorization: <token>` and `Authorization: Bearer <token>`, which can cause confusion.

### API Documentation vs Actual Responses

Multiple community SDK maintainers report that actual API responses sometimes differ from the documentation. Common discrepancies include:

- Optional fields that are documented as required (and vice versa)
- Nested objects that are flatter or more nested than documented
- Inconsistent null handling (some fields return `null` while others are omitted entirely)
- The `orderindex` field appearing as both string and integer in different contexts

### No Bulk Operations

ClickUp's API does not support true bulk/batch operations for creating or updating multiple tasks in a single request. Developers needing to create or update many tasks must make individual API calls for each one, leading to rate limit issues for large workspaces.

### Custom Task IDs

Custom task IDs (e.g., "PROJ-123") are supported but require passing `custom_task_ids=true` and `team_id` query parameters on relevant endpoints. Not all endpoints support custom task IDs, and mixing custom IDs with standard numeric IDs can cause errors.

### Workspace vs Team Terminology

ClickUp uses "Team" in the API to refer to what the UI calls a "Workspace." The API endpoint is `GET /team` but returns Workspace-level objects. This naming inconsistency causes confusion when mapping API concepts to the UI.

---

## Appendix: Informal JSON Schema Specifications

The following schemas are derived from ClickUp's OpenAPI specification, official webhook payload documentation, and community SDK type definitions. They are provided as informal reference specifications and may not capture every edge case or undocumented field.

**Source confidence**: Medium-High for core entities; Medium for webhook payloads (based on documented examples).

### A.1 REST API Entity Schemas

#### A.1.1 Task

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "ClickUpTask",
  "type": "object",
  "required": ["id", "name", "status"],
  "properties": {
    "id": { "type": "string", "description": "Unique task identifier" },
    "custom_id": { "type": ["string", "null"], "description": "Custom task ID (e.g., PROJ-123)" },
    "name": { "type": "string", "description": "Task name/title" },
    "status": { "$ref": "#/definitions/TaskStatus" },
    "orderindex": { "type": ["string", "integer", "null"], "description": "Sort order (type varies by context)" },
    "date_created": { "type": ["string", "null"], "description": "Unix timestamp in milliseconds as string" },
    "date_updated": { "type": ["string", "null"], "description": "Unix timestamp in milliseconds as string" },
    "date_closed": { "type": ["string", "null"], "description": "Unix timestamp in milliseconds as string" },
    "archived": { "type": ["boolean", "null"] },
    "creator": { "$ref": "#/definitions/UserBrief" },
    "assignees": { "type": ["array", "null"], "items": { "$ref": "#/definitions/UserBrief" } },
    "watchers": { "type": ["array", "null"], "items": { "$ref": "#/definitions/UserBrief" } },
    "checklists": { "type": ["array", "null"], "items": { "$ref": "#/definitions/Checklist" } },
    "tags": { "type": ["array", "null"], "items": { "$ref": "#/definitions/Tag" } },
    "parent": { "type": ["string", "null"], "description": "Parent task ID" },
    "priority": {
      "oneOf": [
        { "type": "integer", "description": "Priority level (1-4)" },
        { "type": "string" },
        { "type": "null" }
      ],
      "description": "Priority field - type varies by context (known inconsistency)"
    },
    "due_date": { "type": ["string", "null"], "description": "Unix timestamp in milliseconds as string" },
    "due_date_time": { "type": ["boolean", "null"], "description": "Whether due_date includes time component" },
    "start_date": { "type": ["string", "null"], "description": "Unix timestamp in milliseconds as string" },
    "start_date_time": { "type": ["boolean", "null"], "description": "Whether start_date includes time component" },
    "points": { "type": ["string", "null"] },
    "time_estimate": { "type": ["string", "null"], "description": "Time estimate in milliseconds as string" },
    "time_spent": { "type": ["integer", "null"], "description": "Time spent in milliseconds" },
    "custom_fields": { "type": ["array", "null"], "items": { "$ref": "#/definitions/CustomField" } },
    "list": { "$ref": "#/definitions/TaskListRef" },
    "folder": { "$ref": "#/definitions/TaskFolderRef" },
    "space": { "$ref": "#/definitions/TaskSpaceRef" },
    "url": { "type": ["string", "null"], "format": "uri" },
    "description": { "type": ["string", "null"], "description": "Task description (Quill Delta JSON or plain text)" },
    "dependencies": { "type": ["array", "null"], "items": { "$ref": "#/definitions/Dependency" } },
    "links": { "type": ["array", "null"], "items": { "$ref": "#/definitions/TaskLink" } }
  },
  "definitions": {
    "TaskStatus": {
      "type": "object",
      "required": ["status", "type", "color"],
      "properties": {
        "status": { "type": "string", "description": "Status name" },
        "type": { "type": "string", "enum": ["open", "in progress", "closed", "custom", "done", "complete"], "description": "Status category" },
        "color": { "type": "string", "description": "Hex color code" },
        "orderindex": { "type": ["integer", "null"] }
      }
    },
    "UserBrief": {
      "type": "object",
      "required": ["id"],
      "properties": {
        "id": { "type": "integer", "description": "User ID" },
        "username": { "type": ["string", "null"] },
        "color": { "type": ["string", "null"], "description": "Hex color code" },
        "profile_picture": { "type": ["string", "null"], "format": "uri" },
        "initials": { "type": ["string", "null"] },
        "email": { "type": ["string", "null"], "format": "email" }
      }
    },
    "Tag": {
      "type": "object",
      "required": ["name"],
      "properties": {
        "name": { "type": "string" },
        "tag_fg": { "type": ["string", "null"], "description": "Foreground hex color" },
        "tag_bg": { "type": ["string", "null"], "description": "Background hex color" },
        "creator": { "type": ["integer", "null"], "description": "User ID of tag creator" }
      }
    },
    "Checklist": {
      "type": "object",
      "required": ["id", "name"],
      "properties": {
        "id": { "type": "string" },
        "name": { "type": "string" },
        "orderindex": { "type": ["integer", "null"] },
        "resolved": { "type": ["integer", "null"], "description": "Number of resolved items" },
        "unresolved": { "type": ["integer", "null"], "description": "Number of unresolved items" },
        "items": { "type": ["array", "null"], "items": { "$ref": "#/definitions/ChecklistItem" } }
      }
    },
    "ChecklistItem": {
      "type": "object",
      "required": ["id", "name"],
      "properties": {
        "id": { "type": "string" },
        "name": { "type": "string" },
        "orderindex": { "type": ["integer", "null"] },
        "assignee": { "$ref": "#/definitions/UserBrief" },
        "resolved": { "type": ["boolean", "null"] },
        "parent": { "type": ["string", "null"] }
      }
    },
    "CustomField": {
      "type": "object",
      "required": ["id", "name", "type"],
      "properties": {
        "id": { "type": "string" },
        "name": { "type": "string" },
        "type": {
          "type": "string",
          "enum": [
            "text", "drop_down", "labels", "date", "number", "currency",
            "checkbox", "url", "users", "email", "phone", "formula",
            "relationship", "rollup", "emoji", "automatic_progress",
            "manual_progress", "short_text", "attachment"
          ],
          "description": "Custom field type"
        },
        "type_config": { "type": ["object", "null"], "description": "Type-specific configuration (varies by field type)" },
        "date_created": { "type": ["string", "null"] },
        "hide_from_guests": { "type": ["boolean", "null"] },
        "value": { "description": "Field value - type varies by custom field type" },
        "required": { "type": ["boolean", "null"] },
        "deleted": { "type": ["boolean", "null"] },
        "pinned": { "type": ["boolean", "null"] }
      }
    },
    "Dependency": {
      "type": "object",
      "properties": {
        "task_id": { "type": ["string", "null"] },
        "depends_on": { "type": ["string", "null"], "description": "Task ID that this task depends on" },
        "dependency_type": { "type": ["string", "null"], "enum": ["waiting_on", "blocking", "link_to"] }
      }
    },
    "TaskLink": {
      "type": "object",
      "properties": {
        "task_id": { "type": ["string", "null"] },
        "links_to": { "type": ["string", "null"], "description": "Linked task ID" }
      }
    },
    "TaskListRef": {
      "type": "object",
      "required": ["id"],
      "properties": {
        "id": { "type": "string" },
        "name": { "type": ["string", "null"] },
        "access": { "type": ["boolean", "null"] }
      }
    },
    "TaskFolderRef": {
      "type": "object",
      "required": ["id"],
      "properties": {
        "id": { "type": "string" },
        "name": { "type": ["string", "null"] },
        "hidden": { "type": ["boolean", "null"] },
        "access": { "type": ["boolean", "null"] }
      }
    },
    "TaskSpaceRef": {
      "type": "object",
      "required": ["id"],
      "properties": {
        "id": { "type": "string" },
        "name": { "type": ["string", "null"] },
        "access": { "type": ["boolean", "null"] }
      }
    }
  }
}
```

#### A.1.2 Team (Workspace)

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "ClickUpTeam",
  "type": "object",
  "required": ["id", "name"],
  "properties": {
    "id": { "type": "string" },
    "name": { "type": "string" },
    "color": { "type": ["string", "null"], "description": "Hex color code" },
    "avatar": { "type": ["string", "null"], "format": "uri" },
    "members": {
      "type": ["array", "null"],
      "items": {
        "type": "object",
        "properties": {
          "user": { "$ref": "#/definitions/UserBrief" },
          "invited_by": { "type": ["string", "null"] },
          "userid": { "type": ["string", "null"] },
          "email": { "type": ["string", "null"], "format": "email" },
          "role": { "type": ["string", "null"], "enum": ["owner", "admin", "member", "guest", "viewer"] }
        }
      }
    }
  },
  "definitions": {
    "UserBrief": {
      "type": "object",
      "required": ["id"],
      "properties": {
        "id": { "type": "integer" },
        "username": { "type": ["string", "null"] },
        "color": { "type": ["string", "null"] },
        "profile_picture": { "type": ["string", "null"], "format": "uri" },
        "initials": { "type": ["string", "null"] },
        "email": { "type": ["string", "null"], "format": "email" }
      }
    }
  }
}
```

#### A.1.3 Space

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "ClickUpSpace",
  "type": "object",
  "required": ["id", "name"],
  "properties": {
    "id": { "type": "string" },
    "name": { "type": "string" },
    "access": { "type": ["boolean", "null"] },
    "features": {
      "type": ["object", "null"],
      "properties": {
        "due_dates": { "$ref": "#/definitions/FeatureConfig" },
        "time_tracking": { "$ref": "#/definitions/FeatureConfig" },
        "points": { "$ref": "#/definitions/FeatureConfig" },
        "custom_fields": { "$ref": "#/definitions/FeatureConfig" },
        "priorities": { "$ref": "#/definitions/FeatureConfig" },
        "tags": { "$ref": "#/definitions/FeatureConfig" },
        "checklists": { "$ref": "#/definitions/FeatureConfig" },
        "assignments": { "$ref": "#/definitions/FeatureConfig" },
        "comments": { "$ref": "#/definitions/FeatureConfig" },
        "time_estimates": { "$ref": "#/definitions/FeatureConfig" },
        "milestones": { "$ref": "#/definitions/FeatureConfig" },
        "multiple_assignees": { "$ref": "#/definitions/FeatureConfig" },
        "start_date": { "$ref": "#/definitions/FeatureConfig" },
        "remap_due_dates": { "$ref": "#/definitions/FeatureConfig" },
        "dependency_warning": { "$ref": "#/definitions/FeatureConfig" },
        "folio": { "$ref": "#/definitions/FeatureConfig" },
        "gantt": { "$ref": "#/definitions/FeatureConfig" },
        "rolling_priors": { "$ref": "#/definitions/FeatureConfig" }
      }
    },
    "statuses": {
      "type": ["array", "null"],
      "items": {
        "type": "object",
        "required": ["status", "type", "color"],
        "properties": {
          "status": { "type": "string" },
          "type": { "type": "string", "enum": ["open", "in progress", "closed", "custom", "done", "complete"] },
          "color": { "type": "string" },
          "orderindex": { "type": ["integer", "null"] }
        }
      }
    },
    "color": { "type": ["string", "null"], "description": "Hex color code" },
    "orderindex": { "type": ["integer", "null"] },
    "archived": { "type": ["boolean", "null"] }
  },
  "definitions": {
    "FeatureConfig": {
      "type": "object",
      "required": ["enabled"],
      "properties": {
        "enabled": { "type": "boolean" }
      }
    }
  }
}
```

#### A.1.4 Authorized User

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "ClickUpAuthorizedUser",
  "type": "object",
  "required": ["user"],
  "properties": {
    "user": {
      "type": "object",
      "required": ["id"],
      "properties": {
        "id": { "type": "integer" },
        "username": { "type": ["string", "null"] },
        "email": { "type": ["string", "null"], "format": "email" },
        "color": { "type": ["string", "null"], "description": "Hex color code" },
        "profile_picture": { "type": ["string", "null"], "format": "uri" },
        "initials": { "type": ["string", "null"] },
        "week_start_day": { "type": ["integer", "null"], "description": "0=Sunday, 1=Monday, etc." },
        "timezone": { "type": ["string", "null"] }
      }
    }
  }
}
```

### A.2 Webhook Payload Schemas

All webhook payloads share a common envelope structure. Each payload includes:
- `event`: The event type string
- `webhook_id`: The webhook registration UUID
- Resource-specific ID field(s)
- Optional `history_items` array describing the change

#### A.2.1 Common Webhook Envelope

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "ClickUpWebhookEnvelope",
  "type": "object",
  "required": ["event", "webhook_id"],
  "properties": {
    "event": {
      "type": "string",
      "enum": [
        "taskCreated", "taskUpdated", "taskDeleted", "taskPriorityUpdated",
        "taskStatusUpdated", "taskAssigneeUpdated", "taskDueDateUpdated",
        "taskTagUpdated", "taskMoved", "taskCommentPosted", "taskCommentUpdated",
        "taskTimeEstimateUpdated", "taskTimeTrackedUpdated",
        "listCreated", "listUpdated", "listDeleted",
        "folderCreated", "folderUpdated", "folderDeleted",
        "spaceCreated", "spaceUpdated", "spaceDeleted",
        "goalCreated", "goalUpdated", "goalDeleted",
        "keyResultCreated", "keyResultUpdated", "keyResultDeleted"
      ]
    },
    "webhook_id": { "type": "string", "format": "uuid" },
    "history_items": {
      "type": ["array", "null"],
      "items": { "$ref": "#/definitions/HistoryItem" }
    }
  },
  "definitions": {
    "HistoryItem": {
      "type": "object",
      "required": ["id", "type", "date", "field", "parent_id"],
      "properties": {
        "id": { "type": "string", "description": "Unique history item ID" },
        "type": { "type": "integer", "description": "History item type code" },
        "date": { "type": "string", "description": "Unix timestamp in milliseconds" },
        "field": { "type": "string", "description": "Field that changed (e.g., 'status', 'name', 'content')" },
        "parent_id": { "type": "string", "description": "Parent resource ID (e.g., List ID for task events)" },
        "data": { "type": ["object", "null"], "description": "Event-specific metadata" },
        "source": { "type": ["string", "null"], "description": "Source of the change" },
        "user": { "$ref": "#/definitions/WebhookUser" },
        "before": { "description": "Value before change (type varies by field)" },
        "after": { "description": "Value after change (type varies by field)" }
      }
    },
    "WebhookUser": {
      "type": "object",
      "required": ["id", "username", "email", "color", "initials"],
      "properties": {
        "id": { "type": "integer", "description": "User ID (integer, not string)" },
        "username": { "type": "string" },
        "email": { "type": "string", "format": "email" },
        "color": { "type": "string", "description": "Hex color code" },
        "initials": { "type": "string" },
        "profilePicture": { "type": ["string", "null"], "format": "uri" }
      }
    }
  }
}
```

#### A.2.2 Task Webhook Payloads

Task webhooks extend the envelope with a `task_id` field. The `history_items` array describes what changed.

**Task Created / Status Updated** (`taskCreated`, `taskStatusUpdated`):

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "ClickUpTaskWebhook",
  "allOf": [{ "$ref": "#/definitions/WebhookEnvelope" }],
  "required": ["event", "task_id", "webhook_id"],
  "properties": {
    "task_id": { "type": "string" },
    "data": {
      "type": ["object", "null"],
      "properties": {
        "description": { "type": "string" },
        "interval_id": { "type": "string" }
      }
    }
  },
  "definitions": {
    "WebhookEnvelope": {
      "type": "object",
      "required": ["event", "webhook_id"],
      "properties": {
        "event": { "type": "string" },
        "webhook_id": { "type": "string", "format": "uuid" },
        "history_items": {
          "type": ["array", "null"],
          "items": {
            "type": "object",
            "properties": {
              "id": { "type": "string" },
              "type": { "type": "integer" },
              "date": { "type": "string" },
              "field": { "type": "string" },
              "parent_id": { "type": "string" },
              "data": { "type": ["object", "null"] },
              "source": { "type": ["string", "null"] },
              "user": {
                "type": "object",
                "properties": {
                  "id": { "type": "integer" },
                  "username": { "type": "string" },
                  "email": { "type": "string" },
                  "color": { "type": "string" },
                  "initials": { "type": "string" },
                  "profilePicture": { "type": ["string", "null"] }
                }
              },
              "before": { "description": "Value before change" },
              "after": { "description": "Value after change" }
            }
          }
        }
      }
    }
  }
}
```

**Key history_item field patterns by event type:**

| Event | `field` | `before` / `after` Type |
|-------|---------|------------------------|
| `taskCreated` | `"status"`, `"task_creation"` | Status object / null |
| `taskUpdated` (content) | `"content"` | null / Quill Delta JSON string |
| `taskUpdated` (custom field) | `"custom_field"` | null / value + `custom_field` metadata |
| `taskDeleted` | (none - no history_items) | - |
| `taskPriorityUpdated` | `"priority"` | null / Priority object |
| `taskStatusUpdated` | `"status"` | Status object / Status object |
| `taskAssigneeUpdated` | `"assignee_add"` / `"assignee_remove"` | null / UserBrief object |
| `taskDueDateUpdated` | `"due_date"` | Timestamp string / Timestamp string |
| `taskTagUpdated` | `"tag"` | null / Array of Tag objects |
| `taskMoved` | `"section_moved"` | ListRef object / ListRef object |
| `taskCommentPosted` | `"comment"` | null / Comment ID string |
| `taskCommentUpdated` | `"comment"` | null / Comment ID string |
| `taskTimeEstimateUpdated` | `"time_estimate"` | null / Time string |
| `taskTimeTrackedUpdated` | `"time_spent"` | null / Time entry object |

#### A.2.3 List Webhook Payloads

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "ClickUpListWebhook",
  "type": "object",
  "required": ["event", "list_id", "webhook_id"],
  "properties": {
    "event": { "type": "string", "enum": ["listCreated", "listUpdated", "listDeleted"] },
    "list_id": { "type": "string" },
    "webhook_id": { "type": "string", "format": "uuid" },
    "history_items": {
      "type": ["array", "null"],
      "items": {
        "type": "object",
        "properties": {
          "id": { "type": "string" },
          "type": { "type": "integer" },
          "date": { "type": "string" },
          "field": { "type": "string" },
          "parent_id": { "type": "string" },
          "data": { "type": ["object", "null"] },
          "source": { "type": ["string", "null"] },
          "user": {
            "type": "object",
            "properties": {
              "id": { "type": "integer" },
              "username": { "type": "string" },
              "email": { "type": "string" },
              "color": { "type": "string" },
              "initials": { "type": "string" },
              "profilePicture": { "type": ["string", "null"] }
            }
          },
          "before": { "description": "Value before change" },
          "after": { "description": "Value after change" }
        }
      }
    }
  }
}
```

#### A.2.4 Folder Webhook Payloads

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "ClickUpFolderWebhook",
  "type": "object",
  "required": ["event", "folder_id", "webhook_id"],
  "properties": {
    "event": { "type": "string", "enum": ["folderCreated", "folderUpdated", "folderDeleted"] },
    "folder_id": { "type": "string" },
    "webhook_id": { "type": "string", "format": "uuid" }
  }
}
```

#### A.2.5 Space Webhook Payloads

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "ClickUpSpaceWebhook",
  "type": "object",
  "required": ["event", "space_id", "webhook_id"],
  "properties": {
    "event": { "type": "string", "enum": ["spaceCreated", "spaceUpdated", "spaceDeleted"] },
    "space_id": { "type": "string" },
    "webhook_id": { "type": "string", "format": "uuid" }
  }
}
```

#### A.2.6 Goal and Key Result Webhook Payloads

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "ClickUpGoalWebhook",
  "type": "object",
  "required": ["event", "goal_id", "webhook_id"],
  "properties": {
    "event": { "type": "string", "enum": ["goalCreated", "goalUpdated", "goalDeleted"] },
    "goal_id": { "type": "string", "format": "uuid" },
    "webhook_id": { "type": "string", "format": "uuid" }
  }
}
```

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "ClickUpKeyResultWebhook",
  "type": "object",
  "required": ["event", "goal_id", "key_result_id", "webhook_id"],
  "properties": {
    "event": { "type": "string", "enum": ["keyResultCreated", "keyResultUpdated", "keyResultDeleted"] },
    "goal_id": { "type": "string", "format": "uuid" },
    "key_result_id": { "type": "string", "format": "uuid" },
    "webhook_id": { "type": "string", "format": "uuid" }
  }
}
```

### A.3 Schema Notes and Caveats

1. **Type Inconsistencies**: The `orderindex` field appears as both string and integer across different endpoints. The `priority` field may be integer, string, or null.

2. **Custom Fields**: The `type_config` and `value` fields vary significantly by custom field type. The OpenAPI spec does not fully capture all variations.

3. **Webhook `history_items`**: The `before` and `after` fields can be any JSON type depending on the field being changed. They may be null, strings, objects, arrays, or numbers.

4. **User ID Typing**: In API responses, `user.id` is typically an integer. In webhook payloads, `history_items[x].user.id` is also an integer (not string as some documentation suggests).

5. **Date Formats**: All timestamps are Unix timestamps in milliseconds, represented as strings in most contexts.

6. **Webhook Cascading**: Some operations trigger multiple webhooks. For example, creating a task triggers both `taskCreated` and `taskStatusUpdated`. Moving a task triggers `taskMoved`, `taskUpdated`, and `spaceUpdated`.

7. **Idempotency**: Use `{{webhook_id}}:{{history_item_id}}` as an idempotency key to deduplicate events.

8. **Signature Verification**: Each webhook event includes a signature header for verification. The shared secret is returned when the webhook is created via `POST /team/{team_id}/webhook`.
