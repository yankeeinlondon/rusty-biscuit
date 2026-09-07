---
prompt: "Your task is to research the \"Access\" service in Unifi as well as the \"Access app\" that supports this service.\n\n- discuss the goals and focus for \"Access\"\n- discuss the hardware products which fit under the \"Access\" service\n    - be sure to link to the [security cameras](./security-cameras.md) document to reference all security cameras \n    - but make sure that you include more than just security cameras in your overview of this service line\n- talk about the mobile Access app and how it is organized and what functionality it exposes\n    - focus on the latest version of the app\n        - be sure to specify the latest version of the app as of today\n    - also give a high level overview of how features and UI have changed over time in this app (giving discrete version numbers and/or dates where you can to help identify the period this historic variant was in use).\n- refer to the [web application](./web-app.md) document for discussion of how it is organized (including the Access aspects of the web app)."
last_updated: 2026-09-01
hash: ca4808a57802fd34-007ec4c7eb1e7594
---
I have what I need. Writing the document now.

---

# UniFi Access

UniFi Access is Ubiquiti's door-control platform: the application that decides who may open which door when, plus the hardware line — readers, controllers, locks, intercoms, and credentials — that enforces it. It launched around 2021 as a small two-SKU experiment (a hub and a card reader) and has grown into a full physical-access-control system with elevator and vehicle-gate control, retrofit hardware for existing wired installations, an eight-door enterprise controller, facial recognition, and Apple/Google Wallet credentials.

Access runs on a UniFi console under UniFi OS alongside Network, Protect, Talk, Connect, and Identity. That co-tenancy is not incidental — it is most of the product's argument. Access has no cameras of its own: door video comes from Protect. Access has no end-user app of its own: users unlock doors from **UniFi Endpoint**, the same app they use for one-click WiFi and VPN. Access has no telephony of its own: intercom calls route through Talk and SIP. What Access contributes is the door.

## What Access Is For

Four commitments define the product, and they are recognizably the same four that define [Protect](./protect.md).

**No licensing fees.** Ubiquiti's stated position for door access is identical to its camera position — "licensing is still $0". There is no per-door license, no per-credential charge, no per-admin seat, and no cloud subscription. The whole margin sits in the hardware. There is exactly one exception, and it matters: **Touch Pass**, the Apple/Google Wallet credential, is sold per pass after a one-year free trial. Everything else — cards, fobs, PINs, QR codes, face unlock, fingerprint, mobile unlock through the Endpoint app — is free forever.

**Everything decides locally.** Credential matching, schedule evaluation, and lockdown state all live on the console and, for cached credentials, on the hub itself. The cloud layer at `unifi.ui.com` is remote access and identity brokering, not an authorization path. A severed WAN link does not lock people out of the building — which is a much harder requirement for access control than it is for video, and it is why the hub-per-door architecture exists.

**One PoE cable per device.** Readers, hubs, intercoms, and viewers are all PoE-powered and self-discovering. The Gate Hub and Elevator Hub take PoE++ in and re-emit PoE to their readers, so a gate installation is a single home run. The Retrofit line goes further and reuses the RS-485 wiring already in the wall from a Wiegand or OSDP system.

**Scale from one door to a campus.** The same application runs a $129 Access Ultra on a single office door and an EAH-8 rack of eight-door controllers across a multi-building site, with Fabrics and IdP-backed login above that for multi-site estates.

> **On compliance.** The current hardware carries UL 294 Level I ratings (destructive attack, line security, endurance, standby power), CAN/ULC-60839-11-1 Grade I on the enterprise hub, and NDAA compliance throughout. Fail-safe lock hardware and an emergency input on every controller are what make Access viable on a fire-code egress path — this is the part of the catalog where the certifications, not the features, close the sale.

## The Hardware

### Readers and Entry Cameras

The reader is where the credential is presented, and Ubiquiti's range now spans a $139 tap pad to a $379 12 MP AI camera with a touchscreen. The line is stratified by generation (G2 → G3 → G6) and by how many credential types each device accepts.

