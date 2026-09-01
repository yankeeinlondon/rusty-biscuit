---
prompt: "Your task is to research the \"Network\" service in Unifi as well as the \"Network app\" that supports this service.\n\n- discuss the goals and focus for \"Network\"\n- discuss the hardware products which fit under the \"Network\" service\n    - be sure to link to the [security cameras](./security-cameras.md) document to reference all security cameras \n    - but make sure that you include more than just security cameras in your overview of this service line\n- talk about the mobile Network app and how it is organized and what functionality it exposes\n    - focus on the latest version of the app\n        - be sure to specify the latest version of the app as of today\n    - also give a high level overview of how features and UI have changed over time in this app (giving discrete version numbers and/or dates where you can to help identify the period this historic variant was in use).\n- refer to the [web application](./web-app.md) document for discussion of how it is organized (including the Network aspects of the web app)."
last_updated: 2026-09-01
hash: d19e9d5563f635a6-3f1a582e9e2a5e3e
---
I have enough to write. Here is the replacement body:

---

# UniFi Network

UniFi Network is the oldest and largest of Ubiquiti's product lines — the routing, switching, and WiFi platform that everything else in UniFi plugs into. It is two things at once: a **software application** that runs on a UniFi console and manages adopted infrastructure, and a **hardware catalog** spanning gateways, switches, access points, wireless bridges, cellular backup, rack power, and storage.

The application is the reason people buy the hardware. A UniFi switch is competent but unremarkable silicon; what sells it is that it appears in the same interface as the gateway in front of it, the access points behind it, and the cameras and door readers hanging off its PoE ports — with no controller license, no per-port fee, and no support contract. Network is the application that runs by default on every UniFi console under UniFi OS, sitting beside [Protect](./protect.md), [Access](./access.md), Talk, and Connect.

## What Network Is For

Four commitments shape the product, and they explain nearly every design decision in it.

**No license fees for the network stack.** Zone-based firewalling, IDS/IPS, dynamic routing (BGP and OSPF), Site Magic SD-WAN across up to 1,000 locations, multi-site Fabric management, VPN servers, and the full API surface are all included with the hardware. The single, explicit exception is **CyberSecure** — the Proofpoint-backed threat-signature subscription that layers onto the gateway's own IDS/IPS. Ubiquiti is unusually candid about why: signature intelligence is an ongoing cost, so it is the one thing they charge for. Everything else is a one-time hardware purchase.

**Management is local, with cloud as a convenience.** The Network application runs on the console in your rack. `unifi.ui.com` is a remote-access relay and identity broker, not a control plane you depend on — lose your WAN and the network keeps forwarding, the firewall keeps enforcing, and the switches keep their configuration. Ubiquiti calls this "hybrid cloud." The practical consequence is that a UniFi site has no cloud outage mode.

**One interface, from a studio flat to a thousand sites.** The same application runs a $129 UniFi Express with one AP and a $19,999-class Enterprise Firewall Core fronting a campus. The 2026 releases are almost entirely about the top of that range — Fabrics, Blueprints, Drift Inspector, SAML/IdP login, MSP group administration, and multi-site orchestration — but they ship in the same binary a home user gets.

**Operational safety over feature velocity.** This is the newest of the four and the clearest theme of the 10.x line. **SafeOps** — Test & Confirm with automatic rollback, Link Debounce, Auto STP Edge, Device Supervisor auto-recovery, one-click firmware rollback — exists because the failure mode that hurts a distributed UniFi deployment is not a missing feature, it is an admin pushing a VLAN change from a phone and losing a site. Configuration changes to APs and switches now stage, verify reachability, and revert themselves if the console can't confirm them.

> **On the CyberSecure exception.** It is worth being precise, because it is frequently misreported. The gateway's IDS/IPS engine (Suricata — migrated from version 6 to version 8 in Network 10.5) is free and works out of the box on the built-in signature set. CyberSecure buys you the continuously updated Proofpoint feed and category-level tuning. Disabling it degrades your threat coverage; it does not disable your firewall.

## The Hardware

Network is by far the broadest UniFi catalog. Three families carry the volume and each has its own reference document; the rest of the line is covered here.

### Gateways and Routers

The gateway is the console. In almost every UniFi deployment the box that routes your traffic is also the box running the Network application (and often Protect, Access, and Talk alongside it), which is why gateway selection is really a decision about *how many applications and how many devices* you intend to run, not just about WAN throughput.

