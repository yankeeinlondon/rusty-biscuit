# Research into Asana's API Support

## Overview on Product

Asana is a cloud-based work management and collaboration platform designed to help teams organize, track, and manage their work. It centers around the concept of the "Work Graph" -- a flexible system where any piece of work can have one-to-many relationships with other objects.

### Functional Footprint

- **Task Management**: Tasks are the basic unit of action. Tasks support assignees, due dates, dependencies, subtasks (up to 5 levels deep), custom fields, attachments, comments (called "stories"), and followers. Tasks can be multi-homed across multiple projects.
- **Project Management**: Projects are collections of tasks viewable as lists, boards, timelines (Gantt), and calendars. Projects support sections, custom fields, custom templates, and status updates.
- **Portfolio Management**: Collections of projects for high-level visibility across initiatives. Supports custom fields at the portfolio level.
- **Goals**: OKR-style goal tracking at company, team, and individual levels. Goals can be linked to projects and tasks for automatic progress tracking.
- **Resource Management**: Workload views, time tracking, capacity planning (Enterprise tier). Timesheets and budgets available as add-ons.
- **Workflow Automation**: Rules-based automation engine (triggers and actions), forms with branching logic, custom task types, workflow bundles (Enterprise).
- **Reporting**: Dashboards, charts, advanced search, universal reporting, formula custom fields, exports to CSV/PDF/PowerPoint.
- **AI Features**: AI Studio for no-code workflow building, smart chat, smart summaries, smart status, risk reports.
- **Collaboration**: Comments, messaging, @mentions, approvals, proofing, guest access.
- **Admin & Security**: Admin console, SAML SSO, SCIM provisioning, audit logs, service accounts, view-only licenses, custom roles (Enterprise).

### Key URLs

| Resource                    | URL                                                       |
|-----------------------------|-----------------------------------------------------------|
| Developer Documentation     | https://developers.asana.com/docs/overview                |
| API Reference               | https://developers.asana.com/reference/rest-api-reference |
| Developer Sandbox Request   | https://developers.asana.com/docs/developer-sandbox       |
| API Explorer                | https://developers.asana.com/docs/api-explorer            |
| Developer Console (My Apps) | https://app.asana.com/0/my-apps                           |
| App Directory               | https://asana.com/apps                                    |
| Developer Forum             | https://forum.asana.com/c/developersapi/24                |
| Changelog                   | https://developers.asana.com/docs/change-log              |
| Postman Collection          | https://developers.asana.com/docs/postman-collection      |
| GitHub Examples             | https://github.com/Asana/devrel-examples/                 |
| Node.js SDK                 | https://developers.asana.com/docs/javascript              |
| Python SDK                  | https://developers.asana.com/docs/python                  |
| MCP Server Docs             | https://developers.asana.com/docs/mcp-server              |
| Product (Main Site)         | https://asana.com                                         |
| Pricing                     | https://asana.com/pricing                                 |
| Help Center                 | https://help.asana.com                                    |

### Pricing Structure

As of 2026, Asana offers five pricing tiers:

| Plan            | Price (Annual Billing)       | Key API Implications                                                              |
|-----------------|------------------------------|-----------------------------------------------------------------------------------|
| **Personal**    | Free forever (up to 2 users) | API access with PAT; 150 req/min rate limit                                       |
| **Starter**     | $10.99/user/month            | 1500 req/min rate limit; custom fields, forms, automations, timeline/Gantt views  |
| **Advanced**    | $24.99/user/month            | Portfolios, goals, workload, approvals/proofing, time tracking, formulas          |
| **Enterprise**  | Contact sales                | SAML, SCIM, service accounts, audit log API, view-only licenses, workflow bundles |
| **Enterprise+** | Contact sales                | Organization exports, SIEM integration, data residency, managed workspaces        |

Notable API-impacting pricing differences:

- **Free domains** are rate-limited to 150 requests/minute vs. 1500 requests/minute for paid domains.
- **SCIM endpoints** require Enterprise tier with Service Accounts.
- **Audit Log API** requires Enterprise tier.
- **Organization Exports** require Enterprise+ tier.
- **View-only license users** have limited API access.

---

## API Details

### REST API (Primary)

Asana's primary API is a **RESTful JSON API** hosted at `https://app.asana.com/api/1.0/`.

- **Base URL**: `https://app.asana.com/api/1.0/`
- **Protocol**: HTTPS only, JSON request/response bodies
- **Version**: Currently v1.0 (versioned in URL path)
- **Pagination**: Offset-based pagination using `offset` and `limit` query parameters. Default page size is 100, max is 100. Also supports `opt_fields` for sparse fieldsets.
- **Batch API**: Supports batch requests via `POST /batch` to execute multiple operations in a single HTTP request (max 10 actions per batch).

