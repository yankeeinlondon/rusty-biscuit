---
prompt: |- 
    Your task is to research the "protect" service in Unifi as well as the "protect app" that supports this service.
    
    - discuss the goals and focus for "protect"
    - discuss the hardware products which fit under the "protect" service
    - be sure to link to the [security cameras](./security-cameras.md) document to reference all security cameras 
    - but make sure that you include more than just security cameras in your overview of this service line
    - talk about the mobile Protect app and how it is organized and what functionality it exposes
        - focus on the latest version of the app
        - be sure to specify the latest version of the app as of today
        - also give a high level overview of how features and UI have changed over time in this app (giving discrete version numbers and/or dates where you can to help identify the period this historic variant was in use).
        - refer to the [web application](./web-app.md) document for discussion of how it is organized (including the Protect aspects of the web app).
last_updated: 2026-09-01
hash: 4611f69b6f495997-b6dadce0784a55ce
---
I have enough to write. Here is the replacement body for `homelab/docs/unifi/protect.md`:

---

# UniFi Protect

UniFi Protect is Ubiquiti's physical-security platform: the software application that records, analyzes, and serves video and sensor events, plus the hardware line that feeds it. It began in 2018 as a camera/NVR replacement for the aging UniFi Video product, and it has since grown into something broader — the marketing category on ui.com is no longer "cameras," it is **Physical Security**, and the catalog now includes intrusion sensors, sirens, speakers, smoke and CO alarms, air-quality sensors, and an alarm hub alongside three dozen cameras.

Protect is one of the applications that runs on a UniFi console (a Dream Machine, Cloud Gateway, Cloud Key, or a dedicated NVR) under UniFi OS, sitting beside Network, Access, Talk, and Connect. That co-tenancy is the strategic point of the product and shows up everywhere in the software: doorbell rings can unlock an Access door, camera events can drive Network-side automation, and the console is a single pane for all of it.

## What Protect Is For

Four commitments define the product and explain most of its design decisions.

**No licensing fees, ever.** Ubiquiti's stated position is that Protect "operates identically whether you are managing 4 or 4,000 cameras." There is no per-camera license, no per-seat charge, no recording subscription, and no AI-feature tier. This is the single largest competitive difference against Verkada, Avigilon, Rhombus, and the rest of the cloud-VMS field, and it is why the hardware carries the margin — a $799 AI Key or a $4,999 Enterprise NVR Core is a one-time purchase that unlocks capability permanently. Even the newest 7.2 features (Evidence Trust, PPE detection) explicitly ship without a new SKU or subscription.

**Footage stays local.** Recording and AI inference both happen on-premises — on the camera, on an AI Key or AI Port, or on the recorder. The cloud layer (`unifi.ui.com`) is an encrypted remote-access relay and identity broker, not a storage tier. Ubiquiti calls this "hybrid cloud"; in practice it means you can lose your internet connection and keep recording, and it means the compliance answer to "where does the video live?" is "in that rack."

**AI at the edge, tiered by silicon.** Every current camera does person/vehicle/animal smart detection on-device. Cameras graded **Enhanced AI** additionally do face recognition and license-plate recognition on-camera. Above that sits a second inference tier — the AI Key — which re-processes detections the cameras have already flagged and adds semantic understanding: natural-language search, re-identification across cameras, dwell-time tracking, AI event summaries, speech-to-text, and (since 7.2) PPE compliance detection. The AI Port pushes the first tier onto cameras that lack it, including third-party ONVIF cameras.

**Scale from one doorbell to a multi-site enterprise on one codebase.** The same application runs a $199 NVR Instant with four cameras and a $4,999 Enterprise NVR Core with 300. The 2026 releases lean hard into the top of that range — multi-site Video Walls, Fabrics, Control Plane, MSP group administration, IdP login, Case Management with audit trails, and Evidence Trust for court-admissible exports.

> **On compliance.** Every current UniFi camera is NDAA-compliant, which is a hard procurement requirement for US federal, state, and many education and healthcare buyers. Combined with the no-license model, this is why Protect turns up in school-district and municipal deployments far above its prosumer price point.

## The Hardware

### Cameras