The families, in descending scale:

- **Enterprise Firewalls** — EF-Core and the Enterprise Fortress Gateway (EFG). Up to 100 Gbps, the top of the routing line.
- **Dream Machines (UDM)** — UDM-Beast, UDM-Pro-Max, UDM-SE, UDM-Pro. The rackmount all-in-ones, most with integrated PoE switching and a drive bay for Protect.
- **Cloud Gateways (UCG)** — Fiber, Max, Ultra, Industrial. Compact, no integrated switching, sized for small sites and DIN-rail/industrial installs.
- **WiFi-integrated gateways** — Dream Wall, Dream Router 7, Dream Router 5G Max, Dream Router, UniFi Express 7, UniFi Express. Gateway plus AP in one enclosure; the home and micro-office answer.
- **Standalone gateways (UXG)** — Enterprise, Pro, Fiber, Max, Lite. Routing only, for sites that already have switching.
- **Travel routers (UTR)** — the $79 UniFi Travel Router and the $99 **Travel Router Long-Range (UTR-LR)**, launched 28 August 2026. The UTR-LR is the same Qualcomm IPQ4018 platform with a collapsible high-gain "Super Antenna" that tilts through 180°, aimed at WISP-mode uplinks from hotels and rentals. No battery, no SIM slot, no cellular modem — a persistent complaint in the launch coverage.

Full specifications, port counts, throughput figures, IDS/IPS ceilings, and pricing for every model are in **[UniFi Routers and Cloud Gateways](./routers.md)**.

### Switching

The switch line is the deepest catalog in UniFi — roughly fifty current models from an 8-port desktop Flex to 48-port PoE++ campus switches with 25G and 100G uplinks. The tiers:

- **Enterprise Campus (ECS) and Enterprise Audio/Video (EAV)** — the top of the line, including AV-specific models tuned for multicast-heavy Dante and NDI traffic.
- **Aggregation** — ECS-Aggregation, Pro XG Aggregation, Hi-Capacity Aggregation, and the base Aggregation switch. All-fiber cores.
- **Pro XG** — the 10 GbE access tier, 8 through 48 ports, PoE and non-PoE.
- **Pro Max / Pro HD** — 2.5 GbE multi-gig access with high PoE budgets.
- **Pro / Standard** — the gigabit workhorses.
- **Utility, Lite, Ultra, Flex** — small, fanless, wall- and desk-mount, including PoE-powered switches that need no outlet.
- **Mission Critical** — the UPS-backed PoE switch that keeps cameras and phones alive through a power cut.

Full details in **[UniFi Switches](./switches.md)**.

Two specifications do most of the buying work: the **PoE budget** (which is what actually constrains how many cameras and door readers a closet can carry) and the **uplink** (a 48-port PoE switch on a single 10G uplink is a bottleneck long before its ports are).

### WiFi

The access point line spans WiFi 7 flagships to still-supported WiFi 5 hardware:

- **Enterprise WiFi 7 (E7)** — E7, E7 Campus, E7 Audience and their indoor variants. Multi-radio, high-density, 10G-uplinked.
- **Pro XG / XGS** — the flagship WiFi 7 ceiling APs with 10 GbE uplinks.
- **U7 mainstream** — U7 Pro Max, U7 Pro, U7 Long-Range, U7 Lite.
- **Wall and in-wall** — U7 Pro Wall, U7 In-Wall, U6 Enterprise In-Wall, U6 In-Wall.
- **Outdoor and mesh** — U7 Pro Outdoor, U7 Outdoor, U7 Mesh, U6 Mesh Pro.
- **WiFi 6E / WiFi 6 (U6)** and the legacy **AC** range, still adoptable and still receiving firmware.

Full details in **[UniFi Access Points](./ap.md)**.

### Wireless Bridging

A separate, easily overlooked corner of the line: point-to-point and point-to-multipoint links that appear in Network as ordinary adopted devices.

| Product                                                                  | Notes                                                                                                                                                                                                                             |
|--------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **Building Bridge (UBB)**                                                | 60 GHz 802.11ad PtP, 1.7+ Gbps bidirectional, up to 500 m, with a 5 GHz 802.11ac backup radio for rain fade and partial obstruction. $279 for a single unit; sold as pre-paired 2-packs or as re-pairable singles with Auto-Link. |
| **Building Bridge XG**                                                   | Same 60 GHz PtP with a 10G SFP+ uplink and 5 GHz backup.                                                                                                                                                                          |
| **Device Bridge / Device Bridge Pro / Pro Sector / IoT / Bridge Switch** | Shorter-range device and sector bridging, including an IoT-oriented variant.                                                                                                                                                      |