#### Formal Schema

Asana **does not publish a formal OpenAPI/Swagger specification file** for download. The API reference is rendered via ReadMe.io's interactive documentation system. There is no downloadable OpenAPI JSON/YAML file.

However:

- A **Postman collection** is available for import: https://developers.asana.com/docs/postman-collection
- The API reference documentation includes inline schema descriptions for each endpoint's request body and response.
- The `opt_fields` parameter system lets callers request specific fields, and the documentation lists all available fields per resource.

#### SDKs

Asana provides official client libraries:

| Language               | Status             | Notes                                                                               |
|------------------------|--------------------|-------------------------------------------------------------------------------------|
| **JavaScript/Node.js** | Actively supported | v3 SDK (recent migration from v2). Handles rate limiting and retries automatically. |
| **Python**             | Actively supported | v5 SDK (recent migration from v4). Handles rate limiting and retries automatically. |
| **Ruby**               | End-of-support     | No longer receiving updates.                                                        |
| **Java**               | End-of-support     | No longer receiving updates.                                                        |
| **PHP**                | End-of-support     | No longer receiving updates.                                                        |

Additionally:

- **MCP Server**: Asana provides an official Model Context Protocol (MCP) server (V2 now GA) for integration with AI coding tools like Claude Code, Cursor, and Windsurf. See https://developers.asana.com/docs/mcp-server.

#### Authentication Mechanisms

Asana supports four authentication mechanisms:

1. **Personal Access Token (PAT)**: Long-lived bearer tokens. Simplest method. Grants the same permissions as the user who generated the token. Best for scripts and single-user apps. Generated in the Developer Console at https://app.asana.com/0/my-apps.
2. **Service Account (SA)**: Enterprise-only. Long-lived tokens with org-wide data access (including private user data). Created by super admins in the admin console. Required for audit log, organization exports, and SCIM endpoints. Supports configurable scopes.
3. **OAuth 2.0**: Standard OAuth flow for multi-user applications. Apps start private by default and can be shared with specific organizations or published to the App Directory. Supports granular scopes (e.g., `tasks:read`, `tasks:write`, `projects:read`, etc.).
4. **OpenID Connect (OIDC)**: Layer on top of OAuth 2.0 for single sign-on. Allows users to log into third-party apps using their Asana account.

All authenticated requests use a `Bearer` token in the `Authorization` header:

```text
Authorization: Bearer <token>
```

#### Signup Process for Developers

1. Create a free Asana account at https://asana.com/create-account (or request a developer sandbox at https://developers.asana.com/docs/developer-sandbox for premium feature testing).
2. Go to the Developer Console at https://app.asana.com/0/my-apps.
3. Create a new app or generate a Personal Access Token.
4. For OAuth apps, configure redirect URIs and scopes.
5. No approval is needed to start using the API. Publishing to the App Directory requires a review process.

#### Rate Limits

| Limit Type                            | Free Domain  | Paid Domain  |
|---------------------------------------|--------------|--------------|
| Standard (requests/min)               | 150          | 1,500        |
| Search API (requests/min)             | 60           | 60           |
| Concurrent GET                        | 50           | 50           |
| Concurrent POST/PUT/PATCH/DELETE      | 15           | 15           |
| Duplication/instantiation/export jobs | 5 concurrent | 5 concurrent |
| Webhooks per resource                 | 1,000        | 1,000        |
| Webhooks per token                    | 10,000       | 10,000       |

Rate-limited responses return `429 Too Many Requests` with a `Retry-After` header. The official SDKs handle retries automatically.

A "cost limit" also applies -- an additional quota based on the computational cost of requests (graph traversal depth). This is evaluated dynamically and affects only extremely expensive queries.

### Webhooks (Push)

Asana supports **webhooks** for push-based event notifications.

- Events are delivered via HTTP `POST` to a user-specified target URL.
- A **handshake** process is required when establishing a webhook (Asana sends `X-Hook-Secret`, server must echo it back).
- Webhook payloads are signed with **HMAC-SHA256** (`X-Hook-Signature` header) for verification.
- Events are "compact" -- they contain only basic details (GID, resource type, action). Integrations must make additional API calls to get full resource state.
- **Filtering** is supported: you can specify `resource_type`, `action`, and `fields` filters. Higher-level webhooks (workspace, portfolio, team, goal) require filters.
- Events propagate upward (e.g., task changes propagate to parent project webhooks).
- **Heartbeat** events are sent every 8 hours. If no response after 24 hours, the webhook is deleted.
- Failed deliveries are retried with exponential backoff for up to 24 hours.
- At-most-once delivery -- no replay, no strong guarantees. A fallback polling mechanism is recommended for critical integrations.
- Average delivery time is under 1 minute; most events arrive within 10 minutes.

