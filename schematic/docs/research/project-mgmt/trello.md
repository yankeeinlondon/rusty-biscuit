# Research into Trello's API Support

## Overview on Product

Trello is a visual project management and collaboration tool owned by Atlassian. It uses a Kanban-style board system where users organize work into **Boards** (projects), **Lists** (stages or categories), and **Cards** (individual tasks or items). Cards support descriptions, checklists, attachments, labels, due dates, comments, and member assignments.

Trello also offers **Power-Ups** (extensible plugins that add capabilities to boards), **Butler Automation** (no-code workflow automation), and an **Inbox** feature for capturing items from email, Slack, and Teams.

### Key URLs

| Resource                 | URL                                                                                |
|--------------------------|------------------------------------------------------------------------------------|
| Trello Homepage          | https://trello.com                                                                 |
| Developer Portal         | https://developer.atlassian.com/cloud/trello/                                      |
| REST API Reference       | https://developer.atlassian.com/cloud/trello/rest/                                 |
| API Introduction Guide   | https://developer.atlassian.com/cloud/trello/guides/rest-api/api-introduction/     |
| Authorization Guide      | https://developer.atlassian.com/cloud/trello/guides/rest-api/authorization/        |
| Webhooks Guide           | https://developer.atlassian.com/cloud/trello/guides/rest-api/webhooks/             |
| Rate Limits Guide        | https://developer.atlassian.com/cloud/trello/guides/rest-api/rate-limits/          |
| Limits Guide             | https://developer.atlassian.com/cloud/trello/guides/rest-api/limits/               |
| Action Types             | https://developer.atlassian.com/cloud/trello/guides/rest-api/action-types/         |
| Power-Ups Documentation  | https://developer.atlassian.com/cloud/trello/power-ups/                            |
| client.js Reference      | https://developer.atlassian.com/cloud/trello/guides/client-js/client-js-reference/ |
| OpenAPI 3.0 Spec (JSON)  | https://dac-static.atlassian.com/cloud/trello/swagger.v3.json                      |
| Postman Collection       | https://developer.atlassian.com/cloud/trello/trello.postman.json                   |
| Power-Up Admin / API Key | https://trello.com/power-ups/admin                                                 |
| Developer Community      | https://community.developer.atlassian.com/c/trello/42                              |
| Developer Support        | https://go.trello.com/dev-support                                                  |
| Changelog                | https://developer.atlassian.com/cloud/trello/changelog/                            |
| FAQ                      | https://developer.atlassian.com/cloud/trello/faq/                                  |

### Pricing Structure

Trello has four pricing tiers (all prices per user/month):

| Plan           | Price                                 | Key Limitations                                                                                                         |
|----------------|---------------------------------------|-------------------------------------------------------------------------------------------------------------------------|
| **Free**       | $0                                    | Up to 10 collaborators per Workspace, 10 boards per Workspace, 10MB/file upload limit, 250 automation runs/month        |
| **Standard**   | $5/mo (annual) / $6/mo (monthly)      | Unlimited boards, 250MB/file upload, 1,000 automation runs/month, Custom Fields, advanced checklists                    |
| **Premium**    | $10/mo (annual) / $12.50/mo (monthly) | AI features, Calendar/Timeline/Table/Dashboard/Map views, unlimited automation runs, admin/security features, observers |
| **Enterprise** | $17.50/mo (annual, billed annually)   | Unlimited Workspaces, org-wide permissions, SSO via Atlassian Guard, Power-Up administration, attachment restrictions   |

The API itself is free to use regardless of plan. Rate limits apply uniformly.

---

## API Details

### REST API (Primary)

Trello provides a single comprehensive REST API at `https://api.trello.com/1/`. All core CRUD operations are performed via this API.

- **Base URL**: `https://api.trello.com/1/`
- **Protocol**: HTTPS only
- **Request/Response Format**: JSON
- **HTTP Methods**: GET, POST, PUT, DELETE
- **Versioning**: The `/1/` prefix is the version; Trello has never introduced a v2

#### Resource Groups