| Product                     | SKU                            | Price (USD)            | Credentials accepted                        |
|-----------------------------|--------------------------------|------------------------|---------------------------------------------|
| G6 Pro Entry                | UVC-G6-Pro-Entry               | $379                   | Face, NFC, PIN, QR, Wallet, mobile          |
| G6 Entry                    | UVC-G6-Entry                   | From $249              | Face, NFC, PIN, Wallet, mobile              |
| G3 Reader Pro               | UA-G3-Pro                      | From $359              | NFC, PIN, Wallet, mobile + two-way intercom |
| G3 Reader Fingerprint       | UA-G3-Fingerprint              | Coming soon            | Fingerprint, NFC, PIN, Wallet               |
| G3 Reader Flex              | UA-G3-Flex                     | $199                   | NFC, PIN, Wallet, mobile                    |
| G3 Reader                   | UA-G3                          | From $139              | NFC                                         |
| G3 Intercom                 | UA-G3-Intercom                 | From $399              | Wallet + two-way intercom                   |
| Intercom Viewer             | UA-Intercom-Viewer             | From $199              | Indoor answering station                    |
| Access Ultra                | UA-Ultra                       | $129                   | NFC, mobile — **reader and hub in one**     |
| Retrofit Reader             | UA-Retrofit-Reader             | Sold with Retrofit Hub | NFC, Wallet, mobile over existing RS-485    |
| Retrofit Reader Fingerprint | UA-Retrofit-Reader-Fingerprint | Coming soon            | Fingerprint (300 prints), NFC, PIN          |

Three of these deserve more than a table row.

**The G6 Entry pair are cameras that happen to be readers**, and their SKUs say so — they carry the `UVC-` prefix of the Protect camera line, not the `UA-` prefix of Access. The G6 Pro Entry pairs a wide-angle 1/1.6" 12 MP main sensor with a dedicated 8 MP package-view camera, IR to 5 m, two-way audio, a multi-TOPS on-device AI engine for face recognition, and a 3" customizable touchscreen that can carry branding, a tenant directory, and PIN guest entry. The G6 Entry drops the touchscreen and steps the main sensor down to 5 MP at 30 fps (the 8 MP package camera runs at 5 fps), keeping face unlock, NFC, Wallet, IP55, IK07, and UL 294 Level I. **Both require the Protect application *and* the Access application *and* a separate Access Hub** before NFC, Face ID, or Interface Designer configuration will work — the reader is not a door controller. Both ship with ten free one-year Touch Passes.

Because these devices are also Protect cameras, their video specifications, IR behavior, PoE class, and doorbell siblings (Doorbell Lite at $99, the G4/G5 doorbell families) are documented in **[Security Cameras](./security-cameras.md)** rather than repeated here. That document is also where you find the LPR-capable cameras a Gate Hub needs and the general-purpose cameras you would point at a door to get unlock-event video into the System Log.

**Access Ultra is the shape of the low end.** At $129 it collapses reader and hub into one PoE+ device with a 12 V/1 A lock terminal, a request-to-exit input, a tamper switch, IP55 and a −30 °C to 40 °C range, and a 3,000-user cache. For one door with no exit reader, it is the whole system.

**The Retrofit line is the wedge into existing buildings.** The Retrofit Reader is a 13.3 mm-deep OSDP reader that talks to a Retrofit Hub over the RS-485 pair already running to the door from whatever Wiegand or OSDP system is being replaced. Paired with the PoE Over 2-Wire Retrofit Extender and the Retrofit PSU, it turns a rip-and-replace job into a swap of endpoints.

### Control Hubs

The hub is the door controller — it holds the relay, the credential cache, and the emergency logic. This is the component you size the deployment against.