### Events API (Poll)

An alternative to webhooks, the **Events API** (`GET /events`) allows polling for changes on a specific resource. This is simpler to implement (no publicly accessible server required) but requires repeated polling.

- Events are served from the same infrastructure as webhooks.
- An event stream is tied to a specific resource GID.

### SCIM 2.0 API (Enterprise Only)

Asana supports **SCIM 2.0** for user and group provisioning at `https://app.asana.com/api/1.0/scim`.

- Only accessible via Service Accounts in Enterprise domains.
- Supports full CRUD for Users and Groups (mapped to Asana teams).
- Supports enterprise extensions (department, cost center, organization, division, employee number, manager).
- Group endpoints map to Asana teams.

### MCP Server (Model Context Protocol)

Asana provides an **official MCP server** for integration with AI coding assistants.

- V2 server is now generally available (GA).
- Supports tools like `create_tasks`, `update_tasks`, search, and more.
- Compatible with Claude Code, Cursor, Windsurf, and other MCP clients.
- See https://developers.asana.com/docs/mcp-tools-reference for the full tools reference.

### APIs Not Supported

- **WebSocket**: Asana does not offer a WebSocket API.
- **JSON-RPC**: Not supported.
- **GraphQL**: Not supported. All queries go through the REST API.
- **gRPC**: Not supported.

---

## Schemas

### Task (Todo / Action)

The Task is the fundamental unit of work in Asana. It is the most richly modeled entity in the API.