| Resource          | Description                                                   |
|-------------------|---------------------------------------------------------------|
| Actions           | Audit trail of all events (comments, moves, updates, etc.)    |
| Applications      | App management                                                |
| Batch             | Batch multiple GET requests into one call                     |
| Boards            | Board CRUD and sub-resources                                  |
| Cards             | Card CRUD, attachments, checklists, comments, labels, members |
| Checklists        | Checklist and check-item management                           |
| Custom Fields     | Board-level field definitions and per-card values             |
| Emoji             | Built-in emoji                                                |
| Enterprises       | Enterprise admin features                                     |
| Labels            | Board-level label CRUD                                        |
| Lists             | List CRUD within boards                                       |
| Members           | User profile and preferences                                  |
| Notifications     | User notification management                                  |
| Organizations     | Workspace (formerly "organization") CRUD                      |
| Plugins/Power-Ups | Plugin discovery and management                               |
| Search            | Full-text search across boards, cards, members, orgs          |
| Tokens            | Token management and revocation                               |
| Webhooks          | Webhook CRUD and event delivery                               |

#### Batch Endpoint

Trello supports a `/1/batch` endpoint that allows combining up to 10 GET requests into a single call. This is useful for reducing round trips and staying within rate limits.

### Webhooks

Trello supports outbound webhooks (not to be confused with WebSocket). When a watched model changes, Trello sends an HTTP POST to a registered callback URL.

- Webhooks belong to tokens and can only monitor objects the token can access
- Webhook payload contains: `action` (what changed), `model` (the watched object), `webhook` (the webhook config)
- Callback URL must respond to an initial HEAD request with HTTP 200
- Retries: 3 attempts with exponential backoff (30s, 60s, 120s)
- Signatures: HMAC-SHA1 via `X-Trello-Webhook` header
- Auto-disable: After 30 days of consecutive failures AND 1000+ failures
- No documented limit on the number of webhooks per token

### WebSocket

Trello does **not** offer a WebSocket API. Real-time updates within Trello's own UI are handled through a proprietary mechanism not exposed to third-party developers. The recommended approach for near-real-time updates is webhooks.

### JSON-RPC

Trello does **not** support JSON-RPC.

### GraphQL

Trello does **not** support GraphQL.

### Formal Schema

**Yes**. Trello provides an OpenAPI 3.0 specification:

- **OpenAPI 3.0 (JSON)**: https://dac-static.atlassian.com/cloud/trello/swagger.v3.json
- **Postman Collection**: https://developer.atlassian.com/cloud/trello/trello.postman.json

The OpenAPI spec is linked directly from each API reference page. This is a well-maintained, machine-readable schema that covers all REST endpoints, request parameters, and response types.

### SDKs and Client Libraries

Trello provides and recommends:

| SDK/Language               | Details                                                                                                                                                                                                        |
|----------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **client.js** (JavaScript) | Official client-side JavaScript library wrapping the REST API. Includes built-in authorization methods for Power-Ups. Docs: https://developer.atlassian.com/cloud/trello/guides/client-js/client-js-reference/ |
| **Node.js**                | Code examples in Node.js are provided on every API reference page. No official standalone Node SDK.                                                                                                            |
| **Python**                 | Code examples in Python are provided on every API reference page. No official standalone Python SDK.                                                                                                           |
| **Java**                   | Code examples in Java are provided on every API reference page. No official standalone Java SDK.                                                                                                               |
| **PHP**                    | Code examples in PHP are provided on every API reference page. No official standalone PHP SDK.                                                                                                                 |
| **cURL**                   | cURL examples are provided for every endpoint.                                                                                                                                                                 |

Community-maintained SDKs exist (e.g., `trello` on PyPI, `node-trello` on npm, `trello-ruby` gem) but none are officially supported by Atlassian.

### Authentication Mechanisms

Trello supports two authentication mechanisms:

#### 1. API Key + Token (Recommended for most use cases)

- **API Key**: Generated per Power-Up at https://trello.com/power-ups/admin. Intended to be public.
- **Token**: Generated by directing users to `https://trello.com/1/authorize` with query params for scope and expiration. Tokens grant access to a user's entire account and must be kept secret.
- **Scope options**: `read`, `write`, `account` (comma-separated)
- **Expiration options**: `1hour`, `1day`, `30days`, `never`
- **Passing credentials**: Via query params (`?key=...&token=...`), `Authorization` header (`OAuth oauth_consumer_key="...", oauth_token="..."`), or PUT/POST body
- **Account scope** is required to access member email addresses