| Product               | SKU               | Price (USD) | Doors                | Max users | Notes                                                                                                                                             |
|-----------------------|-------------------|-------------|----------------------|-----------|---------------------------------------------------------------------------------------------------------------------------------------------------|
| Enterprise Access Hub | EAH-8             | $999        | 8                    | 10,000    | Wall cabinet, 8 lock terminals (12 V or dry), 8+8 REX/DPS inputs, 8 PoE reader ports, 4 AUX terminals, 100–240 V AC, 32–48 V battery backup input |
| Gate Hub              | UA-Hub-Gate       | $279        | 1 gate + 1 side door | 6,000     | LPR unlock, 3 dry relays + 1 powered 12 V relay, PoE++ in / 4 PoE out                                                                             |
| Elevator Hub          | UA-Hub-Elevator   | Sold in kit | **18 floors**        | 6,000     | 18 relays, up to 4 readers/cameras total, PoE++ in / 4 PoE+ out                                                                                   |
| Retrofit Hub          | UA-Retrofit-Hub-2 | $229        | 1                    | —         | Reuses existing RS-485/OSDP wiring                                                                                                                |
| Door Hub              | UA-Hub-Door       | $199        | 1                    | —         | 4 RJ45 + PoE in; entry and exit readers                                                                                                           |
| Door Hub Mini         | UA-Hub-Door-Mini  | $129        | 1                    | —         | Entry reader only                                                                                                                                 |
| Access Ultra          | UA-Ultra          | $129        | 1                    | 3,000     | Reader and hub in one device                                                                                                                      |

**One hub per door is the default, and it is the main cost driver.** A five-door office needs five Door Hubs unless you centralize on an EAH-8, which is where the $999 enterprise hub earns its price: eight doors at $124.88 per door, plus battery backup and a single cabinet to secure. Ubiquiti's own guidance frames the choice as door count versus I/O requirements versus cable-run length — hubs near the door minimize cabling but multiply enclosures and tamper surface; a central hub is easier to maintain.

The **Elevator Hub** is the odd one out. It has eighteen relays because it interrupts eighteen call buttons. True floor-level restriction (rather than call-button gating) requires setting Elevator Position Encoding to Binary, BCD, or Gray against the elevator controller's car-position signals — which means coordinating with an elevator technician, and is the one Access deployment that is not a self-service install.

### Locks, Egress, and Door Hardware

This is the part of the catalog that is not electronics at all, and it is why Access can be bought as a complete job rather than a controller looking for a locksmith.

| Product                           | SKU                                  | Price (USD) | Notes                                                      |
|-----------------------------------|--------------------------------------|-------------|------------------------------------------------------------|
| Magnetic Lock                     | UA-Lock-Magnetic                     | From $149   | Fail-safe by nature — releases on power loss               |
| Electric Locks (strike and bolt)  | UACC-Lock-Strike-*, UACC-Lock-Bolt-* | From $89    | Fail-safe and fail-secure variants, 8 mm and 15 mm strikes |
| Panic Bar                         | UACC-PanicBar                        | $399        | Code-compliant egress on a controlled door                 |
| Door Closer                       | UACC-DoorCloser                      | $129        |                                                            |
| Access Button                     | UA-Button                            | $39         | Request-to-exit                                            |
| Access Rescue KeySwitch           | UA-Rescue                            | $79         | Mechanical override                                        |
| Junction Utility                  | UACC-Junction-Utility                | $119        |                                                            |
| Door Lock Relay Cable             | UACC-Cable-DoorLockRelay             | From $129   |                                                            |
| Retrofit PSU 12 V                 | UACC-Retrofit-PSU-12V                | $79         |                                                            |
| PoE Over 2-Wire Retrofit Extender | UACC-Retrofit-PoE-2Wire              | $99         | Reuses coax or low-voltage pair as a PoE run               |

Fail-safe versus fail-secure is the single decision that has legal consequences. Fail-safe hardware releases when power is cut and is required on most egress paths; fail-secure stays locked and is for doors where the fire code does not demand free egress. Ubiquiti sells both variants of both strike sizes and the bolt lock, which is unusual for a networking vendor and reflects how much of this line is aimed at whole-building installs rather than upgrades.

### Credentials

