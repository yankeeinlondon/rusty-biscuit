---
prompt: |-
    When scraping web pages, what variants should be considered in the scraper's request structure?

    - alternative browser-agents?
    - using a VPN to appear from different geographies?
    - using differnt viewport sizes to understand what changes?
    - what else would be a good dimension to measure variance from?
last_updated: 2026-06-03
---
When scraping web pages, the scraper's request structure should make variance explicit. A single URL can produce meaningfully different responses depending on who appears to be requesting it, where the request appears to originate, what device is implied, and which protocol or browser capabilities are advertised.

Useful variance dimensions include:

## Client Identity

Scrapers should support multiple client profiles, rather than only a single hard-coded user agent.

Consider varying:

- `User-Agent`
- browser family, such as Chrome, Safari, Firefox, Edge
- browser version
- operating system, such as macOS, Windows, Linux, iOS, Android
- device class, such as desktop, tablet, mobile
- bot identity, when intentionally identifying as a crawler

This is useful because some sites serve different HTML, redirects, scripts, consent flows, or blocking pages based on the apparent client.

## Geography

Geography can materially affect page content.

Consider varying:

- country
- region or state
- city or metro area
- IP address family
- residential, datacenter, mobile, or corporate network egress
- VPN or proxy provider

This helps detect region-specific pricing, availability, language, cookie banners, legal notices, redirects, CDN behavior, and compliance walls.

Use this carefully. The scraper should not be designed to bypass access controls, paywalls, or legal restrictions. Geography variation is best treated as a measurement dimension, not an evasion mechanism.

## Viewport

Viewport size should be part of the request profile when rendering pages through a browser engine.

Consider varying:

- width
- height
- device pixel ratio
- orientation
- mobile emulation
- touch support
- reduced motion preference
- color scheme preference, such as light or dark mode

This helps identify responsive layout differences, mobile-only content, lazy loading behavior, different navigation structures, and content hidden behind alternate UI paths.

## Language And Locale

Language and locale often change more than text labels.

Consider varying:

- `Accept-Language`
- browser locale
- timezone
- currency
- measurement system
- date and number formatting
- inferred market or store region

Locale can affect canonical URLs, product availability, prices, legal text, sorting, recommendations, and structured metadata.

## Authentication State

Pages may vary substantially by session state.

Consider varying:

- anonymous user
- authenticated user
- account type or subscription tier
- organization or tenant
- role or permission level
- new user versus returning user
- user preferences
- existing cookies

This dimension is important when scraping applications, dashboards, marketplaces, documentation portals, or account-specific pages.

## Cookie And Consent State

Cookie state should be modeled separately from authentication.

Consider varying:

- no cookies
- fresh visitor
- consent accepted
- consent rejected
- partial consent
- previous visit
- A/B test assignment
- dismissed modal state

Many pages serve different DOMs before and after consent, especially in regions with privacy regulations.

## Network And Protocol Shape

The request's network-level behavior can also influence responses.

Consider varying:

- HTTP version
- TLS fingerprint
- header order
- compression support
- cache headers
- referrer
- origin
- connection reuse
- request timing
- retry behavior

This is especially relevant when comparing raw HTTP fetching against browser-driven scraping. Some servers and bot defenses treat those as different clients even when the URL and user agent match.

## Rendering Capability

A scraper should distinguish between static fetching and browser rendering.

Consider varying:

- raw HTTP fetch
- browser-rendered page
- JavaScript enabled or disabled
- images enabled or disabled
- CSS enabled or disabled
- web fonts enabled or disabled
- storage APIs enabled or disabled
- service workers enabled or disabled
- ad blockers or content blockers

This reveals whether content is present in the initial HTML, injected by JavaScript, hidden behind client-side routing, or dependent on third-party services.

## Referrer And Navigation Path

Some sites produce different pages depending on how the user arrived.

Consider varying:

- direct visit
- search engine referrer
- internal navigation
- campaign link
- product/category path
- previous page in session
- deep link versus homepage journey

This can affect personalization, attribution banners, experiments, modals, and redirect behavior.

## Time

Time is a major variance dimension.

Consider varying:

- crawl time
- day of week
- local timezone
- seasonality
- sale or promotion windows
- cache age
- first request versus repeated request

Time-based variance matters for news, prices, inventory, rankings, ads, events, and rate-limited or cache-backed responses.

## Request Headers

The request structure should allow controlled header profiles.

Commonly useful headers include:

- `Accept`
- `Accept-Language`
- `Accept-Encoding`
- `Cache-Control`
- `Referer`
- `Origin`
- `DNT`
- `Sec-CH-UA`
- `Sec-CH-UA-Mobile`
- `Sec-CH-UA-Platform`
- `Sec-Fetch-*`

Headers should be treated as a coherent browser profile. Mixing impossible combinations, such as a Safari user agent with Chromium client hints, can make the scraper easier to detect and can produce unrepresentative results.

## Experiment Assignment

Modern sites often run experiments.

Consider recording or varying:

- A/B test cookies
- feature flag cookies
- personalization IDs
- campaign parameters
- recommendation seeds
- logged-out visitor IDs

The scraper should preserve enough state to explain why two otherwise identical requests returned different pages.

## Rate And Concurrency

Request behavior can change responses.

Consider varying or recording:

- request rate
- concurrency
- crawl depth
- burst pattern
- retry count
- backoff behavior
- session reuse

Aggressive crawling may trigger throttling, degraded responses, CAPTCHA pages, or temporary bans. These should be measured as outcomes, not hidden by retry logic.

## Good Request Profile Shape

A useful scraper request structure might model these as explicit fields:

```text
url
method
headers
body
client_profile
viewport_profile
locale_profile
geo_profile
auth_profile
cookie_profile
rendering_profile
network_profile
navigation_profile
time_profile
rate_profile
```

The key design point is that request variance should be structured and named. Avoid burying these choices inside ad hoc headers, global settings, or proxy configuration.

## Recommended Baseline Variants

For most scraping systems, start with a small matrix:

- desktop Chrome, anonymous, US, JavaScript rendered
- mobile Safari, anonymous, US, JavaScript rendered
- desktop Chrome, anonymous, one non-US geography, JavaScript rendered
- desktop Chrome, anonymous, US, raw HTML fetch
- desktop Chrome, consent accepted, US, JavaScript rendered
- desktop Chrome, consent rejected, US, JavaScript rendered

Then add more dimensions only when the target site or product question justifies them. The goal is not to crawl every possible combination, but to make meaningful variance observable and repeatable.