The camera range is the bulk of the line and is documented separately and exhaustively — sensor sizes, IR reach, PoE class, uplink speed, AI tier, and US/UK pricing for every model — in **[Security Cameras](./security-cameras.md)**. That document covers the G6 Pro, G6 Edge, G6, AI, panoramic/multi-sensor, PTZ, G5, Instant (WiFi), and doorbell families, plus legacy G4/G3 models that Protect still supports.

The short version: roughly three dozen models from a $99 PoE doorbell to a $1,999 31× PTZ with LiDAR autofocus, with three specifications doing most of the buying work — the **AI tier** (Enhanced vs Detections, i.e. whether you get face and plate recognition), the **uplink speed** (much of the standard G6 and all of the G5 range ships a 10/100 port), and the **PoE class** (4 W to 51 W, which is a switch-budget problem long before it is a camera problem).

Everything below is the rest of the service line — the two-thirds of the Protect catalog that is not a camera.

### Recorders and Consoles

The recorder, not the camera, is the scalability ceiling. No UniFi camera has a client limit; every recorder has a published 4K camera count, and that number is the constraint you plan around.

| Product                            | SKU              | Price (USD) | Form    | Bays               | Camera capacity                                      |
|------------------------------------|------------------|-------------|---------|--------------------|------------------------------------------------------|
| Enterprise NVR Core                | ENVR-Core        | $4,999      | 3U      | 16 (+16 expansion) | 300 × 4K / 500 × Full HD                             |
| Enterprise NVR                     | ENVR             | $1,999      | 3U      | 16                 | 70 × 4K / 210 × Full HD                              |
| Network Video Recorder G2 Pro      | UNVR-G2-Pro      | $999        | 2U      | 8                  | 50 × 4K / 100 × Full HD                              |
| Network Video Recorder G2          | UNVR-G2          | $699        | 1U      | 4                  | 30 × 4K / 60 × Full HD                               |
| Network Video Recorder Pro         | UNVR-Pro         | $499        | 2U      | 7                  | 24 × 4K                                              |
| Network Video Recorder             | UNVR             | $299        | Desktop | 4                  | 18 × 4K / 60 × Full HD                               |
| Network Video Recorder Instant     | UNVR-Instant     | $199        | Compact | 1 × 3.5"           | 6 × 4K                                               |
| Network Video Recorder Instant Kit | UNVR-Instant-Kit | $699        | Compact | 1 × 3.5"           | Bundle: NVR Instant + 4 × G5 Turret Ultra + 1 TB HDD |
| CloudKey+                          | UCK-G2-SSD       | $249        | Compact | SSD (fixed)        | Multi-application console                            |

**The G2 generation is the 2026 story.** The UNVR G2 and G2 Pro roughly double their predecessors' camera counts and — more importantly — fold in an **integrated AI Key and an integrated ViewPort**. That is $799 + $199 of separate hardware absorbed into the chassis, which is most of the justification for the G2's higher list price over the UNVR/UNVR Pro it supersedes. The G2 Pro carries a 10G SFP+ plus a 2.5 GbE RJ45 and supports redundant power. Both G2 models have been supply-constrained through most of August 2026.

The original **UNVR at $299 remains in the line** as the entry rackmount for buyers who don't need on-box AI or HDMI output. The **Enterprise NVR Core** is gated behind Ubiquiti's Enterprise Partner Program, which is why its price is inconsistently published.

Protect also runs on general-purpose UniFi consoles — Dream Machine Pro, Cloud Gateway Max/Ultra/Fiber, Cloud Key Gen2 Plus — where camera capacity is much lower and bounded by the console's own storage and CPU. This is the normal home deployment.

### Edge AI Appliances

| Product | SKU        | Price (USD) | What it adds                                                                                                                                                                                                                                             |
|---------|------------|-------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| AI Key  | AI-Key     | $799        | Second-stage AI over the whole deployment: natural-language search, cross-camera re-identification, dwell time, AI event summaries, speech-to-text, PPE detection (7.2+). Rated for **1,800 smart-detection events/hour**; multiple keys can be stacked. |
| AI Port | UP-AI-Port | $199        | First-stage AI for cameras that lack it — older UniFi models and **third-party ONVIF cameras**. Adds detection, classification, and recognition at the port.                                                                                             |

The distinction matters and is frequently confused. The **AI Key does not detect** — it consumes detections the cameras already produced and adds meaning on top, so a camera with no on-board AI gains nothing from it. The **AI Port does detect**, on behalf of a camera that can't. A deployment of old G3s wanting modern search needs both.