Network 10.5 added campus-wide **trunk connectivity over Building Bridges**, license-free — which is what turns a bridge from "a wireless wire between two subnets" into a real VLAN-carrying link between buildings.

> **Not the same as Wave.** Ubiquiti's 60 GHz WISP products (Wave, Wave MLO) live under the **UISP** brand, not UniFi, and are managed by different software. If you want a UniFi-managed link, UBB/UBB-XG is the family.

### Cellular Backup

WAN failover is a Network feature, and Ubiquiti now sells three price points of radio for it:

| Product                   | Price (USD) | Notes                                                                                                                                                                 |
|---------------------------|-------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **UniFi 5G Backup (U5G)** | $99         | Launched 21 May 2026. Compact 5G RedCap modem with a 1.14" status screen, eSIM or nano-SIM, certified on Verizon, AT&T, and T-Mobile. Rated to 220/120 Mbps on 5G SA. |
| **LTE Backup Pro**        | $279        | The previous entry point.                                                                                                                                             |
| **5G Max**                | $399        | The pick when cellular is a primary WAN or you need gigabit-class throughput.                                                                                         |

The U5G is the interesting one architecturally: it does **not** wire into WAN2. It plugs into any PoE port on any UniFi switch, is adopted as a managed Network device, and the gateway reroutes through it within seconds of detecting a primary WAN failure. That means you place it where the signal is, not where the rack is — a genuinely different deployment model, and the reason it needed a Network-application release (10.6.101 removed the Network Override option for WAN-adopted U5G units and fixed a gateway configuration error on LTE adoption).

Ubiquiti sells its own eSIM data packs starting around $79/year standalone. Set data caps: a long outage on a metered plan is an expensive surprise.

### Power, Storage, and Rack Infrastructure

Ubiquiti catalogs these under **Integrations** and **Accessories** rather than under Network proper, but they are adopted by the Network application, appear in its device list, and are configurable from the same interface — including from the mobile app since iOS 10.37.0.

| Product                              | Price (USD) | Notes                                                                                                                                                        |
|--------------------------------------|-------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **SmartPower PDU Pro (USP-PDU-Pro)** | $279        | 2U, 16 individually switchable AC outlets plus four USB-C ports, per-outlet power monitoring and remote power cycling, 1.3" color touchscreen, 4.2" depth.   |
| **UniFi UPS (rackmount, 1.92 kVA)**  | —           | Pure sine wave, 8 individually controllable outlets with per-port monitoring, field-replaceable 720 Wh battery, ~22 min at 960 W.                            |
| **UniFi UPS (rackmount, 1.44 kVA)**  | —           | 4 backup + 4 surge outlets, 216 Wh battery, ~8 min at 500 W. Graceful Shutdown for UNVR and UNAS, NUT-compatible for third-party gear.                       |
| **Enterprise NAS (ENAS)**            | $3,999      | Launched 18 June 2026. 3U, 16 SATA bays, dual 25G SFP28 plus 10GbE RJ45, 64 GB ECC RAM, ZFS with RAID-Z, iSCSI, Mini-SAS HD expansion, redundant CRPS power. |
| **UNAS Pro 8 / Pro / Pro 4**         | —           | 2U 8-bay with dual NVMe cache, 2U 7-bay 10 Gbps, and 1U 4-bay with NVMe cache.                                                                               |
| **UNAS 4 / UNAS 2**                  | $379 / $199 | Desktop NAS. The UNAS 4 shipped Q1 2026, sold out, and returned to stock in July 2026.                                                                       |

The power hardware is not decorative. Network 10.4 added **UPS battery threshold configuration**; 10.2 added **Device Supervisor**, which detects an unresponsive PoE device and power-cycles its port automatically; and the 10.6 mobile release surfaces a **Device Power Supervisor** and UPS power-protection badges. Rack power is now a first-class monitored object in the Network application, not an accessory.

### Consoles and Hosting

Network needs somewhere to run. Four options:

