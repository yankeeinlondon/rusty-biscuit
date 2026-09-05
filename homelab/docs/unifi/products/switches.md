---
$schema:
    prompt: string -> The Agent prompt that will populate the body of this research doc.
prompt: |-
    Research all the switches offerred under the **Unifi** brand.
    
    - identify the key properties/metrics which these switches have that are meaningful in terms of their capability and scalability
    - organize the switches into categories (UDM, ...) as H2 headings 
    - add an H3 heading for each router product:
        - describe the product
        - provide a price point in both USD and GBP
        - provide a link to one or more images of the product
        - provide the properties/metrics which characterize this product
    - add an H2 "Summary" section which includes a comparison table of the switches offered
    
    Once the document's body has been written in prose style, you must add the following frontmatter properties to the document as well:
        
    - `last_updated` as \"{{ctx.today}}\"
    - `switches` as a list of each switch (switch product name is key, key attributes are listed underneath as key/value pairs)
    - `agent` as '{{ctx.agent}}/{{ctx.model}}'

    Make sure this document is saved with these Frontmatter properties included.
last_updated: 2026-08-30
hash: b6ba15ce1464d66a-19488ccaf204366e
---
# UniFi Switches

Ubiquiti sells more switches than any other product category under the UniFi brand — roughly fifty SKUs spanning a £23 five-port desk switch and a £3,180 1.8 Tbps campus aggregation box. All of them are managed by the same UniFi Network application, all of them are NDAA-compliant, and none of them carry a per-port, per-device or per-seat license. What separates them is silicon capacity, port media, PoE budget, and whether the box can route.