### Display, Lighting, and Audio Output

| Product            | SKU                | Price (USD) | Notes                                                                                                                                                            |
|--------------------|--------------------|-------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Protect Viewport   | UP-Viewport        | $199        | HDMI hub rendering up to 16 camera feeds as a 4K live matrix on any TV or monitor. Single-cable PoE with RJ45 passthrough. Now integrated into the UNVR G2 pair. |
| Protect Floodlight | UP-FloodLight      | $99         | Motion-triggered light, controllable as a Protect device and usable as an Alarm Manager action.                                                                  |
| AI Speaker         | UP-AI-Speaker      | $199        | Indoor 100 dB PoE speaker with AI-driven alert playback.                                                                                                         |
| AI Horn Speaker    | UP-AI-Horn-Speaker | $299        | All-weather 120 dB PoE horn with AI alerts — the deterrence/voice-down device.                                                                                   |
| Siren PoE          | UP-Siren-PoE       | $109        | All-weather 110 dB siren with emergency LED, PoE-powered.                                                                                                        |
| PoE Smart Chime    | UACC-Chime-PoE     | $79         | Doorbell chime for any UniFi doorbell.                                                                                                                           |
| WiFi Smart Chime   | UP-Chime           | $59         | Plug-in chime; US store only.                                                                                                                                    |

### Sensors, Alarms, and SuperLink

This is the newest and fastest-growing part of the line, and it runs on **SuperLink** — Ubiquiti's own long-range, low-latency wireless protocol for battery devices, with Thread border-router capability. SuperLink devices need a gateway; the sensors are `USL-*` SKUs, the PoE-powered ones are `UP-*`.

| Product                             | SKU               | Price (USD) | Notes                                                                                                                                 |
|-------------------------------------|-------------------|-------------|---------------------------------------------------------------------------------------------------------------------------------------|
| SuperLink Gateway                   | USL-Gateway       | $129        | Required hub for all SuperLink sensors; also a Thread border router. A high-availability variant (Gateway HA) exists.                 |
| SuperLink High-Gain Antenna         | UACC-USL-ANT-HG   | $49         | IP67 omnidirectional antenna for extending SuperLink range outdoors.                                                                  |
| Entry Sensor                        | USL-Entry         | $39         | Door/window open-closed, up to 6-year battery.                                                                                        |
| Recessed Entry Sensor               | USL-Entry-R       | $39         | Same, for recessed door installation.                                                                                                 |
| Motion Sensor                       | USL-Motion        | $49         | PIR motion, up to 6-year battery.                                                                                                     |
| Glass Break Sensor                  | USL-GlassBreak    | $59         | Glass-break acoustic detection plus motion.                                                                                           |
| Environmental Sensor                | USL-Environmental | $49         | Water leak, temperature, humidity, ambient light.                                                                                     |
| Smoke and CO Alarm                  | USL-Smoke         | $89         | Smoke + carbon monoxide, up to 10-year battery.                                                                                       |
| All-In-One Sensor                   | UP-Sense          | $59         | Motion, light, temperature, humidity in one battery device (the original "UP Sense").                                                 |
| Vape Detection & Air Quality Sensor | UP-AirQuality     | $99         | PoE. Real-time vape detection plus air quality — squarely aimed at schools, hospitality, and healthcare.                              |
| Relay                               | USL-Relay         | $39         | I/O bridge for third-party sensors and devices.                                                                                       |
| Remote Control KeyFob               | USL-FOB           | $39         | Programmable arm/disarm and action fob, up to 10-year battery.                                                                        |
| Siren                               | USL-Siren         | $109        | SuperLink wireless 110 dB siren with emergency LED.                                                                                   |
| Alarm Hub Kit                       | UP-AlarmHub-Kit   | $399        | **32 hardwired input zones**, PoE++, optional battery backup — the retrofit path for buildings with existing wired intrusion sensors. |

The Alarm Hub is the clearest signal of intent: it exists so that Protect can take over an existing conventional burglar-alarm installation without re-cabling it. Paired with the SIA DC-09 support added in Protect 7.1 (May 2026), Protect can now report to a professional central monitoring station — which moves it from "camera system with notifications" to a credible replacement for a monitored alarm contract.

### Accessories