- **A UniFi console** — any Dream Machine, Cloud Gateway, Dream Router, or Cloud Key. The normal case.
- **Official UniFi Hosting** — Ubiquiti's own cloud-hosted console.
- **UniFi OS Server** — the current self-hosting product, and the one Ubiquiti now tells all self-hosters to use. Runs on Windows, Linux, or macOS, serves its UI on port 11443, and requires **Podman 4.3.1+** on Linux (Docker is explicitly not supported). It delivers the full UniFi OS shell — Organizations, IdP integration, Site Magic — and is Site Manager compatible.
- **The legacy UniFi Network Server** — the old standalone controller. Still runnable, but it lacks Organizations, IdP, and Site Magic, and Ubiquiti's guidance is now to migrate off it.

### And the Cameras

Security cameras are **Protect** hardware, not Network hardware, and they are catalogued in **[Security Cameras](./security-cameras.md)** — three dozen models from a $99 PoE doorbell to a $1,999 PTZ, with sensor sizes, PoE classes, uplink speeds, and AI tiers for each.

They belong in a Network overview anyway, because in most real deployments cameras are the **single largest load the Network side has to plan around**. They are usually the majority of the PoE budget, the majority of sustained internal bandwidth, and the devices most likely to need a VLAN, a PoE port power-cycle, or a switch-uplink upgrade. Much of what Network 10.x added — Device Supervisor's automatic PoE recovery, Time Machine for port state, Port Manager's power views, the Multicast Suppressor — reads very differently once you know the ports in question are full of cameras. The same is true of Access door readers and Talk phones: Network is the substrate, and the other applications are its heaviest tenants.