**Source**: Asana API Reference (https://developers.asana.com/reference/tasks) and Object Hierarchy documentation. **Confidence**: High -- the Task schema is well-documented and extensively used in examples and quick-start guides.

```rust
use serde::{Deserialize, Serialize};
use chrono::NaiveDate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsanaTask {
    pub gid: String,
    pub resource_type: String,
    pub name: String,
    pub resource_subtype: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_status: Option<ApprovalStatus>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<AsanaUserCompact>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee_section: Option<AsanaSectionCompact>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee_status: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_by: Option<AsanaUserCompact>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_fields: Option<Vec<AsanaCustomField>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<AsanaTaskCompact>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependents: Option<Vec<AsanaTaskCompact>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_at: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_on: Option<NaiveDate>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub external: Option<ExternalData>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub followers: Option<Vec<AsanaUserCompact>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub hearted: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub hearts: Option<Vec<AsanaHeart>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_notes: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_rendered_as_separator: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub liked: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub likes: Option<Vec<AsanaLike>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub memberships: Option<Vec<TaskMembership>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_hearts: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_likes: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_subtasks: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<AsanaTaskCompact>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub permalink_url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub projects: Option<Vec<AsanaProjectCompact>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_at: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_on: Option<NaiveDate>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<AsanaTagCompact>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<AsanaWorkspaceCompact>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_time_minutes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalStatus {
    pub gid: String,
    pub resource_type: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMembership {
    pub project: AsanaProjectCompact,
    pub section: AsanaSectionCompact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsanaTaskCompact {
    pub gid: String,
    pub resource_type: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_subtype: Option<String>,
}
```

### Person / Contact (User)

In Asana, users are the people who have accounts. Users can be full members or guests (limited access). The User object is relatively compact.

**Source**: Asana API Reference (https://developers.asana.com/reference/users) and SCIM documentation. **Confidence**: High -- the User schema is well-documented in both the REST API and SCIM API references.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsanaUser {
    pub gid: String,
    pub resource_type: String,
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub photo: Option<AsanaPhoto>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspaces: Option<Vec<AsanaWorkspaceCompact>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<AsanaWorkspaceCompact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsanaUserCompact {
    pub gid: String,
    pub resource_type: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsanaPhoto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_1024x1024: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_128x128: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_21x21: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_27x27: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_36x36: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_60x60: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsanaWorkspaceCompact {
    pub gid: String,
    pub resource_type: String,
    pub name: String,
}
```

### Company / Organization

In Asana, the highest-level organizational unit is a **Workspace** (for individuals/small teams) or an **Organization** (a special workspace tied to a company email domain). Organizations contain **Teams** which contain **Projects**.

**Source**: Asana Object Hierarchy documentation and API reference. **Confidence**: Medium -- the Organization/Workspace model is conceptually documented but the Organization type itself does not have many unique fields beyond what a Workspace has. The schema below represents the combined view.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsanaWorkspace {
    pub gid: String,
    pub resource_type: String,
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_domains: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_organization: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<AsanaOrganizationCompact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsanaOrganizationCompact {
    pub gid: String,
    pub resource_type: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsanaTeam {
    pub gid: String,
    pub resource_type: String,
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<AsanaOrganizationCompact>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub permalink_url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
}
```

### Workflow

Asana does not have a single "Workflow" entity in its API. Instead, Asana models workflow concepts through several related constructs:

- **Rules**: Automation rules with triggers and actions (not directly exposed as a standalone API resource -- configured through the Asana UI and via rules-related API endpoints).
- **Project Templates**: Reusable project structures that can be instantiated. Available via `/project_templates` endpoints.
- **Custom Fields**: Typed fields (text, number, enum, multi-enum, date, people) that can be attached to projects and portfolios for structured data.
- **Sections**: Groupings within a project that often represent workflow stages (e.g., "To Do", "In Progress", "Done").
- **Task Dependencies**: Direct relationships between tasks indicating execution order.
- **Multi-home**: Tasks can exist in multiple projects simultaneously, enabling cross-project workflow visibility.

**Source**: Asana Object Hierarchy documentation, Custom Fields guide, and API reference. **Confidence**: Low for a unified "Workflow" entity -- Asana deliberately decomposes workflow into multiple primitives. The struct below models a project template as the closest analog to a reusable workflow definition.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsanaProjectTemplate {
    pub gid: String,
    pub resource_type: String,
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub public: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<AsanaUserCompact>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<AsanaTeam>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<AsanaWorkspaceCompact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsanaSection {
    pub gid: String,
    pub resource_type: String,
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<AsanaProjectCompact>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_on: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsanaCustomField {
    pub gid: String,
    pub resource_type: String,
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_subtype: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_options: Option<Vec<AsanaEnumOption>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub number_value: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_value: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_value: Option<AsanaEnumOption>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub multi_enum_values: Option<Vec<AsanaEnumOption>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_value: Option<AsanaCustomFieldDate>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub people_value: Option<Vec<AsanaUserCompact>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsanaEnumOption {
    pub gid: String,
    pub resource_type: String,
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsanaCustomFieldDate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_time: Option<String>,
}
```

### Status

Asana models "status" in multiple ways, depending on context:

- **Task Completion**: Each task has a boolean `completed` field and a `resource_subtype` that can indicate approval status.
- **Project Status Updates**: Projects can have status updates (rich text updates with color-coded status: green/yellow/red). Available via `/project_statuses` endpoints.
- **Custom Fields**: Many teams use custom enum fields (e.g., "Status: Not Started / In Progress / Done") attached to projects.
- **Portfolio Status**: Portfolios can display status information from their constituent projects.

**Source**: Asana API Reference for project statuses, custom fields documentation. **Confidence**: High for project status updates; Medium for the general "status" concept since it is distributed across multiple features.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsanaProjectStatus {
    pub gid: String,
    pub resource_type: String,
    pub title: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<AsanaUserCompact>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ProjectStatusColor>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<AsanaUserCompact>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<AsanaProjectCompact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectStatusColor {
    Green,
    Yellow,
    Red,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsanaProjectCompact {
    pub gid: String,
    pub resource_type: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsanaSectionCompact {
    pub gid: String,
    pub resource_type: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsanaTagCompact {
    pub gid: String,
    pub resource_type: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsanaHeart {
    pub gid: String,
    pub user: AsanaUserCompact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsanaLike {
    pub gid: String,
    pub user: AsanaUserCompact,
}
```

---

## Gotchas

### Rate Limiting Complexity

Asana enforces **three independent layers** of rate limiting:

1. **Standard rate limits** (150/min free, 1500/min paid) -- straightforward but note that free tiers are extremely limited for any real integration.
2. **Concurrent request limits** (50 GET, 15 write) -- easy to hit when performing bulk operations.
3. **Cost limits** -- a hidden, dynamically-evaluated quota based on graph traversal complexity. This is not documented with specific numbers and can cause unexpected `429` responses even when you're within the standard rate limit. The cost is calculated *after* the response is built, so the request that causes a negative quota still succeeds, but the *next* request will fail.

**Workaround**: Use `opt_fields` to request only the fields you need (reducing graph traversal). Use the official SDKs which handle retry automatically. Implement exponential backoff and respect the `Retry-After` header.

### No OpenAPI Schema

Asana does not provide a downloadable OpenAPI/Swagger specification. The API reference is rendered via ReadMe.io. This makes it harder to auto-generate typed clients.

**Workaround**: Use the Postman collection as a semi-structured reference, or build schemas manually from the API reference documentation.

### Subtasks Do Not Inherit Project Membership

Subtasks do **not** inherit the projects of their parent tasks. This is a frequently misunderstood behavior. A subtask will only appear in a project if it is explicitly added to one.

**Workaround**: When creating subtasks that should appear in the same project as the parent, explicitly set the project membership on the subtask.

### No Way to Fetch All Subtasks for All Tasks in a Project

There is no single API call to retrieve all subtasks of all tasks in a project. You must iterate over each task and fetch its subtasks individually.

**Workaround**: Use batch requests (`POST /batch`) to parallelize subtask fetching, up to 10 operations per batch.

### Webhook Reliability

Webhooks use **at-most-once delivery** with no replay capability. Events may be lost in exceptional circumstances. Webhooks are also deleted if the heartbeat fails (no response after 24 hours) or if the resource is deleted (deleted after 72 hours).

**Workaround**: Build a fallback polling system using the Events API (`GET /events`) to periodically check for changes. Always respond to heartbeat events.

### Story Consolidation Can Cause Phantom Events

When a task is updated multiple times quickly, Asana may consolidate the activity stories (e.g., "moved from Section A to Section B" might be consolidated with a later "moved from Section A to Section C"). Webhook events may reference stories that no longer exist due to consolidation.

**Workaround**: Always fetch the current state of a resource when processing webhook events rather than relying on the event's story data.

### Large Result Sets Are Truncated

API queries that match very large result sets (tens of thousands of items) may be truncated even with pagination. The last page returns a `400` error indicating a truncated data set.

**Workaround**: Split queries by hierarchical structure (e.g., fetch teams first, then projects per team, then tasks per section). For bulk data extraction, use Organization Exports (Enterprise+ only).

### Deeply Nested Subtasks Are Problematic

Asana supports up to 5 levels of subtasks, but deep nesting causes significant performance issues and high API cost. The documentation explicitly recommends against using sub-subtasks.

**Workaround**: Use multi-homing (putting tasks in multiple projects) or custom fields to model hierarchy rather than deeply nested subtasks.

### Projects Are Moving to "Teamless" Model

As of 2026, Asana is transitioning to support "teamless projects." Previously, every project in an organization was required to belong to a team. This is changing, which means the `team` field on projects may be absent for newer projects.

**Workaround**: Handle the `team` field as optional and do not assume it is always populated.

### SDK End-of-Support for Ruby, Java, PHP

The Ruby, Java, and PHP client libraries have been discontinued and will no longer receive updates. They still work but will not be updated for new API features.

**Workaround**: Use the JavaScript or Python SDKs, or call the REST API directly.

### Search API Rate Limits Are Lower

The Search API (`GET /workspaces/{workspace_gid}/tasks/search`) is limited to 60 requests/minute regardless of plan type, which is significantly lower than the standard rate limit.

**Workaround**: Cache search results and use more specific filters to reduce the number of search calls needed.

### Custom Task Types Not Fully Supported in API

As of 2026, the `create_tasks` and `update_tasks` endpoints do not fully support custom task types, which is a frequently requested feature on the developer forum.

**Workaround**: Monitor the changelog and forum for updates. Currently, custom task types can only be set through the Asana UI.

### Inbox/Notification Feed Has No API

There is no API endpoint to access a user's inbox or notification/activity feed. This is a long-standing feature request (dating back to 2019) that remains unimplemented.

**Workaround**: Use webhooks or the Events API to track changes on specific resources. There is no workaround for accessing the unified inbox view.

### Free Tier API Access Is Severely Limited

With only 150 requests/minute on the free tier, any non-trivial integration will quickly hit rate limits. This makes it impractical to develop and test integrations on free accounts with realistic data volumes.

**Workaround**: Request a developer sandbox (https://developers.asana.com/docs/developer-sandbox) which provides premium-tier features and higher rate limits for testing.

### Missing Fields on Newly Created Resources

Some fields returned by GET requests are not accepted on POST/PUT requests. For example, `created_at` and `modified_at` are read-only. This can cause confusion when trying to create a resource with all the same fields as an existing one.

**Workaround**: Consult the API reference for each endpoint to understand which fields are writable. Use `opt_fields` to discover the full schema of a resource via GET before attempting writes.

### Duplicated Tasks Lose Assignee

A known issue reported on the developer forum (as of May 2026) is that duplicated tasks may not include the assignee from the original task.

**Workaround**: After duplication, explicitly set the assignee on the new task via a separate update request.