| Product                | SKU       | Price (USD)          | Notes                          |
|------------------------|-----------|----------------------|--------------------------------|
| Access Card            | UA-Card   | From $30             | 13.56 MHz NFC                  |
| Pocket Keyfob, 10-pack | UA-Pocket | $99                  |                                |
| Touch Pass             | —         | Per pass, in-console | Apple/Google Wallet credential |

Readers accept **all 13.56 MHz NFC protocols — ISO 14443A, ISO 14443B, and ISO 15693** — including MIFARE Classic, Plus, Ultralight, and DESFire, so existing third-party card stock generally works. Beyond cards the platform supports PIN, QR code, fingerprint, face unlock, license-plate unlock (Gate Hub plus an LPR camera), mobile unlock via the UniFi Endpoint app, and **hand wave** — which is an exit-only request-to-exit gesture, not an entry credential, and cannot be combined with other methods on the same reader.

**Touch Pass is the one thing you pay for again.** Each pass carries a one-year free trial, and the clock only starts when the user actually adds it to their Wallet — unassigned or un-added passes never expire. After that, passes are purchased individually in-console, with an auto-purchase option tied to Auto-Scaling. Ubiquiti does not publish a public per-pass price; it is shown on the Touch Pass page inside your own Access application. The credential itself is a genuine Apple Wallet pass: it works with the phone locked, survives about five hours of dead battery on the power reserve, is never stored on Apple's or Ubiquiti's servers, and can be remotely killed through Find My. Availability is country-gated and has been expanding steadily — Taiwan arrived in Access 4.0.21 (October 2025), Brazil in 4.2.27 (May 2026). Passes were migrated to UniFi Fabric during 2026, which required every user to re-add their pass in the Endpoint app.

### Kits

| Kit                  | SKU                                | Price (USD) | Contents                                                           |
|----------------------|------------------------------------|-------------|--------------------------------------------------------------------|
| Door Starter Kit Pro | UA-G3-SK-Pro                       | $599        | Door Hub + two readers — entry and exit control on one door        |
| Door Starter Kit     | UA-G3-SK                           | From $289   | Door Hub Mini + one reader                                         |
| Gate Starter Kit     | UA-SK-Gate / UA-G3-SK-Gate         | From $699   | Gate Hub + connected Intercom                                      |
| Elevator Starter Kit | UA-SK-Elevator / UA-G3-SK-Elevator | $999        | Elevator Hub, G2 Reader, 2 × 2-wire PoE extenders, 10 access cards |

For a first door, kit pricing beats buying the parts individually.