> **Pricing basis.** All figures are Ubiquiti US list prices on [store.ui.com](https://store.ui.com) as of **1 September 2026**. Several SKUs (UNVR-adjacent storage, the UTR-LR) were showing sold out in late August 2026.

## The Network Application

The mobile app and the web interface are both clients. What they can do is bounded by the version of the **Network application** running on the console, and several mobile features carry an explicit server prerequisite — WiFi Doctor on the mobile Dashboard needs Network 10.1; Port Manager on mobile needs app 10.0.0 or newer on both platforms. When a mobile control appears to do nothing, a version mismatch is the usual cause.

The current shipping release is **Network 10.6.101**, dated 26 August 2026.

The 10.x line, in order:

| Version                        | Date         | Theme                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
|--------------------------------|--------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **10.6** (10.6.97 → 10.6.101)  | Aug 2026     | **Topology Spotlight** (collapses unrelated devices while investigating), Time Machine extended to **Radios** and **Ports Overview**, nightly **Channel AI** automation, **Multicast Suppressor** against broadcast storms, first phase of **port locking**, granular bulk device updates, SafeOps extended to management-VLAN changes, **Drift Inspector** for Blueprint deviations across sites, and a high-availability **Readiness Score** covering ISP, power, and gateway resilience. |
| **10.5** (10.5.54 → 10.5.67)   | Jun–Jul 2026 | **Test & Confirm** with automatic rollback on APs and switches, **Client Observability** (a 24-hour per-client timeline of connectivity, roaming, application usage, and flows), **Link Debounce** and **Auto STP Edge**, license-free trunking over Building Bridges, SD-WAN Underlay in Port Manager, PPPoE 1500 MTU, Suricata 6 → 8, SAML login at Fabric-Admin level.                                                                                                                   |
| **10.4** (10.4.57)             | 19 May 2026  | Native **eBGP** in the routing table, OSPF area visibility, a unified FIB view, IPv6 dual-stack auto-detection and **WireGuard over IPv6**, Time Machine folded into infrastructure topology, digital twins for third-party appliances, full **5G radio telemetry**, UPS battery thresholds, **Blueprint synchronization** across sites, WAN Insights and WiFi airtime visibility.                                                                                                          |
| **10.3** (10.3.55 → 10.3.58)   | Apr 2026     | **Identity Firewall** — user-based rather than address-based access control — and Device Supervisor extended from infrastructure to client devices.                                                                                                                                                                                                                                                                                                                                         |
| **10.2** (10.2.93 → 10.2.105)  | Mar 2026     | **Time Machine** for switches (24-hour port-state history, using Protect's timeline design), **Infrastructure Topology** with a rack digital twin, **Enhanced Open / OWE** (WPA3-grade encryption on password-free guest WiFi, including 6 GHz), **Device Supervisor** auto power-cycling, STP Edge and BPDU Guard, one-click firmware rollback.                                                                                                                                            |
| **10.1** (10.1.84 → 10.1.85)   | 2 Feb 2026   | Completely redesigned **high availability** with guided shadow-gateway onboarding and firmware parity validation, **WiFi Doctor** one-tap remediation, roaming-journey visualization in AirView, device tags, LEO satellite RF visibility without a separate app, **UniFi Site Manager** in Early Access.                                                                                                                                                                                   |
| **10.0** (10.0.156 → 10.0.162) | Nov–Dec 2025 | The platform version bump. Ubiquiti published no "Introducing 10.0" blog post; it exists only as community release entries, and the visible feature work resumed with 10.1.                                                                                                                                                                                                                                                                                                                 |

The 9.x line before it, for context on where the current shape came from:

| Version | Date        | Theme                                                                                                                                                                                                                                                                                  |
|---------|-------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **9.0** | Jan 2025    | **Zone-based firewalling** (External / Internal / Gateway / VPN / Hotspot / DMZ, plus custom zones, with automatic migration and backup of the old rule set), CyberSecure with daily threat updates, license-free **Site Magic SD-WAN** to 1,000 sites, and the **Local Network API**. |
| **9.1** | Apr 2025    | Application-aware QoS, redesigned dashboard with prominent precision throughput and one-tap WAN/local speed tests, **Traffic Flows**, Shortcuts.                                                                                                                                       |
| **9.2** | ~Jun 2025   | WAN SLA, 5 GHz roaming assistant, DHCP Manager.                                                                                                                                                                                                                                        |
| **9.3** | 11 Jul 2025 | Redesigned **client table** with filtering by AP, radio, WiFi generation, and vendor; **Alarm Manager**; enhanced content filtering; revamped System Logs; CNAME records.                                                                                                              |
| **9.5** | Oct 2025    | **Channel AI** — automated evaluation and improvement of the channel plan.                                                                                                                                                                                                             |

> **A security note worth carrying forward.** CVE-2026-54405 (CVSS 7.5) affects the Network application and is patched in **10.4.57 and later**. Any console still on 10.3.x or earlier should be updated.

## The Mobile App

### Current Version

The app is branded simply **UniFi** — not "UniFi Network" — and it manages every application on a console, not just Network. Network is nonetheless the bulk of what it does; [Protect](./protect.md) and [Access](./access.md) still ship separate apps for their day-to-day work.

As of **1 September 2026**:

| Platform              | Version              | Released    | Requirement                                   |
|-----------------------|----------------------|-------------|-----------------------------------------------|
| iOS / iPadOS          | **10.37.1**          | 25 Aug 2026 | iOS 18.0 / iPadOS 18.0                        |
| macOS (Apple Silicon) | **10.37.1**          | 25 Aug 2026 | macOS 15.0, M1 or later — runs the iPad build |
| visionOS              | **10.37.1**          | 25 Aug 2026 | visionOS 2.0                                  |
| Android               | **10.39.6** (stable) | 25 Aug 2026 | Android 9.0+ (12L on some variants)           |
| Android (beta)        | 10.39.7              | 31 Aug 2026 | —                                             |

The iOS build is 492 MB, filed under Productivity, published by Ubiquiti Inc., rated **4.7 from roughly 22,000 ratings**, and localized into 21 languages. The Android package is `com.ubnt.easyunifi`. There is no native Windows or Linux client — desktop is the browser.

**The version numbers do not mean the same thing on both platforms.** Android runs roughly two minor versions ahead of iOS on the same feature set: Android 10.37.0 shipped 5 April 2026, while iOS 10.37.0 shipped 13 August 2026. Reading Android 10.39 as "newer than" iOS 10.37 in feature terms is a mistake — compare release notes, not numbers. Ubiquiti's own hardware documentation states minimum app versions per platform for exactly this reason: the UTR-LR launch requires **iOS 10.37.1 or Android 10.39.7**.

### How the App Is Organized

Ubiquiti does not publish a consolidated navigation guide for the app; the structure below is assembled from release notes and help-center articles, and it moves.

The top level is **Site Manager** — the list of every console you own or administer, with a toggle between list and grid layout (added 10.31.0). Sites appear automatically from your UI account with no configuration, because remote management is on by default at setup. This is also the surface where Fabrics, Blueprints, and MSP group administration land.

Selecting a console drops into that site, organized roughly as:

- **Dashboard** — the landing view. WAN status and precision throughput with one-tap WAN and local speed tests, client and device counts, alarms, and since 10.32.1 the **WiFi Doctor** one-tap remediation card (requires Network 10.1). VLAN conflict warnings surface here as of 10.37.1.
- **Devices** — adopted infrastructure, filterable (10.30.1). Per-device detail covers ports, PoE, uplink, radios, system usage charts, firmware, and — for APs — an Airtime Utilization widget (10.34.0). Adoption happens here, over Bluetooth or WiFi, and the app is also the setup path for standalone devices with no console at all.
- **Clients** — connected clients with a filter added in 10.29.1, per-client history, and (with Network 10.5) the 24-hour Client Observability timeline.
- **WiFi** — SSID configuration, band and security settings, hotspot portal, schedules, and VLAN assignment.
- **Insights** — the analysis section: **Flows** (real-time traffic with source, destination, zone, protocol, and the policy that allowed or blocked it), **Ports**, **Radios**, and WAN/ISP SLA with latency, packet loss, and utilization.
- **Settings / Control Plane** — system configuration: Alarm Manager, Port Manager, Storage, Notification Settings, console settings, admins, backup and restore, and installation of additional UniFi OS applications.

### What the App Actually Exposes

The app is now a genuine peer of the web interface for most day-to-day work, not a viewer. The notable capabilities:

- **Network configuration** — VLANs and networks, WiFi, port profiles, 802.1X on switch ports, port aggregation, and management/out-of-band port settings (10.36.0).
- **Security** — firewall zones and policies, **Deep Inspection** and **Content Filter** settings (10.36.0), and traffic identification.
- **VPN** — WireGuard configuration including manual setup and QR-code profile import, OpenVPN, and **Teleport** one-tap remote access. Saved Teleport profiles bound to a console arrived in 10.37.0; UTR-specific WireGuard support in 10.32.0, saved networks in 10.34.1, and a VPN kill switch for UTR.
- **Alarm Manager** — alarm triggers, detail screens, continuous-monitoring fields, and escalation.
- **Power** — UPS outlet configuration, battery status, power-failure information, and **Device Power Supervisor** (10.37.0), plus PDU control.
- **High availability** — shadow-gateway (Shadow Mode) setup and cloud backup restoration, improved in 10.37.0.
- **Multi-site** — Site Manager switching, **SSO / IdP SAML login** (10.36.2), per-application UniFi OS backup and restore.
- **Platform integration** — native iOS Spotlight search across sites and devices (10.37.0), home-screen widgets, and support-file export of app logs from the accounts page (10.29.1).

### How the App Has Changed Over Time

**The "UniFi Network" era — through late 2022.** The app was named *UniFi Network* and had two modes: **Controller mode**, which connected to a controller and gave you dashboard, devices, clients, insights, alerts, and events; and **Standalone mode**, for configuring an access point directly with no controller at all. It was a competent remote viewer with limited configuration reach — real work happened in the browser. Android builds in this era: 3.9.7 (~Nov 2021), 3.10.6 (~Dec 2021), 4.1.3 (18 Aug 2022), 4.5.0 beta (10 Nov 2022), all requiring Android 6.0.

**The renumbering and rename — early/mid 2023.** Somewhere between November 2022 and May 2023 the app jumped from 4.x straight to 10.x and dropped "Network" from its name, becoming simply **UniFi**. The earliest 10.x build I can date is Android **10.4.1 on 26 May 2023**. This coincided with UniFi OS 3.0 and the broader consolidation of Ubiquiti's naming, and the rename reflected a real change in scope: the app now managed the console and all its applications, not just Network. Ubiquiti did not publish a changelog explaining the version jump, so the exact release and rationale are reconstructed rather than sourced.

**The 10.x long march — 2023 to now.** There is no single redesign to point at. The app has shipped a steady stream of minor versions, each closing a specific gap against the web interface, with the Android minimum climbing from 7.0 (10.19.2, October 2024) to 9.0 (10.30.x, late 2025) and iOS to 18.0.

The most recent year, where the record is firm:

| Version (iOS)   | Date                | Change                                                                                                                                                                                                                  |
|-----------------|---------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 10.28.1         | 29 Sep 2025         | VLAN conflict warnings, stability                                                                                                                                                                                       |
| 10.29.1         | 29 Oct 2025         | Support-file export of app logs, **client filter** on the Clients page                                                                                                                                                  |
| 10.30.0–10.30.2 | Dec 2025            | **Traffic Flows**, **Alarm detail screen**, **WAN SLA**, device-list filter                                                                                                                                             |
| 10.31.0–10.31.4 | Dec 2025 – Jan 2026 | Improved **Site Manager** with list/grid toggle, device-detail overview and port aggregation                                                                                                                            |
| 10.32.0         | 2 Feb 2026          | **WireGuard for the UniFi Travel Router**                                                                                                                                                                               |
| 10.32.1         | 23 Feb 2026         | **WiFi Doctor** on the Dashboard, Storage under Settings → Control Plane                                                                                                                                                |
| 10.33.0–10.33.1 | Mar 2026            | Control Plane console settings and Notification Settings                                                                                                                                                                |
| 10.34.0         | 21 Apr 2026         | **Airtime Utilization** widget on AP detail                                                                                                                                                                             |
| 10.34.1         | 30 Apr 2026         | In-app feedback from Settings, Saved Networks on UTR                                                                                                                                                                    |
| 10.35.0         | 22 May 2026         | UTR enhancements, stability                                                                                                                                                                                             |
| 10.36.0         | 30 Jun 2026         | **Deep Inspection** and **Content Filter** settings, management and out-of-band port settings                                                                                                                           |
| 10.36.1         | 9 Jul 2026          | Hotspot Portal link in WiFi settings, per-application UniFi OS backup and restore                                                                                                                                       |
| 10.36.2         | 31 Jul 2026         | **SSO IdP SAML login**, improved sign-in and session restoration                                                                                                                                                        |
| 10.37.0         | 13 Aug 2026         | **Spotlight search** for sites and devices, **UPS settings** (outlets, battery, power-failure info), **Device Power Supervisor**, saved Teleport VPN profiles on UTR, Shadow Mode and cloud-backup restore improvements |
| 10.37.1         | 25 Aug 2026         | VLAN conflict warnings on Dashboard and WiFi settings, **Pro AV QoS guidance**, port grid aggregation and uplink legends, Alarm Manager reliability, UPS power-protection badge fixes                                   |

The trajectory is consistent across all three eras: the app started as a way to look at a controller, became a way to configure a site, and is now a way to run a multi-site estate — identity, orchestration, power, and rollback included.

### WiFiman — the Companion App

Worth knowing because it does things the UniFi app deliberately doesn't. **WiFiman** (free, iOS/Android/desktop) has three tabs: **Speed** (throughput and latency tests, with signal strength and channel width when you're on a Cloud Gateway–managed network), **Discovery** (Layer 2 subnet sweep — the fastest way to diagnose an adoption failure), and **Teleport** (one-click WireGuard-based remote access, with invitations that expire in 24 hours and bind to a single device). The desktop build supports only device discovery and Teleport.

The **WiFiman Wizard** is a separate piece of hardware: a battery-powered passive scanner with a low-gain omnidirectional antenna deliberately matched to a typical phone's RF profile. It exists because Apple locks down the WiFi scanning APIs and Google throttles scans, which means iOS site surveys are otherwise limited to throughput tests and the single connected AP's roaming metrics, and 6 GHz scanning is Android-only. If you do professional surveys, the Wizard is not optional.

## The Web Application

The browser interface is where the heavy Network work happens — the full topology view with Time Machine, Port Manager, routing tables and BGP/OSPF configuration, zone-based firewall policy authoring, Blueprints and Drift Inspector, Site Magic SD-WAN topology, system logs, and admin and permission management are all substantially more workable there than on a phone. It is reached at the console directly on the LAN, or through `unifi.ui.com` for remote and multi-site access, and Network is one application among several — Protect, Access, Talk, Connect, Identity — inside the same UniFi OS shell.

For how that shell is structured, how the per-application views fit together, and the Network-specific surfaces within it, see **[Web Application](./web-app.md)**.

## Programmatic Access

Network exposes two API surfaces. The cloud-side **Site Manager API** covers multi-site deployment and management; the **Local Network API**, introduced with Network 9.0, runs on the console itself and covers device listing and control (including reboot), real-time CPU/memory/uptime, live WiFi, wired, and VPN client statistics, and multi-site queries with pagination for large deployments — all without routing through Ubiquiti's cloud. See **[UniFi APIs](./api.md)** for details.

## Summary

Network is the load-bearing product in UniFi. It is given away with the hardware, runs locally, and has shipped seven application major versions in the twenty months to September 2026 — a pace that has moved it from "a good prosumer controller" to something that credibly manages a thousand-site estate.

The three things that decide a Network deployment:

1. **The gateway is the console, so pick it for applications, not just for throughput.** WAN speed is the easy number; the harder question is how many devices, how many applications, and how much storage the box has to carry alongside routing.
2. **PoE budget and uplink capacity are the real switching constraints.** Port count is rarely what runs out first — especially once cameras, door readers, and phones are on the same closet switch.
3. **The 10.x line is about not breaking things.** SafeOps, Test & Confirm, Device Supervisor, Drift Inspector, and the HA Readiness Score are the current center of gravity. If you manage more than one site, they are the reason to be current rather than stable-and-behind.

**Sources:** [ui.com/cloud-gateways](https://ui.com/cloud-gateways) · [store.ui.com](https://store.ui.com) · [techspecs.ui.com/unifi](https://techspecs.ui.com/unifi) · [UniFi Network Application releases](https://community.ui.com/rss/releases/UniFi%20Network%20Controller/e6712595-81bb-4829-8e42-9e2630fabcfe) · [Introducing UniFi Network 10.6](https://blog.ui.com/article/introducing-unifi-network-10-6) · [Introducing Network 10.5](https://blog.ui.com/article/introducing-network-10-5) · [Introducing UniFi Network 10.4](https://blog.ui.com/article/introducing-unifi-network-10-4) · [Introducing UniFi Network 10.2](https://blog.ui.com/article/introducing-unifi-network-10-2) · [Introducing UniFi Network 10.1](https://blog.ui.com/article/introducing-unifi-network-10-1) · [UniFi Network 9.0 — Built to Scale](https://blog.ui.com/article/unifi-network-9-0-built-to-scale) · [Introducing Network 9.3](https://blog.ui.com/article/introducing-network-9-3) · [Introducing UniFi 5G Backup](https://blog.ui.com/article/introducing-unifi-5g-backup) · [Introducing UniFi OS Server for MSPs](https://blog.ui.com/article/introducing-unifi-os-server) · [UniFi on the App Store](https://apps.apple.com/us/app/unifi/id1057750338) · [UniFi on APKMirror](https://www.apkmirror.com/apk/ubiquiti-networks-inc/unifi/) · [Zone-Based Firewalls in UniFi](https://help.ui.com/hc/en-us/articles/115003173168-Zone-Based-Firewalls-in-UniFi) · [Self-Hosting UniFi](https://help.ui.com/hc/en-us/articles/34210126298775-Self-Hosting-UniFi) · [Using WiFiman](https://help.ui.com/hc/en-us/articles/205204150-Using-WiFiman) · [UniFi Gateway — Teleport VPN](https://help.ui.com/hc/en-us/articles/5246403561495-UniFi-Gateway-Teleport-VPN) · [Ubiquiti release aggregator](https://releasebot.io/updates/ubiquiti)

---

Four things worth flagging about the research:

**The prompt's camera instruction doesn't quite fit this service line.** Cameras are Protect hardware, not Network hardware — the instruction looks like it was carried over from the Protect prompt template. I kept the link to [Security Cameras](./security-cameras.md) and gave it a real justification (cameras are the dominant PoE and bandwidth load a Network deployment must plan for, and several 10.x features exist because of them) rather than implying they're part of the Network catalog.

**The mobile app's pre-2023 history is partly reconstructed.** Ubiquiti's community release pages are JavaScript-rendered and the App Store only exposes about a year of version history, so the 4.x → 10.x renumbering and the "UniFi Network" → "UniFi" rename are dated from APKMirror upload records (4.5.0 beta in Nov 2022, 10.4.1 in May 2023) rather than a published changelog. Everything from September 2025 forward is firm.

**One source conflict, resolved toward the community feed.** The App Store's version-history summary attributes SSO SAML login to 10.37.0; Ubiquiti's community release page and the aggregated feed both put SAML in 10.36.2 and Spotlight search / UPS settings / Device Power Supervisor in 10.37.0. I used the community feed for feature attribution and the App Store only for dates, where the two agree.

**iOS and Android version numbers are not comparable.** Android runs about two minor versions ahead of iOS on the same features — Android 10.37.0 shipped in April 2026, iOS 10.37.0 in August. I called this out in the doc because it's the same trap the Protect app has, and Ubiquiti's own hardware requirements state per-platform minimums for exactly that reason.