The camera-adjacent accessories — the Pro Bullet Enhancer, AI Pro Enhancer, G6 180 Enhancer, G5 Pro Enhancer, AI Theta Hub and lens heads, and the large range of junction boxes, arm/pendant/corner mounts, and weather shields — are catalogued in [Security Cameras](./security-cameras.md), since their effect (extended IR range, added floodlight, added radar) is a per-camera specification.

> **Pricing basis.** All figures are Ubiquiti US list prices on [store.ui.com](https://store.ui.com) as of **1 September 2026**, quoting the base list price rather than the higher "surcharge included" figure the US store also displays. Several SKUs (UNVR G2, UNVR G2 Pro, USL-Smoke, UP-Sense) were showing sold out in late August 2026.

## The Protect Application

The mobile app is a client. What it can do is bounded by the version of the **Protect application** running on the console, and several 2026 mobile features carry an explicit server-version prerequisite (third-party camera configuration in Recording Manager needs Protect 7.2+; PPE detection needs camera firmware 5.4 *and* AI Key firmware 2.2.5; the redesigned Case Management needs UniFi OS 5.1.27). When a mobile feature "does nothing," a version mismatch is the usual cause.

The current shipping release is **Protect 7.2.105**, dated 12 August 2026. Recent major versions:

| Version                    | Date                | Theme                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
|----------------------------|---------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **7.2** (7.2.98 → 7.2.105) | Aug 2026            | **Evidence Trust** (SHA-256 fingerprinted exports, externally verifiable at `evidence.ui.com`, with chain-of-custody), redesigned Case Management with viewer/download audit trails, full ONVIF Profile support for third-party cameras (centralized recording/encoding control, UniFi time-server sync, I/O relay trigger-action), **PPE detection**, second-stage verification for low-confidence detections, dashboard multiview presets, PoS events in the Integration API. |
| **7.1** (7.1.55 → 7.1.87)  | May–Jul 2026        | Multi-site **Video Walls** in Site Manager, retrained smart-detection engine, **vehicle auto-tracking** for PTZ, professional alarm monitoring and **SIA DC-09**, webhook shortcuts, audio and motion for third-party cameras, dewarped video export, native immersive 360° downloads, SuperLink remote management.                                                                                                                                                             |
| **7.0** (7.0.73 → 7.0.107) | Mar–Apr 2026        | Custom Dashboard camera layouts, **Tag Manager**, Case Manager link sharing, inline Off-Site Archiving config, an **Intelligence** settings section, motion-path overlays, image-based search in Find Anything, storage I/O warnings, device version revert.                                                                                                                                                                                                                    |
| **6.x** (6.0 → 6.2.88)     | Jun 2025 – Jan 2026 | The EdgeAI release. **Spotlights**, dwell time and multi-camera tracking, redesigned **Find Anything** with semantic object matching, object-counting charts and parallel multi-camera scrubbing, AI trigger previews in Alarm Manager, expanded audit trails.                                                                                                                                                                                                                  |

## The Mobile App

### Current Version

As of **1 September 2026**:

| Platform              | Version    | Released     | Requirement                                   |
|-----------------------|------------|--------------|-----------------------------------------------|
| iOS / iPadOS          | **3.10.0** | 31 Aug 2026  | iOS 18.0 / iPadOS 18.0                        |
| tvOS (Apple TV)       | **3.10.0** | ~24 Aug 2026 | tvOS 18.0                                     |
| macOS (Apple Silicon) | **3.10.0** | 31 Aug 2026  | macOS 15.0, M1 or later — runs the iPad build |
| visionOS              | **3.10.0** | 31 Aug 2026  | visionOS 2.0                                  |
| Android               | **3.8.2**  | 29 Aug 2026  | —                                             |

The iOS build is 258 MB, filed under Lifestyle, published by Ubiquiti Inc., rated **4.8 from ~73,000 ratings**, and localized into 21 languages. There is no native Windows or Linux client — desktop access is the browser (see below).

**The version numbers no longer align across platforms.** iOS and Android drifted apart in early 2026, and the gap is now a full minor version: Android 3.8.0 (13 Aug 2026) carries the same feature set as iOS 3.9.0 (7 Aug 2026). Reading "Android 3.8" as older than "iOS 3.8" is a mistake — compare feature lists, not numbers. In practice Android trails iOS by one to three weeks. Ubiquiti also ships these as staged rolling updates, so a published release date is a rollout window rather than an availability date.

### How the App Is Organized

Ubiquiti does not publish a consolidated navigation guide; the structure below is assembled from release notes and help-center articles. On iOS 26 the app uses the Liquid Glass navigation treatment — a top bar introduced in 3.1.0 over the bottom bar introduced in 3.0.0 — and both bars auto-hide while scrolling, so the chrome looks different from most published screenshots.

Five primary sections:

- **Dashboard** — the landing view. Live camera tiles in one of three layouts (List, Grid, Compact Grid, added in 3.4.0, reorderable), Recent Detections, **Spotlights** (the AI-curated "what matters right now" strip — familiar faces, known vehicles, pets, custom subjects), IoT/sensor tiles, and Multi-View grids. Since 3.8.0 it also surfaces the **Sensor Manager**, and it carries a UniFi Access door-unlock shortcut where Access is present.
- **Find Anything** — search and review. This was called **Detections** until it was renamed in the 2.10.x cycle, and it absorbed the separate Recognitions tab in 3.4.0. It filters by person, vehicle, animal, package, face, license plate, PPE violation, Point-of-Sale event, device tag, and (3.10.0) by assigned person. With an AI Key present it gains **NeXT AI natural-language search** — plain-English queries over indexed footage — plus image-based search added in server 7.0.
- **Timeline** — scrubbing playback across one or many cameras. Long-press changes playback speed; 3.9.0 added frame-preview event scrubbing directly in the Timeline, and 3.10.0 extended the horizontal Event Scrubber to the Detections Player with reworked scrubbing responsiveness. Playback settings (optical zoom, focus, zones and lines) hang off here. Player zoom went from 4× to 8× in 3.4.0.
- **Devices** — the adopted-device list and per-device configuration: recording mode, detection and privacy zones, smart zones, display settings, PTZ patrol and presets, sensor pairing, firmware. Also where adoption happens, over Bluetooth or WiFi. Since 3.3.0 devices are findable through native iOS Spotlight search.
- **Settings** — system configuration: Recording Manager, Alarm Manager and Alarm Profiles, Storage Manager and Off-Site/Continuous Archiving, Case Manager, Tag Manager, Intelligence, Admins, Control Plane, System Logs.

Above all of that sits **Site Manager** (grid and list, added 3.2.0), the multi-console switcher for anyone running more than one site — and the surface where Fabrics, MSP group administration, and IdP login land.

### What the App Actually Exposes

Beyond live view and playback, the notable capabilities:

- **Alarm Manager and Alarm Profiles.** The automation engine. Each alarm is a *trigger* (motion, person, vehicle, package, face, plate, line crossing, glass break, sensor state, webhook), a *scope* (camera, camera group, or all), and an *action* (push notification, webhook, floodlight, camera spotlight, PTZ move to preset, siren or chime audio, smart-device automation), optionally scheduled to a time window. Alarm Profiles (3.0.0) group these into armed states; Arm/Disarm reached Siri and Shortcuts in 3.10.0.
- **Doorbell handling.** Two-way talk across multiple views, ring notifications delivered as native **phone calls** (3.3.0, opt-in via Alarm Manager, not available in China), a mute control on the ring view (3.10.0), and door-unlock shortcuts into UniFi Access.
- **Siri and Shortcuts.** Webhook shortcuts (3.8.0), Floodlight control (3.9.0), Alarm arm/disarm (3.10.0).
- **Export.** Clip download and share, AV1 export (3.4.0), dewarped clip export for 360°/180° cameras (3.8.0), and — with server 7.2 — Evidence Trust fingerprinting on case exports.
- **Third-party cameras.** ONVIF camera adoption, stream-profile selection, video compression controls, and full Recording Manager configuration (3.10.0, requires Protect 7.2+).
- **Sensors and IoT.** Sensor Manager, per-sensor activity graphs and statistics, battery and connection-strength indicators, inline gateway selection, environmental charts.
- **Geofencing**, notification personalization by detection type, and per-camera bitrate/latency charting.

### How the App Has Changed Over Time

**1.x — mid-2018 to roughly 2022/23.** The app shipped alongside Protect itself, as iOS 1.0.2 / Android 1.0.3. Protect launched on the Cloud Key Gen2 Plus as the successor to UniFi Video, and both were noticeably thinner than what they replaced: at launch Protect supported only full-time recording, with motion-based events and timeline scrubbing added over the following months. The 1.x app was live view, a basic timeline, and device adoption — a viewer, not a management console. Community reports of camera freezes and crashes on 1.x are common, and users held on 1.11 for stability until 2.0 resolved them.

**2.x — roughly 2023 to October 2025.** The long consolidation. The app grew into a genuine peer of the web interface: Recording Manager and PTZ Patrol (2.2.0), QR-code sign-in from Android TV, geofencing, improved two-way audio. The **Detections tab was renamed Find Anything** in the 2.10.x cycle — the rename that marks the pivot from "list of motion events" to "search engine over footage." The tail of 2.x, from mid-2025, is where the modern product appears, driven by the AI Key and Protect 6.0:

| Version | Date     | Change                                                                                                                      |
|---------|----------|-----------------------------------------------------------------------------------------------------------------------------|
| 2.12.0  | Jun 2025 | Advanced encoding, Adaptive recording, Spotlight viewing, Multi-view on TV                                                  |
| 2.13.0  | Aug 2025 | **Vantage Points**, Spotlight creation, SuperLink client list, Floodlight/UP Sense pairing, new Dashboard and Devices icons |
| 2.13.1  | Sep 2025 | Arm feature, Loitering zones, zone-based bitrate, unique device recovery passwords                                          |
| 2.14.0  | Sep 2025 | **Alarm Profiles**, sensor connection/battery indicators, Play Sound – Siren action                                         |

**3.x — October 2025 to now.** Contrary to the version number, 3.0.0 was not a redesign — it was the iOS 26 adoption release (Liquid Glass bottom navigation, webhook alarm triggers, Known Face Spotlights). The redesign arrived incrementally across the following year, and the theme of 3.x is *multi-site, multi-tenant, and non-camera hardware*.

| Version | Date (iOS)  | Change                                                                                                                                                                                                                               |
|---------|-------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 3.0.0   | 10 Oct 2025 | iOS 26 support, Liquid Glass bottom nav, webhook alarm triggers, Known Face Spotlights                                                                                                                                               |
| 3.1.0   | 28 Nov 2025 | Liquid Glass **top** nav, Control Plane and Admins in Settings, LPR Spotlights, PTZ Patrol selection                                                                                                                                 |
| 3.2.0   | 19 Dec 2025 | **Site Manager** (grid and list), dewarp in the detections player, more languages                                                                                                                                                    |
| 3.3.0   | 11 Feb 2026 | Doorbell rings as **phone calls**, Continuous Archiving, Camera Edge recording, native iOS search, sensor activity graphs                                                                                                            |
| 3.4.0   | 20 Mar 2026 | Three **Dashboard layouts**, AV1 export, Intelligence section, Recognitions merged into Find Anything, zoom 4× → 8×                                                                                                                  |
| 3.5.0   | 22 Apr 2026 | PTZ portrait mode, full-screen for tall aspect ratios, dewarp under Advanced encoding                                                                                                                                                |
| 3.6.0   | 17 May 2026 | **Fabrics**, Remote Control KeyFob, dewarp in Multi-View, SIA DC-09 alarm response                                                                                                                                                   |
| 3.7.0   | 18 Jun 2026 | **Vehicle Auto Tracking**, Edge AI latency charting, Spotlights in Recent Detections, glass-break events                                                                                                                             |
| 3.8.0   | 17 Jul 2026 | **Sensor Manager**, IdP login for Fabric/MSP admins, webhook Shortcuts, PTZ digital zoom, dewarped clip download                                                                                                                     |
| 3.9.0   | 7 Aug 2026  | Timeline event scrubbing, **PPE safety violations**, Point-of-Sale events, Watermark settings, Floodlight in Siri/Shortcuts                                                                                                          |
| 3.10.0  | 31 Aug 2026 | Horizontal **Event Scrubber** in Detections Player, person assignment and Device Tag filtering in Find Anything, Alarm Profile actions in Siri/Shortcuts, third-party camera setup in Recording Manager, Entry Sensor activity stats |

The trajectory across all three eras is consistent: the app started as a way to watch cameras, became a way to search footage, and is now a way to run a physical-security estate — arming, dispatching, auditing, and managing evidence, with cameras as one input among many.

> **A note on the 3.x navigation.** The Liquid Glass redesign drew real criticism in the community: the shortened top bar truncates site names that previously displayed in full, and the bottom toolbar disappears inconsistently when opening a device. Several 3.x point releases carry navigation-bar bugfixes.

## The Web Application

Protect's browser interface is where the heavier work happens — Video Walls, Case Manager, bulk Recording Manager configuration, storage and archiving policy, admin and permission management, and system logs are all more workable there than on a phone. It is reached through the console directly on the LAN or via `unifi.ui.com/consoles` for remote access, and it is one application among several (Network, Protect, Access, Talk, Connect, Identity) inside the same UniFi OS shell.

For how the web application is structured, how the console shell and the per-application views fit together, and the Protect-specific surfaces within it, see **[Web Application](./web-app.md)**.

## Programmatic Access

Protect exposes an **Integration API** on the console, expanded steadily through 6.x and 7.x — the 7.2 release added Point-of-Sale endpoints. Cameras also expose RTSPS streams for third-party consumption, and webhooks work in both directions (as an alarm trigger and as an alarm action), which is how most home-automation integrations are wired. Evidence Trust exports are verifiable externally at `evidence.ui.com` without any Ubiquiti account. See [UniFi APIs](./api.md) for details.

## Summary

Protect is Ubiquiti's bet that physical security is a hardware business, not a subscription business. The software is given away with the boxes, runs locally, and improves aggressively — three major application versions and eleven mobile releases in the twelve months to September 2026.

The three things that decide a Protect deployment:

1. **The recorder sets the ceiling.** Cameras are unconstrained; recorders publish hard 4K camera counts from 6 (NVR Instant) to 300 (Enterprise NVR Core). Pick it first.
2. **AI is two tiers, and they are not substitutes.** On-camera detection (or an AI Port standing in for it) is the prerequisite; the AI Key adds meaning on top and does nothing without it.
3. **The catalog is no longer just cameras.** SuperLink sensors, the 32-zone Alarm Hub, sirens, speakers, and SIA DC-09 central-station reporting are what turn Protect from a camera system into an alarm system — and they are what most of the 2026 mobile-app development has been about.

**Sources:** [ui.com/physical-security](https://ui.com/physical-security) · [store.ui.com](https://store.ui.com) · [techspecs.ui.com/unifi/physical-security](https://techspecs.ui.com/unifi/physical-security) · [UniFi Protect Application releases](https://community.ui.com/rss/releases/UniFi-Protect/aada5f38-35d4-4525-9235-b14bd320e4d0) · [UniFi Protect iOS releases](https://community.ui.com/rss/releases/UniFi-Protect-iOS/5e7853c3-7dd0-4c77-9314-88490ba34e9d) · [UniFi Protect Android releases](https://community.ui.com/rss/releases/UniFi-Protect-Android/dccea623-f38c-4d44-84a2-c97bddc0c6cf) · [UniFi Protect on the App Store](https://apps.apple.com/us/app/unifi-protect/id1392492235) · [Welcome to Protect 7.2](https://blog.ui.com/article/welcome-to-protect-7-2) · [Welcome to Protect 7.1](https://blog.ui.com/article/welcome-to-protect-7-1) · [Introducing Protect 6.0](https://blog.ui.com/article/introducing-protect-6-0) · [UniFi AI Key Setup and FAQs](https://help.ui.com/hc/en-us/articles/29221435686039-UniFi-AI-Key-Setup-and-FAQs)

---

Three things worth flagging about the research itself:

**The exact 2.0 date is not recoverable.** Ubiquiti's community release pages are JavaScript-rendered and their RSS feeds only retain roughly the last 18 months, so the 1.x→2.x transition is dated as "roughly 2023" from indirect evidence (2.2.0 feature content and community comments), not from a published changelog. Everything from mid-2025 forward is sourced from the official release feeds and is firm.

**iOS and Android version numbers diverged.** I called this out in the doc because it's an easy trap — Android 3.8.0 ≡ iOS 3.9.0 in feature terms. Android's latest is 3.8.2 (29 Aug), iOS's is 3.10.0 (31 Aug).

**One source conflict, resolved toward the community feed.** The App Store version-history summary attributes Sensor Manager to iOS 3.9.0; Ubiquiti's own release feed puts it in 3.8.0. I used the release feed throughout for feature attribution.