This document covers every switch currently listed on [store.ui.com](https://store.ui.com), grouped by Ubiquiti's own product families, with the specifications that actually determine what a given switch can carry.

## How to Read the Numbers

Ubiquiti publishes around forty specifications per switch. About a dozen change buying decisions.

**Switching capacity and total non-blocking throughput are the same number twice.** Switching capacity counts every port in both directions; non-blocking throughput counts it once. A switch whose non-blocking throughput equals the sum of its port speeds is truly non-blocking — the Enterprise Campus 48 PoE publishes 460 Gbps against (16 × 2.5) + (32 × 10) + (4 × 25) = 460 Gbps of port bandwidth, so no port can ever be starved by another. Do this arithmetic before believing a headline figure; it is the fastest way to spot an oversubscribed design.

**Forwarding rate in Mpps is the small-packet limit.** It tracks non-blocking throughput at roughly 1.5 Mpps per Gbps, which is line rate for 64-byte frames. It only becomes the binding constraint under scanning, VoIP or telemetry loads made of tiny packets.

**Port media and speed mix is the real product differentiator.** The catalog spans 1 GbE RJ45, 2.5 GbE RJ45, 10 GbE RJ45, 1G SFP, 10G SFP+, 25G SFP28 and 100G QSFP28. Two switches at the same price can differ by an order of magnitude in usable bandwidth purely by port mix — the Pro Max 24 and the Pro HD 24 cost within $150 of each other and differ by 2× in non-blocking throughput because one is mostly gigabit and the other is mostly 2.5 GbE.

**PoE tiers, in Ubiquiti's own notation.** Their spec sheets publish four per-port ceilings measured at the switch: PoE (802.3af) 15.4 W, PoE+ (802.3at) 30 W, PoE++ (802.3bt Type 3) 60 W, PoE+++ (802.3bt Type 4) 90 W. Note that Ubiquiti's older help-centre articles still use "PoE++" for all of 802.3bt; the current product pages do not.

**Total PoE availability is a budget, not a per-port promise.** It ranges from 42 W (Switch Ultra, Standard 16 PoE) to 2,150 W (Enterprise Campus 48 PoE). On the enterprise models it is quoted four ways, because it depends on both mains voltage and PSU mode: *shared* mode pools both power supplies for maximum budget, *redundant* mode reserves one so a PSU failure drops nothing. An ECS-48-PoE delivers 2,150 W shared at 230 V but only 950 W redundant — plan for the redundant number if the site needs it.

**Layer 2 versus Layer 3** decides whether the switch can do inter-VLAN routing, run a DHCP server for local networks, and hold static routes without hairpinning everything through the gateway. Standard, Lite, Flex, Ultra, Aggregation and Flex XG are Layer 2. Everything in the Pro, Pro Max, Pro HD, Pro XG and Enterprise families is Layer 3.

**MAC address table size** sets how many endpoints a switch can learn before it starts flooding: 2,000 on the Flex, 4,000 on Ultra and Flex 2.5G, 8,000–16,000 on the mainstream models, 32,000 on Pro XG and Enterprise, and 128,000 on the Enterprise Campus Aggregation. On a flat network with cameras, sensors and guest devices this ceiling arrives sooner than people expect.

**Supported VLANs** is a hard split, not a gradient: 256 on the Ultra and Flex 2.5G models, 1,000 on everything else.

**Packet buffer size** is the microburst tolerance, and it scales from 0.5 MB on the Standard series through 1.5–2 MB on Pro and Pro Max, 4 MB on Pro 48 and Pro Aggregation, 8 MB on Pro XG and Enterprise, to 24 MB on the Enterprise Campus Aggregation. This is the specification that quietly separates a switch that handles video and storage bursts from one that drops them.

**IPv4 static routes and ACL entries** (128–512 depending on class) are the Layer 3 scaling limits. Ubiquiti's published figure of 2 static routes for the Pro 24 and Pro 24 PoE is inconsistent with the 256 quoted for every sibling model and appears to be a data error on their site.

**Redundancy comes in five distinct forms** and they are not interchangeable: DC power backup from a USP-RPS (most Pro/Pro Max/Pro XG models), dual hot-swappable PSUs plus hot-swappable fan modules (all Enterprise models), switch stacking (the ECS "S" variants only), MC-LAG (the Enterprise Campus Aggregation only), and an integrated lithium-ion battery (the UPS PoE Switch only).

**Etherlighting™** is Ubiquiti's per-port RGB light strip that signals link speed, activity and error state at a glance. It is present on Enterprise, Pro XG, Pro Max, Pro HD and the two high-end aggregation switches; absent from Standard, Pro, Lite, Flex and Ultra.

**Form factor and cooling** matter more than the datasheet suggests. The Standard and Pro series are fanless and silent; the Pro XG, Pro HD PoE and every Enterprise model have fans and belong in a closet or rack room. The Pro Max 16, Pro XG 8 PoE, Flex, Ultra and Lite models are desktop or wall units, several of which are themselves PoE-powered and need no outlet.

**Minimum UniFi Network version** is an easily missed deployment constraint. The newest models — Pro XG 48/24 PoE, Pro XG 10 PoE, and both WAN switches — require Network 9.1.120 or later, while the Standard series runs on 7.2.94 and the Flex Mini on 5.12.5.

> **Pricing basis.** All prices below are Ubiquiti's own list prices on [store.ui.com](https://store.ui.com) (USD) and [uk.store.ui.com](https://uk.store.ui.com) (GBP) as of **30 August 2026**. The US store separately displays a higher "surcharge included" figure driven by memory costs; the base list price is quoted here. Regional taxes are added at checkout. Availability is noted where a model is currently sold out or not yet shipping.

## Enterprise Campus and Audio/Video (ECS / EAV)

Ubiquiti's top tier: 1U full-depth chassis with dual hot-swappable power supplies, four or five hot-swappable fan modules, a 1.3" front touchscreen, and PoE+++ on every copper port. These are the only UniFi switches with true PSU redundancy, and the only ones offering stacking or MC-LAG.

### Enterprise Campus 48 PoE (ECS-48-PoE)

The flagship access switch: 48 copper ports every one of which can deliver 90 W, backed by four 25G SFP28 uplinks and a genuinely non-blocking 460 Gbps fabric. With two 1,200 W supplies in shared mode it hands out 2,150 W of PoE, enough to power a floor of PoE+++ access points, cameras and displays from a single rack unit.

- **Price:** $3,499 USD / £2,785 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/c0e18508-feb3-4d93-93d7-2a022815cfac/feb987d6-1eb6-497a-83fd-af24f4116192.png) · [angle](https://cdn.ecomm.ui.com/products/c0e18508-feb3-4d93-93d7-2a022815cfac/a45a9e95-8dae-4d80-858e-60b4e5e75075.png) · [rear](https://cdn.ecomm.ui.com/products/c0e18508-feb3-4d93-93d7-2a022815cfac/060d8d14-c36d-4758-9d5f-78ef56c1217c.png)

| Property                   | Value                                                              |
|----------------------------|--------------------------------------------------------------------|
| Ports                      | (16) 2.5 GbE RJ45 PoE+++, (32) 10 GbE RJ45 PoE+++, (4) 25G SFP28   |
| Switching capacity         | 920 Gbps (460 Gbps non-blocking, 684 Mpps)                         |
| PoE                        | Up to PoE+++ (90 W/port); 2,150 W shared / 950 W redundant @ 230 V |
| Layer                      | Layer 3                                                            |
| MAC table / VLANs / buffer | 32,000 / 1,000 / 8 MB                                              |
| Static routes / ACLs       | 512 / 256 IPv4 + 256 MAC                                           |
| Redundancy                 | (2) hot-swap 1,200 W PSUs, (4) hot-swap fans                       |
| Power                      | 250 W excl. PoE; 2,400 W incl. PoE                                 |
| Form factor                | 1U full-depth rack — 442.4 × 43.7 × 496 mm, 9.46 kg                |

### Enterprise Campus 48S PoE (ECS-48S-PoE)

The stackable 48-port model, adding two 100G QSFP28 stack ports, a console port and a dedicated management port. It is *not* simply a 48 PoE with stacking bolted on — the port mix flips to 32 × 2.5 GbE and 16 × 10 GbE, which is why its published non-blocking throughput is 340 Gbps rather than 460. Note that Ubiquiti excludes the QSFP28 stack links from the published switching-capacity figures.

- **Price:** $3,999 USD / £3,180 GBP — *listed as coming soon*
- **Images:** [front](https://cdn.ecomm.ui.com/products/734c4b7d-fd16-43b8-90d6-f2427a0d499d/e69412b6-93c8-4e35-bead-222f825568f1.png) · [angle](https://cdn.ecomm.ui.com/products/734c4b7d-fd16-43b8-90d6-f2427a0d499d/8cfd39a8-eded-4c76-bbd2-c41c7d1c9c24.png) · [rear](https://cdn.ecomm.ui.com/products/734c4b7d-fd16-43b8-90d6-f2427a0d499d/c10b3fe9-5ea0-49c5-8430-cbd4fe01c01d.png)

| Property                   | Value                                                                                                            |
|----------------------------|------------------------------------------------------------------------------------------------------------------|
| Ports                      | (32) 2.5 GbE RJ45 PoE+++, (16) 10 GbE RJ45 PoE+++, (4) 25G SFP28, (2) 100G QSFP28 stack, (1) 1 GbE mgmt, console |
| Switching capacity         | 680 Gbps (340 Gbps non-blocking, 506 Mpps) — excludes stack ports                                                |
| PoE                        | Up to PoE+++; 2,150 W shared / 950 W redundant @ 230 V                                                           |
| Layer                      | Layer 3                                                                                                          |
| MAC table / VLANs / buffer | 32,000 / 1,000 / 8 MB                                                                                            |
| Redundancy                 | Switch stacking, (2) hot-swap 1,200 W PSUs, (4) hot-swap fans                                                    |
| Power                      | 250 W excl. PoE; 2,400 W incl. PoE                                                                               |
| Form factor                | 1U full-depth rack — 442.4 × 44 × 496 mm, 9.5 kg                                                                 |

### Enterprise Campus 24 PoE (ECS-24-PoE)

Half the ports, the same silicon class, and the same PoE+++ on every copper port. The 460 Gbps switching capacity exactly matches its port bandwidth, so it is fully non-blocking. Shipped with 600 W supplies for a 1,050 W shared budget; fitting 1,200 W modules raises that to 2,250 W.

- **Price:** $2,499 USD / £1,989 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/f83443e3-38b7-415b-85cd-e3da1d0c044c/6cae8c8d-f8ab-4207-b0f9-111898f30f66.png) · [angle](https://cdn.ecomm.ui.com/products/f83443e3-38b7-415b-85cd-e3da1d0c044c/84707017-58bf-4272-894e-0fee38c7cb09.png) · [rear](https://cdn.ecomm.ui.com/products/f83443e3-38b7-415b-85cd-e3da1d0c044c/81168368-2c15-4fe3-875c-a06f6953d136.png)

| Property                   | Value                                                           |
|----------------------------|-----------------------------------------------------------------|
| Ports                      | (8) 2.5 GbE RJ45 PoE+++, (16) 10 GbE RJ45 PoE+++, (2) 25G SFP28 |
| Switching capacity         | 460 Gbps (230 Gbps non-blocking, 342 Mpps)                      |
| PoE                        | Up to PoE+++; 1,050 W shared / 450 W redundant (600 W modules)  |
| Layer                      | Layer 3                                                         |
| MAC table / VLANs / buffer | 32,000 / 1,000 / 8 MB                                           |
| Redundancy                 | (2) hot-swap 600 W PSUs, (4) hot-swap fans                      |
| Power                      | 150 W excl. PoE; 2,400 W incl. PoE                              |
| Form factor                | 1U full-depth rack — 442.4 × 43.7 × 496 mm, 9.46 kg             |

### Enterprise Campus 24S PoE (ECS-24S-PoE)

The stackable 24-port model. Access ports are identical to the ECS-24-PoE — hence identical 460/230 Gbps figures — with four 25G SFP28, two 100G QSFP28 stack ports, a management port and a console added.

- **Price:** $2,999 USD / £2,385 GBP — *listed as coming soon*
- **Images:** [front](https://cdn.ecomm.ui.com/products/a8ebdcc3-9603-4387-b3ba-50f242bf55ae/5f3fdfd2-325c-447a-81fe-b8e24aa94967.png) · [angle](https://cdn.ecomm.ui.com/products/a8ebdcc3-9603-4387-b3ba-50f242bf55ae/5e134109-0e60-4edc-904b-0cbf058ba48d.png) · [rear](https://cdn.ecomm.ui.com/products/a8ebdcc3-9603-4387-b3ba-50f242bf55ae/550480b9-2083-42e0-8308-6a3e093e590d.png)

| Property                   | Value                                                                                                           |
|----------------------------|-----------------------------------------------------------------------------------------------------------------|
| Ports                      | (8) 2.5 GbE RJ45 PoE+++, (16) 10 GbE RJ45 PoE+++, (4) 25G SFP28, (2) 100G QSFP28 stack, (1) 1 GbE mgmt, console |
| Switching capacity         | 460 Gbps (230 Gbps non-blocking, 342 Mpps) — excludes stack ports                                               |
| PoE                        | Up to PoE+++; 1,050 W shared / 450 W redundant (600 W modules)                                                  |
| Layer                      | Layer 3                                                                                                         |
| MAC table / VLANs / buffer | 32,000 / 1,000 / 8 MB                                                                                           |
| Redundancy                 | Switch stacking, (2) hot-swap 600 W PSUs, (4) hot-swap fans                                                     |
| Form factor                | 1U full-depth rack — 442.4 × 44 × 496 mm, 9.2 kg                                                                |

### Enterprise Audio/Video XG 24 PoE (EAV-XG-24-PoE)

A broadcast-media switch: 24 ports of 10 GbE with PoE+++ on every one, four 100G QSFP28 uplinks, and — the part that justifies the price — an on-board OCXO clock with an SMA input for an external GPS grandmaster. It supports PTP timing, SMPTE ST 2110, SDVoE and AES67, which makes it a switch you can put in a broadcast or large-venue AV plant rather than merely near one. At 1,280 Gbps it is the highest-capacity access switch UniFi sells.

- **Price:** $3,999 USD / £3,180 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/d7832d85-03a3-4d3f-a935-e9ed5d4f7b8f/8b8579c0-f46a-46ab-8bac-a40796190a2c.png) · [angle](https://cdn.ecomm.ui.com/products/d7832d85-03a3-4d3f-a935-e9ed5d4f7b8f/c6be6267-a892-428a-a967-17b7997b6238.png) · [rear](https://cdn.ecomm.ui.com/products/d7832d85-03a3-4d3f-a935-e9ed5d4f7b8f/c9f6587b-25b0-4ade-ac3d-a72fba31effe.png)

| Property                   | Value                                                                     |
|----------------------------|---------------------------------------------------------------------------|
| Ports                      | (24) 10 GbE RJ45 PoE+++, (4) 100G QSFP28, (1) 1 GbE RJ45, console, SMA    |
| Switching capacity         | 1,280 Gbps (640 Gbps non-blocking, 810 Mpps)                              |
| PoE                        | Up to PoE+++; 2,150 W shared / 1,050 W redundant @ 230 V                  |
| Media features             | PTP, SMPTE ST 2110, SDVoE, AES67; OCXO clock, GPS grandmaster input (SMA) |
| Layer                      | Layer 3                                                                   |
| MAC table / VLANs / buffer | 32,000 / 1,000 / 8 MB                                                     |
| Redundancy                 | (2) hot-swap 1,200 W PSUs, (4) hot-swap fans                              |
| Power                      | 150 W excl. PoE; 2,400 W incl. PoE                                        |
| Form factor                | 1U full-depth rack — 442.4 × 480 × 43.7 mm, 9.85 kg                       |

### Enterprise Audio/Video Fiber (EAV-Fiber)

The all-optical sibling: 20 × 10G SFP+ and 2 × 100G QSFP28, no PoE, same OCXO clock and GPS input, same media protocol support. Where the XG variant powers cameras and endpoints over copper, this one aggregates fibre-attached media devices and draws just 150 W.

- **Price:** $2,999 USD / £2,385 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/b6dd112b-bedd-4089-8831-c43a1241448a/37b978dc-4a1c-43e1-8a8b-9cc193f66c58.png) · [angle](https://cdn.ecomm.ui.com/products/b6dd112b-bedd-4089-8831-c43a1241448a/4e0eea7c-7e9f-477d-a0bb-9f577ebf1b67.png) · [rear](https://cdn.ecomm.ui.com/products/b6dd112b-bedd-4089-8831-c43a1241448a/50d388b9-7251-43bc-8a06-c0d0ef394600.png)

| Property                   | Value                                                                     |
|----------------------------|---------------------------------------------------------------------------|
| Ports                      | (20) 10G SFP+, (2) 100G QSFP28, (1) 1 GbE RJ45, console, SMA              |
| Switching capacity         | 800 Gbps (400 Gbps non-blocking, 595 Mpps)                                |
| PoE                        | None                                                                      |
| Media features             | PTP, SMPTE ST 2110, SDVoE, AES67; OCXO clock, GPS grandmaster input (SMA) |
| Layer                      | Layer 3                                                                   |
| MAC table / VLANs / buffer | 32,000 / 1,000 / 8 MB                                                     |
| Redundancy                 | (2) hot-swap 150 W PSUs, (4) hot-swap fans                                |
| Power                      | 150 W max                                                                 |
| Form factor                | 1U full-depth rack — 442.4 × 480 × 43.7 mm, 8.95 kg                       |

## Aggregation Switches

Port-dense, PoE-free boxes whose only job is to concentrate uplinks. The spread here is enormous — a factor of 22 in price and 22 in throughput between the cheapest and the dearest.

### Enterprise Campus Aggregation (ECS-Aggregation)

The largest switch UniFi makes: 48 × 25G SFP28 plus 6 × 100G QSFP28 for 1.8 Tbps of non-blocking throughput and 2.4 Bpps of forwarding. It is also the only UniFi switch with MC-LAG, letting two chassis present a single logical LAG to downstream switches so either can fail without dropping a link. The 128,000-entry MAC table and 24 MB buffer are both four to eight times anything else in the range.

- **Price:** $3,999 USD / £3,180 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/82c1c564-0e08-4b27-a7b9-95729cca00bd/273218d0-162e-4389-8143-686302547137.png) · [angle](https://cdn.ecomm.ui.com/products/82c1c564-0e08-4b27-a7b9-95729cca00bd/3ea48feb-2a2d-427b-ae65-5ffd5f1b6063.png) · [rear](https://cdn.ecomm.ui.com/products/82c1c564-0e08-4b27-a7b9-95729cca00bd/35d72080-fd3d-4c90-ade4-4a62815b2a37.png)

| Property                   | Value                                              |
|----------------------------|----------------------------------------------------|
| Ports                      | (48) 25G SFP28, (6) 100G QSFP28                    |
| Switching capacity         | 3.6 Tbps (1.8 Tbps non-blocking, 2.4 Bpps)         |
| PoE                        | None                                               |
| Layer                      | Layer 3                                            |
| MAC table / VLANs / buffer | 128,000 / 1,000 / 24 MB                            |
| Redundancy                 | MC-LAG, (2) hot-swap 550 W PSUs, (5) hot-swap fans |
| Power                      | 340 W max                                          |
| Form factor                | 1U full-depth rack — 442.4 × 43.7 × 496 mm, 9.9 kg |

### Pro XG Aggregation (USW-Pro-XG-Aggregation)

Thirty-two 25G SFP28 ports, 800 Gbps non-blocking, in a single-PSU chassis with USP-RPS DC backup rather than hot-swap supplies. It is the natural core for a site built on Pro XG access switches, and at $2,499 it delivers 44% of the Campus Aggregation's throughput for 62% of the price.

- **Price:** $2,499 USD / £1,989 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/2f42897e-4b8e-4851-af6a-5106211f979e/e541f8bf-a009-427f-815f-7966afbfd1ac.png) · [angle](https://cdn.ecomm.ui.com/products/2f42897e-4b8e-4851-af6a-5106211f979e/6f7967d0-22c5-4b11-b70e-cab744959da1.png) · [rear](https://cdn.ecomm.ui.com/products/2f42897e-4b8e-4851-af6a-5106211f979e/4c03c5d5-5e9b-4416-8fe2-d69f2d3fa4fb.png)

| Property                   | Value                                      |
|----------------------------|--------------------------------------------|
| Ports                      | (32) 25G SFP28                             |
| Switching capacity         | 1.6 Tbps (800 Gbps non-blocking, 1.2 Bpps) |
| PoE                        | None                                       |
| Layer                      | Layer 3                                    |
| MAC table / VLANs / buffer | 32,000 / 1,000 / 8 MB                      |
| Redundancy                 | DC power backup (USP-RPS)                  |
| Power                      | 200 W max                                  |
| Form factor                | 1U rack — 442.4 × 43.7 × 480 mm, 7.1 kg    |

### Hi-Capacity Aggregation (USW-Pro-Aggregation)

Twenty-eight 10G SFP+ plus four 25G SFP28 — the mixed-speed aggregation switch for a network transitioning from 10G to 25G uplinks. 380 Gbps non-blocking, 32,000 MAC entries, 4 MB buffer, Layer 3, and USP-RPS backup.

- **Price:** $899 USD / £715 GBP — *currently sold out in the US store*
- **Images:** [front](https://cdn.ecomm.ui.com/products/35879d83-6169-4d6b-abf6-d3b98b1e8367/f8726fbe-a4fa-4c3b-b9d7-ee426646c1be.png) · [angle](https://cdn.ecomm.ui.com/products/35879d83-6169-4d6b-abf6-d3b98b1e8367/f6f0c75d-378f-4310-81c1-c38d21150ceb.png) · [rear](https://cdn.ecomm.ui.com/products/35879d83-6169-4d6b-abf6-d3b98b1e8367/df5aa84c-508c-46d5-8028-b0440bd55458.png)

| Property                   | Value                                      |
|----------------------------|--------------------------------------------|
| Ports                      | (28) 10G SFP+, (4) 25G SFP28               |
| Switching capacity         | 760 Gbps (380 Gbps non-blocking, 565 Mpps) |
| PoE                        | None                                       |
| Layer                      | Layer 3                                    |
| MAC table / VLANs / buffer | 32,000 / 1,000 / 4 MB                      |
| Redundancy                 | DC power backup (USP-RPS)                  |
| Power                      | 100 W max                                  |
| Form factor                | 1U rack — 442 × 325 × 44 mm, 4.6 kg        |

### Aggregation (USW-Aggregation)

The half-depth eight-port 10G SFP+ box that most homelabs and small offices actually buy. Layer 2 only, fanless, 36 W, and by a wide margin the cheapest way to get eight 10G fibre ports under UniFi management.

- **Price:** $269 USD / £215 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/1c748fb1-b4df-43ef-83e0-d5ed26f9db7c/b06fde87-241d-4b4e-8319-8d7d29f6f6c2.png) · [angle](https://cdn.ecomm.ui.com/products/1c748fb1-b4df-43ef-83e0-d5ed26f9db7c/78dc5054-a468-401c-ac2a-2a00930c96ff.png) · [rear](https://cdn.ecomm.ui.com/products/1c748fb1-b4df-43ef-83e0-d5ed26f9db7c/fcc6deec-8a8c-461c-80c1-6293579aa36b.png)

| Property                   | Value                                              |
|----------------------------|----------------------------------------------------|
| Ports                      | (8) 10G SFP+                                       |
| Switching capacity         | 160 Gbps (80 Gbps non-blocking, 119 Mpps)          |
| PoE                        | None                                               |
| Layer                      | Layer 2                                            |
| MAC table / VLANs / buffer | 16,000 / 1,000 / 1.5 MB                            |
| Power                      | 36 W max, fanless                                  |
| Form factor                | 1U rack (half-depth) — 442 × 120 × 43.7 mm, 2.6 kg |

## Professional XG (USW-Pro-XG)

The 10 GbE copper access family. Every model gives multi-gig on every port and 25G SFP28 uplinks on the 24- and 48-port versions, and the PoE variants deliver PoE+++ across the board. This is where UniFi stops being a prosumer line.

### Pro XG 48 PoE (USW-Pro-XG-48-PoE)

Thirty-two 10 GbE and sixteen 2.5 GbE ports, every one PoE+++, over four 25G SFP28 uplinks and a 460 Gbps non-blocking fabric. At 1,080 W of PoE it is the largest power budget outside the Enterprise Campus line, and it is the only switch in the range whose maximum draw exceeds 1.2 kW.

- **Price:** $2,499 USD / £1,989 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/b4f97072-47c3-4ea1-96aa-616b94c1ca05/ffad1ad8-f360-420b-9a67-39f12065c33e.png) · [angle](https://cdn.ecomm.ui.com/products/b4f97072-47c3-4ea1-96aa-616b94c1ca05/b043e44b-8885-4023-a4a9-68c6d70a40e1.png) · [rear](https://cdn.ecomm.ui.com/products/b4f97072-47c3-4ea1-96aa-616b94c1ca05/ed97670a-e0e0-4351-82cd-66da9be6a7f4.png)

| Property                   | Value                                                            |
|----------------------------|------------------------------------------------------------------|
| Ports                      | (16) 2.5 GbE RJ45 PoE+++, (32) 10 GbE RJ45 PoE+++, (4) 25G SFP28 |
| Switching capacity         | 920 Gbps (460 Gbps non-blocking, 684 Mpps)                       |
| PoE                        | Up to PoE+++; 1,080 W @ 230 V / 972 W @ 115 V                    |
| Layer                      | Layer 3                                                          |
| MAC table / VLANs / buffer | 32,000 / 1,000 / 8 MB                                            |
| Static routes              | 512                                                              |
| Redundancy                 | DC power backup (USP-RPS)                                        |
| Power                      | 200 W excl. PoE; 1,280 W incl. PoE @ 230 V                       |
| Form factor                | 1U rack — 442 × 480 × 44 mm, 8.7 kg                              |

### Pro XG 48 (USW-Pro-XG-48)

The same 920 Gbps fabric and port layout without PoE, for $500 less and a quarter of the power draw. If the endpoints are servers, storage and workstations rather than powered devices, this is the better buy.

- **Price:** $1,999 USD / £1,590 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/3ecbd029-ba8a-4f82-aa5e-0f2902f60576/1ee9bf0b-ece0-4fe9-bfec-64e21a8eed7e.png) · [angle](https://cdn.ecomm.ui.com/products/3ecbd029-ba8a-4f82-aa5e-0f2902f60576/4353204e-2169-44d0-8deb-bc607cad7d0a.png) · [rear](https://cdn.ecomm.ui.com/products/3ecbd029-ba8a-4f82-aa5e-0f2902f60576/708c9643-f559-4f1b-a691-105be13ba816.png)

| Property                   | Value                                              |
|----------------------------|----------------------------------------------------|
| Ports                      | (16) 2.5 GbE RJ45, (32) 10 GbE RJ45, (4) 25G SFP28 |
| Switching capacity         | 920 Gbps (460 Gbps non-blocking, 684 Mpps)         |
| PoE                        | None                                               |
| Layer                      | Layer 3                                            |
| MAC table / VLANs / buffer | 32,000 / 1,000 / 8 MB                              |
| Redundancy                 | DC power backup (USP-RPS)                          |
| Power                      | 250 W max                                          |
| Form factor                | 1U rack — 442 × 480 × 44 mm, 7.5 kg                |

### Pro XG 24 PoE (USW-Pro-XG-24-PoE)

Sixteen 10 GbE and eight 2.5 GbE ports, all PoE+++, with two 25G SFP28 uplinks and a 720 W budget. Fully non-blocking at 230 Gbps.

- **Price:** $1,799 USD / £1,430 GBP — *currently sold out in the US store*
- **Images:** [front](https://cdn.ecomm.ui.com/products/b6f03374-2a31-428c-86fb-62b49739c924/89656ee3-c3f8-424e-b3b1-e493508deffa.png) · [angle](https://cdn.ecomm.ui.com/products/b6f03374-2a31-428c-86fb-62b49739c924/23e138df-49ae-4859-be94-f344cf6dfc51.png) · [rear](https://cdn.ecomm.ui.com/products/b6f03374-2a31-428c-86fb-62b49739c924/9c96bae5-81a1-45f1-a7fd-2f2286133cd9.png)

| Property                   | Value                                                           |
|----------------------------|-----------------------------------------------------------------|
| Ports                      | (8) 2.5 GbE RJ45 PoE+++, (16) 10 GbE RJ45 PoE+++, (2) 25G SFP28 |
| Switching capacity         | 460 Gbps (230 Gbps non-blocking, 342 Mpps)                      |
| PoE                        | Up to PoE+++; 720 W                                             |
| Layer                      | Layer 3                                                         |
| MAC table / VLANs / buffer | 32,000 / 1,000 / 8 MB                                           |
| Static routes              | 512                                                             |
| Redundancy                 | DC power backup (USP-RPS)                                       |
| Power                      | 150 W excl. PoE; 870 W incl. PoE                                |
| Form factor                | 1U rack — 442 × 480 × 44 mm, 7.9 kg                             |

### Pro XG 24 (USW-Pro-XG-24)

The non-PoE 24-port. Same 460 Gbps fabric, 150 W draw, $700 cheaper.

- **Price:** $1,099 USD / £875 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/5f2af015-a9fd-4f79-9779-74860c328e75/68de03e5-b2d6-41e0-bee0-82c106fe2ee6.png) · [angle](https://cdn.ecomm.ui.com/products/5f2af015-a9fd-4f79-9779-74860c328e75/f723c13f-f379-44bd-ae63-d9dd87e50ae5.png) · [rear](https://cdn.ecomm.ui.com/products/5f2af015-a9fd-4f79-9779-74860c328e75/dd0b3972-e3e3-42c2-94bc-ae7548d20535.png)

| Property                   | Value                                             |
|----------------------------|---------------------------------------------------|
| Ports                      | (8) 2.5 GbE RJ45, (16) 10 GbE RJ45, (2) 25G SFP28 |
| Switching capacity         | 460 Gbps (230 Gbps non-blocking, 342 Mpps)        |
| PoE                        | None                                              |
| Layer                      | Layer 3                                           |
| MAC table / VLANs / buffer | 32,000 / 1,000 / 8 MB                             |
| Redundancy                 | DC power backup (USP-RPS)                         |
| Power                      | 150 W max                                         |
| Form factor                | 1U rack — 442 × 480 × 44 mm, 7.2 kg               |

### Pro XG 10 PoE (USW-Pro-XG-10-PoE)

Ten 10 GbE PoE+++ ports and two 10G SFP+ uplinks in a shallower 1U chassis, with a 400 W budget. This is the compact 10G PoE switch for a wiring closet serving a dense cluster of high-power APs or 10G cameras, and at $699 it costs a third of the 24-port model.

- **Price:** $699 USD / £555 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/2617380d-b3a8-47c3-802f-fcc22d24e931/0af98290-e7a4-4d4f-b4e5-bd781f3d2132.png) · [angle](https://cdn.ecomm.ui.com/products/2617380d-b3a8-47c3-802f-fcc22d24e931/f5951c05-aa1e-49d7-93c2-0a0d6cfe4c6d.png) · [rear](https://cdn.ecomm.ui.com/products/2617380d-b3a8-47c3-802f-fcc22d24e931/bff6e87d-363b-48c9-94f4-a78aa3ff2d34.png)

| Property                   | Value                                      |
|----------------------------|--------------------------------------------|
| Ports                      | (10) 10 GbE RJ45 PoE+++, (2) 10G SFP+      |
| Switching capacity         | 240 Gbps (120 Gbps non-blocking, 179 Mpps) |
| PoE                        | Up to PoE+++; 400 W                        |
| Layer                      | Layer 3                                    |
| MAC table / VLANs / buffer | 32,000 / 1,000 / 2 MB                      |
| Redundancy                 | DC power backup (USP-RPS)                  |
| Power                      | 65 W excl. PoE; 520 W incl. PoE            |
| Form factor                | 1U rack — 442.4 × 285 × 44 mm, 4.6 kg      |

### Pro XG 8 PoE (USW-Pro-XG-8-PoE)

Eight 10 GbE PoE++ ports and two 10G SFP+ uplinks in a desktop or wall-mount body — 200 Gbps of switching in something the size of a hardback book. Note it tops out at PoE++ (60 W/port, 155 W total) rather than the PoE+++ of its rack-mount siblings.

- **Price:** $499 USD / £395 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/104a0f86-8c2e-462d-a95c-ebbc5675ebb8/a78f6eeb-f4ef-4401-aae4-4d8a50e30fce.png) · [angle](https://cdn.ecomm.ui.com/products/104a0f86-8c2e-462d-a95c-ebbc5675ebb8/fc9091aa-8685-4069-bc96-7af9496d7b8f.png) · [rear](https://cdn.ecomm.ui.com/products/104a0f86-8c2e-462d-a95c-ebbc5675ebb8/4f3afd43-46ce-4648-8511-d738be7e9d4d.png)

| Property                   | Value                                                    |
|----------------------------|----------------------------------------------------------|
| Ports                      | (8) 10 GbE RJ45 PoE++, (2) 10G SFP+                      |
| Switching capacity         | 200 Gbps (100 Gbps non-blocking, 149 Mpps)               |
| PoE                        | Up to PoE++; 155 W                                       |
| Layer                      | Layer 3                                                  |
| MAC table / VLANs / buffer | 32,000 / 1,000 / 2 MB                                    |
| Power                      | 61 W excl. PoE; 210 W incl. PoE (external adapter)       |
| Form factor                | Compact desktop / wall — 210.4 × 173.8 × 43.7 mm, 1.6 kg |

## Professional Max and Pro HD

The mainstream multi-gig tier. Pro Max mixes gigabit and 2.5 GbE copper with 10G SFP+ uplinks; Pro HD goes almost all 2.5 GbE with four 10G SFP+ uplinks. Both are Layer 3 with Etherlighting and USP-RPS backup.

### Pro Max 48 PoE (USW-Pro-Max-48-PoE)

Forty-eight copper ports — 32 gigabit and 16 at 2.5 GbE, with PoE++ on sixteen of them — plus four 10G SFP+ uplinks and a 720 W budget. The volume choice for a floor of access points and cameras that don't need 10 GbE to the edge.

- **Price:** $1,299 USD / £1,035 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/51e22689-9b81-4717-beed-fe2c65c57362/18b790aa-f0b3-4e6b-9aca-63948c65535d.png) · [angle](https://cdn.ecomm.ui.com/products/51e22689-9b81-4717-beed-fe2c65c57362/4e67c0d8-4641-4555-bc01-c84e3ad7da14.png) · [rear](https://cdn.ecomm.ui.com/products/51e22689-9b81-4717-beed-fe2c65c57362/041bab49-f8ef-4d73-ad50-ebc5cfd76fee.png)

| Property                   | Value                                                                                 |
|----------------------------|---------------------------------------------------------------------------------------|
| Ports                      | (32) 1 GbE RJ45 (24 PoE+, 8 PoE++), (16) 2.5 GbE RJ45 (8 PoE+, 8 PoE++), (4) 10G SFP+ |
| Switching capacity         | 224 Gbps (112 Gbps non-blocking, 167 Mpps)                                            |
| PoE                        | Up to PoE++; 720 W                                                                    |
| Layer                      | Layer 3                                                                               |
| MAC table / VLANs / buffer | 32,000 / 1,000 / 2 MB                                                                 |
| Redundancy                 | DC power backup (USP-RPS)                                                             |
| Power                      | 100 W excl. PoE; 820 W incl. PoE                                                      |
| Form factor                | 1U rack — 442.4 × 400 × 44 mm, 6.2 kg                                                 |

### Pro Max 48 (USW-Pro-Max-48)

Same fabric and port mix, no PoE, half the price. 100 W total draw.

- **Price:** $649 USD / £515 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/0966f626-63a9-4dc6-b07d-2e0b40e2cead/9bfb5a25-50a7-4c22-b2b9-ee3c8b4a82c0.png) · [angle](https://cdn.ecomm.ui.com/products/0966f626-63a9-4dc6-b07d-2e0b40e2cead/53a5e461-6a51-4367-8445-40a308801e6b.png) · [rear](https://cdn.ecomm.ui.com/products/0966f626-63a9-4dc6-b07d-2e0b40e2cead/e43884b9-4f64-47e8-b9c6-c1944f4b49c1.png)

| Property                   | Value                                            |
|----------------------------|--------------------------------------------------|
| Ports                      | (32) 1 GbE RJ45, (16) 2.5 GbE RJ45, (4) 10G SFP+ |
| Switching capacity         | 224 Gbps (112 Gbps non-blocking, 167 Mpps)       |
| PoE                        | None                                             |
| Layer                      | Layer 3                                          |
| MAC table / VLANs / buffer | 32,000 / 1,000 / 2 MB                            |
| Redundancy                 | DC power backup (USP-RPS)                        |
| Power                      | 100 W max                                        |
| Form factor                | 1U rack — 442 × 325 × 44 mm, 4.8 kg              |

### Pro Max 24 PoE (USW-Pro-Max-24-PoE)

Sixteen gigabit and eight 2.5 GbE ports, all of the 2.5 GbE and half the gigabit at PoE++, two 10G SFP+ uplinks, 400 W budget. The single most commonly deployed switch in the range for small and mid-sized sites.

- **Price:** $799 USD / £635 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/58922518-88f6-4c75-89c1-f57ba3d8253a/797246c6-70eb-4606-be9a-77515ac74451.png) · [angle](https://cdn.ecomm.ui.com/products/58922518-88f6-4c75-89c1-f57ba3d8253a/4abcc76a-18d7-49b7-8ce8-047f318049d2.png) · [rear](https://cdn.ecomm.ui.com/products/58922518-88f6-4c75-89c1-f57ba3d8253a/89655de7-03a2-4ff4-ae80-3ae24dc5204d.png)

| Property                   | Value                                                                   |
|----------------------------|-------------------------------------------------------------------------|
| Ports                      | (16) 1 GbE RJ45 (8 PoE+, 8 PoE++), (8) 2.5 GbE RJ45 PoE++, (2) 10G SFP+ |
| Switching capacity         | 112 Gbps (56 Gbps non-blocking, 83 Mpps)                                |
| PoE                        | Up to PoE++; 400 W                                                      |
| Layer                      | Layer 3                                                                 |
| MAC table / VLANs / buffer | 16,000 / 1,000 / 1.5 MB                                                 |
| Redundancy                 | DC power backup (USP-RPS)                                               |
| Power                      | 50 W excl. PoE; 450 W incl. PoE                                         |
| Form factor                | 1U rack — 442 × 325 × 44 mm, 5.1 kg                                     |

### Pro Max 24 (USW-Pro-Max-24)

The non-PoE 24-port Pro Max: same 112 Gbps fabric, 50 W draw.

- **Price:** $449 USD / £355 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/dd435390-1705-4159-852c-fd210b4e7c17/756296a7-f55d-4ac6-9aa0-fb946ee30434.png) · [angle](https://cdn.ecomm.ui.com/products/dd435390-1705-4159-852c-fd210b4e7c17/9a1c9290-17c4-49c5-b795-a282b758b4ef.png) · [rear](https://cdn.ecomm.ui.com/products/dd435390-1705-4159-852c-fd210b4e7c17/f3d2548b-ce69-44ec-a642-fab0afed3f2f.png)

| Property                   | Value                                           |
|----------------------------|-------------------------------------------------|
| Ports                      | (16) 1 GbE RJ45, (8) 2.5 GbE RJ45, (2) 10G SFP+ |
| Switching capacity         | 112 Gbps (56 Gbps non-blocking, 83 Mpps)        |
| PoE                        | None                                            |
| Layer                      | Layer 3                                         |
| MAC table / VLANs / buffer | 16,000 / 1,000 / 1.5 MB                         |
| Redundancy                 | DC power backup (USP-RPS)                       |
| Power                      | 50 W max                                        |
| Form factor                | 1U rack — 442 × 325 × 44 mm, 4.2 kg             |

### Pro Max 16 PoE (USW-Pro-Max-16-PoE)

A half-width chassis that mounts in a rack, sits on a desk or hangs on a wall — twelve gigabit PoE+ ports, four 2.5 GbE PoE++, two 10G SFP+ uplinks and a 180 W external supply. The most flexible mid-size switch UniFi makes.

- **Price:** $399 USD / £315 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/0694226c-9791-4d7e-9fb1-d98c91fe3fda/7b73b27e-e77a-4771-b6bf-0aed35b4a16e.png) · [angle](https://cdn.ecomm.ui.com/products/0694226c-9791-4d7e-9fb1-d98c91fe3fda/346874ce-466e-4221-8ee1-bac91f1af1da.png) · [rear](https://cdn.ecomm.ui.com/products/0694226c-9791-4d7e-9fb1-d98c91fe3fda/59448338-5c2b-47c3-8c76-5173bd54b458.png)

| Property                   | Value                                                                 |
|----------------------------|-----------------------------------------------------------------------|
| Ports                      | (12) 1 GbE RJ45 PoE+, (4) 2.5 GbE RJ45 PoE++, (2) 10G SFP+            |
| Switching capacity         | 84 Gbps (42 Gbps non-blocking, 62 Mpps)                               |
| PoE                        | Up to PoE++; 180 W                                                    |
| Layer                      | Layer 3                                                               |
| MAC table / VLANs / buffer | 16,000 / 1,000 / 1.5 MB                                               |
| Power                      | 25 W excl. PoE; 210 W incl. PoE (external adapter)                    |
| Form factor                | 1U rack (half-width), desktop or wall — 325.1 × 160 × 43.7 mm, 2.1 kg |

### Pro Max 16 (USW-Pro-Max-16)

The PoE-free 16-port, drawing 25 W from an external adapter with the same mounting flexibility.

- **Price:** $279 USD / £220 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/065e9d0f-19c7-4b9e-8b36-8e900363af80/b888cf82-f499-4460-943a-5e92a7a9cf79.png) · [angle](https://cdn.ecomm.ui.com/products/065e9d0f-19c7-4b9e-8b36-8e900363af80/ec518f84-8d77-44bb-9d88-b385dd977288.png) · [rear](https://cdn.ecomm.ui.com/products/065e9d0f-19c7-4b9e-8b36-8e900363af80/0bc19ab6-5410-450f-9207-2d4981aa2d47.png)

| Property                   | Value                                                                  |
|----------------------------|------------------------------------------------------------------------|
| Ports                      | (12) 1 GbE RJ45, (4) 2.5 GbE RJ45, (2) 10G SFP+                        |
| Switching capacity         | 84 Gbps (42 Gbps non-blocking, 62 Mpps)                                |
| PoE                        | None                                                                   |
| Layer                      | Layer 3                                                                |
| MAC table / VLANs / buffer | 16,000 / 1,000 / 1.5 MB                                                |
| Power                      | 25 W max (external adapter)                                            |
| Form factor                | 1U rack (half-width), desktop or wall — 325.1 × 160 × 43.7 mm, 1.95 kg |

### Pro HD 24 PoE (USW-Pro-HD-24-PoE)

Twenty-two 2.5 GbE plus two 10 GbE ports, every one PoE++, over four 10G SFP+ uplinks. It doubles the Pro Max 24's throughput (115 Gbps non-blocking) by dropping gigabit entirely, and carries a 600 W budget. This is the right switch for a WiFi 7 deployment where every AP wants 2.5 GbE and 60 W.

- **Price:** $999 USD / £795 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/17d901a5-c99b-4b02-8d3b-8cb7a5da0512/2ce20101-767d-4e15-b0dc-80449d2807a1.png) · [angle](https://cdn.ecomm.ui.com/products/17d901a5-c99b-4b02-8d3b-8cb7a5da0512/b70c75ef-09a0-48ed-904b-1d77fe507b2b.png) · [rear](https://cdn.ecomm.ui.com/products/17d901a5-c99b-4b02-8d3b-8cb7a5da0512/d71cfe34-031b-4c2c-bb45-06f53ffc2eb4.png)

| Property                   | Value                                                        |
|----------------------------|--------------------------------------------------------------|
| Ports                      | (22) 2.5 GbE RJ45 PoE++, (2) 10 GbE RJ45 PoE++, (4) 10G SFP+ |
| Switching capacity         | 230 Gbps (115 Gbps non-blocking, 171 Mpps)                   |
| PoE                        | Up to PoE++; 600 W                                           |
| Layer                      | Layer 3                                                      |
| MAC table / VLANs / buffer | 32,000 / 1,000 / 2 MB                                        |
| Redundancy                 | DC power backup (USP-RPS)                                    |
| Power                      | 60 W excl. PoE; 660 W incl. PoE                              |
| Form factor                | 1U rack — 442 × 400 × 44 mm, 6.2 kg                          |

### Pro HD 24 (USW-Pro-HD-24)

The non-PoE Pro HD. Same 230 Gbps fabric for $400 less and 60 W of draw — the cheapest way to get 22 ports of 2.5 GbE with quad 10G uplinks under UniFi management.

- **Price:** $599 USD / £475 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/66804bfc-9afd-453c-adbb-7327d8cebe25/387ab337-f5c8-4f8f-94e1-cffc8888ad1d.png) · [angle](https://cdn.ecomm.ui.com/products/66804bfc-9afd-453c-adbb-7327d8cebe25/c84e3166-fbc6-40ab-8b03-a99a644860e0.png) · [rear](https://cdn.ecomm.ui.com/products/66804bfc-9afd-453c-adbb-7327d8cebe25/5287581d-44ce-4ac8-b0d7-b0d27b543dcf.png)

| Property                   | Value                                            |
|----------------------------|--------------------------------------------------|
| Ports                      | (22) 2.5 GbE RJ45, (2) 10 GbE RJ45, (4) 10G SFP+ |
| Switching capacity         | 230 Gbps (115 Gbps non-blocking, 171 Mpps)       |
| PoE                        | None                                             |
| Layer                      | Layer 3                                          |
| MAC table / VLANs / buffer | 32,000 / 1,000 / 2 MB                            |
| Redundancy                 | DC power backup (USP-RPS)                        |
| Power                      | 60 W max                                         |
| Form factor                | 1U rack — 442 × 285 × 44 mm, 4.2 kg              |

## Professional (USW-Pro)

The original Layer 3 gigabit line: all-gigabit copper with 10G SFP+ uplinks, fanless on the non-PoE models, USP-RPS backup throughout. Ubiquiti has largely superseded these with the Pro Max, and both non-PoE models are currently sold out in the US.

### Pro 48 PoE (USW-Pro-48-PoE)

Forty-eight gigabit ports (forty PoE+, eight PoE++) and four 10G SFP+ uplinks with a 600 W budget and a 4 MB buffer — the largest packet buffer outside the Pro XG and Enterprise tiers.

- **Price:** $1,099 USD / £875 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/6e019f0c-26b5-4fdf-b4e1-994abd9ce6e1/483da8b5-c584-4100-a4f2-18d11fa7187e.png) · [angle](https://cdn.ecomm.ui.com/products/6e019f0c-26b5-4fdf-b4e1-994abd9ce6e1/4748196a-d2ef-4766-b167-1e2dc9d55620.png) · [rear](https://cdn.ecomm.ui.com/products/6e019f0c-26b5-4fdf-b4e1-994abd9ce6e1/f52355cf-9680-4b82-85ef-977c4bcfb62d.png)

| Property                   | Value                                            |
|----------------------------|--------------------------------------------------|
| Ports                      | (48) 1 GbE RJ45 (40 PoE+, 8 PoE++), (4) 10G SFP+ |
| Switching capacity         | 176 Gbps (88 Gbps non-blocking, 131 Mpps)        |
| PoE                        | Up to PoE++; 600 W                               |
| Layer                      | Layer 3                                          |
| MAC table / VLANs / buffer | 16,000 / 1,000 / 4 MB                            |
| Redundancy                 | DC power backup (USP-RPS)                        |
| Power                      | 60 W excl. PoE; 660 W incl. PoE                  |
| Form factor                | 1U rack — 442 × 400 × 44 mm, 6.2 kg              |

### Pro 48 (USW-Pro-48)

The fanless non-PoE 48-port, drawing 60 W.

- **Price:** $599 USD / £475 GBP — *currently sold out in the US store*
- **Images:** [front](https://cdn.ecomm.ui.com/products/17a1db0a-a705-4ea0-b538-2972ca71615a/e5caa219-3faa-48a5-a4e1-d7eea81d738b.png) · [angle](https://cdn.ecomm.ui.com/products/17a1db0a-a705-4ea0-b538-2972ca71615a/9ff10997-647b-4a9a-be52-3b24a2f96ea2.png) · [rear](https://cdn.ecomm.ui.com/products/17a1db0a-a705-4ea0-b538-2972ca71615a/018b898c-87d0-4091-8b84-60db2e548211.png)

| Property                   | Value                                     |
|----------------------------|-------------------------------------------|
| Ports                      | (48) 1 GbE RJ45, (4) 10G SFP+             |
| Switching capacity         | 176 Gbps (88 Gbps non-blocking, 131 Mpps) |
| PoE                        | None                                      |
| Layer                      | Layer 3                                   |
| MAC table / VLANs / buffer | 16,000 / 1,000 / 4 MB                     |
| Redundancy                 | DC power backup (USP-RPS)                 |
| Power                      | 60 W max, fanless                         |
| Form factor                | 1U rack — 442 × 285 × 44 mm, 4 kg         |

### Pro 24 PoE (USW-Pro-24-PoE)

Twenty-four gigabit ports (sixteen PoE+, eight PoE++), two 10G SFP+ uplinks, 400 W budget.

- **Price:** $699 USD / £555 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/5b69cdb5-e7ea-44e6-ae16-8714339038fb/18d39964-78e7-45f9-874d-e331f1408730.png) · [angle](https://cdn.ecomm.ui.com/products/5b69cdb5-e7ea-44e6-ae16-8714339038fb/ca459cf0-f563-4958-9443-e30ce3bcdfa5.png) · [rear](https://cdn.ecomm.ui.com/products/5b69cdb5-e7ea-44e6-ae16-8714339038fb/9346cd35-0b2c-47e2-b298-f8cbb8496827.png)

| Property                   | Value                                            |
|----------------------------|--------------------------------------------------|
| Ports                      | (24) 1 GbE RJ45 (16 PoE+, 8 PoE++), (2) 10G SFP+ |
| Switching capacity         | 88 Gbps (44 Gbps non-blocking, 65 Mpps)          |
| PoE                        | Up to PoE++; 400 W                               |
| Layer                      | Layer 3                                          |
| MAC table / VLANs / buffer | 16,000 / 1,000 / 2 MB                            |
| Redundancy                 | DC power backup (USP-RPS)                        |
| Power                      | 50 W excl. PoE; 450 W incl. PoE                  |
| Form factor                | 1U rack — 442 × 285 × 44 mm, 4.3 kg              |

### Pro 24 (USW-Pro-24)

The fanless non-PoE 24-port, drawing 30 W.

- **Price:** $399 USD / £315 GBP — *currently sold out in the US store*
- **Images:** [front](https://cdn.ecomm.ui.com/products/2315330e-7a37-4c6b-87df-0743d04e87ca/5281dd32-ad14-40c5-a8d0-2df56a340bae.png) · [angle](https://cdn.ecomm.ui.com/products/2315330e-7a37-4c6b-87df-0743d04e87ca/546e26a6-6db5-431e-aa3c-bab58f000461.png) · [rear](https://cdn.ecomm.ui.com/products/2315330e-7a37-4c6b-87df-0743d04e87ca/5701c133-d1cd-46e7-827f-dc8dfc52da85.png)

| Property                   | Value                                   |
|----------------------------|-----------------------------------------|
| Ports                      | (24) 1 GbE RJ45, (2) 10G SFP+           |
| Switching capacity         | 88 Gbps (44 Gbps non-blocking, 65 Mpps) |
| PoE                        | None                                    |
| Layer                      | Layer 3                                 |
| MAC table / VLANs / buffer | 16,000 / 1,000 / 2 MB                   |
| Redundancy                 | DC power backup (USP-RPS)               |
| Power                      | 30 W max, fanless                       |
| Form factor                | 1U rack — 442 × 285 × 44 mm, 3.5 kg     |

### UPS PoE Switch (USW-Mission-Critical)

A category of one: a nine-port gigabit switch with a 368 Wh lithium-ion battery inside a full-depth 1U chassis. It keeps eight PoE devices — typically cameras, door readers and the access points that carry the alarm path — running through a mains failure, and it does so as a managed UniFi switch rather than a dumb UPS. The switching specifications are modest by design; you buy it for the battery.

- **Price:** $999 USD / £799 GBP (listed under the EU/UK SKU; currently shown as unavailable in the UK store)
- **Images:** [front](https://cdn.ecomm.ui.com/products/0a2a7ba1-12c3-4736-a31d-ed9a0e9f44d1/f09f5ea1-7307-41bf-89d4-bd864b4cb895.png) · [angle](https://cdn.ecomm.ui.com/products/0a2a7ba1-12c3-4736-a31d-ed9a0e9f44d1/72cd7221-c9d6-4aec-94d6-9df69bdbde09.png) · [rear](https://cdn.ecomm.ui.com/products/0a2a7ba1-12c3-4736-a31d-ed9a0e9f44d1/66573b52-445c-4970-a627-6d6f07f511fb.png)

| Property                   | Value                                                         |
|----------------------------|---------------------------------------------------------------|
| Ports                      | (9) 1 GbE RJ45 (4 PoE+, 4 PoE++)                              |
| Switching capacity         | 18 Gbps (9 Gbps non-blocking, 13 Mpps)                        |
| PoE                        | Up to PoE++; 120 W                                            |
| Battery                    | 368 Wh integrated lithium-ion; 48 V DC external battery input |
| Layer                      | Layer 3                                                       |
| MAC table / VLANs / buffer | 8,000 / 1,000 / 0.5 MB                                        |
| Power                      | 50 W excl. PoE; 240 W incl. PoE and AC output                 |
| Form factor                | 1U full-depth rack — 442 × 480 × 44 mm, 9 kg                  |

## Standard (USW)

Fanless, Layer 2, all-gigabit copper with 1G SFP uplinks. The entry-level rack switches: no routing, no multi-gig, small buffers, but silent and inexpensive.

### Standard 48 PoE (USW-48-PoE)

Forty-eight gigabit ports with PoE+ on thirty-two of them and four 1G SFP uplinks, in a silent fanless chassis. The 195 W budget is the constraint — that is roughly six PoE+ access points, not thirty-two.

- **Price:** $589 USD / £469 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/147e90c2-da47-44ad-ad19-b98e223cb54b/378bcc80-2ebd-45f1-a2fc-0e66669824e1.png) · [angle](https://cdn.ecomm.ui.com/products/147e90c2-da47-44ad-ad19-b98e223cb54b/d7570b62-96d1-4c56-8edf-bb5a7c3a4a00.png) · [rear](https://cdn.ecomm.ui.com/products/147e90c2-da47-44ad-ad19-b98e223cb54b/6afbd24e-02b6-49c7-b0f7-9260b8c7ae00.png)

| Property                   | Value                                    |
|----------------------------|------------------------------------------|
| Ports                      | (48) 1 GbE RJ45 (32 PoE+), (4) 1G SFP    |
| Switching capacity         | 104 Gbps (52 Gbps non-blocking, 77 Mpps) |
| PoE                        | Up to PoE+; 195 W                        |
| Layer                      | Layer 2                                  |
| MAC table / VLANs / buffer | 16,000 / 1,000 / 1.5 MB                  |
| Power                      | 45 W excl. PoE; 240 W incl. PoE; fanless |
| Form factor                | 1U rack — 442 × 285 × 44 mm, 4.5 kg      |

### Standard 48 (USW-48)

The non-PoE 48-port, 40 W, fanless.

- **Price:** $399 USD / £315 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/8a649eb8-c497-4240-92ba-a5c7a792c395/fcf3841a-9b8c-45b1-9d77-8f878bebdcba.png) · [angle](https://cdn.ecomm.ui.com/products/8a649eb8-c497-4240-92ba-a5c7a792c395/72ca8594-377f-4086-8e7c-95bad3559492.png) · [rear](https://cdn.ecomm.ui.com/products/8a649eb8-c497-4240-92ba-a5c7a792c395/16adba9c-cef1-4917-8c85-76f1e6d28370.png)

| Property                   | Value                                    |
|----------------------------|------------------------------------------|
| Ports                      | (48) 1 GbE RJ45, (4) 1G SFP              |
| Switching capacity         | 104 Gbps (52 Gbps non-blocking, 77 Mpps) |
| PoE                        | None                                     |
| Layer                      | Layer 2                                  |
| MAC table / VLANs / buffer | 16,000 / 1,000 / 1.5 MB                  |
| Power                      | 40 W max, fanless                        |
| Form factor                | 1U rack — 442 × 285 × 44 mm, 3.9 kg      |

### Standard 24 PoE (USW-24-PoE)

Twenty-four gigabit ports with PoE+ on sixteen, two 1G SFP uplinks, 95 W budget.

- **Price:** $379 USD / £300 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/467359c4-e5c3-487b-ae00-f6b7de29c6fc/1fd41f67-8fd9-4689-989e-c03b43217e3a.png) · [angle](https://cdn.ecomm.ui.com/products/467359c4-e5c3-487b-ae00-f6b7de29c6fc/fd3c57e8-dd82-4f37-908b-8e95bd4bc184.png) · [rear](https://cdn.ecomm.ui.com/products/467359c4-e5c3-487b-ae00-f6b7de29c6fc/f963dc67-5b33-4249-b767-e355723ad1ad.png)

| Property                   | Value                                   |
|----------------------------|-----------------------------------------|
| Ports                      | (24) 1 GbE RJ45 (16 PoE+), (2) 1G SFP   |
| Switching capacity         | 52 Gbps (26 Gbps non-blocking, 39 Mpps) |
| PoE                        | Up to PoE+; 95 W                        |
| Layer                      | Layer 2                                 |
| MAC table / VLANs / buffer | 8,000 / 1,000 / 0.5 MB                  |
| Power                      | 25 W excl. PoE; 120 W incl. PoE         |
| Form factor                | 1U rack — 442 × 200 × 44 mm, 3 kg       |

### Standard 24 (USW-24)

The cheapest 24-port rack switch UniFi sells, at 25 W and silent.

- **Price:** $225 USD / £179 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/d443e087-efcf-47a1-8465-7fa3e60cf916/23c2a6bc-a606-43c0-9f3c-27f04f437bdf.png) · [angle](https://cdn.ecomm.ui.com/products/d443e087-efcf-47a1-8465-7fa3e60cf916/509b49ea-cbc6-4ef8-a575-59e127dff7ed.png) · [rear](https://cdn.ecomm.ui.com/products/d443e087-efcf-47a1-8465-7fa3e60cf916/dfe9747c-e61f-4eb3-b63a-2735f5f40cfb.png)

| Property                   | Value                                   |
|----------------------------|-----------------------------------------|
| Ports                      | (24) 1 GbE RJ45, (2) 1G SFP             |
| Switching capacity         | 52 Gbps (26 Gbps non-blocking, 39 Mpps) |
| PoE                        | None                                    |
| Layer                      | Layer 2                                 |
| MAC table / VLANs / buffer | 8,000 / 1,000 / 0.5 MB                  |
| Power                      | 25 W max, fanless                       |
| Form factor                | 1U rack — 442 × 200 × 44 mm, 2.7 kg     |

### Standard 16 PoE (USW-16-PoE)

Sixteen gigabit ports with PoE+ on eight, two 1G SFP uplinks, and a 42 W budget that is the tightest in the rack-mount range.

- **Price:** $299 USD / £239 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/ab04370e-f45d-4651-828c-b290de8df45b/8fe1e8dc-fb88-4102-91b5-e1826d079d37.png) · [angle](https://cdn.ecomm.ui.com/products/ab04370e-f45d-4651-828c-b290de8df45b/52b4001a-a88f-40a1-b8a5-9c4da4d8501b.png) · [rear](https://cdn.ecomm.ui.com/products/ab04370e-f45d-4651-828c-b290de8df45b/7c1c49b0-aa3e-4c82-800f-0156576dd8bc.png)

| Property                   | Value                                   |
|----------------------------|-----------------------------------------|
| Ports                      | (16) 1 GbE RJ45 (8 PoE+), (2) 1G SFP    |
| Switching capacity         | 36 Gbps (18 Gbps non-blocking, 27 Mpps) |
| PoE                        | Up to PoE+; 42 W                        |
| Layer                      | Layer 2                                 |
| MAC table / VLANs / buffer | 8,000 / 1,000 / 0.5 MB                  |
| Power                      | 18 W excl. PoE; 60 W incl. PoE; fanless |
| Form factor                | 1U rack — 442 × 200 × 44 mm, 2.8 kg     |

## Utility, Lite, Ultra and Flex

Ubiquiti's catch-all category for compact switches: desktop, wall, DIN-rail, magnetic and outdoor units, several of which are themselves PoE-powered and need no mains outlet. It also contains three eight-port models that logically belong to the Pro families but are sold here because of their form factor.

### Enterprise 8 PoE (USW-Enterprise-8-PoE)

Eight 2.5 GbE PoE+ ports and two 10G SFP+ uplinks in a compact desktop body, Layer 3, 120 W budget. Ubiquiti now labels it *Vintage* — it remains on sale but has been overtaken by the Pro XG 8 PoE, which offers 10 GbE copper and 2.5× the throughput for $20 more.

- **Price:** $479 USD / £380 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/f9af6d87-2024-475d-a062-2038566ac850/726a4b28-9eb2-46c6-925e-4eef0adb4ad4.png) · [angle](https://cdn.ecomm.ui.com/products/f9af6d87-2024-475d-a062-2038566ac850/2d1d1a32-584f-4f08-96de-47411cc2cc35.png) · [rear](https://cdn.ecomm.ui.com/products/f9af6d87-2024-475d-a062-2038566ac850/730a98a7-bbb9-4766-a29e-3e2d444bf907.png)

| Property                   | Value                                       |
|----------------------------|---------------------------------------------|
| Ports                      | (8) 2.5 GbE RJ45 PoE+, (2) 10G SFP+         |
| Switching capacity         | 80 Gbps (40 Gbps non-blocking, 60 Mpps)     |
| PoE                        | Up to PoE+; 120 W                           |
| Layer                      | Layer 3                                     |
| MAC table / VLANs / buffer | 16,000 / 1,000 / 1.5 MB                     |
| Power                      | 30 W excl. PoE; 150 W incl. PoE             |
| Form factor                | Compact desktop — 200 × 248 × 44 mm, 2.4 kg |

### Pro 8 PoE (USW-Pro-8-PoE)

Eight gigabit ports (six PoE+, two PoE++) and two 10G SFP+ uplinks, Layer 3, 120 W budget, desktop or wall. The small-office equivalent of the Pro 24.

- **Price:** $349 USD / £239 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/37c8c814-f786-4f8a-ac58-96525f22e029/d4a21d75-7b98-43d5-9868-36cab2ba8161.png) · [angle](https://cdn.ecomm.ui.com/products/37c8c814-f786-4f8a-ac58-96525f22e029/046a6cab-ae9d-4f48-94f3-3c948701deb6.png) · [rear](https://cdn.ecomm.ui.com/products/37c8c814-f786-4f8a-ac58-96525f22e029/704a92a6-8ca5-4ba3-91d1-0bd26a3f6d92.png)

| Property                   | Value                                              |
|----------------------------|----------------------------------------------------|
| Ports                      | (8) 1 GbE RJ45 (6 PoE+, 2 PoE++), (2) 10G SFP+     |
| Switching capacity         | 56 Gbps (28 Gbps non-blocking, 42 Mpps)            |
| PoE                        | Up to PoE++; 120 W                                 |
| Layer                      | Layer 3                                            |
| MAC table / VLANs / buffer | 16,000 / 1,000 / 1.5 MB                            |
| Power                      | 30 W excl. PoE; 150 W incl. PoE                    |
| Form factor                | Compact desktop / wall — 200 × 248 × 44 mm, 2.1 kg |

### Flex 10 GbE (USW-Flex-XG)

Four 10 GbE RJ45 ports plus one gigabit port in a compact wall-mountable body, powered by PoE+ or USB-C. Layer 2 only, no PoE output — this is a 10G workgroup switch for a desk of workstations or a small NAS cluster.

- **Price:** $299 USD / £229 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/61d6b40e-0997-4ce0-b723-bb0c2c4aafa2/9d558a4b-49c7-435d-b574-94f6581e23a9.png) · [angle](https://cdn.ecomm.ui.com/products/61d6b40e-0997-4ce0-b723-bb0c2c4aafa2/06ad6692-b17a-46ce-8094-0fd4d20d162c.png) · [rear](https://cdn.ecomm.ui.com/products/61d6b40e-0997-4ce0-b723-bb0c2c4aafa2/6d903053-3984-4c67-85e7-78adeb103f69.png)

| Property                   | Value                                              |
|----------------------------|----------------------------------------------------|
| Ports                      | (4) 10 GbE RJ45, (1) 1 GbE RJ45                    |
| Switching capacity         | 82 Gbps (41 Gbps non-blocking, 61 Mpps)            |
| PoE                        | None (out); powered by PoE+ or USB-C               |
| Layer                      | Layer 2                                            |
| MAC table / VLANs / buffer | 16,000 / 1,000 / 1.5 MB                            |
| Power                      | 25 W max                                           |
| Form factor                | Compact desktop / wall — 135 × 185 × 32 mm, 1.2 kg |

### Flex 2.5G PoE (USW-Flex-2.5G-8-PoE)

Eight 2.5 GbE PoE++ ports plus a combination 10 GbE RJ45 / 10G SFP+ uplink, in a body that mounts on a desk, a wall, a DIN rail or magnetically to a rack post. It can be powered by PoE+++ from an upstream switch (yielding 76 W of downstream budget) or by a 210 W adapter (yielding 196 W). Note the 256-VLAN ceiling, shared across the Flex 2.5G and Ultra models.

- **Price:** $199 USD / £159 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/e9d97121-f363-4261-83db-5f74e093a304/dd55113e-93b2-40da-b10b-945ab4d609d0.png) · [angle](https://cdn.ecomm.ui.com/products/e9d97121-f363-4261-83db-5f74e093a304/fe2ed56d-fab4-4051-b3e9-100bb823afd4.png) · [rear](https://cdn.ecomm.ui.com/products/e9d97121-f363-4261-83db-5f74e093a304/4aac0ed4-5ba1-4128-9cfa-b4c453bcd25b.png)

| Property           | Value                                                                   |
|--------------------|-------------------------------------------------------------------------|
| Ports              | (8) 2.5 GbE RJ45 PoE++, (1) 10 GbE RJ45 (PoE+++ input), (1) 10G SFP+    |
| Switching capacity | 60 Gbps (30 Gbps non-blocking, 45 Mpps)                                 |
| PoE                | Up to PoE++; 196 W (210 W adapter) / 76 W (PoE+++ input) / 46 W (PoE++) |
| Layer              | Layer 2                                                                 |
| MAC table / VLANs  | 4,000 / 256                                                             |
| Power              | 14–17 W excl. PoE; 210 W incl. PoE                                      |
| Form factor        | Desktop, wall, DIN, magnetic — 212.9 × 99.4 × 33.5 mm, 567 g            |

### Flex 2.5G (USW-Flex-2.5G-8)

The PoE-free sibling: same eight 2.5 GbE ports and combination 10 GbE / 10G SFP+ uplink, powered by USB-C or PoE+, drawing 14 W.

- **Price:** $159 USD / £125 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/1d18e099-3be0-4f49-8126-5ade125543b5/19c0d063-e56d-4fbe-958f-b6209eafc8cb.png) · [angle](https://cdn.ecomm.ui.com/products/1d18e099-3be0-4f49-8126-5ade125543b5/f8ae2e00-dce6-47f8-a822-47f86dae3763.png) · [rear](https://cdn.ecomm.ui.com/products/1d18e099-3be0-4f49-8126-5ade125543b5/4c0c1394-8a71-4a3e-9a90-96b3390e029a.png)

| Property           | Value                                                      |
|--------------------|------------------------------------------------------------|
| Ports              | (8) 2.5 GbE RJ45, (1) 10 GbE RJ45, (1) 10G SFP+            |
| Switching capacity | 60 Gbps (30 Gbps non-blocking, 45 Mpps)                    |
| PoE                | None (out); powered by USB-C or PoE+                       |
| Layer              | Layer 2                                                    |
| MAC table / VLANs  | 4,000 / 256                                                |
| Power              | 14 W max                                                   |
| Form factor        | Desktop, wall, DIN, magnetic — 212.9 × 76 × 33.5 mm, 395 g |

### Lite 16 PoE (USW-Lite-16-PoE)

Sixteen gigabit ports with PoE+ on eight, no SFP uplinks, in a fanless wall-mountable body a fraction the width of a rack switch. A 45 W budget and a 32 Gbps fabric.

- **Price:** $199 USD / £159 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/e726eace-a772-4f12-bfad-c68baf20e51f/9ecfc657-5e31-4135-89b5-46b3537b35fc.png) · [angle](https://cdn.ecomm.ui.com/products/e726eace-a772-4f12-bfad-c68baf20e51f/41fb0696-fca8-4916-989c-3560bac061a3.png) · [rear](https://cdn.ecomm.ui.com/products/e726eace-a772-4f12-bfad-c68baf20e51f/6b0f85e8-0a52-434d-9761-f260d265b8bd.png)

| Property                   | Value                                              |
|----------------------------|----------------------------------------------------|
| Ports                      | (16) 1 GbE RJ45 (8 PoE+)                           |
| Switching capacity         | 32 Gbps (16 Gbps non-blocking, 24 Mpps)            |
| PoE                        | Up to PoE+; 45 W                                   |
| Layer                      | Layer 2                                            |
| MAC table / VLANs / buffer | 8,000 / 1,000 / 0.5 MB                             |
| Power                      | 15 W excl. PoE; 60 W incl. PoE; fanless            |
| Form factor                | Compact desktop / wall — 192 × 185 × 44 mm, 1.2 kg |

### Ultra, Ultra 60W and Ultra 210W (USW-Ultra)

One switch sold three ways. All three are the same eight-port gigabit unit — seven PoE+ outputs and one PoE++ input, magnetically or wall mountable, 16 Gbps of switching — and differ only in the power supply in the box. The base **Ultra** ships without an adapter and expects to be powered over PoE++ (42 W of downstream budget); **Ultra 60W** includes a 60 W adapter (52 W budget); **Ultra 210W** includes a 210 W adapter (202 W budget), which is what you want if it is feeding cameras or PoE+ access points. Like the Flex 2.5G models it is capped at 256 VLANs and a 4,000-entry MAC table.

- **Price:** Ultra $129 / £79 · Ultra 60W $159 / £109 · Ultra 210W $199 / £159
- **Images:** [Ultra](https://cdn.ecomm.ui.com/products/d4e5408e-e2b4-4b32-b9d6-efdde2bbaaf3/69808d5d-ba9c-4198-b3e9-749bbb02582c.png) · [Ultra 60W](https://cdn.ecomm.ui.com/products/d1af5d9b-b74c-4881-99af-033b71ed1590/f80567f8-ba09-4a75-982d-6fd636623492.png) · [Ultra 210W](https://cdn.ecomm.ui.com/products/04d0418c-989e-4316-b143-e7a89c90b72d/fbfadf63-3eeb-4173-8340-3f6119fd7d72.png)

| Property           | Value                                                                      |
|--------------------|----------------------------------------------------------------------------|
| Ports              | (8) 1 GbE RJ45 (7 PoE+ output, 1 PoE++ input)                              |
| Switching capacity | 16 Gbps (8 Gbps non-blocking, 12 Mpps)                                     |
| PoE budget         | 42 W (PoE++ input or 60 W PoE adapter) / 52 W (60 W AC) / 202 W (210 W AC) |
| Layer              | Layer 2                                                                    |
| MAC table / VLANs  | 4,000 / 256                                                                |
| Power              | 8–9 W excl. PoE; up to 210 W incl. PoE                                     |
| Form factor        | Desktop, wall, magnetic — 203 × 76 × 33 mm, 320 g                          |

### Lite 8 PoE (USW-Lite-8-PoE)

Eight gigabit ports with PoE+ on four and a 52 W budget from an included external adapter. Fanless, wall-mountable, and the cheapest way to power a pair of access points under UniFi management.

- **Price:** $109 USD / £85 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/75c44878-4e73-446e-8e86-f207db6b2b7c/53b8b06b-69c7-424f-bb81-2f8405356c65.png) · [angle](https://cdn.ecomm.ui.com/products/75c44878-4e73-446e-8e86-f207db6b2b7c/ec62bac5-a997-4f76-b8ca-037558f12e0a.png) · [rear](https://cdn.ecomm.ui.com/products/75c44878-4e73-446e-8e86-f207db6b2b7c/53c6cf52-66c0-41dd-9fcb-32667f60d940.png)

| Property                   | Value                                                  |
|----------------------------|--------------------------------------------------------|
| Ports                      | (8) 1 GbE RJ45 (4 PoE+)                                |
| Switching capacity         | 16 Gbps (8 Gbps non-blocking, 12 Mpps)                 |
| PoE                        | Up to PoE+; 52 W                                       |
| Layer                      | Layer 2                                                |
| MAC table / VLANs / buffer | 8,000 / 1,000 / 0.5 MB                                 |
| Power                      | 8 W excl. PoE; 60 W incl. PoE; fanless                 |
| Form factor                | Compact desktop / wall — 99.6 × 163.7 × 31.7 mm, 295 g |

### Flex (USW-Flex)

A five-port gigabit switch rated for outdoor as well as indoor use, itself powered by PoE++ on port 1 and passing up to 46 W down to four PoE+ ports. At 230 g it is the switch you put in a soffit, on a pole or inside a camera housing. Ubiquiti sells two weatherproof enclosures for it — the **Flex Utility** ($49 / £38) and the larger **Flex Utility Pro** ($59 / £46), which are accessories rather than switches.

- **Price:** $99 USD / £79 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/c9a07d37-b390-4a5b-89c5-3cdab8e011c7/4bcd3c2b-a8b1-4be1-baab-deda172291cd.png) · [angle](https://cdn.ecomm.ui.com/products/c9a07d37-b390-4a5b-89c5-3cdab8e011c7/41f8774d-6a46-4110-932e-20b3e6f96f77.png) · [rear](https://cdn.ecomm.ui.com/products/c9a07d37-b390-4a5b-89c5-3cdab8e011c7/6ec97d45-3b19-490b-a7a1-8638c4473df5.png)

| Property           | Value                                                              |
|--------------------|--------------------------------------------------------------------|
| Ports              | (5) 1 GbE RJ45 (4 PoE+ output, 1 PoE++ input)                      |
| Switching capacity | 10 Gbps (5 Gbps non-blocking, 7 Mpps)                              |
| PoE                | Up to PoE+; 46 W (PoE++/60 W input) / 20 W (PoE+) / 8 W (PoE)      |
| Layer              | Layer 2                                                            |
| MAC table / VLANs  | 2,000 / 1,000                                                      |
| Power              | 5 W excl. PoE; 51 W incl. PoE                                      |
| Form factor        | Desktop, wall, pole; indoor/outdoor — 122.5 × 107.1 × 28 mm, 230 g |

### Flex Mini 2.5G (USW-Flex-2.5G-5)

Five 2.5 GbE ports in a palm-sized body powered by USB-C or PoE, drawing 5 W. No PoE output. For $49 it is the cheapest managed multi-gig switch on the market from any major vendor.

- **Price:** $49 USD / £39 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/50830d51-4d7e-47ea-92f4-11043d3d664f/c956d05e-4351-46ba-b71e-afaafa3f1144.png) · [angle](https://cdn.ecomm.ui.com/products/50830d51-4d7e-47ea-92f4-11043d3d664f/e8313dc6-212a-4309-b798-13aad3380c93.png) · [rear](https://cdn.ecomm.ui.com/products/50830d51-4d7e-47ea-92f4-11043d3d664f/2a2a8da0-2e72-4aac-9961-c44ca7d9556f.png)

| Property           | Value                                         |
|--------------------|-----------------------------------------------|
| Ports              | (5) 2.5 GbE RJ45                              |
| Switching capacity | 25 Gbps (12.5 Gbps non-blocking, 19 Mpps)     |
| PoE                | None (out); powered by USB-C or PoE           |
| Layer              | Layer 2                                       |
| MAC table / VLANs  | 4,000 / 256                                   |
| Power              | 5 W (USB-C) / 6.4 W (PoE)                     |
| Form factor        | Compact desktop — 117.1 × 90 × 21.2 mm, 206 g |

### Flex Mini (USW-Flex-Mini)

The original five-port gigabit pocket switch, powered by USB-C or PoE at 2.5 W. Still the cheapest managed switch UniFi sells and still the standard answer for adding a few wired ports behind a desk or TV.

- **Price:** $29 USD / £23 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/5a176b22-af34-40f2-820c-958610df1825/19394e07-5146-4f8c-b72d-7fdbdf679c97.png) · [angle](https://cdn.ecomm.ui.com/products/5a176b22-af34-40f2-820c-958610df1825/5f495f89-3eb9-4092-8070-e6c61b02d3c3.png) · [rear](https://cdn.ecomm.ui.com/products/5a176b22-af34-40f2-820c-958610df1825/ca87552f-df02-4b82-a56c-a2c655e8230f.png)

| Property           | Value                                     |
|--------------------|-------------------------------------------|
| Ports              | (5) 1 GbE RJ45                            |
| Switching capacity | 10 Gbps (5 Gbps non-blocking, 7 Mpps)     |
| PoE                | None (out); powered by USB-C or PoE       |
| Layer              | Layer 2                                   |
| MAC table / VLANs  | 2,000 / 1,000                             |
| Power              | 2.5 W max                                 |
| Form factor        | Compact desktop — 107 × 70 × 21 mm, 150 g |

## WAN Switches

A two-model family with one specific job: sitting between a single ISP handoff and a pair of UniFi gateways running Shadow Mode high availability, so both gateways see the same WAN without the ISP needing to hand out two circuits. Both have dual AC inputs — the switch itself must not be the single point of failure it was introduced to eliminate.

### WAN Switch (USW-WAN)

Three 10G SFP+ ports plus a gigabit management port, for fibre or DAC-attached gateways. Requires UniFi Network 9.1.119 or later.

- **Price:** $249 USD / £199 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/280f3aca-e9e2-4e8e-9fd2-41349966e754/aae0726d-c438-40fa-8fb6-d71d1fdd0eda.png) · [angle](https://cdn.ecomm.ui.com/products/280f3aca-e9e2-4e8e-9fd2-41349966e754/988a37ea-5dc7-406d-88f0-157463c54125.png) · [rear](https://cdn.ecomm.ui.com/products/280f3aca-e9e2-4e8e-9fd2-41349966e754/8bd49617-bbaf-4eba-8e47-f4a0768e0f4b.png)

| Property           | Value                                                |
|--------------------|------------------------------------------------------|
| Ports              | (3) 10G SFP+, (1) 1 GbE RJ45                         |
| Switching capacity | 20 Gbps (10 Gbps non-blocking, 15 Mpps)              |
| PoE                | None                                                 |
| Purpose            | Shadow Mode HA gateway pair to single ISP handoff    |
| Redundancy         | (2) AC inputs, (2) internal 36 W supplies            |
| Power              | 13 W max                                             |
| Form factor        | 1U rack (half-depth) — 442.4 × 120 × 43.7 mm, 2.2 kg |

### WAN Switch RJ45 (USW-WAN-RJ45)

The copper equivalent: three 10 GbE RJ45 ports plus a gigabit management port, for gateways and ISP handoffs on multi-gig copper. Requires UniFi Network 9.1.120 or later.

- **Price:** $249 USD / £199 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/c3e77652-db6d-41d5-882c-55a2da9b37f7/020638c1-490d-48e7-96e6-580ae3561e9c.png) · [angle](https://cdn.ecomm.ui.com/products/c3e77652-db6d-41d5-882c-55a2da9b37f7/1fa31692-8afb-437f-b606-f627402540a7.png) · [rear](https://cdn.ecomm.ui.com/products/c3e77652-db6d-41d5-882c-55a2da9b37f7/7e0b2ed1-85d3-452b-8af0-66de9e239422.png)

| Property           | Value                                                |
|--------------------|------------------------------------------------------|
| Ports              | (3) 10 GbE RJ45, (1) 1 GbE RJ45                      |
| Switching capacity | 20 Gbps (10 Gbps non-blocking, 15 Mpps)              |
| PoE                | None                                                 |
| Purpose            | Shadow Mode HA gateway pair to single ISP handoff    |
| Redundancy         | (2) AC inputs, (2) internal 36 W supplies            |
| Power              | 18 W max                                             |
| Form factor        | 1U rack (half-depth) — 442.4 × 120 × 43.7 mm, 2.3 kg |

## Summary

| Model                         | SKU                    | Family      | USD    | GBP    | Ports                                | Fastest port | PoE tier | PoE budget | Switching cap. | Non-blocking | MAC     | Layer | Form factor       |
|-------------------------------|------------------------|-------------|--------|--------|--------------------------------------|--------------|----------|------------|----------------|--------------|---------|-------|-------------------|
| Enterprise Campus Aggregation | ECS-Aggregation        | Aggregation | $3,999 | £3,180 | (48) 25G SFP28, (6) 100G QSFP28      | 100G         | —        | —          | 3.6 Tbps       | 1.8 Tbps     | 128,000 | L3    | 1U full-depth     |
| Enterprise Campus 48S PoE     | ECS-48S-PoE            | Enterprise  | $3,999 | £3,180 | 32×2.5G, 16×10G, 4×25G, 2×100G stack | 100G         | PoE+++   | 2,150 W    | 680 Gbps       | 340 Gbps     | 32,000  | L3    | 1U full-depth     |
| Enterprise A/V XG 24 PoE      | EAV-XG-24-PoE          | Enterprise  | $3,999 | £3,180 | (24) 10 GbE, (4) 100G QSFP28         | 100G         | PoE+++   | 2,150 W    | 1,280 Gbps     | 640 Gbps     | 32,000  | L3    | 1U full-depth     |
| Enterprise Campus 48 PoE      | ECS-48-PoE             | Enterprise  | $3,499 | £2,785 | 16×2.5G, 32×10G, 4×25G SFP28         | 25G          | PoE+++   | 2,150 W    | 920 Gbps       | 460 Gbps     | 32,000  | L3    | 1U full-depth     |
| Enterprise Campus 24S PoE     | ECS-24S-PoE            | Enterprise  | $2,999 | £2,385 | 8×2.5G, 16×10G, 4×25G, 2×100G stack  | 100G         | PoE+++   | 1,050 W    | 460 Gbps       | 230 Gbps     | 32,000  | L3    | 1U full-depth     |
| Enterprise A/V Fiber          | EAV-Fiber              | Enterprise  | $2,999 | £2,385 | (20) 10G SFP+, (2) 100G QSFP28       | 100G         | —        | —          | 800 Gbps       | 400 Gbps     | 32,000  | L3    | 1U full-depth     |
| Enterprise Campus 24 PoE      | ECS-24-PoE             | Enterprise  | $2,499 | £1,989 | 8×2.5G, 16×10G, 2×25G SFP28          | 25G          | PoE+++   | 1,050 W    | 460 Gbps       | 230 Gbps     | 32,000  | L3    | 1U full-depth     |
| Pro XG 48 PoE                 | USW-Pro-XG-48-PoE      | Pro XG      | $2,499 | £1,989 | 16×2.5G, 32×10G, 4×25G SFP28         | 25G          | PoE+++   | 1,080 W    | 920 Gbps       | 460 Gbps     | 32,000  | L3    | 1U rack           |
| Pro XG Aggregation            | USW-Pro-XG-Aggregation | Aggregation | $2,499 | £1,989 | (32) 25G SFP28                       | 25G          | —        | —          | 1.6 Tbps       | 800 Gbps     | 32,000  | L3    | 1U rack           |
| Pro XG 48                     | USW-Pro-XG-48          | Pro XG      | $1,999 | £1,590 | 16×2.5G, 32×10G, 4×25G SFP28         | 25G          | —        | —          | 920 Gbps       | 460 Gbps     | 32,000  | L3    | 1U rack           |
| Pro XG 24 PoE                 | USW-Pro-XG-24-PoE      | Pro XG      | $1,799 | £1,430 | 8×2.5G, 16×10G, 2×25G SFP28          | 25G          | PoE+++   | 720 W      | 460 Gbps       | 230 Gbps     | 32,000  | L3    | 1U rack           |
| Pro Max 48 PoE                | USW-Pro-Max-48-PoE     | Pro Max     | $1,299 | £1,035 | 32×1G, 16×2.5G, 4×10G SFP+           | 10G          | PoE++    | 720 W      | 224 Gbps       | 112 Gbps     | 32,000  | L3    | 1U rack           |
| Pro XG 24                     | USW-Pro-XG-24          | Pro XG      | $1,099 | £875   | 8×2.5G, 16×10G, 2×25G SFP28          | 25G          | —        | —          | 460 Gbps       | 230 Gbps     | 32,000  | L3    | 1U rack           |
| Pro 48 PoE                    | USW-Pro-48-PoE         | Pro         | $1,099 | £875   | 48×1G, 4×10G SFP+                    | 10G          | PoE++    | 600 W      | 176 Gbps       | 88 Gbps      | 16,000  | L3    | 1U rack           |
| Pro HD 24 PoE                 | USW-Pro-HD-24-PoE      | Pro HD      | $999   | £795   | 22×2.5G, 2×10G, 4×10G SFP+           | 10G          | PoE++    | 600 W      | 230 Gbps       | 115 Gbps     | 32,000  | L3    | 1U rack           |
| UPS PoE Switch                | USW-Mission-Critical   | Pro         | $999   | £799   | (9) 1 GbE                            | 1G           | PoE++    | 120 W      | 18 Gbps        | 9 Gbps       | 8,000   | L3    | 1U full-depth     |
| Hi-Capacity Aggregation       | USW-Pro-Aggregation    | Aggregation | $899   | £715   | (28) 10G SFP+, (4) 25G SFP28         | 25G          | —        | —          | 760 Gbps       | 380 Gbps     | 32,000  | L3    | 1U rack           |
| Pro Max 24 PoE                | USW-Pro-Max-24-PoE     | Pro Max     | $799   | £635   | 16×1G, 8×2.5G, 2×10G SFP+            | 10G          | PoE++    | 400 W      | 112 Gbps       | 56 Gbps      | 16,000  | L3    | 1U rack           |
| Pro XG 10 PoE                 | USW-Pro-XG-10-PoE      | Pro XG      | $699   | £555   | (10) 10 GbE, (2) 10G SFP+            | 10G          | PoE+++   | 400 W      | 240 Gbps       | 120 Gbps     | 32,000  | L3    | 1U rack           |
| Pro 24 PoE                    | USW-Pro-24-PoE         | Pro         | $699   | £555   | 24×1G, 2×10G SFP+                    | 10G          | PoE++    | 400 W      | 88 Gbps        | 44 Gbps      | 16,000  | L3    | 1U rack           |
| Pro Max 48                    | USW-Pro-Max-48         | Pro Max     | $649   | £515   | 32×1G, 16×2.5G, 4×10G SFP+           | 10G          | —        | —          | 224 Gbps       | 112 Gbps     | 32,000  | L3    | 1U rack           |
| Pro HD 24                     | USW-Pro-HD-24          | Pro HD      | $599   | £475   | 22×2.5G, 2×10G, 4×10G SFP+           | 10G          | —        | —          | 230 Gbps       | 115 Gbps     | 32,000  | L3    | 1U rack           |
| Pro 48                        | USW-Pro-48             | Pro         | $599   | £475   | 48×1G, 4×10G SFP+                    | 10G          | —        | —          | 176 Gbps       | 88 Gbps      | 16,000  | L3    | 1U rack           |
| Standard 48 PoE               | USW-48-PoE             | Standard    | $589   | £469   | 48×1G, 4×1G SFP                      | 1G           | PoE+     | 195 W      | 104 Gbps       | 52 Gbps      | 16,000  | L2    | 1U rack           |
| Pro XG 8 PoE                  | USW-Pro-XG-8-PoE       | Pro XG      | $499   | £395   | (8) 10 GbE, (2) 10G SFP+             | 10G          | PoE++    | 155 W      | 200 Gbps       | 100 Gbps     | 32,000  | L3    | Desktop / wall    |
| Enterprise 8 PoE (Vintage)    | USW-Enterprise-8-PoE   | Utility     | $479   | £380   | (8) 2.5 GbE, (2) 10G SFP+            | 10G          | PoE+     | 120 W      | 80 Gbps        | 40 Gbps      | 16,000  | L3    | Desktop           |
| Pro Max 24                    | USW-Pro-Max-24         | Pro Max     | $449   | £355   | 16×1G, 8×2.5G, 2×10G SFP+            | 10G          | —        | —          | 112 Gbps       | 56 Gbps      | 16,000  | L3    | 1U rack           |
| Standard 48                   | USW-48                 | Standard    | $399   | £315   | 48×1G, 4×1G SFP                      | 1G           | —        | —          | 104 Gbps       | 52 Gbps      | 16,000  | L2    | 1U rack           |
| Pro Max 16 PoE                | USW-Pro-Max-16-PoE     | Pro Max     | $399   | £315   | 12×1G, 4×2.5G, 2×10G SFP+            | 10G          | PoE++    | 180 W      | 84 Gbps        | 42 Gbps      | 16,000  | L3    | 1U ½-width / wall |
| Pro 24                        | USW-Pro-24             | Pro         | $399   | £315   | 24×1G, 2×10G SFP+                    | 10G          | —        | —          | 88 Gbps        | 44 Gbps      | 16,000  | L3    | 1U rack           |
| Standard 24 PoE               | USW-24-PoE             | Standard    | $379   | £300   | 24×1G, 2×1G SFP                      | 1G           | PoE+     | 95 W       | 52 Gbps        | 26 Gbps      | 8,000   | L2    | 1U rack           |
| Pro 8 PoE                     | USW-Pro-8-PoE          | Utility     | $349   | £239   | (8) 1 GbE, (2) 10G SFP+              | 10G          | PoE++    | 120 W      | 56 Gbps        | 28 Gbps      | 16,000  | L3    | Desktop / wall    |
| Standard 16 PoE               | USW-16-PoE             | Standard    | $299   | £239   | 16×1G, 2×1G SFP                      | 1G           | PoE+     | 42 W       | 36 Gbps        | 18 Gbps      | 8,000   | L2    | 1U rack           |
| Flex 10 GbE                   | USW-Flex-XG            | Utility     | $299   | £229   | (4) 10 GbE, (1) 1 GbE                | 10G          | —        | —          | 82 Gbps        | 41 Gbps      | 16,000  | L2    | Desktop / wall    |
| Pro Max 16                    | USW-Pro-Max-16         | Pro Max     | $279   | £220   | 12×1G, 4×2.5G, 2×10G SFP+            | 10G          | —        | —          | 84 Gbps        | 42 Gbps      | 16,000  | L3    | 1U ½-width / wall |
| Aggregation                   | USW-Aggregation        | Aggregation | $269   | £215   | (8) 10G SFP+                         | 10G          | —        | —          | 160 Gbps       | 80 Gbps      | 16,000  | L2    | 1U half-depth     |
| WAN Switch                    | USW-WAN                | WAN         | $249   | £199   | (3) 10G SFP+, (1) 1 GbE              | 10G          | —        | —          | 20 Gbps        | 10 Gbps      | n/p     | L2    | 1U half-depth     |
| WAN Switch RJ45               | USW-WAN-RJ45           | WAN         | $249   | £199   | (3) 10 GbE, (1) 1 GbE                | 10G          | —        | —          | 20 Gbps        | 10 Gbps      | n/p     | L2    | 1U half-depth     |
| Standard 24                   | USW-24                 | Standard    | $225   | £179   | 24×1G, 2×1G SFP                      | 1G           | —        | —          | 52 Gbps        | 26 Gbps      | 8,000   | L2    | 1U rack           |
| Lite 16 PoE                   | USW-Lite-16-PoE        | Utility     | $199   | £159   | (16) 1 GbE                           | 1G           | PoE+     | 45 W       | 32 Gbps        | 16 Gbps      | 8,000   | L2    | Desktop / wall    |
| Flex 2.5G PoE                 | USW-Flex-2.5G-8-PoE    | Utility     | $199   | £159   | 8×2.5G, 1×10 GbE, 1×10G SFP+         | 10G          | PoE++    | 196 W      | 60 Gbps        | 30 Gbps      | 4,000   | L2    | Desktop / DIN     |
| Ultra 210W                    | USW-Ultra-210W         | Utility     | $199   | £159   | (8) 1 GbE                            | 1G           | PoE+     | 202 W      | 16 Gbps        | 8 Gbps       | 4,000   | L2    | Desktop / wall    |
| Ultra 60W                     | USW-Ultra-60W          | Utility     | $159   | £109   | (8) 1 GbE                            | 1G           | PoE+     | 52 W       | 16 Gbps        | 8 Gbps       | 4,000   | L2    | Desktop / wall    |
| Flex 2.5G                     | USW-Flex-2.5G-8        | Utility     | $159   | £125   | 8×2.5G, 1×10 GbE, 1×10G SFP+         | 10G          | —        | —          | 60 Gbps        | 30 Gbps      | 4,000   | L2    | Desktop / DIN     |
| Ultra                         | USW-Ultra              | Utility     | $129   | £79    | (8) 1 GbE                            | 1G           | PoE+     | 42 W       | 16 Gbps        | 8 Gbps       | 4,000   | L2    | Desktop / wall    |
| Lite 8 PoE                    | USW-Lite-8-PoE         | Utility     | $109   | £85    | (8) 1 GbE                            | 1G           | PoE+     | 52 W       | 16 Gbps        | 8 Gbps       | 8,000   | L2    | Desktop / wall    |
| Flex                          | USW-Flex               | Utility     | $99    | £79    | (5) 1 GbE                            | 1G           | PoE+     | 46 W       | 10 Gbps        | 5 Gbps       | 2,000   | L2    | Desktop / outdoor |
| Flex Mini 2.5G                | USW-Flex-2.5G-5        | Utility     | $49    | £39    | (5) 2.5 GbE                          | 2.5G         | —        | —          | 25 Gbps        | 12.5 Gbps    | 4,000   | L2    | Compact desktop   |
| Flex Mini                     | USW-Flex-Mini          | Utility     | $29    | £23    | (5) 1 GbE                            | 1G           | —        | —          | 10 Gbps        | 5 Gbps       | 2,000   | L2    | Compact desktop   |

*n/p = not published. PoE budgets for the Enterprise models are quoted in shared mode at 200–240 V; redundant-mode budgets are roughly half. The Flex Utility ($49 / £38) and Flex Utility Pro ($59 / £46) are weatherproof enclosures, not switches.*

### What the table says

**Ports, not silicon, drive the price.** Every switch in the Pro XG family shares the same feature set — Layer 3, 32,000 MAC entries, 8 MB buffer, 512 static routes. The $2,499 Pro XG 48 PoE and the $499 Pro XG 8 PoE differ almost entirely in port count and PoE budget. The same is true within Pro Max and within Standard. Once you have chosen a family, count ports and PoE watts; nothing else changes much.

**PoE is the single most expensive feature.** Across every matched pair in the range, adding PoE costs 40–100% of the switch's price: Pro Max 48 $649 → Pro Max 48 PoE $1,299; Pro HD 24 $599 → $999; Standard 24 $225 → $379. If the endpoints have their own power, the non-PoE variant is always the better value, and Ubiquiti's own PoE injectors are cheaper than the difference for small counts.

**The 2.5 GbE inflection point sits around $500.** Below it you are buying gigabit copper with 10G uplinks; above it, multi-gig copper. The Pro HD 24 at $599 is the cheapest way to get 22 ports of 2.5 GbE, and the Pro Max 16 at $279 the cheapest way to get any at all in a rack-mountable body. At the very bottom, the $49 Flex Mini 2.5G undercuts everything by an order of magnitude — the catalog has a genuine hole between $49 and $279.

**Non-blocking is not universal.** Most models publish non-blocking throughput exactly equal to their port bandwidth, but the two stackable Enterprise Campus models do not: their 100G QSFP28 stack links are excluded from the published figures. And the ECS-48S PoE is not a superset of the ECS-48 PoE — it trades sixteen 10 GbE ports for sixteen 2.5 GbE ones, which is why it publishes 340 Gbps against the 48's 460 Gbps.

**Two ceilings bite earlier than expected.** The 256-VLAN and 4,000-entry MAC limits on the Ultra and Flex 2.5G models are fine for a branch or a home but will not carry a segmented multi-tenant network. And the 0.5 MB packet buffer shared by the Standard series and the Lite models is the reason those switches behave poorly under video or backup bursts, regardless of how much headroom their port speeds suggest.

**Redundancy is only available at the top and in one odd corner.** True dual hot-swap PSU redundancy exists solely on the six Enterprise Campus and Audio/Video models. Everything from the Pro family upward can take DC backup from a USP-RPS, which covers supply failure but not module failure. Stacking exists on two SKUs, MC-LAG on one, and battery backup on one — the UPS PoE Switch, which is a nine-port gigabit switch with enterprise-grade availability and entry-level switching, and exists purely so that cameras and door readers survive a power cut.

**Sources:** [UniFi Switching](https://ui.com/us/en/switching) · [Ubiquiti Tech Specs — Switching](https://techspecs.ui.com/unifi/switching) · [Ubiquiti Store US — All Switching](https://store.ui.com/us/en/category/all-switching) · [Ubiquiti Store UK — All Switching](https://uk.store.ui.com/uk/en/category/all-switching) · [Ubiquiti Help Center: PoE Availability and Modes](https://help.ui.com/hc/en-us/articles/115000263008-PoE-Availability-and-Modes) · [Ubiquiti Help Center: Intro to Power over Ethernet](https://help.ui.com/hc/en-us/articles/360015399993-Intro-to-Networking-Power-Over-Ethernet-PoE)
