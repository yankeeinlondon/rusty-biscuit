---
prompt: |-
    Research all the routers offerred under the **Unifi** brand (https://ui.com).
    
    - identify the key properties/metrics which these routers have that are meaningful in terms of their capability and scalability
    - organize the routers into categories (UDM, ...) as H2 headings 
    - add an H3 heading for each router product:
        - describe the product
    - provide a price point in both USD and GBP
    - provide a link to one or more images of the product
    - provide the properties/metrics which characterize this product
    - add an H2 \"Summary\" section which includes a comparison table of the routers offered
    
    Once the document's body has been written in prose style, you must add the following frontmatter properties to the document as well:
        
    - `last_updated` as \"{{ctx.today}}\"
    - `routers` as a list of each router (router product name is key, key attributes are listed underneath as key/value pairs)
    - `agent` as '{{ctx.agent}}/{{ctx.model}}'
    
    Note: prefer US english for this content
last_updated: 2026-08-30
hash: d85d3b4d8d28e406-a4745bf1f30ae4ac
---
# UniFi Routers and Cloud Gateways

Ubiquiti's UniFi line no longer distinguishes meaningfully between "router" and "network controller". Almost every routing product it sells today is a **Cloud Gateway** — a box that runs UniFi OS, routes and firewalls your traffic, *and* hosts the UniFi Network application (plus, on most models, Protect, Access, Talk and Connect) with no separate controller and no recurring licence. A smaller parallel family, the **UXG gateways**, deliberately omits the controller for people who already run one elsewhere. At the extremes sit the rack-mount **Enterprise Firewalls** and the pocket-sized **Travel Routers**.

This document covers every routing product currently listed on [ui.com](https://ui.com), grouped by family, with the specifications that actually determine what a given box can carry.

## How to Read the Numbers

Ubiquiti publishes a lot of specifications. Only a handful of them change buying decisions.

**IDS/IPS throughput is the real speed limit.** Every gateway will forward traffic at line rate with the firewall doing nothing interesting. The number Ubiquiti publishes — 1 Gbps, 2.3 Gbps, 5 Gbps, 12.5 Gbps, 25 Gbps, 79 Gbps — is throughput *with intrusion detection and prevention enabled*, and it is the figure that matters. A Cloud Gateway Ultra behind a 2.5 Gbps fibre service will cap you at roughly 40% of what you pay for once threat detection is on. Match this number to your WAN speed, not to your port speeds.

**Managed UniFi devices** is the count of switches, access points, cameras, door readers and phones the built-in controller is rated to adopt. It scales from 4 (UniFi Express) through 30+, 50+, 100+, 200+, 500+ to 2,250+ (Enterprise Firewall Core). Exceeding it doesn't hard-fail, it degrades — the controller is doing the work.

**Simultaneous connected users** is the client-device ceiling: 50+ at the bottom, 22,500+ at the top.

**Concurrent sessions and new sessions per second** only appear on the enterprise models (10 million sessions; 71,000–120,000 new sessions/sec). These are the metrics that separate a large-campus firewall from a big prosumer box, and they are what NAT-heavy or scanner-heavy environments actually exhaust first.

**Maximum WAN port count and default WAN ports** determine multi-WAN capability. Ubiquiti lets you remap LAN ports to WAN on most models, so "max WAN ports: 8" means eight independently load-balanced or failover-ordered uplinks. This, plus the presence of a SIM slot or a cellular modem, is the availability story.

**Port media and speed** — 1 GbE RJ45, 2.5 GbE RJ45, 10 GbE RJ45, 10G SFP+, 25G SFP28, 100G QSFP28 — sets both the WAN ceiling and how the box attaches to your switching fabric. SFP+ and SFP28 also mean fibre without media converters.

**Integrated switching and PoE budget** decides whether the gateway is the whole network or just the head of it. Budgets range from none (Ultra, Max, Pro-Max) through 15.4 W (Dream Router 7) and 30 W (Cloud Gateway Fiber) to 180 W (UDM-SE), 270 W (Industrial) and 420 W (Dream Wall).

**Integrated WiFi** separates the all-in-one models (Express 7, Dream Router 7, Dream Wall) from the routing-only models that assume you will add access points.

**NVR storage** — 3.5" bays, NVMe slots, microSD, onboard SSD — is what makes a gateway able to run UniFi Protect. The published **managed camera counts** (HD / 2K / 4K) are the honest capacity figure, since 4K cameras consume roughly three times the resources of HD ones.

**Managed access hubs** is the equivalent metric for UniFi Access door controllers.

**High availability** appears only at the top: Shadow Mode with VRRP failover between two gateways, dual hot-swappable power supplies, and DC input from a UniFi Power Backup / USP-RPS unit.

**SSL/TLS inspection sessions** (10,000 concurrent on EFG, UXG-Enterprise and EF-Core) reflect Ubiquiti's licence-free NeXT AI encrypted-traffic inspection — a genuine differentiator against vendors who charge per-seat for the same capability.

**Signature count** varies by hardware class: 20,000+ on the small WiFi-integrated models, 55,000+ on mainstream gateways, and 95,000+ on the Enterprise Firewall with CyberSecure Enterprise.

One commercial note running through all of it: the core software carries no per-device or per-seat licence. The only paid subscriptions are CyberSecure (enhanced signature feeds) and UI Care (extended hardware coverage).

> **Pricing basis.** All prices below are Ubiquiti's own list prices on [store.ui.com](https://store.ui.com) (USD) and [uk.store.ui.com](https://uk.store.ui.com) (GBP) as of **30 August 2026**. The US store separately displays a higher "surcharge included" figure; the base list price is quoted here. Regional taxes are added at checkout.

## Enterprise Firewalls

The two rack-mount models Ubiquiti positions as firewalls first and gateways second. Both carry redundant hot-swap power, both do licence-free SSL/TLS inspection, and both are rated in millions of concurrent sessions rather than hundreds of clients.

### Enterprise Firewall Core (EF-Core)

The largest thing Ubiquiti makes. A 1U firewall built around four 100G QSFP28 ports and four 25G SFP28 ports, rated at 79 Gbps of IDS/IPS throughput and 22,500+ concurrent users — an aggregation-layer device for a campus core or a service-provider edge, not a branch box. It is the only UniFi product where the published headline figure ("100 Gbps") refers to fabric capacity rather than inspected throughput.

- **Price:** $3,499 USD / £2,785 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/8f56712e-ed57-48ca-a180-f8fea2d343c3/c2638146-3fb0-4e52-88c9-c6cec6ef051a.png) · [angle](https://cdn.ecomm.ui.com/products/8f56712e-ed57-48ca-a180-f8fea2d343c3/8e2ae7be-0dcd-47cf-9332-6d36b60a2bf9.png) · [rear](https://cdn.ecomm.ui.com/products/8f56712e-ed57-48ca-a180-f8fea2d343c3/2ba5d7f9-ff22-4bbd-999f-5ed3b3fbe687.png)

| Property                    | Value                                                                                     |
|-----------------------------|-------------------------------------------------------------------------------------------|
| IDS/IPS throughput          | 79 Gbps                                                                                   |
| Ports                       | (4) 100G QSFP28, (4) 25G SFP28, (8) 10G RJ45, (2) 1G RJ45, (1) 1G management, (1) console |
| Max WAN ports               | 8                                                                                         |
| Managed UniFi devices       | 2,250+                                                                                    |
| Simultaneous users          | 22,500+                                                                                   |
| Concurrent sessions         | 10 million                                                                                |
| New sessions/sec            | 120,000                                                                                   |
| SSL/TLS inspection sessions | 10,000                                                                                    |
| Power                       | 171 W max; (2) hot-swappable 550 W AC/DC PSUs                                             |
| Form factor                 | 1U rack — 442.4 × 43.7 × 400 mm, 7.85 kg                                                  |

### Enterprise Firewall / Enterprise Fortress Gateway (EFG)

A 25G firewall on an 18-core ARM v8.2 platform, with dual 25G SFP28 uplinks, dual hot-swap CRPS supplies and Shadow Mode high availability (an idle standby unit that takes over via VRRP). At 12.5 Gbps of IDS/IPS it is slower than the cheaper Dream Machine Beast; what you are buying is redundancy and session capacity, not raw inspection speed. Ubiquiti has been migrating the store name from "Enterprise Fortress Gateway" to plain "Enterprise Firewall", but the SKU remains EFG.

- **Price:** $1,999 USD / £1,659 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/65adb8bd-c318-45f9-8b9f-9c15fb025ec2/141e6fb0-c7af-4bd7-a3a1-782d9f2c69ad.png) · [angle](https://cdn.ecomm.ui.com/products/65adb8bd-c318-45f9-8b9f-9c15fb025ec2/e377d956-0a02-41ea-9ee6-1a40288fd8fa.png) · [rear](https://cdn.ecomm.ui.com/products/65adb8bd-c318-45f9-8b9f-9c15fb025ec2/f2057450-0b8e-452a-a6c4-6e7284c4c758.png)

| Property                    | Value                                         |
|-----------------------------|-----------------------------------------------|
| IDS/IPS throughput          | 12.5 Gbps                                     |
| Ports                       | (2) 25G SFP28, (2) 10G SFP+, (2) 2.5 GbE RJ45 |
| Default WAN                 | (1) 25G SFP28 + (1) 2.5 GbE RJ45              |
| Max WAN ports               | 5                                             |
| Managed UniFi devices       | 500+                                          |
| Simultaneous users          | 5,000+                                        |
| Concurrent sessions         | 10 million                                    |
| New sessions/sec            | 71,000                                        |
| SSL/TLS inspection sessions | 10,000                                        |
| Signatures                  | 95,000+ with CyberSecure Enterprise           |
| High availability           | Shadow Mode (VRRP); (2) hot-swap 150 W CRPS   |
| Processor                   | 18-core ARM v8.2 @ 2 GHz                      |
| Power                       | 82 W max                                      |
| Form factor                 | 1U rack — 442.4 × 43.7 × 325 mm, 6.5 kg       |

## Dream Machines (UDM)

The rack-mount Cloud Gateways: full UniFi OS, full application suite, integrated NVR storage, and — on the SE — integrated PoE switching. This is the mainstream choice for a building.

### Dream Machine Beast (UDM-Beast)

The most capable non-firewall gateway Ubiquiti sells and, on paper, the best value in the whole lineup. Eight 10 GbE RJ45 ports, dual 25G SFP28, 25 Gbps of IDS/IPS throughput and 7,500+ users, in the same 1U chassis as the UDM-Pro-Max. It out-performs the more expensive Enterprise Firewall on raw inspection and port density; what it gives up is the EFG's dual hot-swap power supplies and Shadow Mode failover.

- **Price:** $1,499 USD / £1,195 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/352e353d-16e7-45b5-805e-712848145c65/6f1026d5-fa0c-424d-83a1-5f648213a814.png) · [angle](https://cdn.ecomm.ui.com/products/352e353d-16e7-45b5-805e-712848145c65/813bba48-7429-4816-a94d-c27e4ca84bb0.png) · [rear](https://cdn.ecomm.ui.com/products/352e353d-16e7-45b5-805e-712848145c65/5f247467-4330-4a9b-9c04-e7e4b0951ce9.png)

| Property              | Value                                                        |
|-----------------------|--------------------------------------------------------------|
| IDS/IPS throughput    | 25 Gbps                                                      |
| Ports                 | (2) 25G SFP28, (2) 10G SFP+, (8) 10 GbE RJ45, (2) 1 GbE RJ45 |
| Default WAN           | (1) 25G SFP28 + (1) 10 GbE RJ45                              |
| Max WAN ports         | 8                                                            |
| Managed UniFi devices | 750+                                                         |
| Simultaneous users    | 7,500+                                                       |
| Managed cameras       | 100 HD / 60 2K / 40 4K                                       |
| Managed access hubs   | 200                                                          |
| Storage               | 128 GB SSD + (2) 3.5" NVR HDD bays; 16 GB RAM                |
| Power                 | 100 W max; internal 150 W AC/DC                              |
| Form factor           | 1U rack — 442.4 × 43.7 × 325 mm, 5.5 kg                      |

### Dream Machine Pro Max (UDM-Pro-Max)

The redundancy-minded sibling of the UDM-Pro: twice the device and client capacity, 5 Gbps of IDS/IPS, and — the reason most people buy it — two 3.5" drive bays for mirrored Protect recordings rather than one. It also accepts DC input from a USP-RPS redundant power system.

- **Price:** $599 USD / £475 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/401190d7-6a49-4c2e-bef1-7fe087d2b6b6/d7080e47-b0ae-409a-ac4b-28d9cc5c541a.png) · [rear](https://cdn.ecomm.ui.com/products/401190d7-6a49-4c2e-bef1-7fe087d2b6b6/aafdb4ed-dda2-4b8f-a80d-84cb521845bb.png)

| Property              | Value                                                    |
|-----------------------|----------------------------------------------------------|
| IDS/IPS throughput    | 5 Gbps                                                   |
| Ports                 | (2) 10G SFP+, (1) 2.5 GbE RJ45, (8) 1 GbE RJ45           |
| Max WAN ports         | 8                                                        |
| Managed UniFi devices | 200+                                                     |
| Simultaneous users    | 2,000+                                                   |
| Managed cameras       | 50 HD / 25 2K / 15 4K                                    |
| Managed access hubs   | 150                                                      |
| PoE                   | None                                                     |
| Storage               | 128 GB SSD + (2) 3.5" NVR HDD bays; 32 GB eMMC           |
| Power                 | 60 W max; internal 100 W AC/DC, or 11.5 V DC via USP-RPS |
| Form factor           | 1U rack — 442.4 × 43.7 × 285.6 mm, 4.7 kg                |

### Dream Machine Special Edition (UDM-SE)

The UDM-Pro with a 180 W PoE switch built in. Eight of its gigabit ports deliver PoE/PoE+, which for a small site means the gateway, the switch and the camera/AP power all collapse into one rack unit. Routing performance is identical to the UDM-Pro at 3.5 Gbps.

- **Price:** $499 USD / £395 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/1b6fcc08-a6b8-4496-a831-6125a47c412f/c1d1e0e0-4ec6-4760-9bc2-81cdfdf3eaa5.png) · [angle](https://cdn.ecomm.ui.com/products/1b6fcc08-a6b8-4496-a831-6125a47c412f/522e6fc0-1cb4-4b32-afd5-2495c26625fc.png) · [rear](https://cdn.ecomm.ui.com/products/1b6fcc08-a6b8-4496-a831-6125a47c412f/ba13e628-0e59-4993-b8e4-4a7835c787d9.png)

| Property              | Value                                                     |
|-----------------------|-----------------------------------------------------------|
| IDS/IPS throughput    | 3.5 Gbps                                                  |
| Ports                 | (2) 10G SFP+, (1) 2.5 GbE RJ45, (8) 1 GbE RJ45 (PoE/PoE+) |
| Max WAN ports         | 8                                                         |
| Managed UniFi devices | 100+                                                      |
| Simultaneous users    | 1,000+                                                    |
| PoE budget            | 180 W (15.4 W PoE / 30 W PoE+ per port)                   |
| Storage               | 128 GB SSD + (1) 3.5" NVR HDD bay; 16 GB eMMC             |
| Power                 | 50 W max excl. PoE; internal 240 W AC/DC, or 52 V DC      |
| Form factor           | 1U rack — 442.4 × 43.7 × 285.6 mm, 5 kg                   |

### Dream Machine Pro (UDM-Pro)

The model that defined the category and still the cheapest way into a rack-mount UniFi gateway with Protect storage. Nine gigabit RJ45 ports, two 10G SFP+, one 3.5" drive bay, a 1.3" front touchscreen, and 3.5 Gbps of IDS/IPS. Its constraint is the gigabit-only copper: with a multi-gig WAN you will be using the SFP+ port.

- **Price:** $379 USD / £300 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/9df27ed4-c4ae-471a-8982-f5b0650da76a/7997cc11-b8c5-48e0-8b7e-bed4ded30898.png) · [angle](https://cdn.ecomm.ui.com/products/9df27ed4-c4ae-471a-8982-f5b0650da76a/7109a335-609b-4663-976b-e76dd142c23c.png) · [rear](https://cdn.ecomm.ui.com/products/9df27ed4-c4ae-471a-8982-f5b0650da76a/e657c9ee-b453-4182-bc31-b22e51a6111f.png)

| Property              | Value                                            |
|-----------------------|--------------------------------------------------|
| IDS/IPS throughput    | 3.5 Gbps                                         |
| Ports                 | (2) 10G SFP+, (9) 1 GbE RJ45                     |
| Default WAN           | (1) 10G SFP+ + (1) 1 GbE RJ45                    |
| Max WAN ports         | 8                                                |
| Managed UniFi devices | 100+                                             |
| Simultaneous users    | 1,000+                                           |
| Managed cameras       | 24 HD / 14 2K / 8 4K                             |
| PoE                   | None                                             |
| Storage               | (1) 3.5" NVR HDD bay; 16 GB onboard, 4 GB RAM    |
| Power                 | 33 W max; internal 50 W AC/DC, or DC via USP-RPS |
| Form factor           | 1U rack — 442.4 × 43.7 × 285.6 mm, 3.9 kg        |

## Cloud Gateways (UCG)

Compact, desktop or shelf-mounted Cloud Gateways with no integrated WiFi. These assume you will pair them with a UniFi switch and access points, and they are where the price/performance curve is steepest.

### Cloud Gateway Fiber (UCG-Fiber)

The value standout of the range. Two 10G SFP+ ports, a 10 GbE RJ45 port, a four-port 2.5 GbE switch with one PoE+ port, and 5 Gbps of IDS/IPS — matching the $599 UDM-Pro-Max on inspection throughput for less than half the money. Protect storage is an optional NVMe SSD rather than a spinning bay. The "fiber" in the name is the pair of SFP+ cages, one usable as WAN.

- **Price:** $279 USD / £220 GBP (no-storage variant; 1 TB and 2 TB NVMe options cost more)
- **Images:** [front](https://cdn.ecomm.ui.com/products/48cf74fa-0456-4c5f-bbcc-c1a1ffdc11f9/465257f3-0acc-4a11-bb15-762e7f6c0e9c.png) · [rear](https://cdn.ecomm.ui.com/products/48cf74fa-0456-4c5f-bbcc-c1a1ffdc11f9/8893271d-4212-4471-b3fb-942a7a72ac57.png)

| Property              | Value                                                    |
|-----------------------|----------------------------------------------------------|
| IDS/IPS throughput    | 5 Gbps                                                   |
| Ports                 | (2) 10G SFP+, (1) 10 GbE RJ45, (4) 2.5 GbE RJ45 (1 PoE+) |
| Default WAN           | (1) 10G SFP+ + (1) 10 GbE RJ45                           |
| Max WAN ports         | 6                                                        |
| Managed UniFi devices | 50+                                                      |
| Simultaneous users    | 500+                                                     |
| Managed cameras       | 15 HD / 8 2K / 5 4K                                      |
| Managed access hubs   | 50                                                       |
| PoE budget            | 30 W                                                     |
| Storage               | Selectable NVMe SSD up to 2 TB                           |
| Power                 | 29.4 W max excl. PoE; 54 V DC/1.1 A adapter              |
| Form factor           | Desktop — 212.8 × 127.6 × 30 mm, 675 g                   |

### Cloud Gateway Max (UCG-Max)

Five 2.5 GbE ports, 2.3 Gbps of IDS/IPS, and an optional NVMe slot for Protect. It is the smallest gateway that can genuinely run cameras, and the right answer for a 2 Gbps fibre service where the Ultra would throttle you. Sold as UCG-Max-NS (no storage) or with 512 GB / 1 TB / 2 TB pre-installed.

- **Price:** from $199 USD / from £159 GBP (no-storage variant; the 512 GB SKU lists at $279 / £220)
- **Images:** [front](https://cdn.ecomm.ui.com/products/8cca3680-14a6-496a-af7d-beba49cea3f2/7c6f4e54-1f20-485a-a0f0-22e968b66a66.png) · [rear](https://cdn.ecomm.ui.com/products/8cca3680-14a6-496a-af7d-beba49cea3f2/c0131143-50cb-4ce2-b9ce-4dd61820302c.png)

| Property              | Value                                        |
|-----------------------|----------------------------------------------|
| IDS/IPS throughput    | 2.3 Gbps                                     |
| Ports                 | (5) 2.5 GbE RJ45                             |
| Default WAN           | (1) 2.5 GbE RJ45                             |
| Max WAN ports         | 4                                            |
| Managed UniFi devices | 30+                                          |
| Simultaneous users    | 300+                                         |
| Managed cameras       | 15 HD / 8 2K / 5 4K                          |
| Managed access hubs   | 50                                           |
| PoE                   | None                                         |
| Storage               | Selectable NVMe SSD up to 2 TB               |
| Processor             | Quad-core ARM Cortex-A53 @ 1.5 GHz, 3 GB RAM |
| Power                 | 16.1 W max; USB-C (5 V DC/5 A)               |
| Form factor           | Desktop — 141.8 × 127.6 × 30 mm, 460 g       |

### Cloud Gateway Ultra (UCG-Ultra)

The entry point: a USB-C-powered box that folds the old USG-plus-Cloud-Key pairing into one unit, with a 2.5 GbE WAN port, four gigabit LAN ports, and multi-WAN load balancing. There is no NVR storage, no PoE and no WiFi — it is a router and controller and nothing else, which is exactly why it is $129. The 1 Gbps IDS/IPS ceiling is the thing to check against your broadband speed.

- **Price:** $129 USD / £79 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/8d2d9e4b-89f3-49a1-9c17-5d774c0067b4/2e179331-f85a-4bc9-bf3e-d00192522732.png) · [rear](https://cdn.ecomm.ui.com/products/8d2d9e4b-89f3-49a1-9c17-5d774c0067b4/62a5adb2-e21a-4b3d-bad8-7485ddb87256.png)

| Property              | Value                                  |
|-----------------------|----------------------------------------|
| IDS/IPS throughput    | 1 Gbps                                 |
| Ports                 | (1) 2.5 GbE RJ45, (4) 1 GbE RJ45       |
| Max WAN ports         | 4                                      |
| Managed UniFi devices | 30+                                    |
| Simultaneous users    | 300+                                   |
| PoE                   | None                                   |
| Storage               | 16 GB onboard, 3 GB RAM — no NVR       |
| Processor             | Quad-core ARM Cortex-A53 @ 1.5 GHz     |
| Power                 | 6.2 W max; USB-C (5 V DC/3 A)          |
| Form factor           | Desktop — 141.8 × 127.6 × 30 mm, 520 g |

### Cloud Gateway Industrial (UCG-Industrial)

A ruggedised, fanless, −30 °C to 50 °C gateway with a 270 W PoE budget across two PoE+++ (90 W) and two PoE+ (30 W) ports, plus a 10 GbE PoE+++ port and a 10G SFP+ uplink. It carries a SIM slot for remote-SIM operation with a UniFi 5G Max Outdoor unit, making it the natural head-end for outdoor, industrial or vehicle-adjacent installations where a Dream Machine's fan and temperature range are disqualifying.

- **Price:** $579 USD / £460 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/b5db03fd-a838-4c6c-a1ee-1451a09d41d4/20c550c3-e1be-4c33-ab66-c2b9e57e845f.png) · [angle](https://cdn.ecomm.ui.com/products/b5db03fd-a838-4c6c-a1ee-1451a09d41d4/9635f89d-db57-4d0f-bb2d-40999debed54.png) · [rear](https://cdn.ecomm.ui.com/products/b5db03fd-a838-4c6c-a1ee-1451a09d41d4/f2c58d7f-4d3a-4fee-b6c8-9345cd4175b0.png)

| Property              | Value                                                                       |
|-----------------------|-----------------------------------------------------------------------------|
| IDS/IPS throughput    | 5 Gbps                                                                      |
| Ports                 | (1) 10G SFP+, (1) 10 GbE RJ45 (PoE+++), (4) 2.5 GbE RJ45 (2 PoE+++, 2 PoE+) |
| Default WAN           | (1) 10G SFP+ + (1) 10 GbE RJ45 + (1) 2.5 GbE RJ45                           |
| Max WAN ports         | 5                                                                           |
| Managed UniFi devices | 50+                                                                         |
| Simultaneous users    | 500+                                                                        |
| Managed cameras       | 15 HD / 8 2K / 5 4K                                                         |
| PoE budget            | 270 W DC input + 75 W ATX (90 W per PoE+++ port)                            |
| Cellular              | SIM card slot; remote-SIM with UniFi 5G Max Outdoor                         |
| Storage               | Pre-installed 128 GB microSD for NVR                                        |
| Environment           | Fanless; −30 to 50 °C                                                       |
| Power                 | 28 W max excl. PoE; 54 V DC 350 W supply                                    |
| Form factor           | Shelf/DIN — 307.9 × 203.5 × 43.7 mm, 2.4 kg                                 |

## WiFi-Integrated Gateways (UDR / UDW / UX)

All-in-one boxes: gateway, controller and access point in a single unit, usually with a small PoE switch attached. These are the "one device is the whole network" products.

### Dream Wall (UDW)

A wall-mounted 420 W PoE switch, WiFi 6 access point and 10G Cloud Gateway in one flat panel, with dual hot-swappable 550 W power supplies. Seventeen gigabit ports (four PoE++, four PoE+, four PoE), two 10G SFP+ and a 2.5 GbE port make it a complete IDF replacement for a home or small office where there is no rack. At 3.5 Gbps IDS/IPS it routes like a UDM-Pro.

- **Price:** $999 USD / £765 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/df8c3478-6280-45c0-80e3-78915f9c17c1/cda265d1-74c0-4eec-b6aa-e54bbf1fa0c8.png) · [angle](https://cdn.ecomm.ui.com/products/df8c3478-6280-45c0-80e3-78915f9c17c1/55ea5bc4-33b7-4a44-964f-1d2311e75f04.png) · [detail](https://cdn.ecomm.ui.com/products/df8c3478-6280-45c0-80e3-78915f9c17c1/b6fa08e7-c3c4-43e1-971d-e226558d9d4d.png)

| Property              | Value                                                                    |
|-----------------------|--------------------------------------------------------------------------|
| IDS/IPS throughput    | 3.5 Gbps                                                                 |
| Ports                 | (2) 10G SFP+, (1) 2.5 GbE RJ45, (17) 1 GbE RJ45 (4 PoE++, 4 PoE+, 4 PoE) |
| Max WAN ports         | 8                                                                        |
| Managed UniFi devices | 100+                                                                     |
| Simultaneous users    | 300+                                                                     |
| Managed cameras       | 12 HD / 7 2K / 4 4K                                                      |
| WiFi                  | WiFi 6 — 5 GHz 2.4 Gbps 4×4, 2.4 GHz 300 Mbps 2×2                        |
| PoE budget            | 420 W (15.4 / 30 / 60 W per port)                                        |
| Storage               | 512 GB microSD (NVR) + 128 GB SSD + 16 GB eMMC                           |
| Power                 | 532 W max; (2) hot-swappable 550 W PSUs                                  |
| Form factor           | Wall mount — 549 × 342 × 62 mm, 8.6 kg                                   |

### Dream Router 5G Max (UDR-5G-Max)

A Dream Router 7 with a full-performance 5G NR sub-6 modem bolted on — up to 3.4 Gbps downlink — plus two nano-SIM slots (one eSIM-capable) and a 4.7" touchscreen. This is the model for sites where cellular is the primary link or a first-class failover path rather than a slow last resort. Certified on AT&T and T-Mobile, with Verizon support pending.

- **Price:** $499 USD / £395 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/59acabe1-9d17-43dc-8015-dfb95897fa11/7da91ff3-a17a-4c0d-80f5-f9f611c6c094.png) · [angle](https://cdn.ecomm.ui.com/products/59acabe1-9d17-43dc-8015-dfb95897fa11/b246b2f5-4e6c-450a-a8dc-659d7806b897.png) · [rear](https://cdn.ecomm.ui.com/products/59acabe1-9d17-43dc-8015-dfb95897fa11/1552469e-2fd2-4883-81be-243458222cc6.png)

| Property              | Value                                                                            |
|-----------------------|----------------------------------------------------------------------------------|
| IDS/IPS throughput    | 2.3 Gbps                                                                         |
| Ports                 | (1) 10G SFP+, (4) 2.5 GbE RJ45 (1 PoE)                                           |
| Default WAN           | (1) 10G SFP+ + (1) 2.5 GbE RJ45                                                  |
| Max WAN ports         | 4                                                                                |
| Cellular              | 5G NR sub-6 (n1–n79), 4G LTE (31 bands), 3G UMTS; peak 3.4 Gbps DL / 560 Mbps UL |
| SIM                   | (2) nano-SIM, (1) of which supports eSIM                                         |
| WiFi                  | WiFi 7 tri-band, 2×2 MU-MIMO — 5.7 / 4.3 Gbps / 688 Mbps                         |
| Managed UniFi devices | 30+                                                                              |
| Simultaneous users    | 300+                                                                             |
| Managed cameras       | 5 HD / 2 2K / 1 4K                                                               |
| PoE                   | 15.4 W on one port                                                               |
| Storage               | Pre-installed 64 GB microSD for NVR                                              |
| Display               | 4.7" touchscreen                                                                 |
| Power                 | 34.5 W max excl. PoE                                                             |
| Form factor           | Desktop — 110 × 110 × 250 mm, 1.4 kg                                             |

### Dream Router 7 (UDR7)

The mainstream all-in-one: WiFi 7 tri-band, a four-port 2.5 GbE switch with one PoE port, a 10G SFP+ uplink, 64 GB of microSD Protect storage and 2.3 Gbps of IDS/IPS, in a cylinder the size of a large coffee tin. For a house or a small office this is the single-box answer.

- **Price:** $279 USD / £220 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/5fd748ec-76b6-48ca-9256-9fb09d50b4b0/c57b6e85-cf5b-48c8-9e92-9f25e4dd0f39.png) · [angle](https://cdn.ecomm.ui.com/products/5fd748ec-76b6-48ca-9256-9fb09d50b4b0/ef4b2c32-4b83-467d-97b6-a265e1b1b6a6.png) · [rear](https://cdn.ecomm.ui.com/products/5fd748ec-76b6-48ca-9256-9fb09d50b4b0/164cc5ac-4310-4d43-83a9-ac8318e00701.png)

| Property              | Value                                                    |
|-----------------------|----------------------------------------------------------|
| IDS/IPS throughput    | 2.3 Gbps                                                 |
| Ports                 | (1) 10G SFP+, (4) 2.5 GbE RJ45 (1 PoE)                   |
| Max WAN ports         | 4                                                        |
| WiFi                  | WiFi 7 tri-band, 2×2 MU-MIMO — 5.7 / 4.3 Gbps / 688 Mbps |
| Managed UniFi devices | 30+                                                      |
| Simultaneous users    | 300+                                                     |
| Managed cameras       | 5 HD / 2 2K / 1 4K                                       |
| PoE budget            | 15.4 W                                                   |
| Storage               | Pre-installed 64 GB microSD for NVR                      |
| Coverage              | 160 m² (1,750 ft²)                                       |
| Power                 | 26 W max excl. PoE; internal 50 W AC/DC                  |
| Form factor           | Desktop — ⌀110 × 184.1 mm, 1.1 kg                        |

### UniFi Express 7 (UX7)

The smallest current Cloud Gateway: a 117 mm cube with a 10 GbE RJ45 port, a 2.5 GbE port, WiFi 7 tri-band radios and 2.3 Gbps of IDS/IPS, running from a USB-C adapter. It can act as the gateway for a small network or drop into an existing UniFi site as a mesh AP. Single WAN port and no NVR storage are the trade-offs for the size.

- **Price:** $199 USD / £159 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/d3220754-f4a9-4ecc-a0bd-64e1520ef471/5985679e-1cac-4885-ad85-82e2d5c94f89.png) · [angle](https://cdn.ecomm.ui.com/products/d3220754-f4a9-4ecc-a0bd-64e1520ef471/9f074c9b-13c5-424d-9b34-42b142832c32.png) · [rear](https://cdn.ecomm.ui.com/products/d3220754-f4a9-4ecc-a0bd-64e1520ef471/46ab6c67-5063-466a-99e2-c2e77f265aa6.png)

| Property              | Value                                            |
|-----------------------|--------------------------------------------------|
| IDS/IPS throughput    | 2.3 Gbps                                         |
| Ports                 | (1) 10 GbE RJ45, (1) 2.5 GbE RJ45                |
| Max WAN ports         | 1                                                |
| WiFi                  | WiFi 7 tri-band, 2×2 — 5.7 / 4.3 Gbps / 688 Mbps |
| Managed UniFi devices | 30+                                              |
| Simultaneous users    | 300+                                             |
| Signatures            | 20,000+ with CyberSecure                         |
| PoE / storage         | None                                             |
| Power                 | 22 W max; USB-C (5 V DC/5 A)                     |
| Form factor           | Desktop — 117 × 117 × 42.5 mm, 443 g             |

### Dream Router (UDR)

The original desktop all-in-one: gigabit WAN, five gigabit LAN ports (two PoE) on a 40 W budget, WiFi 6 with 4×4 radios, and a 128 GB SSD for Protect. Its ceilings are low — 1 Gbps IDS/IPS, 20+ devices, 150+ clients, and a WAN capped around 700 Mbps in practice — and Ubiquiti notes that application support is UniFi Network plus two of Protect/Access/Talk/Connect rather than all of them. Largely superseded by the UDR7.

- **Price:** $199 USD / £159 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/60459473-c989-41db-93f2-3c0f40df84f3/8153ca71-b9ac-47c5-b8e1-2cf6b868d372.png) · [angle](https://cdn.ecomm.ui.com/products/60459473-c989-41db-93f2-3c0f40df84f3/ff29d04e-23f5-4371-907d-e713fc3656e8.png) · [rear](https://cdn.ecomm.ui.com/products/60459473-c989-41db-93f2-3c0f40df84f3/b4fd2ae0-8d83-4ad0-ab4e-138d034a32f3.png)

| Property              | Value                                             |
|-----------------------|---------------------------------------------------|
| IDS/IPS throughput    | 1 Gbps                                            |
| Ports                 | (1) 1 GbE RJ45 WAN, (5) 1 GbE RJ45 LAN (2 PoE)    |
| Max WAN ports         | 1                                                 |
| WiFi                  | WiFi 6 — 5 GHz 2.4 Gbps 4×4, 2.4 GHz 600 Mbps 4×4 |
| Managed UniFi devices | 20+                                               |
| Simultaneous users    | 150+                                              |
| Managed cameras       | 5 HD / 2 2K / 1 4K                                |
| PoE budget            | 40 W (15.4 W per port)                            |
| Storage               | 128 GB SSD + microSD slot                         |
| Coverage              | 140 m² (1,500 ft²)                                |
| Power                 | 19.4 W max excl. PoE; internal 50 W AC/DC         |
| Form factor           | Desktop — ⌀110 × 184.1 mm, 1.2 kg                 |

### UniFi Express (UX)

The first-generation 98 mm cube: a WiFi 6 access point and minimal Cloud Gateway that manages up to four other UniFi devices. It is end-of-line — currently sold out, and Ubiquiti has confirmed it stays on the UniFi Network 8.x branch with security updates only, directing new deployments to the UniFi Express 7. Listed at $149 when in stock.

- **Price:** $149 USD (sold out) / GBP not currently listed
- **Images:** [front](https://cdn.ecomm.ui.com/products/4ed25b4c-db92-4b98-bbf3-b0989f007c0e/a32c8b40-e814-4981-99a8-46fc575abba0.png) · [angle](https://cdn.ecomm.ui.com/products/4ed25b4c-db92-4b98-bbf3-b0989f007c0e/67e2dfef-da9e-4d88-8324-b9830c111698.png)

| Property              | Value                                                         |
|-----------------------|---------------------------------------------------------------|
| Ports                 | (1) 1 GbE RJ45 WAN, (1) 1 GbE RJ45 LAN                        |
| Max WAN ports         | 1                                                             |
| WiFi                  | WiFi 6 — 5 GHz 2.4 Gbps 2×2, 2.4 GHz 573.5 Mbps 2×2           |
| Managed UniFi devices | 4                                                             |
| Simultaneous users    | 50+                                                           |
| Security              | Stateful + L7 firewall, DPI; IDS/IPS throughput not published |
| Coverage              | 140 m² (1,500 ft²)                                            |
| Software              | UniFi Network 8.x only                                        |
| Power                 | 10 W max; USB-C (5 V DC/3 A)                                  |
| Form factor           | Desktop — 98 × 98 × 30 mm, 302 g                              |

## Standalone Gateways (UXG)

The UXG family is the same routing silicon without the controller. These require an external UniFi Network instance — a Cloud Key, Ubiquiti's Official UniFi Hosting, or a self-hosted Network Server — which is exactly what you want if you already run a controller for multiple sites, and exactly what you don't want if you were hoping for a single box. One caveat worth knowing: some cloud features are unavailable when a UXG is paired with a self-hosted controller rather than a Cloud Key or Official Hosting.

### Gateway Enterprise (UXG-Enterprise)

The controller-less counterpart to the EFG: 25G SFP28 uplinks, 12.5 Gbps IDS/IPS, dual hot-swap PSUs, Shadow Mode VRRP failover, licence-free NeXT AI SSL/TLS inspection, and a 1.3" touchscreen. Requires UniFi Network 8.3.32 or later.

- **Price:** $1,999 USD / £1,659 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/3dbb7c83-aa76-4110-9ddf-7bfd7d8511cc/2a5803e1-5c1f-48fa-a7c3-13a22b4e59f8.png) · [angle](https://cdn.ecomm.ui.com/products/3dbb7c83-aa76-4110-9ddf-7bfd7d8511cc/456c05ab-62f9-45d3-863a-67723f681dc8.png) · [rear](https://cdn.ecomm.ui.com/products/3dbb7c83-aa76-4110-9ddf-7bfd7d8511cc/09d208b3-ed70-4ef0-8d60-7c2815adc33a.png)

| Property                    | Value                                         |
|-----------------------------|-----------------------------------------------|
| IDS/IPS throughput          | 12.5 Gbps                                     |
| Ports                       | (2) 25G SFP28, (2) 10G SFP+, (2) 2.5 GbE RJ45 |
| Max WAN ports               | 5                                             |
| Concurrent sessions         | 10 million                                    |
| New sessions/sec            | 71,000                                        |
| SSL/TLS inspection sessions | 10,000                                        |
| High availability           | Shadow Mode (VRRP); (2) hot-swap PSUs         |
| Controller                  | External required (Network 8.3.32+)           |
| Power                       | 82 W max                                      |
| Form factor                 | 1U rack — 442.4 × 43.7 × 325 mm               |

### Gateway Pro (UXG-Pro)

A 1U multi-WAN gateway with 10G SFP+ on both WAN and LAN sides, 3.5 Gbps IDS/IPS and support for UniFi Power Backup DC input. Effectively a UDM-Pro without the controller, the drive bay or the copper switch ports.

- **Price:** $499 USD / £395 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/40f3906a-6663-4c6f-9059-abb517fff6fe/08793338-739b-4d65-86d4-38d73e8842c0.png) · [angle](https://cdn.ecomm.ui.com/products/40f3906a-6663-4c6f-9059-abb517fff6fe/0f9da404-44d6-4380-860d-f9bd3babdf61.png) · [rear](https://cdn.ecomm.ui.com/products/40f3906a-6663-4c6f-9059-abb517fff6fe/025affcd-15fc-4465-8392-f47bf4eaf861.png)

| Property           | Value                             |
|--------------------|-----------------------------------|
| IDS/IPS throughput | 3.5 Gbps                          |
| WAN ports          | (1) 10G SFP+ + (1) 1 GbE RJ45     |
| LAN ports          | (1) 10G SFP+ + (1) 1 GbE RJ45     |
| Max WAN ports      | 3                                 |
| Power backup       | UniFi Power Backup DC input       |
| Controller         | External required                 |
| Form factor        | 1U rack — 442.4 × 43.7 × 285.6 mm |

### Gateway Fiber (UXG-Fiber)

The UCG-Fiber without the controller: 5 Gbps IDS/IPS, dual 10G uplink options (SFP+ and 10 GbE RJ45), a 10G SFP+ LAN port and a four-port 2.5 GbE switch with one PoE+ port.

- **Price:** $279 USD / £229 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/7310b331-fede-4e5f-9392-b6661ffce39f/5d85e895-d769-4302-aba6-a7fd9996174f.png) · [angle](https://cdn.ecomm.ui.com/products/7310b331-fede-4e5f-9392-b6661ffce39f/7aa64804-7871-4f98-911e-f9c826408322.png) · [rear](https://cdn.ecomm.ui.com/products/7310b331-fede-4e5f-9392-b6661ffce39f/70c73b7b-421c-42cc-87a9-e18e8f110a38.png)

| Property           | Value                                   |
|--------------------|-----------------------------------------|
| IDS/IPS throughput | 5 Gbps                                  |
| WAN ports          | (1) 10G SFP+ + (1) 10 GbE RJ45          |
| LAN ports          | (1) 10G SFP+, (4) 2.5 GbE RJ45 (1 PoE+) |
| PoE budget         | 30 W                                    |
| Controller         | External required                       |
| Form factor        | Desktop                                 |

### Gateway Max (UXG-Max)

Five 2.5 GbE ports, 2.3 Gbps IDS/IPS, USB-C powered, 9.6 W. The controller-less UCG-Max, and the cheapest way to get multi-gig inspected routing into an existing controller-managed estate. Requires UniFi Network 8.1.113 or later.

- **Price:** $199 USD / £159 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/8733d39f-7799-4268-bb24-3185c7cd0877/50250105-89b6-4133-829e-8d32894ce9c9.png) · [angle](https://cdn.ecomm.ui.com/products/8733d39f-7799-4268-bb24-3185c7cd0877/6f217fe7-b579-4686-887e-7b15bd16f17e.png) · [rear](https://cdn.ecomm.ui.com/products/8733d39f-7799-4268-bb24-3185c7cd0877/ec0f52d7-ac20-4353-8137-216319a3ca34.png)

| Property           | Value                                        |
|--------------------|----------------------------------------------|
| IDS/IPS throughput | 2.3 Gbps                                     |
| Ports              | (5) 2.5 GbE RJ45                             |
| Max WAN ports      | 4                                            |
| MAC address table  | 2,000 entries                                |
| Processor          | Quad-core ARM Cortex-A53 @ 1.5 GHz, 2 GB RAM |
| Controller         | External required (Network 8.1.113+)         |
| Power              | 9.6 W max; USB-C (5 V DC/3 A)                |
| Form factor        | Desktop — 141.8 × 127.6 × 30 mm, 520 g       |

### Gateway Lite (UXG-Lite)

The cheapest UniFi router at $89: one gigabit WAN, one gigabit LAN, USB-C power, and roughly ten times the routing performance of the old USG with IPS, QoS and Smart Queues enabled. A pure replacement for a legacy USG in a controller-managed site. Requires UniFi Network 8.0.7 or later.

- **Price:** $89 USD / £70 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/2eb8bbf3-3f14-4d50-a4b7-e575310eccf2/c37ee48f-c4c5-40f6-a416-6f2d074c52d4.png) · [angle](https://cdn.ecomm.ui.com/products/2eb8bbf3-3f14-4d50-a4b7-e575310eccf2/7b8f1117-bd6e-4b86-9bb9-036b45e6a134.png) · [rear](https://cdn.ecomm.ui.com/products/2eb8bbf3-3f14-4d50-a4b7-e575310eccf2/3860d871-6412-4c4b-89c4-1d01a5fe418b.png)

| Property           | Value                                  |
|--------------------|----------------------------------------|
| IDS/IPS throughput | 1 Gbps                                 |
| Ports              | (1) 1 GbE RJ45 WAN, (1) 1 GbE RJ45 LAN |
| Max WAN ports      | 1                                      |
| Controller         | External required (Network 8.0.7+)     |
| Power              | USB-C, adapter included                |
| Form factor        | Desktop — 98 × 98 × 30 mm              |

## Travel Routers (UTR)

A category of two. Both are pocket routers whose defining feature is not throughput but one-click WireGuard/OpenVPN back to a UniFi Cloud Gateway or UXG at home, with selected SSIDs replicated so devices reconnect automatically in a hotel or rental. Captive portals are handled transparently. Neither has cellular; both are WiFi 5 only, which is the honest limitation.

### UniFi Travel Router (UTR)

A 12.5 mm-thick, 89 g slab with two gigabit ports, dual-band 2×2 WiFi 5, and uplink over Ethernet, WiFi (WISP mode) or USB tethering from a phone. Runs from a 5 W USB-C port, so a laptop or power bank will do.

- **Price:** $79 USD / £65 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/56664a45-ee21-460f-a8fb-e607a959a379/481be0e8-0ffd-4a13-903e-6f414711f1c7.png) · [angle](https://cdn.ecomm.ui.com/products/56664a45-ee21-460f-a8fb-e607a959a379/24394849-e8ff-4ba6-9924-9fa354d44a15.png) · [detail](https://cdn.ecomm.ui.com/products/56664a45-ee21-460f-a8fb-e607a959a379/9284ab41-1113-4249-9b64-5c57d04f97d9.png)

| Property     | Value                                                  |
|--------------|--------------------------------------------------------|
| WiFi         | WiFi 5 (802.11ac) 2×2, up to 866.7 Mbps                |
| Ports        | (2) 1 GbE RJ45                                         |
| Uplink modes | Ethernet, WiFi (WISP), USB tethering (iOS/Android)     |
| VPN          | WireGuard and OpenVPN client; one-click UniFi site VPN |
| Antennas     | (2) embedded                                           |
| Power        | USB-C, ~5 W                                            |
| Form factor  | 95.95 × 65 × 12.5 mm, 89 g, polycarbonate              |

### UniFi Travel Router Long-Range (UTR-LR)

The UTR with a collapsible, 180°-tilting "super antenna" and close to double the broadcast power. Same silicon, same ports, same WiFi 5 ceiling — the difference is reception and range in WISP mode, where you are pulling a distant hotel or campsite signal. Two USB-C ports (one power, one tethering).

- **Price:** $99 USD / £79 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/0defc770-d70c-4938-9648-72722361dc79/c20df17e-1ee3-49f2-a28d-24b4f9f0668b.png) · [antenna deployed](https://cdn.ecomm.ui.com/products/0defc770-d70c-4938-9648-72722361dc79/065a0914-58d9-41c1-99fd-ff68e83c9503.png) · [rear](https://cdn.ecomm.ui.com/products/0defc770-d70c-4938-9648-72722361dc79/5966eca8-648c-4fc2-b7f5-0b0f4f227835.png)

| Property     | Value                                                                       |
|--------------|-----------------------------------------------------------------------------|
| WiFi         | WiFi 5 (802.11ac) 2×2, up to 866.7 Mbps                                     |
| Antenna      | Foldable directional "Super Antenna", 180° tilt; ~2× broadcast power vs UTR |
| Ports        | (2) 1 GbE RJ45; (2) USB-C (power + tethering)                               |
| Uplink modes | Ethernet, WiFi (WISP), USB tethering                                        |
| VPN          | WireGuard and OpenVPN client; one-click UniFi site VPN                      |
| Form factor  | Ultra-slim, foldable                                                        |

## Summary

Prices are Ubiquiti list, 30 August 2026. "IDS/IPS" is the inspected-throughput figure and is the practical WAN ceiling.

| Model                          | SKU            | Family          | USD        | GBP       | IDS/IPS   | Devices   | Clients   | Max WAN | Fastest ports     | PoE    | WiFi       | NVR storage                 | Form factor  |
|--------------------------------|----------------|-----------------|------------|-----------|-----------|-----------|-----------|---------|-------------------|--------|------------|-----------------------------|--------------|
| Enterprise Firewall Core       | EF-Core        | Enterprise      | $3,499     | £2,785    | 79 Gbps   | 2,250+    | 22,500+   | 8       | (4) 100G QSFP28   | —      | —          | —                           | 1U rack      |
| Enterprise Firewall (Fortress) | EFG            | Enterprise      | $1,999     | £1,659    | 12.5 Gbps | 500+      | 5,000+    | 5       | (2) 25G SFP28     | —      | —          | —                           | 1U rack      |
| Dream Machine Beast            | UDM-Beast      | UDM             | $1,499     | £1,195    | 25 Gbps   | 750+      | 7,500+    | 8       | (2) 25G SFP28     | —      | —          | 128 GB SSD + (2) 3.5"       | 1U rack      |
| Dream Machine Pro Max          | UDM-Pro-Max    | UDM             | $599       | £475      | 5 Gbps    | 200+      | 2,000+    | 8       | (2) 10G SFP+      | —      | —          | 128 GB SSD + (2) 3.5"       | 1U rack      |
| Dream Machine SE               | UDM-SE         | UDM             | $499       | £395      | 3.5 Gbps  | 100+      | 1,000+    | 8       | (2) 10G SFP+      | 180 W  | —          | 128 GB SSD + (1) 3.5"       | 1U rack      |
| Dream Machine Pro              | UDM-Pro        | UDM             | $379       | £300      | 3.5 Gbps  | 100+      | 1,000+    | 8       | (2) 10G SFP+      | —      | —          | (1) 3.5"                    | 1U rack      |
| Cloud Gateway Industrial       | UCG-Industrial | UCG             | $579       | £460      | 5 Gbps    | 50+       | 500+      | 5       | 10G SFP+ / 10 GbE | 270 W  | —          | 128 GB microSD              | Rugged shelf |
| Cloud Gateway Fiber            | UCG-Fiber      | UCG             | $279       | £220      | 5 Gbps    | 50+       | 500+      | 6       | (2) 10G SFP+      | 30 W   | —          | NVMe ≤ 2 TB                 | Desktop      |
| Cloud Gateway Max              | UCG-Max        | UCG             | from $199  | from £159 | 2.3 Gbps  | 30+       | 300+      | 4       | (5) 2.5 GbE       | —      | —          | NVMe ≤ 2 TB                 | Desktop      |
| Cloud Gateway Ultra            | UCG-Ultra      | UCG             | $129       | £79       | 1 Gbps    | 30+       | 300+      | 4       | (1) 2.5 GbE       | —      | —          | —                           | Desktop      |
| Dream Wall                     | UDW            | WiFi-integrated | $999       | £765      | 3.5 Gbps  | 100+      | 300+      | 8       | (2) 10G SFP+      | 420 W  | WiFi 6 4×4 | 512 GB microSD + 128 GB SSD | Wall mount   |
| Dream Router 5G Max            | UDR-5G-Max     | WiFi-integrated | $499       | £395      | 2.3 Gbps  | 30+       | 300+      | 4       | (1) 10G SFP+      | 15.4 W | WiFi 7 2×2 | 64 GB microSD               | Desktop      |
| Dream Router 7                 | UDR7           | WiFi-integrated | $279       | £220      | 2.3 Gbps  | 30+       | 300+      | 4       | (1) 10G SFP+      | 15.4 W | WiFi 7 2×2 | 64 GB microSD               | Desktop      |
| UniFi Express 7                | UX7            | WiFi-integrated | $199       | £159      | 2.3 Gbps  | 30+       | 300+      | 1       | (1) 10 GbE RJ45   | —      | WiFi 7 2×2 | —                           | Desktop      |
| Dream Router                   | UDR            | WiFi-integrated | $199       | £159      | 1 Gbps    | 20+       | 150+      | 1       | (1) 1 GbE         | 40 W   | WiFi 6 4×4 | 128 GB SSD                  | Desktop      |
| UniFi Express                  | UX             | WiFi-integrated | $149 (EOL) | —         | n/p       | 4         | 50+       | 1       | (1) 1 GbE         | —      | WiFi 6 2×2 | —                           | Desktop      |
| Gateway Enterprise             | UXG-Enterprise | UXG             | $1,999     | £1,659    | 12.5 Gbps | ext. ctrl | ext. ctrl | 5       | (2) 25G SFP28     | —      | —          | —                           | 1U rack      |
| Gateway Pro                    | UXG-Pro        | UXG             | $499       | £395      | 3.5 Gbps  | ext. ctrl | ext. ctrl | 3       | (2) 10G SFP+      | —      | —          | —                           | 1U rack      |
| Gateway Fiber                  | UXG-Fiber      | UXG             | $279       | £229      | 5 Gbps    | ext. ctrl | ext. ctrl | —       | (2) 10G SFP+      | 30 W   | —          | —                           | Desktop      |
| Gateway Max                    | UXG-Max        | UXG             | $199       | £159      | 2.3 Gbps  | ext. ctrl | ext. ctrl | 4       | (5) 2.5 GbE       | —      | —          | —                           | Desktop      |
| Gateway Lite                   | UXG-Lite       | UXG             | $89        | £70       | 1 Gbps    | ext. ctrl | ext. ctrl | 1       | (1) 1 GbE         | —      | —          | —                           | Desktop      |
| Travel Router Long-Range       | UTR-LR         | Travel          | $99        | £79       | n/p       | —         | —         | 1       | (2) 1 GbE         | —      | WiFi 5 2×2 | —                           | Pocket       |
| Travel Router                  | UTR            | Travel          | $79        | £65       | n/p       | —         | —         | 1       | (2) 1 GbE         | —      | WiFi 5 2×2 | —                           | Pocket       |

*n/p = not published; ext. ctrl = capacity determined by the external controller, not the gateway.*

### What the table says

Three things stand out once the lineup is laid side by side.

**The price/performance curve is not monotonic.** The Cloud Gateway Fiber at $279 delivers the same 5 Gbps of inspected throughput as the $599 Dream Machine Pro Max and beats the $379 UDM-Pro. Likewise, the $1,499 Dream Machine Beast doubles the inspected throughput of the $1,999 Enterprise Firewall. You pay the premium at the top of each family for redundancy — dual hot-swap PSUs, RAID drive bays, Shadow Mode failover — and for rack-mount port density, not for speed.

**IDS/IPS throughput clusters into four tiers.** 1 Gbps (Ultra, UDR, UXG-Lite), 2.3 Gbps (Max, UX7, UDR7, UDR-5G-Max, UXG-Max), 3.5–5 Gbps (UDM-Pro, UDM-SE, UDW, UDM-Pro-Max, UCG-Fiber, UCG-Industrial, UXG-Pro, UXG-Fiber), and 12.5 Gbps and up (EFG, UXG-Enterprise, UDM-Beast, EF-Core). Pick your tier from your WAN speed first; everything else is form factor and features.

**Storage and PoE, not routing, drive most of the price spread at the low end.** The Ultra, the Max and the Fiber share a family resemblance and a $150 spread, and almost all of that money buys NVMe camera storage, a PoE port and 10G optics rather than faster packet processing.

**Sources:** [UniFi Cloud Gateways](https://ui.com/us/en/cloud-gateways) · [Ubiquiti Tech Specs](https://techspecs.ui.com/) · [Ubiquiti Store US](https://store.ui.com/us/en) · [Ubiquiti Store UK](https://uk.store.ui.com/uk/en) · [iFeeltech UniFi Gateway Comparison](https://ifeeltech.com/blog/unifi-gateway-comparison-guide) · [Dong Knows Tech: UCG-Fiber review](https://dongknows.com/ubiquiti-ucg-fiber-unifi-cloud-gateway-fiber-review/) · [Dong Knows Tech: UTR review](https://dongknows.com/ubiquiti-utr-unifi-travel-router-review/) · [McCann Tech comparison charts](https://evanmccann.net/blog/ubiquiti/unifi-comparison-charts)