#### 2. OAuth 1.0a

Trello supports standard OAuth 1.0a with the following endpoints:

- Request Token: `https://trello.com/1/OAuthGetRequestToken`
- Authorize: `https://trello.com/1/OAuthAuthorizeToken`
- Access Token: `https://trello.com/1/OAuthGetAccessToken`

An application secret (found alongside the API key) is used to sign requests.

#### Important Note on Forge and OAuth2

The Trello API documentation explicitly states: **"Forge and OAuth2 apps cannot access this REST resource."** This appears on every single endpoint. This means Atlassian's newer Forge platform and OAuth2-based integrations are not compatible with Trello's REST API. You must use the API Key + Token or OAuth 1.0a flow.

### Signup Process for API Access

1. Create a Trello account at https://trello.com
2. Go to https://trello.com/power-ups/admin and create a new Power-Up (even if you only want API access)
3. Navigate to the Power-Up's **API Key** tab and generate a new API key
4. Use the API key to authorize yourself (or your users) and obtain a token via the `1/authorize` route or OAuth 1.0a
5. Use the API key + token pair to make API requests

There is no approval process, no app review, and no paid developer account required. API access is immediately available.

### Rate Limits

| Limit Type              | Threshold                                                                                                        |
|-------------------------|------------------------------------------------------------------------------------------------------------------|
| Per API key             | 300 requests per 10 seconds                                                                                      |
| Per token               | 100 requests per 10 seconds                                                                                      |
| Per `/1/members/` route | 100 requests per 900 seconds                                                                                     |
| Excess 429 threshold    | If 200+ 429 errors per key within a 10s window, all further requests are blocked for the remainder of the window |

Rate limit headers are returned in every response:

- `x-rate-limit-api-key-interval-ms`
- `x-rate-limit-api-key-max`
- `x-rate-limit-api-key-remaining`
- `x-rate-limit-api-token-interval-ms`
- `x-rate-limit-api-token-max`
- `x-rate-limit-api-token-remaining`

---

## Schemas

Trello's data model is centered around a hierarchy: **Organization** (Workspace) contains **Boards**, Boards contain **Lists**, Lists contain **Cards**. Cards are the primary unit of work.

### Card (Task / Todo / Action)

The **Card** is Trello's equivalent of a Task or Todo item. It is the fundamental unit of work.

**Source**: OpenAPI 3.0 spec and REST API reference (https://developer.atlassian.com/cloud/trello/rest/api-group-cards/). High confidence; the schema is well-documented with full field listings.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: String,
    pub name: String,
    pub desc: String,
    pub closed: bool,
    pub id_board: String,
    pub id_list: String,
    pub id_members: Vec<String>,
    pub id_labels: Vec<String>,
    pub id_checklists: Vec<String>,
    pub id_members_voted: Vec<String>,
    pub id_short: u64,
    pub pos: f64,
    pub short_link: String,
    pub short_url: String,
    pub url: String,
    pub subscribed: bool,
    pub due: Option<String>,
    pub start: Option<String>,
    pub due_complete: bool,
    pub due_reminder: Option<String>,
    pub date_last_activity: String,
    pub address: Option<String>,
    pub location_name: Option<String>,
    pub coordinates: Option<String>,
    pub cover: CardCover,
    pub badges: CardBadges,
    pub check_item_states: Vec<String>,
    pub labels: Vec<Label>,
    pub limits: CardLimits,
    pub manual_cover_attachment: bool,
}
```

### Member (Person / Contact)

The **Member** entity represents a Trello user/person.

**Source**: OpenAPI 3.0 spec and REST API reference (https://developer.atlassian.com/cloud/trello/rest/api-group-members/). High confidence.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub id: String,
    pub full_name: String,
    pub username: String,
    pub initials: String,
    pub bio: Option<String>,
    pub url: String,
    pub email: Option<String>,
    pub avatar_hash: Option<String>,
    pub avatar_url: Option<String>,
    pub confirmed: bool,
    pub member_type: String,
    pub activity_blocked: bool,
    pub id_enterprise: Option<String>,
    pub id_boards: Vec<String>,
    pub id_organizations: Vec<String>,
    pub status: Option<String>,
    pub prefs: MemberPrefs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberPrefs {
    pub locale: Option<String>,
    pub timezone: Option<String>,
    pub color_blind: bool,
    pub send_summaries: Option<bool>,
    pub minutes_between_summaries: Option<u32>,
    pub two_factor: TwoFactorPrefs,
}
```

