---
prompt: |-
  Ubiquiti the company and even more specifically [Unifi](https://ui.com) the brand are a growing force in the "prosumer" networking space that continues to gather momentum.

  Their network products include:

  - WAN Failover
  - WAN Routers
  - Switches
  - Access Points
  - UPS's

  and more.

  In addition to "network" gear Unifi has added the both the Protect and Access service lines: [Physical Security](https://ui.com/physical-security).

  Your job is to do the research so that you can provide an accurate overview of information on the **Unifi** offering, including:

  - who is the company? what is their history? who are there corporate officers? what are the companies stated goals?
  - how does "Unifi" the brand relate to "Ubiquiti" the company?
  - overview of the service offering including Network, Protect, Access, Identity, Innerspace, Talk, Connect, and any other service offereing you find

  Once the research has been written up as prose content in the body of this document, update the following Frontmatter in this document:

  - `last_updated` as "{{ctx.today}}"
  - `agent` as `{{ctx.agent}}/{{ctx.model}}`
  - `summary` as a one paragraph summary of the Unifi brand
last_updated: 2026-08-30
hash: 0605508c662877b0-d43641221f3cad3c
---
# Ubiquiti and the UniFi Brand

## Summary

UniFi is Ubiquiti Inc.'s flagship technology platform — a vertically integrated ecosystem in which Ubiquiti designs the hardware (gateways, switches, access points, cameras, door readers, phones, NAS units, displays and EV chargers), the embedded operating system (UniFi OS), the management applications that run on it (Network, Protect, Access, Talk, Connect, Identity, InnerSpace, Drive) and the cloud control plane that ties sites together (Site Manager / UniFi Fabrics). Its defining commercial choice is the absence of recurring per-device or per-seat licensing for the core stack: buy the box, get the software, manage it locally or through a free cloud plane. That combination of enterprise-grade capability, consumer-grade industrial design and near-zero software rent has made UniFi the default choice for the "prosumer" segment while pushing it steadily upmarket into mid-size and distributed enterprise, MSP and physical-security deployments. UniFi now accounts for roughly 91% of Ubiquiti's revenue, and Ubiquiti's fiscal 2026 revenue of $3.27 billion — up 27% year over year — is very largely a UniFi growth story.

## The Company

### Identity and structure

Ubiquiti Inc. is a Delaware corporation headquartered at 685 Third Avenue, 27th Floor, New York, NY 10017, trading on the New York Stock Exchange under the ticker **UI**. Its fiscal year ends 30 June. The company reports as a **single operating segment**, though it classifies revenue into two product categories: *Enterprise Technology* (the UniFi platform) and *Service Provider Technology* (airMAX, airFiber, UFiber GPON, Wave).

### History

Robert J. Pera founded the company after working as a wireless engineer at Apple from January 2003 to February 2005. There is a persistent, minor inconsistency in the public record about the founding date: Ubiquiti's own SEC filings state plainly that "the Company was founded by Robert Pera in 2005," while the corporate-registration lineage places incorporation in **October 2003 in San Jose, California**. The practical reading is that the entity was formed in 2003 and commercial operations began in 2005.

The first product was the **SR5**, a high-power mini-PCI radio card that let commodity PC hardware drive long-range wireless links. That set the pattern the company still follows: strip cost out of the hardware, put the differentiation in firmware and software, and sell through a community rather than a sales force.

Key milestones:

| Date        | Event                                                                          |
|-------------|--------------------------------------------------------------------------------|
| Oct 2003    | Incorporated in San Jose, California                                           |
| 2005        | Operations begin; SR5 radio card ships                                         |
| 13 Oct 2011 | IPO on NASDAQ (UBNT) — 7.04M shares at $15, raising ~$30.5M                                   |
| Oct 2012    | Pera acquires the NBA's Memphis Grizzlies                                      |
| Aug 2015    | Discloses a $46.7M business email compromise fraud                             |
| 2018–2019   | Moves listing to NYSE as **UI**; headquarters relocates to New York City       |
| 19 Aug 2019 | Legally renamed from *Ubiquiti Networks, Inc.* to **Ubiquiti Inc.**            |
| Jan 2021    | Discloses third-party cloud breach; insider extortion case follows             |
| May 2023    | Former employee Nickolas Sharp sentenced to six years for the extortion scheme |
| Apr 2026    | UniFi Fabrics reaches general availability                                     |

Two incidents are worth knowing because they shape how Ubiquiti talks about security and internal controls today. In 2015 the finance function of a Hong Kong subsidiary was defrauded of **$46.7 million** through impersonated executive requests; $8.1M was recovered immediately and the company booked a $39.1M net loss. No systems breach was found. Then in December 2020 a cloud-provider breach was disclosed; the "anonymous hacker" who had exfiltrated data and demanded 50 BTC turned out to be **Nickolas Sharp**, a Ubiquiti cloud engineer who was simultaneously serving on the incident-response team and who subsequently posed as a whistleblower to the press. He pleaded guilty in February 2023 and was sentenced in May 2023 to six years' imprisonment and $1,590,487 in restitution.

### Leadership and governance

Ubiquiti runs one of the leanest executive structures of any company its size. As of the most recent proxy, it designates just **two** executive officers:

- **Robert J. Pera** — Founder, Chief Executive Officer, and Chairman of the Board (chairman since December 2012). He is described in the company's own risk factors as "central to our product development efforts and overall strategic direction."
- **Kevin Radigan** — Chief Accounting and Finance Officer, and Principal Financial Officer. Notably, **Ubiquiti has no titled CFO**; Radigan has held the accounting role since May 2016, with the finance title added in fiscal 2019.

Other senior leaders who are not SEC-designated executive officers include **John R. Sanford** (Chief Technology Officer since May 2010), **Sandy Ro** (SVP Operations) and **Hartley Nisenbaum** (EVP Operations and Legal Affairs).

The board is similarly small: Pera as chairman, plus independent directors **Brandon Arrindell** and **Rafael Torres**, the latter chairing the Audit Committee.

Control is effectively absolute. As of August 2026 Pera beneficially owned **56,278,181 of 60,528,381 shares outstanding — roughly 93%** — and has stated he intends to continue owning a majority. The company flags this concentration explicitly as a risk factor. It also means headline market-cap figures are misleading: the aggregate value of stock held by *non-affiliates* was only about $2.34 billion at the end of calendar 2025, against a share price of $553.35.

### Stated goals and operating philosophy

Ubiquiti's mission statement in its own words is that it is **"focused on democratizing network technology on a global scale."** The operating model behind that phrase is unusually explicit in the filings:

- **Price-performance over feature checklists.** "Utilizing low-cost hardware and innovative software and firmware, we seek to build price-performance solutions to address both enterprises and service providers."
- **No traditional direct sales force.** Brand awareness is driven through online reviews, the website, distributors and the **Ubiquiti Community** — where "customers can interface directly with our R&D, marketing, and support teams."
- **Decentralized, flat R&D.** Development runs through "individual contributors or small development teams" with "little overlap in knowledge and responsibilities" — fast, but a concentration risk the company acknowledges.
- **Ease of deployment as a first-class design goal.** Solutions are "designed from the ground up with a focus on delivering highly-advanced and easily-deployable solutions."

### Financial and operational profile (FY2026, year ended 30 June 2026)

| Metric       | FY2026               | FY2025    | FY2024  |
|--------------|----------------------|-----------|---------|
| Revenue      | **$3,274.2M** (+27%) | $2,573.5M | $1.9B   |
| Net income   | **$960.3M**          | $711.9M   | $350.0M |
| Gross margin | **46%**              | 43%       | —       |
| R&D expense  | $204.2M              | $169.7M   | $159.8M |

Revenue by category and region:

| Breakdown                     | FY2026    | Change |
|-------------------------------|-----------|--------|
| Enterprise Technology (UniFi) | $2,972.3M | +32%   |
| Service Provider Technology   | $301.9M   | −5%    |
| North America                 | $1,744.0M | +35%   |
| EMEA                          | $1,179.2M | +18%   |
| Asia Pacific                  | $220.2M   | +30%   |
| South America                 | $130.8M   | +19%   |

Headcount as of 30 June 2026 was **1,818 full-time equivalents** including contractors — **1,321 in R&D**, 370 in operations, and just **127 in sales, general and administrative**. That ratio is the operating philosophy made visible. Geographically the workforce is 65% Asia Pacific, 23% EMEA, 12% Americas, with R&D teams in the United States, Taiwan, China, Latvia, the Czech Republic, Lithuania, Ukraine and Sweden.

Manufacturing is outsourced to contract manufacturers **primarily in Vietnam and China**. Distribution runs through **over 100 distributors**, online retailers and Ubiquiti's own webstores, reaching customers in over 75 countries during FY2026, with devices deployed in **over 200 countries and territories**. No single customer represents 10% or more of revenue.

On capital return, the board declared a **$1.00 per share quarterly dividend** in August 2026 and signalled intent to maintain at least that level through fiscal 2027. A $500M repurchase authorization approved in August 2025 was extended to September 2027, though no shares were repurchased during FY2026.

Two headwinds are called out prominently in the FY2026 filing: **US tariffs** on imports from China and Vietnam, which the company says have increased product costs and will continue to pressure margins for as long as they remain; and **component supply volatility**, where Ubiquiti depends on a small number of chipset suppliers under annual, terminable licence agreements that let it replace vendor firmware with its own.

### Competitive position

Ubiquiti names its competitors by market:

| Market                        | Named competitors                                                             |
|-------------------------------|-------------------------------------------------------------------------------|
| Enterprise WLAN and switching | Cisco, Fortinet, Hewlett Packard Enterprise (Aruba, Juniper), Ruckus (Belden) |
| Video surveillance            | Axis Communications, Hikvision, Hanwha Vision, Verkada                        |
| Wireless backhaul             | Cambium Networks, Ceragon Networks, MikroTik, Trango                          |
| CPE / ISP                     | Cambium Networks, MikroTik, Tarana Wireless, TP-Link                          |

The competitive factors it claims to win on are total cost of ownership, simplicity of deployment, speed of integrated product development, centralized management and breadth of suite — which is a fair description of why UniFi displaces Cisco/Meraki and Verkada in mid-market accounts.

## How "UniFi" Relates to "Ubiquiti"

**Ubiquiti is the company; UniFi is its flagship platform brand.** It is not a subsidiary, joint venture or separate legal entity — it is a product-line brand, and it has become so dominant that the two names are used interchangeably in the field.

Ubiquiti maintains several brand families:

| Brand                                 | Audience                                               | Management plane                               | Status                    |
|---------------------------------------|--------------------------------------------------------|------------------------------------------------|---------------------------|
| **UniFi**                             | Prosumer → enterprise, MSP, physical security          | UniFi OS + Site Manager                        | Flagship; ~91% of revenue |
| **UISP**                              | ISPs and WISPs (successor to EdgeMAX branding)         | UISP controller, with CRM/billing              | Active, declining         |
| **airMAX / airFiber / Wave / UFiber** | Point-to-point and point-to-multipoint transport, GPON | Per-device, with UISP overlay                  | Active, declining         |
| **AmpliFi**                           | Consumer mesh Wi-Fi                                    | AmpliFi mobile app only — a separate ecosystem | Legacy/consumer           |
| **WiFiman**                           | Free diagnostic utility                                | Standalone app                                 | Companion tool            |

The financial framing in the 10-K makes the relationship precise: *Enterprise Technology* "includes our UniFi platforms" and generated $2.97B of $3.27B in FY2026, while *Service Provider Technology* — the airMAX/airFiber/UFiber/Wave heritage that the company was originally built on — contributed $302M and **shrank 5%**. Ubiquiti today is, commercially, a UniFi company with a service-provider legacy business attached.

Within UniFi, the brand is itself an umbrella covering four layers:

1. **UniFi hardware** — gateways, switches, access points, cameras, door readers, phones, NAS, displays, EV chargers, power.
2. **UniFi OS** — the embedded operating system running on "consoles" (Dream Machines, Cloud Gateways, CloudKeys, NVRs), or self-hosted via **UniFi OS Server**.
3. **UniFi applications** — the modular apps installed onto a console (Network, Protect, Access, and so on).
4. **UniFi Site Manager / Fabrics** — the hybrid-cloud control plane spanning sites and organizations.

> **Naming caution.** "Unifi" is a heavily contested trademark string. Unrelated entities include **unifi**, the consumer broadband/TV/mobile brand of Malaysia's Telekom Malaysia, and **Unifi, Inc. (NYSE: UFI)**, a US textile-yarn manufacturer. Search results for "Unifi earnings" or "Unifi mobile" frequently return these instead of Ubiquiti.

## The UniFi Service Offering

### UniFi OS and the console model

Everything begins with a **UniFi console** — a device running UniFi OS onto which applications are installed, much like an app store for infrastructure. Consoles range from the compact Dream Router through the Dream Machine Pro / SE / Pro Max and Dream Wall up to the rack-mount **Enterprise Fortress Gateway (EFG)**, plus purpose-built consoles such as the NVR / NVR Pro (Protect) and UNAS units (Drive). Application availability depends on console storage and horsepower, which is why models increasingly ship with microSD, M.2 or SATA expansion.

Since 2025 the same stack can run on your own hardware via **UniFi OS Server**, which replaces the legacy standalone UniFi Network Server (now in maintenance mode — security fixes only). It installs on Windows via WSL2 or Linux via Podman (Docker is explicitly unsupported), serves its UI on port 11443, and is compatible with Site Manager. Ubiquiti also offers **Official UniFi Hosting** as a paid cloud-hosted alternative for MSPs that would rather not run the control plane themselves.

### The applications

**UniFi Network** — the core and the reason most people arrive. Manages gateways, switches and access points: routing, VLANs, firewalling, IPS/IDS, PoE, RF management, VPN, guest portals, and **Site Magic SD-WAN** for meshing gateways across sites. Pre-installed on every Network-capable console.

**UniFi Protect** — video surveillance, and now Ubiquiti's most aggressive competitive wedge. It is a **local-first** system: recording and AI inference happen on-premises, with secure remote access layered on top. Ubiquiti's own positioning is blunt — *"Run 4 or 4,000 cameras — licensing is still $0."* The camera portfolio spans bullet (G6 Pro, G6 Edge), dome and turret, compact/instant, PTZ, multi-sensor (AI Multi Sensor 2 and 4) and 360° panoramic, all PoE+ with 4K support. AI features on the G6 and AI series include person/vehicle/motion detection, **face recognition and licence-plate recognition**, audio classification (glass break, alarms) and detection-driven search. Against Verkada or Axis, the absence of per-camera licensing is the entire argument.

**UniFi Access** — door access control: touchscreen readers, door hubs, electronic locks, intercoms, and mobile credentials including **NFC badges in Apple Wallet**. It integrates natively with Protect (live video on unlock events) and with Identity/Endpoint for user provisioning. Together with Protect it forms the **UniFi Physical Security** line, which is Ubiquiti's most significant expansion beyond networking.

**UniFi Talk** — an on-premises VoIP PBX running on your own gateway rather than a vendor cloud, which is what enables desk phones to display camera feeds and unlock doors. Commercially it is the exception to the no-subscription rule: service costs roughly **$9.99 per number per month** (not per phone — numbers can be shared across devices via Groups), with tiered Plus and Pro plans carrying pooled minute and SMS allowances, call transcription, and CNAM lookup. Local extension-to-extension calling is free. Subscription service is limited to the **US, UK and Canada**; elsewhere, unlocked Talk phones can register against third-party SIP providers. Recent releases added call queuing and multi-line support.

**UniFi Connect** — Ubiquiti's "Enterprise of Things" application, covering the building-systems layer rather than the network layer:

- **Digital signage** — Connect Display and HDMI adapters turn any screen into a managed endpoint playing images, playlists, YouTube, web pages, custom URLs or Android apps, with a full mount/stand accessory ecosystem.
- **EV charging** — Level 2 AC stations (EV Station, EV Station Pro) with scheduling, user authentication, payment terminals, weatherproof enclosures, and integrated signage and cameras. NFC-based authentication requires UniFi Access.
- **Building lighting** and other connected fixtures.

**UniFi Identity** — the identity and access layer: one-click Wi-Fi, one-click VPN, door unlock and SaaS SSO from a single credential, with SCIM provisioning and SAML 2.0 / OIDC support. It ships in two tiers: a **license-free** tier providing one-click Wi-Fi, VPN and door access on any UniFi console, and **Identity Enterprise**, a cloud-managed paid tier (list ~$5/user/month, 5-user minimum, annual discount, 50% non-profit discount) adding multi-site management, adaptive VPN policies with behaviour-based MFA, and third-party SSO integration. Paid plans are currently **US-only**; the license-free tier is global.

**UniFi Endpoint** — the 2026 evolution of the Identity client, and strategically significant because it is **license-free**. It is a single desktop and mobile app through which users reach Wi-Fi, VPN and door access; combined with UniFi Fabrics it delivers identity-based access control, RBAC and **zero-trust network access** with optional external IdP binding (Microsoft Entra and FreeIPA LDAP are both supported, with SCIM sync for onboarding and offboarding). Anyone evaluating Identity Enterprise in 2026 should first check whether Endpoint plus Fabrics already covers the requirement at no cost.

**UniFi InnerSpace** — spatial visualization and RF planning. You upload or draw a floor plan, place devices, draw walls, and get modelled Wi-Fi coverage and camera fields of view. It replaced the older Maps feature and requires UniFi OS 3.2 or newer. Recent versions added camera calibration to map Protect detection coordinates onto the floor plan, plus detection-movement animation on the timeline. Its practical limits are worth knowing: walls must be traced manually and it does not distinguish material types, so predictions are directional rather than survey-grade. It is a **coverage planner, not an occupancy-analytics product** — a distinction that matters because an unrelated company, InnerSpace.io, sells exactly that.

**UniFi Drive** — the newest application, turning a console into network-attached storage. It powers the **UNAS** hardware line: rack-mount UNAS Pro (launched at $499), UNAS Pro 4 and UNAS Pro 8, plus desktop UNAS 2 and UNAS 4 (the UNAS 2 takes two 3.5" drives over a single PoE++-powered 2.5GbE port). The app is now in its 4.x generation. It handles file storage, backup, sharing and remote access with no subscription, but it is a single-purpose app, not a general-purpose NAS OS — there is no third-party app catalogue in the Synology sense.

### The control plane: Site Manager and UniFi Fabrics

**UniFi Site Manager** (`unifi.ui.com`) is the cloud interface for administering every site you own or hold admin rights on. **UniFi Fabrics**, announced in January 2026 and promoted to the stable branch in **April 2026**, rebuilt Site Manager into what Ubiquiti calls "a single control plane built for unlimited scale of sites and users."

Fabrics groups multiple sites under a shared structure on a hybrid-cloud architecture — policies orchestrated centrally, data retained on-premises. What it adds:

- **People and permissions at the Fabric level**, so user and admin access scales without per-site duplication.
- **Zero-touch provisioning and device blueprints/templates**, including full-site ZTP via mobile app.
- **Cross-site device visibility**, centralized logs and alarms, and staged firmware updates.
- **Native multi-tenancy**, turning Site Manager into a genuine MSP platform with isolated customer environments in one interface.
- **A Fabric API** with full documentation and Ansible support.
- **Native Identity integration** for zero-trust networking, binding external identity providers and applying user-based access policy.

Crucially: **no per-site licensing, no controller hosting fee, no tier upgrade.** Ubiquiti's stated 2026 theme is "UniFi at scale," and Fabrics is the mechanism.

### Networking and power hardware

The hardware families that the applications manage:

- **Cloud Gateways** — from Dream Router and Dream Machine up to the **Enterprise Fortress Gateway**, a 25G-class gateway with an 18-core ARM CPU and 16GB DDR4, dual 25G SFP28 / dual 10G SFP+ / dual 2.5GbE remappable ports, ~23.5 Gbps firewall routing and 12.5 Gbps with IPS, supporting 500+ devices and 5,000+ clients. It includes **NeXT AI Inspection** for license-free real-time inspection of encrypted TLS traffic.
- **Switching** — from compact PoE switches through Pro and Enterprise lines to **Enterprise Campus Aggregation** 100G switches with MC-LAG, plus specialist units such as the **USW Mission Critical**, which carries an integrated 368Wh lithium-ion battery to keep door access and critical PoE devices alive through an outage.
- **Wi-Fi** — the U6/U7 access point families and Enterprise-class APs, configured at scale through UniFi Network.
- **WAN resilience** — multi-WAN with **SLA-based failover** driven by real performance metrics (latency, packet loss, jitter) rather than link state alone, so a degraded-but-up circuit still triggers failover; cellular **LTE/5G backup**; **UniFi WAN Switches** for splitting a single ISP handoff across two gateways; and on the EFG, **Shadow Mode** — VRRP-based active/passive gateway HA with connection-state tracking that makes failover largely invisible to users.
- **Power** — the **SmartPower** ecosystem: a dedicated SmartPower port on rack-mount gateways, NVRs and Professional/Enterprise switches; the Redundant Power Supply (RPS) and UniFi Power Backup for automatic failover if an internal PSU fails; hot-swappable redundant PSUs on the EFG; and UniFi UPS units providing monitored battery backup with graceful shutdown and automatic recovery.
- **Wireless bridging** — building-to-building links, drawing on the airMAX/Wave heritage.

### Surrounding programmes and tooling

Beyond the products themselves, Ubiquiti has been building out the commercial scaffolding an enterprise buyer expects — historically its weakest area:

- **UniFi Design Center** and **UISP Design Center** — browser-based network planning tools.
- **WiFiman** — free Wi-Fi diagnostics and speed testing.
- **UniFi Academy** — structured training and certification.
- **UI Care** — five-year extended hardware protection, now resellable by partners.
- **Partner portal and directory**, tiered reseller pricing, global solution architects, and post-sales phone support with published SLAs.
- **Documentation improvements** including UniFi GPT and a version-controlled wiki.
- **Ecosystem integrations** with PSA, PMS, CRM and SIEM platforms, plus security partnerships with Proofpoint and Cloudflare.

## What to Watch

Three dynamics define UniFi's trajectory going into fiscal 2027:

1. **The license-free wedge is widening, not narrowing.** Fabrics, Endpoint and UniFi OS Server all shipped as free capabilities that competitors monetize as subscriptions. This is a deliberate attack on Meraki's and Verkada's economics, funded by a 46% hardware gross margin.
2. **Physical security is the growth engine.** Protect and Access give Ubiquiti a second, larger addressable market than networking alone, and the AI-camera line with on-device face and plate recognition puts it in direct contention with Verkada at a fraction of the running cost.
3. **The concentration risks are real and disclosed.** A single controlling shareholder who is also CEO and chief product strategist, two designated executive officers, a three-person board, contract manufacturing concentrated in Vietnam and China under active tariff pressure, and single-source chipset dependencies under annually terminable licences. Ubiquiti's speed comes from the same structural choices that create these exposures.

## Sources

**Primary (company filings and official material)**

- [Ubiquiti Inc. Form 10-K, fiscal year ended 30 June 2026](https://www.sec.gov/Archives/edgar/data/0001511737/000151173726000056/ubnt-20260630.htm) — financials, segments, geography, headcount, strategy, competitors, risk factors, Pera ownership
- [Ubiquiti Inc. Definitive Proxy Statement (DEF 14A), October 2025](https://www.sec.gov/Archives/edgar/data/1511737/000151173725000063/ubnt-20251024.htm) — executive officers
- [Ubiquiti Investor Relations — Board of Directors](https://ir.ui.com/company/board-of-directors)
- [Ubiquiti Inc. Form 10-K, FY2022](https://www.sec.gov/Archives/edgar/data/1511737/000151173722000049/ubnt-20220630.htm)
- [ui.com](https://ui.com/) — product and application lineup
- [UniFi Physical Security](https://ui.com/physical-security)
- [UniFi Identity](https://ui.com/identity)
- [Introducing UniFi Fabrics — Ubiquiti blog](https://blog.ui.com/article/introducing-unifi-fabrics)
- [Introducing UniFi OS Server for MSPs — Ubiquiti blog](https://blog.ui.com/article/introducing-unifi-os-server)
- [UniFi Enterprise Fortress Gateway — Tech Specs](https://techspecs.ui.com/unifi/cloud-gateways/efg)
- [Enterprise Scale UniFi Cloud Gateways](https://ui.com/cloud-gateways/enterprise-scale)
- [UniFi Digital Signage](https://ui.com/integrations/digital-signage) · [UniFi EV Charging](https://ui.com/integrations/premium-iot/ev-charging) · [UniFi Managed VoIP](https://ui.com/integrations/managed-voip)
- [Getting Started with UniFi Connect — Ubiquiti Help Center](https://help.ui.com/hc/en-us/articles/21171115391255-Getting-Started-with-UniFi-Connect)
- [UniFi Endpoint Overview — Ubiquiti Help Center](https://help.ui.com/hc/en-us/articles/16936895868823-UniFi-Endpoint-Overview)
- [UID Enterprise Plan and Billing — Ubiquiti Help Center](https://help.ui.com/hc/en-us/articles/18114884352663-UID-Enterprise-Plan-and-Billing)
- [Manage UniFi Talk Subscriptions — Ubiquiti Help Center](https://help.ui.com/hc/en-us/articles/360058776614-Manage-UniFi-Talk-Subscriptions)
- [An Overview of High Availability in UniFi — Ubiquiti Help Center](https://help.ui.com/hc/en-us/articles/30297446527767-An-Overview-of-High-Availability-in-UniFi)
- [Binding an Identity Provider to a UniFi Fabric — Ubiquiti Help Center](https://help.ui.com/hc/en-us/articles/30967924245527-Binding-an-Identity-Provider-IdP-To-A-UniFi-Fabric)
- [UniFi InnerSpace 1.2.9 release notes — Ubiquiti Community](https://community.ui.com/releases/UniFi-InnerSpace-1-2-9/7ef34b2f-607d-4f52-bf2c-658894651ac1)
- [UniFi Drive Application 4.0.12 — Ubiquiti Community](https://community.ui.com/releases/UniFi-Drive-Application-4-0-12/b609bfd0-c2af-41c1-bd15-2ebc456dd8f2)

**Secondary (history, incidents, analysis)**

- [Ubiquiti — Wikipedia](https://en.wikipedia.org/wiki/Ubiquiti) · [Robert Pera — Wikipedia](https://en.wikipedia.org/wiki/Robert_Pera)
- [Tech Firm Ubiquiti Suffers $46M Cyberheist — Krebs on Security](https://krebsonsecurity.com/2015/08/tech-firm-ubiquiti-suffers-46m-cyberheist/)
- [Former Ubiquiti dev who extorted the firm gets six years in prison — BleepingComputer](https://www.bleepingcomputer.com/news/security/former-ubiquiti-dev-who-extorted-the-firm-gets-six-years-in-prison/)
- [Former Ubiquiti Employee Gets 6 Years — The Hacker News](https://thehackernews.com/2023/05/former-ubiquiti-employee-gets-6-years.html)
- [What's New with Ubiquiti in 2026? — Tech Field Day](https://techfieldday.com/video/whats-new-with-ubiquiti-in-2026/)
- [UniFi Fabrics Is Out of Beta — GhostSyght](https://www.ghostsyght.com/p/unifi-fabrics-is-out-of-beta-heres)
- [UniFi OS, Explained — Dong Knows Tech](https://dongknows.com/ubiquiti-unifi-network-ecosystem-review/) · [UniFi Drive, Explained](https://dongknows.com/ubiquiti-unifi-drive-review/) · [UniFi Protect Review](https://dongknows.com/ubiquiti-unifi-protect-security-camera-system-review/)
- [Ubiquiti UNAS Pro Review — StorageReview](https://www.storagereview.com/review/ubiquiti-unas-pro-review-streamlined-storage-for-unifi-enthusiasts)
- [UniFi Talk Business Review 2026 — iFeeltech](https://ifeeltech.com/blog/unifi-talk-voip-business-communication)
- [Ubiquiti FAQ / Guide — McCann Tech](https://evanmccann.net/ubiquiti)
- [UniFi InnerSpace: Managing Floorplans and Network Coverage — Securing The Universe](https://securingtheuniverse.com/2025/11/10/unifi-innerspace-managing-floorplans-and-network-coverage/)
- [Everything You Need to Know About UniFi OS Server — Cody Deluisio](https://deluisio.com/networking/unifi/2025/08/03/everything-you-need-to-know-about-unifi-os-server-before-you-waste-time-testing-it/)