> **Pricing basis.** All figures are Ubiquiti US list prices on [store.ui.com](https://store.ui.com) as of **1 September 2026**, quoting the base list price rather than the higher "surcharge included" figure the US store also shows. The G3 Reader Fingerprint and Retrofit Reader Fingerprint were both listed as *Coming Soon* on that date, and the Elevator Starter Kit was showing sold out.

### What Access Does Not Ship

Two things a complete access-control system needs, and Access borrows rather than builds:

- **Cameras.** Access has no recorder and no camera of its own. Unlock-event video, live view at a door, and Recording Manager all come from Protect — see [Security Cameras](./security-cameras.md) for the camera range and [Protect](./protect.md) for the recorder sizing that constrains it. The G6 Entry readers are Protect cameras carrying an Access role, not the other way round.
- **The end-user experience.** Employees and tenants do not install the Access app. They install **UniFi Endpoint** (previously UniFi Identity Endpoint), which combines door unlock with one-click WiFi, VPN, camera sharing, EV charging, file access, and a softphone. Endpoint is licence-free. This split — admin app versus user app — is the single most common source of confusion about the product, and it means "the Access app" in the section below is unambiguously an *administrator's* tool.

## The Access Application

The mobile app is a client. What it can do is bounded by the version of the **Access application** running on the console, and several 2026 mobile features carry explicit server-version prerequisites (setting a custom access policy as the default needs Access 4.1.31+; hand-wave sensitivity needs UA G2 firmware 3.11.3.0+). When a mobile toggle appears to do nothing, a version mismatch is the usual cause.

The current shipping release is **Access 4.3.7, dated 26 August 2026**. Recent history:

| Version                     | Date                    | Theme                                                                                                                                                                                                                                                                                                                                                     |
|-----------------------------|-------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **4.3.x** (4.3.3 → 4.3.7)   | Jul–Aug 2026            | Bulk-import stability, NFC detail in exported logs, Touch Pass backup/restore. **4.3.5 and 4.2.29 are security releases** (Security Advisory Bulletins 067 and 066). 4.3.7 fixed NFC enrollment on Fabric-joined consoles, doorbell call scheduling, and SIP `*`-to-unlock in Direct Call Mode.                                                           |
| **4.2.x** (4.2.16 → 4.2.29) | Mar–Jun 2026            | **Double Badge** (badge twice to override a scheduled unlock), Smart Door Access without an installed reader, reader screen brightness and SIP account configuration in **Interface Designer**, reader live view in the Intercom Viewer, NFC card ID in the System Log, 7- and 14-day retention options, Touch Pass suspend/resume, Touch Pass in Brazil. |
| **4.1.x** (4.1.15 → 4.1.40) | Dec 2025 – Feb 2026     | NFC card CSV import, **non-admin remote unlock through the Endpoint app**, custom access policies assignable as default, Entry/Exit filters in the System Log, PoE power-usage display per hub, Unlock Door mode for EAH-8 exit-request terminals.                                                                                                        |
| **4.0.x** (4.0.21 → 4.0.31) | Oct–Nov 2025            | Granular Alarm Manager unlock triggers (per user, per unlock method), redesigned Access Control Hub terminal manager, TLS for third-party SIP, Touch Pass in Taiwan, ten new UI languages.                                                                                                                                                                |
| **3.4.23**                  | Aug 2025                | **Alarm Manager support** (requires UniFi OS 4.3.6+), UA Cards usable across multiple sites, **Kiosk Mode** for visitor self-check-in with badge printing.                                                                                                                                                                                                |
| **3.3.22**                  | Jul 2025                | Two-step authentication on Elevator and Gate hubs, third-party SIP server support, visitor SMS invitations, dual unlock buttons for double driveways, two-way live-view audio with Protect cameras, Emergency Lockdown overriding Protect's Alarm Manager unlock.                                                                                         |
| **3.2**                     | Apr 2025 (Early Access) | **Interface Designer** — branded, visual layouts pushed onto Reader Pro and Intercom screens — and **second-person / multi-factor authentication**, requiring two authorized users to unlock a door.                                                                                                                                                      |

The shape of that list is worth reading: the 2025 releases added *capabilities* (alarms, kiosks, SIP, dual authentication), and the 2026 releases have almost entirely added *administration at scale* — bulk CSV import, log export, retention policy, Fabric migration, and the plumbing to run doors from an identity provider rather than a spreadsheet.

## The Mobile App

### Current Version

As of **1 September 2026**:

| Platform              | Version    | Released    | Requirement                                   |
|-----------------------|------------|-------------|-----------------------------------------------|
| iOS / iPadOS          | **2.17.1** | 10 Jul 2026 | iOS 17.0 / iPadOS 17.0                        |
| macOS (Apple Silicon) | **2.17.1** | 10 Jul 2026 | macOS 14.0, M1 or later — runs the iPad build |
| Android               | **2.17.1** | 10 Jul 2026 | Android 7.0+                                  |

The iOS build is 293.8 MB, filed under Business and Utilities, published by Ubiquiti Inc., rated **4.92 from ~4,720 ratings**, and localized into 21 languages. There is no tvOS, visionOS, Windows, or Linux client; desktop access is the browser.

Two things stand out against Protect's mobile app. First, **iOS and Android are in lockstep** — same version number, same day — where Protect's two platforms have drifted a full minor version apart. Second, **the release cadence is much slower**: 2.17.1 shipped on 10 July 2026 and nothing has followed it in the seven weeks since, while Protect shipped three releases in the same window. Access is a mature, low-churn app; the development energy in this service line is going into the console application and the hardware, not the phone.

### How the App Is Organized

Ubiquiti publishes no consolidated navigation guide for this app; the structure below is assembled from release notes, help-center articles, and store copy. Five primary areas:

- **Dashboard** — the landing view. Locations and their doors, current door status, recent unlocks, pending-adoption prompts, and device-offline warnings. Locations are searchable (2.10.x) and groupable (2.11.0), with optional location thumbnails toggled under Account → App Preferences. This is also where a location's **Lock Now** control lives.
- **Locations and Doors** — the hierarchy Access is organized around. Doors belong to locations; access policies bind users and schedules to locations rather than to individual readers. Temporary Unlock and scheduled-unlock overrides are driven from here, with remaining unlock time displayed since 2.15.x.
- **Devices** — the adopted-device list and per-device configuration: greeting messages, broadcast name, keypad layout, volume, display brightness and dim duration, reader settings, receiver management on Reader Pro / G2 Reader Pro / Intercom, hub terminal configuration, and firmware. Device-list scrolling was specifically reworked in 2.14.1 for deployments over 100 devices.
- **System Log** — the audit trail, and the app's most-used screen after the Dashboard. Every unlock, denial, policy change, and admin action, filterable by date range and by Entry/Exit, with the associated Protect video inline. You can page to the previous or next video without leaving the log.
- **Settings** — Card Inventory (NFC enrollment, including scanning a card with the phone itself), Touch Pass assignment and purchase, Admins (added 2.15.x), Caller & Receivers, and General → Doorbell and Live View.

### What the App Actually Exposes

- **Doorbell and intercom handling.** Calls arrive as either a push notification or a native incoming call (2.8.0), with live video, two-way audio, and unlock-or-deny from the call screen. Receivers and call routing are managed per device; call schedules and directories are configured on the console.
- **Live view and door video.** Live feeds from Reader Pro, Intercom, and paired Protect cameras, with separately configurable session durations for manually initiated versus doorbell-triggered live view, and an admin switch to disable live-view initiation from the Endpoint app and Intercom Viewer entirely.
- **Credential management on the phone.** Enroll an NFC card by tapping it to the handset, assign or revoke it, and assign Touch Passes — the reason most administrators have this app installed at all.
- **Door control.** Remote unlock, Temporary Unlock with a visible countdown, Lock Now, and emergency lockdown.
- **Device commissioning.** Adoption, firmware, and per-reader interface settings, including hand-wave sensitivity (high or low, 2.16.1).
- **Policy touch-ups.** Assigning a custom access policy as the default and managing admins — deliberately shallow. Building policies, schedules, holidays, visitor workflows, Interface Designer layouts, and Alarm Manager rules are all console-side work.

The dividing line is consistent and worth stating plainly: **the phone is for operating and auditing a system somebody else configured in a browser.** That is a narrower remit than the Protect app has taken on, and it explains the slower release cadence.

### How the App Has Changed Over Time

**1.x — mid-2021 to mid-2024.** The app shipped alongside Access itself. Its App Store identifier places first submission in mid-2021, though Ubiquiti's own 1.0.0 release post carries no machine-readable date and the community release archive no longer reaches back that far. The 1.x app was an admin viewer: adopted devices, door status, and an event list. Notably, **it could not open a door** — unlocking from a phone was the Identity app's job from the start, and it has stayed that way.

**2.0 – 2.7 — roughly August 2024 to early 2025.** The app became genuinely operational. 2.0.0 put pending-adoption prompts and device-offline notifications on the Dashboard, added receiver management for the UA-Pro / UA-G2-Pro / UA-Intercom, and gave the system log a date-range filter. 2.3.0 (~November 2024) brought Dashboard features gated on Access 3.0.11, License Plate Unlock, and full Intercom support. 2.4.0 landed 24 December 2024, 2.5.1 on 27 January 2025.

**2.8 – 2.12 — May to October 2025.** The doorbell era. 2.8.0 introduced doorbell calls as either a notification *or* a native incoming call and reworked the Caller Manager; 2.8.1 and 2.8.2 cleaned up unlock-on-answer and Touch Pass. 2.9.0 shipped 26 June 2025. 2.10.0 added Dashboard location search, prev/next video navigation in the System Log, configurable screen-dim duration, and the admin switch to disable live-view initiation. 2.11.0 (26 August 2025) added location groups, Siren and Chime modes on the Door Hub's Door Operator terminal, and dual unlock buttons for the Gate Hub.

**2.13 – 2.17 — November 2025 to now.** Consolidation and scale. The theme is fleet management and internationalization rather than new capability.

| Version         | Date        | Change                                                                                                                                       |
|-----------------|-------------|----------------------------------------------------------------------------------------------------------------------------------------------|
| 2.13.1          | 13 Nov 2025 | Add receivers to Reader Pro / Intercom wired directly to a switch or console port; Catalan, Norwegian, Turkish, Greek, Portuguese (Portugal) |
| 2.13.2          | Nov 2025    | Two-way audio over WiFi, third-party NFC registration, crash fixes                                                                           |
| 2.14.1          | 16 Jan 2026 | Device-list scrolling for 100+ devices, landscape video playback, Arabic                                                                     |
| 2.15.0 / 2.15.1 | 12 Feb 2026 | **Temporary Unlock with remaining time**, custom access policy as default (Access 4.1.31+), **Admins under Settings**, Dashboard refresh     |
| 2.16.1          | 25 Mar 2026 | **Hand-wave sensitivity** high/low (UA G2 3.11.3.0+), NFC card enrollment fix                                                                |
| 2.16.2          | 14 May 2026 | UX pass; Lock Now button missing from location details fixed                                                                                 |
| 2.17.0          | Jul 2026    | Lock Now and video-playback-start fixes                                                                                                      |
| 2.17.1          | 10 Jul 2026 | Performance                                                                                                                                  |

The arc across all three eras is narrow by design: the app started as a way to see whether the doors were online, became a way to answer the doorbell, and is now a way to enroll a credential and audit an unlock from the parking lot. It has never become a configuration console, and Ubiquiti shows no sign of making it one.

> **A live defect worth knowing.** The 2.17.0 release thread carries unresolved reports of NFC card enrollment failing on iPhone 17 when scanning with the phone's own reader, while the same flow works on Android. Ubiquiti support is collecting logs. If you enroll cards from a recent iPhone, expect to fall back to a reader-side enrollment.

## The Web Application

Access's browser interface is where the system is actually built. Access policies and schedules, holiday calendars, visitor workflows and Kiosk Mode, Interface Designer layouts for reader screens, Alarm Manager rules, admin roles and permissions, Touch Pass purchasing, retention policy, SIP configuration, and bulk CSV import of users and cards are all console-side and have no mobile equivalent. It is reached through the console directly on the LAN or via `unifi.ui.com/consoles` for remote access, and it is one application among several — Network, Protect, Access, Talk, Connect, Identity — inside the same UniFi OS shell.

For how the web application is structured, how the console shell and the per-application views fit together, and the Access-specific surfaces within it, see **[Web Application](./web-app.md)**.

## Programmatic Access

Access exposes a local API on the console, with tokens generated under Security → Advanced → API Token. It has expanded steadily — Access 3.3.22 added custom unlock-action naming, license-plate assignment, user profile pictures, QR-code assignment, NFC bulk import, reader access-method changes, and webhook event triggers; 3.4.23 kept the open APIs available after upgrading to Identity Enterprise or Identity Hub.

Real-time integration takes two forms. A **WebSocket** event stream is the modern path (polling was only necessary below Access 1.90) and is what community bindings for openHAB and Home Assistant use. **Outbound webhooks** are configured through Alarm Manager, with payloads carrying `alarm_id`, `direction`, an event `id` such as `access.unlocks.location_unlocked`, `location`, `scope`, and `trigger_user`. The notable gap: **camera feeds are not offered through the Access API** — video is Protect's surface, not Access's. See [UniFi APIs](./api.md) for details.

## Summary

Access is Ubiquiti applying the Protect playbook to door hardware: free software, local decisions, one cable per device, and a catalog that keeps absorbing the adjacent trade — first controllers, then readers, then intercoms, then elevators and gates, and now the locks, closers, and panic bars themselves.

The three things that decide an Access deployment:

1. **Hubs are per-door, and that is your budget.** One Door Hub per door at $129–$199 adds up fast; the EAH-8 breaks even against roughly six separate hubs and adds battery backup. Decide the topology before you pick readers.
2. **Access is not self-sufficient.** Video comes from Protect, end-user unlocking comes from the UniFi Endpoint app, and telephony from Talk or a third-party SIP server. A deployment that ignores this ends up with an access-control system with no footage of the door.
3. **Touch Pass is the only meter running.** Everything else is free forever; Wallet credentials are per-pass after a one-year trial, country-gated, and — since the 2026 Fabric migration — re-enrolled by every user. Cards and PINs remain the zero-cost path.

**Sources:** [ui.com/door-access](https://ui.com/door-access) · [ui.com/door-access/readers](https://ui.com/door-access/readers) · [store.ui.com](https://store.ui.com) · [techspecs.ui.com/unifi/door-access](https://techspecs.ui.com/unifi/door-access) · [UniFi Access Application releases](https://community.ui.com/rss/releases/UniFi%20Access%20Controller/685a79da-5fdd-44b0-a479-268d8ea1d619) · [UniFi Access on the App Store](https://apps.apple.com/us/app/unifi-access/id1578429007) · [Introducing the all-new G6 Entry lineup](https://blog.ui.com/article/introducing-all-new-g6-entry-lineup) · [New Physical Security Features](https://blog.ui.com/article/physical-security-features) · [Configuring Touch Pass in UniFi Access](https://help.ui.com/hc/en-us/articles/27130425853079-Configuring-Touch-Pass-in-UniFi-Access) · [Choosing a UniFi Access Control Hub](https://help.ui.com/hc/en-us/articles/23964054398871-Choosing-a-UniFi-Access-Control-Hub) · [Choosing a UniFi Access Reader](https://help.ui.com/hc/en-us/articles/25849483463959-Choosing-a-UniFi-Access-Reader) · [UniFi Endpoint Overview](https://help.ui.com/hc/en-us/articles/16936895868823-UniFi-Endpoint-Overview) · [Getting Started with the Official UniFi API](https://help.ui.com/hc/en-us/articles/30076656117655-Getting-Started-with-the-Official-UniFi-API)

---

Four things worth flagging about the research itself:

**Mobile app dates are firm; the 1.x era is not.** Versions 2.11.0 onward are dated from APKMirror upload timestamps (which lag Play rollout by hours to a couple of days) cross-checked against the App Store; 2.17.1's 10 July 2026 date is confirmed directly from Apple's own metadata endpoint. The 1.0.0 launch is dated only as "mid-2021" from the App Store identifier range — Ubiquiti's community release archive no longer reaches back that far, and its release pages are JavaScript-rendered so the published date is not machine-readable.

**iOS and Android are genuinely the same version here.** Unlike the Protect app, where Android trails iOS by a full minor version, both Access clients sit at 2.17.1 on the same date. I checked this specifically because the Protect document warns about the opposite trap.

**One conflict resolved toward Ubiquiti.** A reseller listing describes the Gate Hub as controlling three doors; Ubiquiti's tech specs say one gate plus one side door. Another describes the EAH-8 as having ten gigabit ports with a 240 W PoE budget, which the official datasheet contradicts. I used techspecs.ui.com throughout.

**The Touch Pass price is deliberately absent.** Ubiquiti does not publish a per-pass figure on any public page — it appears only inside your own console's Touch Pass store. Community threads quote figures around $10, but I could not verify that against a Ubiquiti source, so I left the number out rather than print a guess into a reference document.