### Organization (Company / Workspace)

The **Organization** entity (referred to in the UI as "Workspace") represents a company or team grouping.

**Source**: OpenAPI 3.0 spec and REST API reference (https://developer.atlassian.com/cloud/trello/rest/api-group-organizations/). High confidence.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub url: String,
    pub desc: Option<String>,
    pub website: Option<String>,
    pub id_enterprise: Option<String>,
    pub id_boards: Vec<String>,
    pub memberships: Vec<String>,
    pub prefs: OrganizationPrefs,
    pub premium_features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationPrefs {
    pub permission_level: String,
    pub board_visibility_restrict: serde_json::Value,
    pub board_delete_restrict: serde_json::Value,
    pub attachment_restrictions: Vec<String>,
}
```

### Workflow

Trello does not have an explicit "Workflow" entity. Instead, workflow is modeled implicitly through:

- **Board**: Represents a project or process
- **Lists on a Board**: Represent stages in a workflow (e.g., "To Do", "In Progress", "Done")
- **Card movement between Lists**: Represents progression through the workflow
- **Butler Automation**: Provides no-code rules, triggers, and actions for workflow automation (not directly exposed via the REST API)
- **Actions**: The audit trail records every workflow transition as action types like `updateCard`, `moveCardToBoard`, `createCard`, etc.

**Source**: Conceptual understanding from the API documentation and Trello's product model. No formal schema for "workflow" exists. The `List` entity below represents workflow stages.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrelloList {
    pub id: String,
    pub name: String,
    pub closed: bool,
    pub pos: f64,
    pub id_board: String,
    pub subscribed: Option<bool>,
    pub soft_limit: Option<serde_json::Value>,
    pub limits: ListLimits,
}
```

### Status

Trello does not have an explicit "Status" entity in its API. Status is modeled through:

1. **Labels**: Color-coded tags with optional names, attached to cards (e.g., "Blocked", "Urgent")
2. **List membership**: A card's position in a list implicitly defines its status
3. **`dueComplete` field**: Boolean on the card indicating whether the due date has been marked complete
4. **`closed` field**: Boolean on boards, lists, and cards indicating archival
5. **Custom Fields**: Board-level typed fields (text, number, date, checkbox, list) that can be used to track custom status values

**Source**: Labels and Custom Fields from the API reference. No formal "Status" entity exists. The Label and CustomField entities below represent status-like concepts.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: String,
    pub id_board: String,
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomField {
    pub id: String,
    pub id_model: String,
    pub model_type: String,
    pub field_group: String,
    pub display: CustomFieldDisplay,
    #[serde(rename = "type")]
    pub field_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomFieldDisplay {
    pub card_front: bool,
    pub name: String,
    pub pos: String,
    pub options: Vec<CustomFieldOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomFieldOption {
    pub id: String,
    pub id_custom_field: String,
    pub value: CustomFieldValue,
    pub color: Option<String>,
    pub pos: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomFieldValue {
    pub text: Option<String>,
    pub checked: Option<bool>,
    pub date: Option<String>,
    pub number: Option<String>,
}
```

---

## Gotchas

### Forge and OAuth2 Incompatibility

Every single Trello REST API endpoint is annotated with: **"Forge and OAuth2 apps cannot access this REST resource."** If you are building on Atlassian's newer Forge platform or using OAuth2 flows, you cannot access Trello's REST API. You must use the legacy API Key + Token or OAuth 1.0a authentication.

**Workaround**: Use the API Key + Token authentication flow directly. Create a Power-Up to generate your API key.

### No WebSocket or Real-Time Streaming API

Trello does not expose any real-time streaming or WebSocket API. The only near-real-time option is webhooks (outbound HTTP POST).

**Workaround**: Use webhooks for push notifications when models change. Polling is the only alternative, but beware of rate limits.

### Rate Limits Are Per-Key, Not Per-User

Rate limits are enforced per API key (300 req/10s) and per token (100 req/10s). If your integration serves many users from a single API key, the 300 req/10s key-level limit is shared across all users.

**Workaround**: Each user should have their own token. Use the batch endpoint (`/1/batch`) to combine up to 10 GET requests. Use webhooks instead of polling.

### `/1/members/` Has Extremely Strict Limits

The `/1/members/` route is limited to 100 requests per 900 seconds (15 minutes). This is far more restrictive than the general rate limit.

**Workaround**: Use nested resources instead. For example, to get members of a board, use `/1/boards/{id}/members` instead of iterating through individual member IDs.

### Action History Is Limited to 1000 Per Request

The API returns at most 1000 actions per request. Boards with extensive history require multiple paginated requests using `since` and `before` parameters.

**Workaround**: Page through actions using the ID of the last returned action as the `before` parameter in the next request.

### Cards with Actions Can Hit Response Size Limits

Requesting all cards on a board with `actions=all` will fail on large boards with the error `API_TOO_MANY_CARDS_REQUESTED`.

**Workaround**: Fetch cards first without actions, then request actions for individual cards or in smaller batches.

### Webhook Callback URL Must Be Reachable at Creation Time

Trello performs an HTTP HEAD request to the callback URL when creating a webhook. If it does not receive a 200 response, the webhook creation fails. This makes local development difficult.

**Workaround**: Use a tunneling service like ngrok during development. Ensure your callback endpoint responds to HEAD requests.

### Webhooks Auto-Disable After 30 Days of Failures

Webhooks that fail consecutively for 30 days and over 1000 times are automatically disabled.

**Workaround**: Monitor your webhook health. A single successful delivery resets the failure counters. Implement the `X-Trello-Client-Identifier` header to prevent infinite loops in automation scenarios.

### API Key Requires a Power-Up

To obtain an API key, you must create a Trello Power-Up at https://trello.com/power-ups/admin. There is no standalone "API key" registration. This can be confusing for developers who just want API access without building a Power-Up.

**Workaround**: Create a Power-Up even if you only need API access. The Power-Up does not need to be published or submitted for review.

### No Versioned API Beyond v1

Trello has only ever had one API version (`/1/`). Breaking changes can occur without a new version path, which means existing integrations may break.

**Workaround**: Monitor the changelog at https://developer.atlassian.com/cloud/trello/changelog/ and subscribe to updates.

### Token Scope Is All-or-Nothing

Tokens grant access to a user's entire account within the requested scope (`read`, `write`, `account`). There is no way to scope a token to a specific board or organization.

**Workaround**: Request only the minimum scope needed (`read` if you only need to read data). Use short-lived tokens (`1hour`, `1day`) where possible.

### Deleted Cards Are Not Recoverable via API

Deleting a card via `DELETE /1/cards/{id}` is permanent. Unlike the Trello UI which has an "undo" option, the API provides no recovery mechanism.

**Workaround**: Consider using `closed: true` (archiving) instead of deletion. Archived cards can be reopened.

### No Native Multi-Board or Cross-Board Views in the API

While Trello Premium offers Table and Calendar views across boards in the UI, the API does not provide dedicated cross-board query endpoints.

**Workaround**: Query boards individually and aggregate results client-side, or use the `/1/search` endpoint to search across boards.

### `pos` Field Uses Floating-Point Positioning

The `pos` (position) field on cards, lists, and stickers is a floating-point number used for ordering. It is not a simple integer index. Moving items requires calculating midpoint positions.

**Workaround**: Use `pos: "top"`, `pos: "bottom"`, or specific numeric values. When repositioning, calculate the midpoint between neighboring items.

### Community SDKs Are Unofficial and Often Abandoned

While community SDKs exist for Python, Node.js, Ruby, and other languages, none are officially maintained by Atlassian and many are abandoned or outdated.

**Workaround**: Use the OpenAPI spec to generate a client for your language of choice, or call the REST API directly. The OpenAPI spec is actively maintained by Atlassian.

### Labels Are Board-Scoped, Not Global

Labels belong to individual boards and cannot be shared across boards. Each board has its own set of up to 10 color slots with optional names.

**Workaround**: If you need consistent labeling across boards, manage label names programmatically or use Custom Fields instead.
