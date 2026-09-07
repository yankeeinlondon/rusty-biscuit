---
prompt: |-
  Research the API's that Unifi products expose and how to leverage them.

  - start by providing a broad narrative of the history of Unifi APIs
      - APIs are relatively new and have been updated significantly since release
      - describe the intentions of these APIs (and how that's changed over time ... if it's changed)
      - discuss local API's versus cloud based API's and how they compare contrast
  - then organize the remaining research and reporting into different API surfaces
      - for each API surface describe the API at a high level
      - list out the endpoints exposed by the API and how each end point is used
      - discuss authentication/authorization for the API surface
      - provide a reference to the official docs for this API
      - mention any quirks or "gotchas" that developers report in working with the given API surface and how to successfully navigate these issues
last_updated: 2026-08-30
hash: c4a5aa8fb8733dbd-80f28a35ab781d85
---
I have a comprehensive research base across all six official portals, the Access PDF reference, and the reverse-engineered surfaces. Here is the replacement body content.

---

# UniFi APIs

For roughly fifteen years, integrating with UniFi meant reverse-engineering the private API that the web console talks to. That ended in stages between late 2024 and 2025. Ubiquiti now publishes **six versioned, OpenAPI-documented API surfaces** at [developer.ui.com](https://developer.ui.com) — one cloud API and five local application APIs — plus a seventh, older, separately-documented API for UniFi Access.

The short version of the decision, if you are building today:

- **Building automation against your own hardware on the LAN?** Use the local Integration APIs (`/proxy/{app}/integration/v1`) with an `X-API-KEY` header.
- **Building a fleet dashboard across sites you don't have a network path to?** Use the Site Manager cloud API at `https://api.ui.com/v1`, and reach into individual consoles with the Connector Proxy.
- **Need something the official APIs don't expose yet?** The private API still works and still powers the web console — but you are back to session cookies, CSRF tokens, and no compatibility guarantee.

That last bullet is the whole story of UniFi API development in 2026: the official surface is real, supported, and expanding fast, but it is still a strict subset of what the private API can do. Most serious integrations run a hybrid.

> **Currency.** Versions and endpoint listings below reflect the developer portal as of **30 August 2026**: Network **10.4.57**, Protect **7.2.105**, Site Manager **1.0.0**, Mobility **1.0.0**, InnerSpace **1.3.23**, Carrier Fabric **1.0.0**. Ubiquiti publishes docs per firmware release, so treat every endpoint list as version-pinned rather than permanent.

---

## A Short History

### The reverse-engineering era (2010–2024)

The UniFi Network Controller has always had an HTTP API, because its own web interface is a JavaScript client that talks to one. Ubiquiti never documented it, never promised stability, and never officially acknowledged it — but it was fully functional and, for over a decade, it was the only way in.

The community filled the gap. The [Ubiquiti Community Wiki API page](https://ubntwiki.com/products/software/unifi-controller/api) — which describes itself frankly as "a reverse engineering project based on browser captures, jar dumps, and reviewing other software" — became the de facto reference. Art of WiFi's PHP `UniFi-API-client` has been maintained since 2015 and spans controller versions 5.x through 10.x. Python (`pyunifi`), Node (`node-unifi`), and Go clients followed the same map.

This era had a defining structural break in the middle of it: the arrival of **UniFi OS**. Legacy software controllers listened on port 8443, logged in at `/api/login`, and served everything under `/api/s/{site}/`. UniFi OS appliances (UDM, UDM Pro, UDR, UCG, Cloud Key Gen2) listen on 443, log in at `/api/auth/login`, and serve the same controller under an nginx proxy prefix at `/proxy/network/api/s/{site}/`. Every client library since carries a branch for this, and getting it wrong remains the single most common cause of mysterious 404s.

### The MFA break (July 2024)

The forcing function was security policy. In **July 2024**, Ubiquiti enforced multi-factor authentication on UI.com cloud accounts. Overnight, every automated integration that logged in with a cloud account credential broke — non-interactive login cannot satisfy a TOTP challenge.

The community workaround was immediate and ugly: create a **local-only admin account**, which is exempt from MFA, and use that for automation. It worked, and it still works, but it meant the recommended integration pattern was "deliberately provision a credential that bypasses your own security policy." That was not a tenable long-term answer, and Ubiquiti clearly knew it.

### The official era (late 2024 → 2026)

The response was a genuine, staged API program.

| When               | What shipped                                                                                                                                                                                                                                                                          |
|--------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **2024**           | **Site Manager API** — cloud-scoped, `X-API-KEY` auth, launched under an Early Access tier at `api.ui.com/ea/` before promotion to `api.ui.com/v1/`. Positioned at MSPs and multi-site operators.                                                                                     |
| **3 January 2025** | **UniFi Network 9.0** ships the **Local Network API** (the "Network Integration API"), announced in [*UniFi Network 9.0 — Built to Scale*](https://blog.ui.com/article/unifi-network-9-0-built-to-scale). Devices, clients, sites, statistics; API-key auth; pagination from day one. |
| **2025**           | **UniFi Protect 5.3** ships the **Protect Integration API**. The versioned doc archive on the developer portal begins at Protect v5.3.38.                                                                                                                                             |
| **2025–2026**      | Rapid surface expansion through Network 9.1 → 9.5 → 10.x: networks, WLANs, firewall zones and policies, ACL rules, DNS policies, traffic-matching lists, switch stacks, LAGs, VPN, RADIUS. Protect grows from cameras to the full device catalog plus alarm profiles and POS.         |
| **2026**           | The portal broadens beyond Network/Protect: **Mobility**, **InnerSpace**, and **Carrier Fabric** appear as first-class documented APIs. The **Connector Proxy** (console firmware ≥ 5.0.3) lets cloud API keys execute local API calls without any inbound network path.              |

UniFi Access sits outside this lineage. Its Developer API predates the portal, has always been officially documented (as a downloadable PDF reference generated from the Access application itself), and uses a different port, a different auth header, and a different response envelope. It has not been folded into developer.ui.com.

### What Ubiquiti is actually trying to do

The intent has shifted, and reading the shift correctly tells you what to expect next.

**The original intent was fleet monitoring.** The Site Manager API launched read-only: hosts, sites, devices, ISP metrics. It was aimed squarely at managed service providers who needed to answer "is everything up, and how is everyone's internet doing" across hundreds of customer sites. Nothing about it suggested Ubiquiti wanted third parties *configuring* UniFi.

**The current intent is programmable infrastructure.** That changed decisively with the local APIs. Network 10.x will now let you create networks, author firewall policies and reorder them, define ACL rules, manage DNS policies, adopt and remove devices, and issue vouchers. Protect will let you arm and disarm the alarm system, drive PTZ cameras, activate relays, and post point-of-sale transactions onto a video stream. Carrier Fabric will provision and suspend ISP subscribers. This is not a monitoring API anymore; it is a configuration API with a monitoring API attached.

**The consistent thread is licensing.** None of this is metered or paywalled. Ubiquiti's commercial model puts the margin in hardware, so the API is a feature that sells boxes rather than a product that sells seats. That is a meaningful difference from the incumbent enterprise networking vendors and it is the main reason the surface has grown as fast as it has.

**What remains unresolved is the private API.** Ubiquiti has never announced a deprecation, and cannot easily do so — the private API is what the web console itself uses. So the honest current state is two parallel surfaces: one supported and narrower, one unsupported and complete. Every source in the community says the same thing, and none of them expects that to resolve soon.

---

## Local APIs vs. Cloud APIs

This is the first architectural decision, and it is mostly about network topology, not features.

|                                     | **Local (Integration APIs)**                          | **Cloud (Site Manager)**                     | **Cloud → Local (Connector Proxy)**                                  |
|-------------------------------------|-------------------------------------------------------|----------------------------------------------|----------------------------------------------------------------------|
| **Base URL**                        | `https://{console}/proxy/{app}/integration/v1`        | `https://api.ui.com/v1`                      | `https://api.ui.com/v1/connector/consoles/{id}/{app}/integration/v1` |
| **Auth**                            | `X-API-KEY`, key created **on the console**           | `X-API-KEY`, key created at **unifi.ui.com** | Cloud key                                                            |
| **Network path required**           | Yes — LAN, VPN, or exposed port                       | No                                           | No                                                                   |
| **Works behind CGNAT / dynamic IP** | No                                                    | Yes                                          | Yes                                                                  |
| **TLS certificate**                 | Self-signed by default; verification usually disabled | Public CA; verification stays on             | Public CA; verification stays on                                     |
| **Latency**                         | Low, deterministic                                    | Higher                                       | Higher, plus console round-trip                                      |
| **Survives internet outage**        | Yes                                                   | No                                           | No                                                                   |
| **Scope**                           | One console, deep control                             | All consoles on the account, shallow         | One console, deep control                                            |
| **Rate limit**                      | Not documented                                        | 10,000 req/min (v1 GA)                       | Cloud limit applies                                                  |
| **Dependency on Ubiquiti**          | None                                                  | Total                                        | Total                                                                |

**The important nuance:** these are not competing feature sets. The Connector Proxy makes the choice largely orthogonal. The path segment *after* `/integration/v1` is byte-identical in local and proxied mode, so a well-built client swaps a base URL and nothing else. Ubiquiti clearly designed it that way.

**Recommendation.** Prefer local for anything latency-sensitive, safety-relevant, or that must survive a WAN outage — home automation, alarm integration, door control. Prefer cloud for fleet inventory, ISP health reporting, and any site you don't control the network path to. Use the Connector Proxy when you need local depth at a site you can't reach, and accept that it inherits every failure mode of Ubiquiti's cloud.

One caution worth stating plainly: the Connector Proxy hands Ubiquiti's cloud the ability to execute privileged configuration calls against your gateway. That is a deliberate trade of blast radius for reachability. It is the right call for an MSP and the wrong call for an air-gapped deployment.

---

## Reading the Developer Portal

Before the individual surfaces, three portal mechanics that save real time:

**Docs are pinned to firmware versions.** The URL shape is `https://developer.ui.com/{service}/{version}/{page}` — e.g. `https://developer.ui.com/network/v10.4.57/gettingstarted`. Filterable properties, available fields, and whole endpoint groups change between releases. Always read the docs for *your* build. Your console also serves its own copy, matched exactly to its firmware, under **Network → Settings → Control Plane → Integrations**.

**Machine-readable artifacts exist for every surface.** Each service/version publishes:

- `openapi.json` — the OpenAPI contract, the authoritative source for enums, filterable properties, and pagination bounds
- `postman-collection.json` — importable collection
- `llms.txt` — a flat, complete endpoint index, by far the fastest way to see a whole surface at once
- `ai-*.md` — Markdown variants intended to be pasted into an LLM

The root index at [`https://developer.ui.com/llms.txt`](https://developer.ui.com/llms.txt) lists all six services, their current versions, and every historical version — it is the single most useful URL on the portal.

**Generate a client, don't hand-write one.** Given `openapi.json`, code generation is strictly better than transcribing endpoint tables, and it survives version bumps.

There is also a community mirror at [opastorello/unifi-api-docs](https://github.com/opastorello/unifi-api-docs), which snapshots the official OpenAPI specs daily and renders them at [opastorello.github.io/unifi-api-docs](https://opastorello.github.io/unifi-api-docs/). Useful for diffing versions and for reading docs for firmware you don't run.

---

## Surface 1 — UniFi Site Manager API (Cloud)

### Overview

The account-level, cross-site, cloud-hosted API. It answers questions of the form *"what do I own, where is it, and is it healthy?"* across every console adopted to a UI.com account or organization. It is deliberately shallow: it will not configure a firewall or reboot an access point. Its unique value is **ISP metrics** — WAN latency, throughput, packet loss, and uptime, aggregated per site with real retention — which no local API provides in the same form.

This is the surface MSPs build on.

### Base URL and authentication

```text
https://api.ui.com/v1/...        # Official (GA, stable, long-term support)
https://api.ui.com/ea/...        # Early Access (evaluation tier)
```

Authentication is a single header:

```text
X-API-KEY: <key>
```

Generate the key at **unifi.ui.com → Settings → API Keys → Create New API Key**. It is displayed exactly once. Keys are bound to the UI account or organization that created them; a non-organization key can only reach consoles owned by that account.

### Endpoints

| Method | Path                                | Purpose                                                                  |
|--------|-------------------------------------|--------------------------------------------------------------------------|
| `GET`  | `/v1/hosts`                         | All hosts (consoles) on the account                                      |
| `GET`  | `/v1/hosts/{id}`                    | One host in detail                                                       |
| `GET`  | `/v1/sites`                         | All sites across hosts running UniFi Network                             |
| `GET`  | `/v1/devices`                       | All adopted devices across all hosts                                     |
| `GET`  | `/v1/isp-metrics/{type}`            | ISP metrics for all sites; `type` is `5m` or `1h`                        |
| `POST` | `/v1/isp-metrics/{type}/query`      | Batch ISP metrics for an explicit list of `{hostId, siteId, begin, end}` |
| `GET`  | `/v1/sd-wan-configs`                | All SD-WAN configurations                                                |
| `GET`  | `/v1/sd-wan-configs/{id}`           | One SD-WAN configuration                                                 |
| `GET`  | `/v1/sd-wan-configs/{id}/status`    | Deployment status of an SD-WAN configuration                             |
| `GET`  | `/v1/connector/consoles/{id}/*path` | **Connector Proxy** — see Surface 2                                      |

**Hosts vs. sites vs. devices** is the mental model to internalize: a *host* is a physical console, a *site* is a Network application tenant on that host, and a *device* is adopted hardware. One host can carry many sites; one account can carry many hosts. Site Manager is the only surface that sees across host boundaries.

Pagination uses `pageSize` and an opaque `nextToken`. The response envelope is `{ code, data, httpStatusCode, traceId, nextToken }` — note `traceId`, which is what to quote when opening a support ticket.

### Official docs

[developer.ui.com/site-manager/v1.0.0/gettingstarted](https://developer.ui.com/site-manager/v1.0.0/gettingstarted) · [version control policy](https://developer.ui.com/site-manager/v1.0.0/versioncontrol) · [openapi.json](https://developer.ui.com/site-manager/v1.0.0/openapi.json)

### Gotchas

**ISP metrics retention is asymmetric and the parameters are mutually exclusive.** `5m` granularity is retained for at least 24 hours; `1h` granularity for at least 30 days. There is no `5m` data beyond a day, and no `24h` duration shortcut for `1h` data. The `duration` parameter (`24h` for 5m; `7d`/`30d` for 1h) **cannot be combined with** `beginTimestamp`/`endTimestamp` — pick one addressing mode. *Navigate it:* poll `1h` hourly for durable history and persist locally; if you need `5m`, poll every 5–15 minutes and store it yourself, because the API will never backfill past retention.

**Timestamps must be RFC 3339, not dates.** Passing `2024-06-30` returns `400 parameter_invalid`. Use `2024-06-30T13:35:00Z`, aligned to the interval boundary.

**The batch query endpoint fails whole, not partial.** If the account lacks access to any one site in a `POST /v1/isp-metrics/{type}/query` request, the entire call returns 502. *Navigate it:* validate every `hostId`/`siteId` against `/v1/hosts` and `/v1/sites` before batching, and chunk conservatively.

**`periods` arrays are sparse by design.** Offline or unadopted gateways produce absent periods. Never assume a dense time series; interpolate or gap-render explicitly.

**Rate limiting returns a precise retry hint.** The GA limit is 10,000 requests/minute (the Early Access tier was 100/min). On 429 the body carries something like `rate limit exceeded, retry after 5.372786998s` alongside a `Retry-After` header. *Navigate it:* honor the returned value rather than a fixed backoff.

**Early Access fields are all optional.** Ubiquiti states explicitly that in the `ea` tier, every field within `response.data` should be treated as optional. Model EA responses with optionality throughout or you will crash on a field that vanishes. Promotion from `ea` to `v1` does not break existing EA integrations — the old path keeps working.

**Response shapes drift with console firmware.** The docs warn that `userData` and `reportedState` vary by UniFi OS / Network Server version, and `meta`/`statistics` vary by Network version. In a mixed-firmware fleet — which is every real fleet — parse defensively.

---

## Surface 2 — UniFi Connector Proxy (Cloud → Local)

### Overview

Not a separate API so much as a transport. The Connector Proxy lets a **cloud** API key execute **local** Integration API calls by tunneling them through `api.ui.com` down to the console over the connection the console already maintains for remote management. No port forwarding, no VPN, no dynamic DNS. It works behind CGNAT.

This is what makes "cloud vs. local" mostly a false choice.

### Base URL and authentication

```text
Local:   https://{console}/proxy/{app}/integration/v1/{path}
Proxied: https://api.ui.com/v1/connector/consoles/{consoleId}/{app}/integration/v1/{path}
```

The segment after `/integration/v1` is identical in both modes. Authentication is `X-API-KEY` with a **cloud** key from unifi.ui.com (not the console-local key).

Examples:

```text
GET /v1/connector/consoles/{id}/network/integration/v1/sites
GET /v1/connector/consoles/{id}/protect/integration/v1/meta/info
```

### Requirements and access rules

- Console firmware **≥ 5.0.3**
- Console must be cloud-adopted and reachable by Ubiquiti's infrastructure
- A **personal** API key reaches only consoles owned by the key's account
- An **organization** API key reaches any console in the organization

### Official docs

[developer.ui.com/network/v10.4.57/connectorget](https://developer.ui.com/network/v10.4.57/connectorget) (Network) · equivalent page under each application's docs

### Gotchas

**TLS gets easier, not harder.** In proxy mode every request terminates at `api.ui.com`, which presents a publicly-trusted certificate. Leave certificate verification **on**. This is one of the few places where the cloud path is strictly more secure than the local one.

**You inherit the cloud rate limit.** Local calls are effectively unmetered; the same calls through the proxy count against 10,000/min. High-frequency polling that is fine locally will get throttled proxied.

**Console ID is not host ID is not site ID.** Three different identifiers, and mixing them produces unhelpful errors. Resolve the console ID from `/v1/hosts` first.

**Design your client to swap base URLs, not code paths.** Because the suffix is identical, the correct abstraction is a single client with a configurable prefix. Art of WiFi's PHP client models this well with `enable_site_manager_proxy($console_id, $key)` / `disable_site_manager_proxy()`.

**Availability is a hard dependency.** If Ubiquiti's cloud is down, or the console loses its outbound connection, the proxy is down. Anything safety-relevant needs a local fallback.

---

## Surface 3 — UniFi Network Integration API (Local)

### Overview

The flagship local API and the largest official surface by a wide margin — around 60 documented paths in Network 10.4. Introduced in Network 9.0 (January 2025) as a read-and-restart API, it has since grown into a genuine configuration API covering networks, WLANs, zone-based firewall policy, ACLs, DNS policy, and switching topology.

If you are automating a UniFi network from inside that network, this is the surface.

### Base URL and authentication

```text
https://{console}/proxy/network/integration/v1/...
```

Port 443 on a UniFi OS console (UDM, UCG, UDR, Cloud Key Gen2); port 11443 on UniFi OS Server.

```text
X-API-KEY: <key>
```

Generate at **UniFi Network → Settings → Control Plane → Integrations → Create New API Key** (the exact menu path has moved between 8.x, 9.x, and 10.x — in some builds it is under Settings → Integrations directly). Shown once; revocable individually; tied to the generating admin account and its permissions.

### Endpoints

**Application and sites**

| Method | Path        | Purpose                          |
|--------|-------------|----------------------------------|
| `GET`  | `/v1/info`  | Application version and metadata |
| `GET`  | `/v1/sites` | Local sites on this console      |

**Devices**

| Method   | Path                                                                       | Purpose                                     |
|----------|----------------------------------------------------------------------------|---------------------------------------------|
| `GET`    | `/v1/pending-devices`                                                      | Discovered but unadopted devices            |
| `GET`    | `/v1/sites/{siteId}/devices`                                               | Adopted devices                             |
| `POST`   | `/v1/sites/{siteId}/devices`                                               | Adopt device(s)                             |
| `GET`    | `/v1/sites/{siteId}/devices/{deviceId}`                                    | One device in detail                        |
| `DELETE` | `/v1/sites/{siteId}/devices/{deviceId}`                                    | Forget / unadopt                            |
| `POST`   | `/v1/sites/{siteId}/devices/{deviceId}/actions`                            | Device action — body `{"action":"RESTART"}` |
| `POST`   | `/v1/sites/{siteId}/devices/{deviceId}/interfaces/ports/{portIdx}/actions` | Port action — PoE power-cycle               |
| `GET`    | `/v1/sites/{siteId}/devices/{deviceId}/statistics/latest`                  | CPU, memory, uptime, throughput             |

**Clients**

| Method | Path                                            | Purpose                                  |
|--------|-------------------------------------------------|------------------------------------------|
| `GET`  | `/v1/sites/{siteId}/clients`                    | Connected clients (wired, wireless, VPN) |
| `GET`  | `/v1/sites/{siteId}/clients/{clientId}`         | One client                               |
| `POST` | `/v1/sites/{siteId}/clients/{clientId}/actions` | Client action — guest authorization      |

**Networks and WiFi**

| Method               | Path                                                   | Purpose                                                   |
|----------------------|--------------------------------------------------------|-----------------------------------------------------------|
| `GET` `POST`         | `/v1/sites/{siteId}/networks`                          | List / create networks (VLANs)                            |
| `GET` `PUT` `DELETE` | `/v1/sites/{siteId}/networks/{networkId}`              | Read / replace / delete a network                         |
| `GET`                | `/v1/sites/{siteId}/networks/{networkId}/references`   | What else references this network — check before deleting |
| `GET` `POST`         | `/v1/sites/{siteId}/wifi/broadcasts`                   | List / create SSIDs                                       |
| `GET` `PUT` `DELETE` | `/v1/sites/{siteId}/wifi/broadcasts/{wifiBroadcastId}` | Read / replace / delete an SSID                           |

**Firewall, ACLs, DNS, traffic matching**

| Method                       | Path                                                      | Purpose                                                    |
|------------------------------|-----------------------------------------------------------|------------------------------------------------------------|
| `GET` `POST`                 | `/v1/sites/{siteId}/firewall/zones`                       | List / create custom zones                                 |
| `GET` `PUT` `DELETE`         | `/v1/sites/{siteId}/firewall/zones/{firewallZoneId}`      | Manage a zone                                              |
| `GET` `POST`                 | `/v1/sites/{siteId}/firewall/policies`                    | List / create zone-based policies                          |
| `GET` `PUT` `PATCH` `DELETE` | `/v1/sites/{siteId}/firewall/policies/{firewallPolicyId}` | Manage a policy (`PATCH` for partial, e.g. enable/disable) |
| `GET` `PUT`                  | `/v1/sites/{siteId}/firewall/policies/ordering`           | Read / rewrite evaluation order                            |
| `GET` `POST`                 | `/v1/sites/{siteId}/acl-rules`                            | List / create switch ACL rules                             |
| `GET` `PUT` `DELETE`         | `/v1/sites/{siteId}/acl-rules/{aclRuleId}`                | Manage an ACL rule                                         |
| `GET` `PUT`                  | `/v1/sites/{siteId}/acl-rules/ordering`                   | Read / rewrite ACL order                                   |
| `GET` `POST`                 | `/v1/sites/{siteId}/dns/policies`                         | List / create DNS policies (content filtering, blocklists) |
| `GET` `PUT` `DELETE`         | `/v1/sites/{siteId}/dns/policies/{dnsPolicyId}`           | Manage a DNS policy                                        |
| `GET` `POST`                 | `/v1/sites/{siteId}/traffic-matching-lists`               | Reusable match lists referenced by policies                |
| `GET` `PUT` `DELETE`         | `/v1/sites/{siteId}/traffic-matching-lists/{id}`          | Manage a match list                                        |

**Switching topology**

| Method | Path                                                    | Purpose                   |
|--------|---------------------------------------------------------|---------------------------|
| `GET`  | `/v1/sites/{siteId}/switching/lags` · `/lags/{lagId}`   | Link aggregation groups   |
| `GET`  | `/v1/sites/{siteId}/switching/mc-lag-domains` · `/{id}` | Multi-chassis LAG domains |
| `GET`  | `/v1/sites/{siteId}/switching/switch-stacks` · `/{id}`  | Switch stacks             |

**Hotspot**

| Method       | Path                                              | Purpose                        |
|--------------|---------------------------------------------------|--------------------------------|
| `GET` `POST` | `/v1/sites/{siteId}/hotspot/vouchers`             | List / generate guest vouchers |
| `GET`        | `/v1/sites/{siteId}/hotspot/vouchers/{voucherId}` | One voucher                    |
| `DELETE`     | `/v1/sites/{siteId}/hotspot/vouchers`             | Bulk delete                    |
| `DELETE`     | `/v1/sites/{siteId}/hotspot/vouchers/{voucherId}` | Delete one                     |

**Supporting reference data**

| Method | Path                                          | Purpose                           |
|--------|-----------------------------------------------|-----------------------------------|
| `GET`  | `/v1/sites/{siteId}/wans`                     | WAN interfaces                    |
| `GET`  | `/v1/sites/{siteId}/vpn/site-to-site-tunnels` | Site-to-site VPN tunnels          |
| `GET`  | `/v1/sites/{siteId}/vpn/servers`              | VPN servers                       |
| `GET`  | `/v1/sites/{siteId}/radius/profiles`          | RADIUS profiles                   |
| `GET`  | `/v1/sites/{siteId}/device-tags`              | Device tags                       |
| `GET`  | `/v1/dpi/categories` · `/v1/dpi/applications` | DPI taxonomy for policy authoring |
| `GET`  | `/v1/countries`                               | Country codes for geo rules       |

### Pagination, filtering, and response shape

Every list endpoint takes `offset` (default 0), `limit` (default **25**, max **200**), and `filter`. Responses wrap results as:

```json
{ "offset": 0, "limit": 25, "count": 25, "totalCount": 312, "data": [ ... ] }
```

`count` is this page; `totalCount` is the match total. Drive your paging loop from those two.

The **filter syntax is unusual and catches everyone**. It is neither `field=value` nor OData. It is method-call style, composed with logical functions:

```text
macAddress.eq('00:1a:2b:3c:4d:5e')
and(not(ipAddress.eq('192.168.1.5')), not(ipAddress.eq('192.168.1.10')))
```

Operator availability is per-property-type: enums and strings get `eq`, `ne`, `in`, `notIn`; numeric and date properties add `gt`, `lt`, `ge`, `le`; text properties add `like`. Each endpoint's doc page has a *Filterable properties* section listing exactly which fields accept which operators — filtering on anything else errors.

### Official docs

[developer.ui.com/network/v10.4.57/gettingstarted](https://developer.ui.com/network/v10.4.57/gettingstarted) · [llms.txt](https://developer.ui.com/network/v10.4.57/llms.txt) · [openapi.json](https://developer.ui.com/network/v10.4.57/openapi.json) · [Ansible quick start](https://developer.ui.com/network/v10.4.57/quick_start.ansible) · your console's own copy at **Settings → Control Plane → Integrations**

### Gotchas

**`siteId` is a UUID, not `default`.** Every private-API tutorial on the internet uses the short site name `default`. The Integration API does not accept it. *Navigate it:* call `GET /v1/sites` first and cache the UUID. This is the single most common first-request failure.

**Not available on the legacy self-hosted software controller.** The Integration API requires a UniFi OS console or UniFi OS Server. A classic controller on 8443 has the private API and nothing else. *Navigate it:* if you must support legacy controllers, you need a session-auth fallback path; there is no way around it.

**Self-signed certificates fail verification by default.** Consoles ship self-signed certs and are reached by IP. Every client library disables verification for local connections. *Navigate it:* disable verification for local, keep it enabled for `api.ui.com`, and if you care, install a real certificate on the console and pin to the hostname.

**The surface is a strict subset of the private API.** Still missing at the time of writing: network events, alarms, DPI *statistics* (the taxonomy is exposed; the per-client counters are not), speed tests, port profiles, and routing tables. *Navigate it:* build a hybrid client with an explicit "legacy fallback" seam, and re-evaluate the gap on every Network release — items have been closing steadily.

**Filter values must be URL-encoded, and curl needs `-g`.** The syntax contains parentheses, quotes, and colons. Ubiquiti's own examples pass `-g` to disable curl's glob parsing. *Navigate it:* encode the whole filter value; in curl, always `-g`.

**MAC address filters appear to be case-sensitive.** Uppercase MACs do not match. Normalize to lowercase before filtering.

**Endpoints get removed, not just added.** Network 10 removed endpoints that existed in 9.x. Zone-pair firewall semantics and WPA3-transition handling also changed. *Navigate it:* pin to the doc version matching your firmware and test upgrades against a staging console.

**Guest pre-authorization is impossible.** A client must be connected and visible before it can be authorized by `clientId`, so you cannot authorize a device in advance of it joining. *Navigate it:* the captive-portal flow (see *Side Channels* below) is the supported pattern.

**API keys carry the creating admin's permissions and cannot be scoped read-only.** A widely-reported complaint since launch. *Navigate it:* create a dedicated admin with the narrowest role the console allows and generate the key as that admin.

**The auth header is `X-API-KEY`.** Some community examples show `Authorization: Bearer`. The official docs consistently specify `X-API-KEY` for all six portal APIs; Bearer is the Access API's convention (Surface 5). If you see 401 with a syntactically valid key, check the header name first.

---

## Surface 4 — UniFi Protect Integration API (Local)

### Overview

The official local API for UniFi Protect, introduced in Protect 5.3 and substantially expanded since. It covers the full Protect device catalog — cameras, lights, sensors, sirens, speakers, chimes, relays, fobs, bridges, link stations, alarm hubs, viewers — plus live views, arm profiles, snapshots, RTSPS stream management, and two WebSocket subscriptions for realtime device and event updates.

It is the correct starting point for new Protect integrations. It is also, as of today, meaningfully less complete than the private API it is replacing, and that gap is the dominant theme of developer feedback.

### Base URL and authentication

```text
https://{console}/proxy/protect/integration/v1/...
```

```text
X-API-KEY: <key>
```

Generate at **UniFi Console → Settings → Control Plane → Integrations**. On current consoles this is a single console-wide key page rather than a per-application one.

### Endpoints

**Application**

| Method | Path            | Purpose                          |
|--------|-----------------|----------------------------------|
| `GET`  | `/v1/meta/info` | Application version and metadata |
| `GET`  | `/v1/nvrs`      | NVR / recording-engine details   |

**Realtime (WebSocket)**

| Method | Path                    | Purpose                                                   |
|--------|-------------------------|-----------------------------------------------------------|
| `GET`  | `/v1/subscribe/devices` | Stream of device state updates                            |
| `GET`  | `/v1/subscribe/events`  | Stream of Protect events (motion, smart detections, ring) |

**Cameras**

| Method                | Path                                                            | Purpose                                       |
|-----------------------|-----------------------------------------------------------------|-----------------------------------------------|
| `GET`                 | `/v1/cameras` · `/v1/cameras/{id}`                              | List / read cameras                           |
| `PATCH`               | `/v1/cameras/{id}`                                              | Update camera settings                        |
| `GET`                 | `/v1/cameras/{id}/snapshot`                                     | Current JPEG snapshot                         |
| `POST` `GET` `DELETE` | `/v1/cameras/{id}/rtsps-stream`                                 | Create / read / delete RTSPS stream endpoints |
| `POST`                | `/v1/cameras/{id}/talkback-session`                             | Open a two-way audio session                  |
| `POST`                | `/v1/cameras/{id}/disable-mic-permanently`                      | Irreversibly disable the microphone           |
| `POST`                | `/v1/cameras/{id}/ptz/goto/{slot}`                              | Move PTZ to a preset                          |
| `POST`                | `/v1/cameras/{id}/ptz/patrol/start/{slot}` · `/ptz/patrol/stop` | Start / stop PTZ patrol                       |

**Other device classes** — all follow `GET /v1/{class}`, `GET /v1/{class}/{id}`, `PATCH /v1/{class}/{id}`:

`lights`, `sensors`, `sirens`, `speakers`, `chimes`, `relays`, `fobs`, `bridges`, `link-stations`, `alarm-hubs`, `viewers`

with these device-specific actions:

| Method | Path                                             | Purpose                  |
|--------|--------------------------------------------------|--------------------------|
| `POST` | `/v1/sirens/{id}/play` · `/stop` · `/test-sound` | Siren control            |
| `POST` | `/v1/speakers/{id}/test-sound`                   | Speaker test             |
| `POST` | `/v1/relays/{id}/outputs/{outputId}/activate`    | Fire a relay output      |
| `POST` | `/v1/alarm-hubs/{id}/outputs/{outputId}/trigger` | Fire an alarm-hub output |

**Alarm system**

| Method           | Path                                   | Purpose                                                                                                       |
|------------------|----------------------------------------|---------------------------------------------------------------------------------------------------------------|
| `GET` `POST`     | `/v1/arm-profiles`                     | List / create arm profiles                                                                                    |
| `PATCH` `DELETE` | `/v1/arm-profiles/{id}`                | Update / delete a profile                                                                                     |
| `PATCH`          | `/v1/arm-profiles/settings`            | Set the active profile                                                                                        |
| `POST`           | `/v1/arm-profiles/enable` · `/disable` | Arm / disarm                                                                                                  |
| `POST`           | `/v1/alarm-manager/webhook/{id}`       | Fire an Alarm Manager webhook trigger — the inbound hook that lets external systems drive Protect automations |

**Live views, users, files, POS**

| Method        | Path                                   | Purpose                                                     |
|---------------|----------------------------------------|-------------------------------------------------------------|
| `GET` `POST`  | `/v1/liveviews`                        | List / create live views                                    |
| `GET` `PATCH` | `/v1/liveviews/{id}`                   | Read / update a live view                                   |
| `GET`         | `/v1/users` · `/v1/users/{id}`         | Protect users                                               |
| `GET`         | `/v1/ulp-users` · `/v1/ulp-users/{id}` | UniFi Identity users                                        |
| `POST` `GET`  | `/v1/files/{fileType}`                 | Upload / retrieve device asset files (chime tones, etc.)    |
| `POST`        | `/v1/pos/cameras/{id}/transactions`    | Overlay point-of-sale transaction data onto a camera stream |

### Official docs

[developer.ui.com/protect/v7.2.105/gettingstarted](https://developer.ui.com/protect/v7.2.105/gettingstarted) · [llms.txt](https://developer.ui.com/protect/v7.2.105/llms.txt) · [openapi.json](https://developer.ui.com/protect/v7.2.105/openapi.json) · [Ansible quick start](https://developer.ui.com/protect/v7.2.105/quick_start.ansible)

### Gotchas

**There is no `bootstrap` equivalent, and this is the big one.** The private API's `/proxy/protect/api/bootstrap` returns the entire controller state — every device, every setting — in one call. The Integration API has no such endpoint; you assemble the same picture from a dozen per-class list calls. For a large installation that is a substantial increase in request count and a substantial rewrite for anyone porting. *Navigate it:* fetch each device class once at startup, then maintain state from the `/v1/subscribe/devices` WebSocket rather than re-polling. Treat the initial fan-out as a cold-start cost.

**Fields are missing relative to the private API.** Confirmed omissions include RTSP URLs on `GET /v1/cameras` (they require a separate `rtsps-stream` call), camera privacy zones, and chime volume — where `ringSettings` returns an empty array despite the docs describing a `volume` attribute. *Navigate it:* for these specific fields, fall back to the private API, or use a hybrid client such as `uiprotect` that maintains both paths. Re-check on each Protect release.

**`/snapshot` takes no timestamp.** The private API allowed requesting a frame at a given moment; the Integration API returns "now," and "now" is approximate — it can be several seconds off, and cached frames have historically refreshed only every 10–15 seconds. *Navigate it:* if you need a specific frame, pull from the RTSPS stream. If you need a *fresh* frame, enabling Anonymous Snapshots per-camera bypasses the cache.

**404 on event media is normal for long events.** Fetching thumbnails or heatmaps for an event still in progress — anything outlasting your retry window — reliably returns 404. *Navigate it:* treat 404 on media as "not ready," retry with backoff, and widen the window rather than logging it as an error.

**Event query parameters are all-or-nothing.** With no `limit`/`start`/`end` you get the last 24 hours. Supply `start` and you must also supply `end` or `limit`, and vice versa. Partial parameter sets error.

**The WebSocket needs priming, and priming has a race.** Frames arriving between "subscribe" and "initial snapshot loaded" will be lost if you don't buffer them. *Navigate it:* subscribe **first**, buffer incoming frames, then fetch the snapshot, then replay the buffer onto it. Re-run the whole sequence on every reconnect.

**Realtime data is eventually consistent.** A freshly-enrolled user can arrive as `ulp_user_not_cached`; a not-yet-known device can arrive with `device_mac: null`. Both resolve on the next resync. *Navigate it:* treat unknown identities as pending, not as errors, and re-resolve after the next bootstrap refresh.

**Smart detections need debouncing.** Thumbnails for a vehicle or person detection stream in over several seconds; a new event can cancel a pending one; and Protect occasionally emits updated data *after* an event has already fired, producing a duplicate. *Navigate it:* a resetting ~3-second debounce timer per event, with a dedupe on event ID.

**API keys are effectively console-wide and owner-only.** Creating a key requires super-admin or owner credentials, and the resulting key grants access to Protect *and* Network, bypassing per-user restrictions. A read-only Protect user's key can still reboot devices. This has generated sustained criticism, including formal issues in Home Assistant. *Navigate it:* there is no clean fix today. Minimize what holds the key, rotate on any suspicion, and keep the console off the open internet.

**The UI location for key creation keeps moving, and on some firmware it isn't there at all.** Multiple reports of consoles with no Integrations tab under Control Plane even for the owner account. *Navigate it:* update the console firmware; the tab is version-gated.

**Version-gate your features.** Minimum useful version for current clients is Protect 7.1.0; individual capabilities landed in specific releases and the portal documents which. Early Access and RC builds are unsupported.

---

## Surface 5 — UniFi Access Developer API (Local)

### Overview

The oldest officially-documented UniFi API and the odd one out. It predates developer.ui.com, is documented as a downloadable PDF generated from the Access application, listens on its own port, uses Bearer tokens rather than `X-API-KEY`, and returns a different response envelope. It is also the most complete of the official APIs relative to its application — it covers essentially everything the Access UI can do.

It is a physical-access-control API: users, visitors, credentials (NFC, PIN, QR, Touch Pass, license plate), access policies with schedules and holiday groups, door and door-group topology, remote unlock, emergency states, device settings, system logs, UniFi Identity resource assignment, and both WebSocket and webhook event delivery.

### Base URL and authentication

```text
https://{console}:12445/api/v1/developer/...
```

```text
Authorization: Bearer <API_TOKEN>
```

Create the token at **UniFi Console → Access → Settings → General → Advanced → API Token → Create New**, choosing a name, a **validity period**, and **permission scopes**. The same screen offers the API documentation PDF download. The token is displayed once.

Note that Access tokens, unlike the portal APIs' keys, are genuinely scoped — each endpoint documents a required permission key (e.g. `view:device`, `view:webhook`). This is the best authorization model in the UniFi API family.

### Endpoints

Paths are relative to `/api/v1/developer`.

**Users and groups**

| Path                                                                    | Operations                                      |
|-------------------------------------------------------------------------|-------------------------------------------------|
| `/users` · `/users/{id}`                                                | Register, fetch, fetch-all, update, delete      |
| `/users/search`                                                         | Search users                                    |
| `/users/{id}/access_policies`                                           | Assign / fetch a user's access policies         |
| `/users/{id}/nfc_cards` · `/nfc_cards/delete`                           | Assign / unassign NFC cards                     |
| `/users/{id}/pin_codes`                                                 | Assign / unassign PIN codes                     |
| `/users/{id}/license_plates` · `/{plateId}`                             | Assign / unassign license plates                |
| `/users/{id}/touch_passes/{touchPassId}` · `/users/touch_passes/assign` | Assign, unassign, batch-assign Touch Passes     |
| `/users/{id}/avatar`                                                    | Upload profile picture                          |
| `/user_groups` · `/user_groups/{id}`                                    | Create, fetch, fetch-all, update, delete groups |
| `/user_groups/{id}/users` · `/users/all` · `/users/delete`              | Group membership                                |
| `/user_groups/{id}/access_policies`                                     | Group policy assignment                         |

**Visitors**

| Path                                                                        | Operations                               |
|-----------------------------------------------------------------------------|------------------------------------------|
| `/visitors` · `/visitors/{id}`                                              | Create, fetch, fetch-all, update, delete |
| `/visitors/{id}/nfc_cards` · `/pin_codes` · `/qr_codes` · `/license_plates` | Credential assignment and removal        |

**Access policies, schedules, holidays**

| Path                                        | Operations                               |
|---------------------------------------------|------------------------------------------|
| `/access_policies` · `/{id}`                | Create, update, delete, fetch, fetch-all |
| `/access_policies/schedules` · `/{id}`      | Create, update, fetch, fetch-all, delete |
| `/access_policies/holiday_groups` · `/{id}` | Create, update, delete, fetch, fetch-all |

**Credentials**

| Path                                                              | Operations                                              |
|-------------------------------------------------------------------|---------------------------------------------------------|
| `/credentials/pin_codes`                                          | Generate a PIN code                                     |
| `/credentials/nfc_cards/sessions` · `/{sessionId}`                | Start an NFC enrollment session, poll status, cancel it |
| `/credentials/nfc_cards/tokens` · `/{token}`                      | Fetch, fetch-all, update, delete NFC cards              |
| `/credentials/nfc_cards/import`                                   | Import third-party NFC card IDs                         |
| `/credentials/touch_passes` · `/{id}` · `/search` · `/assignable` | Touch Pass lifecycle and purchase                       |
| `/credentials/qr_codes/download/{visitorId}`                      | Download a visitor QR code image                        |

**Space — doors and door groups**

| Path                                    | Operations                                                                                         |
|-----------------------------------------|----------------------------------------------------------------------------------------------------|
| `/door_groups` · `/{id}` · `/topology`  | Door-group CRUD and topology tree                                                                  |
| `/doors` · `/doors/{id}`                | Fetch, fetch-all                                                                                   |
| `/doors/{id}/unlock` · `/remote_unlock` | Remote unlock (supports `actor_id`, `actor_name`, and `extra` for audit attribution since v3.3.21) |
| `/doors/{id}/lock_rule`                 | Set / fetch temporary locking rules                                                                |
| `/doors/settings/emergency`             | Set / fetch emergency lockdown or evacuation state                                                 |

**Devices, logs, identity, webhooks, server**

| Path                                                    | Operations                                                          |
|---------------------------------------------------------|---------------------------------------------------------------------|
| `/devices`                                              | Fetch devices                                                       |
| `/devices/{deviceId}/settings`                          | Fetch / update access-method settings                               |
| `/devices/{deviceId}/doorbell`                          | Trigger a doorbell (Intercom, Reader Pro; v4.0.10+)                 |
| `/devices/notifications`                                | **WebSocket** event stream (v1.20.11+, permission `view:device`)    |
| `/system/logs` · `/logs/export` · `/system/static/...`  | Query, export, and fetch log resources (thumbnails, video, avatars) |
| `/users/identity/invitations` · `/identity/assignments` | UniFi Identity invitations and resource assignment                  |
| `/webhooks/endpoints`                                   | List, add, update, delete webhook endpoints (v2.2.10+)              |
| `/api_server/certificates`                              | Upload / delete the HTTPS certificate for the API server            |

**Webhook events**

`access.doorbell.incoming` · `access.doorbell.completed` · `access.doorbell.incoming.REN` · `access.device.dps_status` · `access.door.unlock` · `access.device.emergency_status` · `access.unlock_schedule.activate` · `access.unlock_schedule.deactivate` · `access.temporary_unlock.start` · `access.temporary_unlock.end` · `access.visitor.status.changed`

(The last five require Access 3.3.10 or later.)

### Response envelope

```json
{ "code": "SUCCESS", "msg": "success", "data": {} }
```

Errors return a `CODE_*` string — `CODE_PARAMS_INVALID`, `CODE_AUTH_FAILED`, `CODE_ACCESS_TOKEN_INVALID`, `CODE_UNAUTHORIZED`, `CODE_RESOURCE_NOT_FOUND`, `CODE_OPERATION_FORBIDDEN`, and a long tail of domain-specific codes. Note the unusual use of **HTTP 402 "Request Failed"** for valid-but-rejected requests, distinct from 400.

### Official docs

[assets.identity.ui.com/unifi-access/api_reference.pdf](https://assets.identity.ui.com/unifi-access/api_reference.pdf) — also downloadable from the token creation screen inside the Access application, where it matches your installed version. Reverse-engineered coverage of the *private* Access API is at [hjdhjd/unifi-access](https://github.com/hjdhjd/unifi-access/blob/main/docs/access-api.md).

### Gotchas

**The API disappears if you migrate to Identity Enterprise.** Ubiquiti states this outright in the reference: "The API is not available after upgrading to Identity Enterprise." This is a one-way door and it will silently destroy an integration. *Navigate it:* confirm the deployment's Identity tier before committing to this surface.

**Minimum version is Access 1.9.1**, with individual features gated much later — WebSocket notifications at 1.20.11, webhook endpoints at 2.2.10, Touch Pass at 3.2.20, license plates and profile pictures at 3.3.10, attributed remote unlock at 3.3.21, doorbell triggering at 4.0.10. *Navigate it:* read the Change Logs chapter at the end of the PDF; it maps every feature to the release that introduced it.

**Port 12445 with a self-signed certificate.** Ubiquiti says so plainly: "The server certificate is self-generated and untrusted." Every curl example needs `--insecure`. *Navigate it:* upload a real certificate via `/api_server/certificates` if you need verified TLS — that endpoint exists precisely for this.

**Some deployments answer on the UniFi OS proxy path instead.** Multiple community reports of `:12445` + Bearer failing while `https://{console}/proxy/access/integration/v1/developer/...` on port 443 with `X-API-KEY` succeeds. This appears to be a newer, portal-aligned path shipping alongside the documented one. *Navigate it:* try `:12445` + Bearer first as documented; if you get 401 with a token you just created, retry against the proxy path with `X-API-KEY` before assuming the token is bad.

**Console-wide keys and Access keys are not interchangeable.** On recent consoles the Control Plane → Integrations page creates one key for Protect and Network; Access rejects it with a message that it belongs to Protect. Access tokens come from the Access application only.

**Token permission scopes exist but have been buggy.** Reports of tokens created with unlock scope that still could not trigger an unlock, and of tokens that all read as READ regardless of selected scope. *Navigate it:* test the exact operation immediately after creating a token, and if unlock fails, regenerate with the owner account before debugging your client.

**Do not expose 12445 to the internet.** The port is open on the LAN by default. Port-forwarding it publishes a door-unlock API. *Navigate it:* if remote access is genuinely required, restrict the firewall rule to a single static source address, and prefer a VPN.

---

## Surface 6 — UniFi Mobility API

### Overview

A small, focused API for the UniFi Mobile Router (UMR) line — cellular-backed routers deployed in vehicles, temporary sites, and anywhere a fixed WAN isn't available. It is organized around **workspaces** rather than sites, reflecting a different tenancy model from Network.

Read for inventory and client visibility; write for device, network, and wireless configuration.

### Base URL and authentication

Cloud-hosted alongside Site Manager: `https://api.ui.com/v1/mobility/...`, authenticated with `X-API-KEY` from unifi.ui.com.

### Endpoints

| Method | Path                                                               | Purpose                       |
|--------|--------------------------------------------------------------------|-------------------------------|
| `GET`  | `/v1/mobility/workspaces`                                          | List workspaces               |
| `GET`  | `/v1/mobility/workspaces/{workspaceID}/admins`                     | Workspace administrators      |
| `GET`  | `/v1/mobility/workspaces/{workspaceID}/devices`                    | Devices in a workspace        |
| `GET`  | `/v1/mobility/workspaces/{workspaceID}/devices/{deviceID}`         | One device                    |
| `GET`  | `/v1/mobility/workspaces/{workspaceID}/devices/{deviceID}/clients` | Clients attached to a device  |
| `PUT`  | `/v1/mobility/workspaces/{workspaceID}/devices/{deviceID}`         | Update device configuration   |
| `PUT`  | `.../devices/{deviceID}/network`                                   | Update network configuration  |
| `PUT`  | `.../devices/{deviceID}/wireless`                                  | Update wireless configuration |

### Official docs

[developer.ui.com/mobility/v1.0.0/getting-started](https://developer.ui.com/mobility/v1.0.0/getting-started) · [openapi.json](https://developer.ui.com/mobility/v1.0.0/openapi.json)

### Gotchas

**Configuration writes are `PUT`, not `PATCH`.** These are replace semantics, not merge semantics. *Navigate it:* `GET` the current configuration, mutate the object, and `PUT` it back whole. A partial `PUT` will drop settings.

**Workspaces are not sites.** Do not try to resolve a Mobility workspace ID from `/v1/sites`. The tenancy models are separate.

**v1.0.0 with no version history.** Unlike Network and Protect, there is only one published version, so there is no changelog to consult and no way to pin against an older contract. Expect the surface to move.

---

## Surface 7 — UniFi InnerSpace API

### Overview

InnerSpace is Ubiquiti's spatial/floor-plan layer — it maps a deployment onto building geometry. The API is entirely read-only and exists to export that spatial model into external systems: floor plans, the access points and switches placed on them, an inventory, and the plan image assets themselves.

Six endpoints, no writes. Useful for asset management, CAD/BIM integration, and generating documentation.

### Base URL and authentication

`https://api.ui.com/v1/...` with `X-API-KEY`.

### Endpoints

| Method | Path                             | Purpose                              |
|--------|----------------------------------|--------------------------------------|
| `GET`  | `/v1/project`                    | Project metadata                     |
| `GET`  | `/v1/floor_plans`                | Floor plans                          |
| `GET`  | `/v1/access_points`              | Access points with spatial placement |
| `GET`  | `/v1/switches`                   | Switches with spatial placement      |
| `GET`  | `/v1/inventory`                  | Full inventory                       |
| `GET`  | `/v1/assets/{planId}/{filename}` | Fetch a plan asset file (image)      |

### Official docs

[developer.ui.com/innerspace/v1.3.23/gettingstarted](https://developer.ui.com/innerspace/v1.3.23/gettingstarted) · [openapi.json](https://developer.ui.com/innerspace/v1.3.23/openapi.json)

### Gotchas

**Read-only by design.** There is no write path. Spatial edits happen in the InnerSpace application.

**`/assets/{planId}/{filename}` returns binary.** It is the only non-JSON endpoint in the surface — handle the content type explicitly rather than assuming JSON.

**Versioning moves fast relative to the others.** InnerSpace is at 1.3.23 while Mobility and Carrier Fabric sit at 1.0.0, and it has shipped through 1.2.x into 1.3.x rapidly. Pin your doc version.

---

## Surface 8 — UniFi Carrier Fabric API

### Overview

The newest and most commercially specific surface: subscriber lifecycle management for ISPs and WISPs running UniFi as their access network. Provision a subscriber, attach a service plan, bind a host, suspend for non-payment, resume on payment. Thirteen endpoints, tightly scoped to that workflow.

This is a billing-system integration API, not a network management API.

### Base URL and authentication

`https://api.ui.com/v1/carrier/...` with `X-API-KEY`.

### Endpoints

| Method         | Path                                   | Purpose                             |
|----------------|----------------------------------------|-------------------------------------|
| `GET` `POST`   | `/v1/carrier/subscribers`              | List / create subscribers           |
| `GET` `PATCH`  | `/v1/carrier/subscribers/{id}`         | Read / update a subscriber          |
| `PUT` `DELETE` | `/v1/carrier/subscribers/{id}/host`    | Bind / unbind the subscriber's host |
| `PUT`          | `/v1/carrier/subscribers/{id}/plan`    | Change service plan                 |
| `POST`         | `/v1/carrier/subscribers/{id}/suspend` | Suspend service                     |
| `POST`         | `/v1/carrier/subscribers/{id}/resume`  | Resume service                      |
| `GET`          | `/v1/carrier/service-plans` · `/{id}`  | List / read service plans           |

### Official docs

[developer.ui.com/carrier-fabric/v1.0.0/getting-started](https://developer.ui.com/carrier-fabric/v1.0.0/getting-started) · [openapi.json](https://developer.ui.com/carrier-fabric/v1.0.0/openapi.json)

### Gotchas

**`suspend` and `resume` are the highest-consequence calls in the entire UniFi API family.** They cut off a paying customer's internet. *Navigate it:* require an explicit confirmation step, log every call with the operator identity and the billing event that triggered it, and never wire them directly to an automated dunning process without a human review gate.

**Plan changes are `PUT`, subscriber edits are `PATCH`.** Inconsistent within one surface; read the spec rather than pattern-matching.

**v1.0.0, brand new, no version history.** Treat contracts as provisional and build with an anti-corruption layer between Carrier Fabric and your billing system.

---

## Surface 9 — The Legacy Network Controller API (Unofficial)

### Overview

The private API that the UniFi web console has always used. Undocumented, unsupported, unannounced — and complete. Every feature in the Network application is reachable here, because this is how the application itself works.

You need it today for network events, alarms, DPI statistics, speed tests, port profiles, and routing tables, none of which the Integration API exposes. You will keep needing it until Ubiquiti closes those gaps.

### Base URL and authentication

The path shape depends entirely on the controller flavor:

|               | **Legacy software controller** | **UniFi OS console**           | **UniFi OS Server**            |
|---------------|--------------------------------|--------------------------------|--------------------------------|
| Port          | 8443                           | 443                            | 11443                          |
| Login         | `POST /api/login`              | `POST /api/auth/login`         | `POST /api/auth/login`         |
| Site prefix   | `/api/s/{site}/`               | `/proxy/network/api/s/{site}/` | `/proxy/network/api/s/{site}/` |
| CSRF required | No                             | **Yes**                        | **Yes**                        |

Login posts `{"username": ..., "password": ...}` (legacy also accepts `remember`; newer builds use `rememberMe` and an empty `token`). The response sets a session cookie and, on UniFi OS, returns an `X-CSRF-Token` header. A `TOKEN` cookie is also set — it is a JWT whose payload contains the same CSRF value, which is why many clients extract it from the cookie rather than the header.

Every state-changing request on UniFi OS must carry `X-CSRF-Token`.

Responses are wrapped:

```json
{ "meta": { "rc": "ok" }, "data": [ ... ] }
```

Errors return `"rc": "error"` with a message such as `api.err.LoginRequired`.

### Endpoint groups

| Group      | Shape                               | Purpose                                                                             |
|------------|-------------------------------------|-------------------------------------------------------------------------------------|
| Status     | `GET /status`                       | Basic server info, **no auth required** — the cheapest reachability probe available |
| Self       | `GET /api/self` · `/api/self/sites` | Current admin and their sites                                                       |
| Statistics | `/api/s/{site}/stat/*`              | Health, events, alarms, sessions, connected clients, devices, DPI, routing          |
| Lists      | `/api/s/{site}/list/*`              | Configuration object listings                                                       |
| REST       | `/api/s/{site}/rest/*`              | CRUD over WLANs, firewall rules, port forwards, user groups, networks               |
| Commands   | `/api/s/{site}/cmd/{manager}`       | Imperative operations, dispatched by manager                                        |
| Updates    | `/api/s/{site}/upd/*`               | Object updates                                                                      |

**Command managers** — the `cmd` endpoints take a manager name and a `cmd` in the body:

| Manager   | Operations                                                         |
|-----------|--------------------------------------------------------------------|
| `stamgr`  | Client operations — block, unblock, kick, forget, authorize guest  |
| `devmgr`  | Device operations — adopt, restart, upgrade, locate, spectrum scan |
| `sitemgr` | Site administration — add, delete, move devices between sites      |
| `evtmgr`  | Event management — archive alarms                                  |
| `backup`  | Backup list, create, delete                                        |

### Official docs

None. Ubiquiti has never documented this API. The community references are:

- [Ubiquiti Community Wiki — UniFi Controller API](https://ubntwiki.com/products/software/unifi-controller/api) — the canonical map
- [Art-of-WiFi/UniFi-API-client](https://github.com/Art-of-WiFi/UniFi-API-client) — PHP, maintained since 2015, the most complete open implementation

### Gotchas

**The proxy prefix is the #1 source of 404s.** `/api/s/{site}/...` on a legacy controller; `/proxy/network/api/s/{site}/...` on UniFi OS. Detect the flavor at connect time — try `/api/auth/login` and fall back to `/api/login` — and set the prefix accordingly. Do not make it a user-configured setting; users get it wrong.

**Missing CSRF produces a silent 403, not a useful error.** The UniFi OS nginx layer drops the request before the controller sees it, so there is no `meta.msg` to read. PacketFence's issue #9107 documents the exact failure mode against `cmd/stamgr`. *Navigate it:* capture `X-CSRF-Token` from the login response (or decode it from the `TOKEN` JWT) and attach it to every POST, PUT, and DELETE. If a state change 403s while reads succeed, this is always the cause.

**MFA returns HTTP 499, which is not a standard status code.** Older builds return `499` with `meta.msg = "api.err.Ubic2faTokenRequired"`; newer ones return `499` with `{"code":"MFA_AUTH_REQUIRED"}`, a list of available authenticators, and a `UBIC_2FA=` cookie. *Navigate it:* resubmit the login with a `token` field containing the 6-digit code. For non-interactive automation, use a **local-only admin account**, which is exempt from MFA — this remains the standard workaround, at the cost of provisioning a credential outside your MFA policy.

**Sessions go stale and 401.** No documented session lifetime. *Navigate it:* detect `meta.rc == "error"` with `api.err.LoginRequired`, re-login transparently, and retry once. Build this into the client, not into every call site.

**Login attempts are rate-limited.** Repeated failed logins trigger "You've reached the login attempt limit." An aggressive re-login loop will lock you out. *Navigate it:* exponential backoff on auth failure, and never re-login on a non-auth error.

**There is no compatibility contract.** The surface changes between Network versions without notice, because it only has to satisfy Ubiquiti's own web client. *Navigate it:* pin controller versions in production, test upgrades against a staging console, and isolate private-API calls behind an interface so you can swap them for Integration API calls as the gaps close.

**The API does not support concurrent mutations.** Apply configuration changes sequentially; parallel writes produce inconsistent state.

---

## Surface 10 — Private Protect API and the Realtime Updates WebSocket (Unofficial)

### Overview

The private Protect API remains necessary for the fields the Integration API omits, and its realtime WebSocket is a genuinely different protocol worth understanding even if you never implement it by hand.

Two facts make it worth the trouble: `GET /proxy/protect/api/bootstrap` returns the **entire** controller state in one request, and the `updates` WebSocket carries a binary, deflate-compressed stream that is more bandwidth-efficient than JSON.

### Base URL and authentication

```text
https://{console}/proxy/protect/api/bootstrap
wss://{console}/proxy/protect/ws/updates
```

Cookie-based session auth only. **`/proxy/protect/api/bootstrap` does not accept `X-API-KEY`** — this is the specific incompatibility that forces hybrid clients to maintain two auth paths.

There is also a `system` WebSocket on UniFi OS carrying plain JSON, shared across applications rather than Protect-specific.

### The updates protocol

The `updates` WebSocket is binary because WebSockets are only a transport — each frame needs a self-describing header. The framing is:

- An **8-byte packet header** describing what follows, including a payload-size field in network byte order (big-endian) and a flag indicating whether the payload is deflated
- An **action frame** identifying the action (`add`, `update`) and the category
- A **data frame** carrying the payload, zlib-inflated first if the deflate flag is set

The correct architecture, and the one every mature client converges on, is: **controller state is a reducer over the packet stream, with periodic re-bootstrap as a permanent failsafe.** You do not poll; you fold updates into a snapshot and refresh the snapshot occasionally to correct drift.

### Reference implementations

- [hjdhjd/unifi-protect](https://github.com/hjdhjd/unifi-protect) — TypeScript; the definitive protocol documentation lives in its README, with the decoder in `src/protocol/packet.ts` and the event classifier in `src/protocol/events.ts`
- [uilibs/uiprotect](https://github.com/uilibs/uiprotect) — Python; powers the Home Assistant integration and supports hybrid public/private operation

### Gotchas

**Do not implement the binary protocol yourself.** The header layout is undocumented by Ubiquiti and has changed. Use one of the libraries above, or port their decoder rather than re-deriving it.

**Re-bootstrap on every reconnect.** Missed frames during a disconnect are gone. Treat reconnect as "discard state, re-bootstrap, resubscribe."

**Cookie auth means the MFA problem applies here too** — same local-admin workaround as the Network private API.

**Hybrid clients have a mode trap.** In `uiprotect`, a public-API-only client raises `PublicOnlyModeError` on `update()`, `authenticate()`, and `get_bootstrap()`. Decide up front whether you are public-only, private-only, or hybrid, and make the mode explicit in configuration.

**WebSockets require UniFi OS.** Older Cloud Key deployments had no WebSocket and required polling. Modern hardware is fine.

---

## Event and Telemetry Side Channels

Not REST APIs, but often the right answer — particularly for event-driven work where polling is the wrong shape.

### Alarm Manager webhooks (outbound)

UniFi's cross-application alerting engine, originally Protect-only and now present in Network as well. The model is **Trigger + Scope + Action**: pick an event, scope it to devices, clients, VLANs, or system-wide, then act via push notification, email, in-app automation, or **custom webhook**.

Configure at **Alarm Manager → Create Alarm → Add Action → Webhook → Custom Webhook**. Default method is `GET`; `POST` is available and is what you want for Home Assistant.

This is the cheapest path to event-driven automation and it covers things the APIs do not — line-crossing detections, for example, surface via Alarm Manager webhooks but not through the Home Assistant Protect integration.

The Protect Integration API's `POST /v1/alarm-manager/webhook/{id}` is the *inbound* counterpart: it lets an external system fire a Protect automation.

**Docs:** [UniFi Alarm Manager](https://help.ui.com/hc/en-us/articles/27721287753239-UniFi-Alarm-Manager-Customize-Alerts-Integrations-and-Automations-Across-UniFi) · [Send UniFi Protect Alerts to Web Services using Webhooks](https://help.ui.com/hc/en-us/articles/25478744592023-Send-UniFi-Protect-Alerts-to-Web-Services-using-Webhooks)

**Gotcha:** the default `GET` webhook carries little context. Switch to `POST` and confirm the payload shape empirically — it is not formally documented.

### System logs / SIEM export

System logs can be exported in **Common Event Format (CEF)** to an external syslog or SIEM collector. Configure at **Integration → System Logging / SIEM**, choose SIEM Server as destination, select categories (security, system, client activity), and supply the collector's IP and port.

The right tool for compliance retention and correlation. The wrong tool for real-time control.

**Docs:** [UniFi System Logs & SIEM Integration](https://help.ui.com/hc/en-us/articles/33349041044119-UniFi-System-Logs-SIEM-Integration)

### SNMP

Read-only device metrics for Zabbix, PRTG, Nagios, LibreNMS. Enable in Network settings, then load Ubiquiti's MIB files so numeric OIDs resolve to named metrics.

Worth using when you already have an SNMP-based NMS and want UniFi devices in the same pane of glass. Not worth introducing otherwise — the Integration API's `statistics/latest` endpoint gives richer data with less setup.

**Docs:** [SNMP Monitoring in UniFi Network](https://help.ui.com/hc/en-us/articles/33502980942615-SNMP-Monitoring-in-UniFi-Network)

### External Hotspot API (captive portal)

The supported pattern for running your own captive portal. The flow:

1. A client joins an SSID with **Hotspot → Captive Portal** enabled and is tagged `{"access": {"type": "GUEST", "authorized": false}}`
2. Any web request redirects it to your External Portal Server
3. Your server authenticates the user however it likes, then finds the client via `GET /v1/sites/{siteId}/clients?filter=macAddress.eq('aa:bb:cc:dd:ee:ff')` to obtain the `clientId`
4. Your server `POST`s the authorize action to `/v1/sites/{siteId}/clients/{clientId}/actions`, optionally with time, data, and rate limits
5. The client flips to `authorized: true`

**Docs:** [External Hotspot API for Authorization Clients](https://help.ui.com/hc/en-us/articles/31228198640023-External-Hotspot-API-for-Authorization-Clients)

**Gotchas:** you cannot pre-authorize — the client must be connected and visible first. Lowercase the MAC in the filter. And on a Cloud Gateway, `GET /v1/sites` returns exactly one site, so the lookup is trivial there but not on multi-site consoles.

---

## What Has No API

Worth stating explicitly so you don't go looking:

- **UniFi Talk** — no public API. VoIP configuration and call records are UI-only.
- **UniFi Connect** — no public API.
- **UniFi Drive** — no public API.
- **WiFiman** — a diagnostic client application, not a server-side surface.

For these, the private API is the only option, and coverage there is thin because these applications are newer and less reverse-engineered than Network and Protect.

---

## Cross-Cutting Gotchas

Six issues that will bite you regardless of which surface you choose.

**1. Key type and endpoint must match.** Cloud keys (`cloud-v1`, `cloud-ea` prefixes) do not authenticate against local endpoints, and console-local keys do not authenticate against `api.ui.com`. Access tokens work on neither. A 401 with a key you just created is nearly always this. *Navigate it:* name your keys after their scope and validate the pairing at client startup with a cheap call (`/v1/info` locally, `/v1/hosts` in the cloud).

**2. Keys are shown once and are irrecoverable.** Losing one means generating a new one and updating every consumer. *Navigate it:* write to your secret store as part of the creation ritual, never to a scratch file.

**3. Self-signed certificates on every local endpoint.** Consoles ship self-signed certs and are addressed by IP. Every client disables verification for local connections. *Navigate it:* make TLS verification a per-endpoint setting, not a global one — off for the local console, on for `api.ui.com`. Never ship a client that disables verification globally.

**4. Nothing supports concurrent mutation.** Configuration writes must be sequential across all surfaces. *Navigate it:* serialize writes behind a single-flight queue per console. Reads can be parallel.

**5. The official surface is version-gated in both directions.** Features appear in specific releases and occasionally disappear. *Navigate it:* call `/v1/info` (Network) or `/v1/meta/info` (Protect) at startup, record the version, and gate optional features on it. Pin doc versions to firmware.

**6. API keys cannot be scoped read-only, and inherit the creating admin's rights.** This is the most substantive open criticism of the official APIs and has produced formal issues in downstream projects. *Navigate it:* create a dedicated admin with the narrowest console role available and generate the key as that admin; rotate on any change of custody; keep the console off the public internet regardless.

---

## Choosing a Surface

| If you need…                                          | Use                                     | Why                                          |
|-------------------------------------------------------|-----------------------------------------|----------------------------------------------|
| Fleet inventory across many customers                 | Site Manager                            | Only surface that crosses console boundaries |
| WAN health / ISP SLA reporting                        | Site Manager ISP metrics                | No local equivalent with retention           |
| Deep control of a remote console                      | Connector Proxy                         | Local depth, no inbound network path         |
| Home automation, low latency, WAN-outage tolerance    | Network / Protect Integration (local)   | No cloud dependency                          |
| Firewall, VLAN, or DNS policy as code                 | Network Integration                     | Full CRUD plus ordering endpoints            |
| Camera events, alarm arming, PTZ                      | Protect Integration + `/v1/subscribe/*` | WebSockets beat polling                      |
| Door control, credentials, visitors                   | Access Developer API                    | Only surface that covers Access              |
| Event-driven triggers with minimal code               | Alarm Manager webhooks                  | No client to write at all                    |
| Events, alarms, DPI stats, speed tests, port profiles | Legacy private API                      | Not yet in the official surface              |
| Complete Protect state in one call, or RTSP URLs      | Private Protect API `bootstrap`         | The Integration API has no equivalent        |
| Compliance log retention                              | SIEM / CEF export                       | Purpose-built                                |
| ISP subscriber provisioning                           | Carrier Fabric                          | Purpose-built                                |
| Spatial / floor-plan export                           | InnerSpace                              | Purpose-built, read-only                     |
| Cellular router fleet                                 | Mobility                                | Separate workspace tenancy                   |

**The realistic architecture for a serious integration in 2026** is a client that speaks the Integration API by default, keeps a private-API fallback behind a narrow interface for the handful of gaps, maintains state from WebSocket subscriptions rather than polling, and swaps a base URL to move between local and proxied operation. That is more machinery than a single API would require — but it is the shape the platform currently rewards, and the fallback layer keeps shrinking with each release.

---

## Sources

**Official**

- [UniFi Developer Portal](https://developer.ui.com) and its [root service index](https://developer.ui.com/llms.txt)
- [Site Manager API — Getting Started](https://developer.ui.com/site-manager/v1.0.0/gettingstarted) · [Version Control](https://developer.ui.com/site-manager/v1.0.0/versioncontrol) · [Get ISP Metrics](https://developer.ui.com/site-manager/v1.0.0/getispmetrics) · [Query ISP Metrics](https://developer.ui.com/site-manager/v1.0.0/queryispmetrics)
- [Network API — Getting Started](https://developer.ui.com/network/v10.4.57/gettingstarted) · [endpoint index](https://developer.ui.com/network/v10.4.57/llms.txt) · [Connector Proxy](https://developer.ui.com/network/v10.1.84/connectorget)
- [Protect API — Getting Started](https://developer.ui.com/protect/v7.2.105/gettingstarted) · [endpoint index](https://developer.ui.com/protect/v7.2.105/llms.txt)
- [Mobility API](https://developer.ui.com/mobility/v1.0.0/getting-started) · [InnerSpace API](https://developer.ui.com/innerspace/v1.3.23/gettingstarted) · [Carrier Fabric API](https://developer.ui.com/carrier-fabric/v1.0.0/getting-started)
- [UniFi Access API Reference (PDF)](https://assets.identity.ui.com/unifi-access/api_reference.pdf)
- [Getting Started with the Official UniFi API](https://help.ui.com/hc/en-us/articles/30076656117655-Getting-Started-with-the-Official-UniFi-API)
- [UniFi Network 9.0 — Built to Scale](https://blog.ui.com/article/unifi-network-9-0-built-to-scale) (3 January 2025)
- [External Hotspot API for Authorization Clients](https://help.ui.com/hc/en-us/articles/31228198640023-External-Hotspot-API-for-Authorization-Clients) · [UniFi Alarm Manager](https://help.ui.com/hc/en-us/articles/27721287753239-UniFi-Alarm-Manager-Customize-Alerts-Integrations-and-Automations-Across-UniFi) · [System Logs & SIEM Integration](https://help.ui.com/hc/en-us/articles/33349041044119-UniFi-System-Logs-SIEM-Integration) · [SNMP Monitoring in UniFi Network](https://help.ui.com/hc/en-us/articles/33502980942615-SNMP-Monitoring-in-UniFi-Network)

**Community and reference implementations**

- [Ubiquiti Community Wiki — UniFi Controller API](https://ubntwiki.com/products/software/unifi-controller/api)
- [Art of WiFi — UniFi APIs: A Practical Guide](https://artofwifi.net/unifi-api) · [UniFi API Authentication: Local Admin vs. API Key vs. Site Manager](https://artofwifi.net/blog/unifi-api-authentication-local-admin-vs-api-key-vs-site-manager)
- [Art-of-WiFi/UniFi-API-client](https://github.com/Art-of-WiFi/UniFi-API-client) (private API, PHP) · [unifi-network-application-api-client](https://github.com/Art-of-WiFi/unifi-network-application-api-client) (official API, PHP)
- [hjdhjd/unifi-protect](https://github.com/hjdhjd/unifi-protect) — realtime updates protocol documentation · [hjdhjd/unifi-access](https://github.com/hjdhjd/unifi-access)
- [uilibs/uiprotect](https://github.com/uilibs/uiprotect) and [its Protect 5.3 official-API discussion](https://github.com/uilibs/uiprotect/discussions/442)
- [uchkunr/unifi-best-practices](https://github.com/uchkunr/unifi-best-practices) — cloud, local, and Connector Proxy integration reference
- [opastorello/unifi-api-docs](https://github.com/opastorello/unifi-api-docs) — daily OpenAPI mirror, [rendered](https://opastorello.github.io/unifi-api-docs/)
- [Home Assistant UniFi Protect integration](https://www.home-assistant.io/integrations/unifiprotect/) and its issue tracker, for API-key permission and version-gating reports
- [PacketFence issue #9107](https://github.com/inverse-inc/packetfence/issues/9107) — the canonical CSRF failure write-up
