---
prompt: |-
    Research all the Access Points offered under the **Unifi** brand.

    - identify the key properties/metrics which these Access Points have that are meaningful in terms of their capability and scalability
    - organize the Access Points into categories (UDM, ...) as H2 headings 
    - add an H3 heading for each router product:
        - describe the product
        - provide a price point in both USD and GBP
        - provide a link to one or more images of the product
        - provide the properties/metrics which characterize this product
    - add an H2 "Summary" section which includes a comparison table of the Access Points offered
    
    Once the document's body has been written in prose style, you must add the following frontmatter properties to the document as well:
        
    - `last_updated` as \"{{ctx.today}}\"
    - `access_points` as a list of each switch (AP product name is key, key attributes are listed underneath as key/value pairs)
    - `agent` as '{{ctx.agent}}/{{ctx.model}}'
    
    Make sure this document is saved with these Frontmatter properties included.

    Note: prefer US english for this content
last_updated: 2026-08-30
hash: 422e6048d0532249-b6c6aa4c87d5292c
---
# UniFi Access Points

Ubiquiti sells roughly thirty-five access points under the UniFi brand, from a $89 palm-sized WiFi 5 puck to a $1,999 twelve-stream stadium radio. Every one of them is adopted and managed by the same UniFi Network application, every one is NDAA-compliant, and none of them carries a per-AP, per-seat or per-controller license. What separates them is radio count, spatial streams, uplink speed, PoE class and antenna pattern — and, increasingly, whether the box has a *spare* radio that does nothing but watch the spectrum.

This document covers every access point currently listed on [store.ui.com](https://store.ui.com), grouped by product family, with the specifications that actually determine what a given AP can carry.

> **Scope note.** Ubiquiti files several non-AP products under the same "WiFi" tech-specs category: the Device Bridge and Building Bridge families (point-to-point/point-to-multipoint links), AirWire, and the UniFi Travel Routers. Those are transport and gateway products, not access points, and are excluded here — the travel routers are covered in [UniFi Routers](./routers.md).

## How to Read the Numbers

Ubiquiti publishes about thirty specifications per access point. Roughly ten of them change a buying decision.

**Spatial streams are the headline, and the per-band split is the substance.** Ubiquiti markets "6-stream", "8-stream", "10-stream", "12-stream" — but the total is a sum across radios, and where those streams land matters more than how many there are. The U7 Pro and U7 Pro XG are both 6-stream (2×2 + 2×2 + 2×2). The U7 Pro Max and U7 Pro XGS are both 8-stream, and every extra stream went to 5 GHz (2×2 + 4×4 + 2×2), which is where most real client traffic still lives. The E7 is 10-stream because it runs 4×4 on *both* 5 and 6 GHz. Count the streams on the band your clients actually use.

**The uplink port is the honest throughput ceiling, and it is frequently lower than the radios.** A U7 Pro Max advertises 8.6 Gbps on 5 GHz and 5.8 Gbps on 6 GHz, then hands all of it to a single 2.5 GbE port. That is not a defect — no realistic client mix saturates a 2.5G uplink — but it does mean the *entire* practical difference between a U7 Pro Max ($279) and a U7 Pro XGS ($299) is the port: 2.5 GbE versus 10 GbE, and PoE+ versus PoE++. Same streams, same coverage, same 500+ clients, same scanning radio. Twenty dollars buys you headroom, not radios.

**Maximum client count is the scalability number.** It runs 200+ (U7 Lite, U7 Mesh, In-Wall HD) → 250+ → 300+ → 500+ (U7 Pro Max/XGS) → 600+ (U6 Enterprise) → 1000+ (E7, E7 Campus) → 1500+ (E7 Audience, BaseStation XG). Like the gateway figures, exceeding it degrades rather than fails. Note the ordering surprise: the WiFi 6E **U6 Enterprise** carries 600+ clients, more than any U7-series AP.

**Coverage area is a marketing figure shaped by antenna pattern, not by power.** 465 m² (5,000 ft²) appears on the U7 Outdoor, U7 Pro Outdoor, all three E7 Campus/Audience models and the BaseStation XG — every one of which uses a *directional* antenna. Omnidirectional ceiling APs cluster at 115–185 m². A 465 m² AP does not cover five times the area of a 115 m² one; it covers a narrower cone, further away. Read coverage together with antenna gain and beamwidth, never alone.

**PoE class is a switch-budget constraint, and it is the specification most often discovered too late.** UniFi APs span PoE (802.3af, 6.5–14 W), PoE+ (802.3at, 18.5–25 W) and PoE++ (802.3bt, 29–51 W). The E7 Audience draws 51 W; twelve of them need a 612 W budget before you count anything else. The In-Wall models add a second constraint: they will run on the lower class, but their downstream PoE *output* port only works if you feed them the higher one.

**A dedicated spectral-scanning radio is the real premium tier marker.** The U7 Pro Max, U7 Pro XGS and the entire E7 family carry a fourth radio that does nothing but analyse the RF environment, plus Zero-Wait DFS — the AP resumes on a DFS channel immediately after radar activity instead of dropping clients for the mandatory dwell. The U7 Pro and U7 Pro XG do *not* have it, despite being adjacent in name and price. This is the single most consequential undocumented difference in the lineup.

**AFC (Automatic Frequency Coordination) unlocks standard-power 6 GHz, and only two families have it.** With AFC the AP queries a coordination database and may raise 6 GHz transmit power by up to 6 dBm — roughly 25% more range on the band that otherwise reaches only about three-quarters as far as 5 GHz. Ubiquiti restricts it to the enterprise **E7** line and the **U7 Pro Outdoor**, and only in FCC/IC regulatory regions. The E7 was the first business-class AP shipped with it. Everything else runs 6 GHz at low power.

**MLO (Multi-Link Operation) is generation-wide, not model-specific.** Every WiFi 7 UniFi AP supports it on firmware 7.1.18+; it is enabled per-SSID. Whether it helps depends entirely on the client — an STR-capable client (Intel BE200, recent flagship phones) genuinely aggregates links, a cheaper one only gains latency stability.

**A redundant uplink port appears only on the E7 family.** E7, E7 Campus, E7 Campus Indoor, E7 Audience and E7 Audience Indoor all pair a 10 GbE primary with a 1 GbE failover port. No other UniFi AP has uplink redundancy; the second GbE port on an AC Pro or U6 Mesh Pro is a passthrough, not a failover.

**Weatherproofing has four distinct grades and they are not interchangeable.** IPX4 (AC Mesh, AC Mesh Pro — splash only), IPX5 (U6 Mesh), IPX6 (U7 Outdoor, U6 Mesh Pro, Swiss Army Knife, E7 Campus, U7 Pro Outdoor, U7 Mesh with the outdoor mount), and IP67/IP68 (U7 Pro Outdoor and E7 Campus *with* their cable-gland door kits; E7 Audience and BaseStation XG natively). The door kit is the difference between "rain" and "submersion", and on two models it is the difference between IPX6 and IP67 on the *same* hardware.

**Integrated switching and PoE passthrough define the In-Wall class.** The U7 In-Wall provides three 2.5 GbE ports with one PoE output; the U6 Enterprise In-Wall and U6 In-Wall provide a 2.5 GbE or GbE input plus four GbE outputs with one PoE output; the In-Wall HD offers five GbE ports; the AC In-Wall three. This is what makes them hospitality and desk-drop products rather than merely wall-mounted APs.

**Minimum UniFi Network version is an easily missed deployment constraint.** The Pro XG family requires Network 9.0.114 or later; the legacy AC models and In-Wall HD run on 6.0.45. Adopting a new AP onto an old controller silently fails.

**Two commercial notes.** First, the software is unlicensed — there is no controller subscription, no AP license and no feature paywall; the only paid add-ons are UI Care hardware coverage. Second, several legacy and mesh SKUs are perpetually out of stock; availability is noted per product below.

> **Pricing basis.** All prices below are Ubiquiti's own list prices on [store.ui.com](https://store.ui.com) (USD) and [uk.store.ui.com](https://uk.store.ui.com) (GBP) as of **30 August 2026**. The US store separately displays a higher "surcharge included" figure; the base list price is quoted here. Regional taxes are added at checkout.

## Enterprise WiFi 7 (E7)

Ubiquiti's top tier and the only family with AFC, a redundant uplink port and a dedicated spectral analyser as standard. All five models take PoE++ and a 10 GbE primary uplink. The "Indoor" variants are electrically identical to their siblings but drop the weatherproofing in exchange for the expanded indoor 6 GHz spectrum allocation.

### E7 Audience

The largest access point Ubiquiti makes, and the only one that omits 2.4 GHz entirely. Twelve streams split across three 4×4 radios — a low 6 GHz band, a high 6 GHz band, and 5 GHz — behind directional internal antennas selectable between 15 dBi at 50°×50° and 11 dBi at 90°×90°. It is a seat-bowl radio: rated for 1500+ clients over 465 m², IP68-sealed, and 3.3 kg before the mount. If your deployment does not involve pointing an antenna at a crowd from a catwalk, this is the wrong product.

- **Price:** $1,999 USD / £1,590 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/993e484e-4058-49eb-9cdb-4b5e89aec544/667cb8ca-38a5-4289-870d-05b230472fe5.png) · [angle](https://cdn.ecomm.ui.com/products/993e484e-4058-49eb-9cdb-4b5e89aec544/712f8478-37e0-42f9-821c-43ae09e23d6e.png) · [mounted](https://cdn.ecomm.ui.com/products/993e484e-4058-49eb-9cdb-4b5e89aec544/a264e56d-9160-42ad-b71f-31a3adf82a8d.png)

| Property        | Value                                                                                  |
|-----------------|----------------------------------------------------------------------------------------|
| WiFi generation | WiFi 7 (802.11be)                                                                      |
| Radios          | 6 GHz low 4×4, 6 GHz high 4×4, 5 GHz 4×4 — **no 2.4 GHz**                              |
| Spatial streams | 12                                                                                     |
| Max PHY rates   | 11.5 Gbps (6 GHz low, BW320) / 11.5 Gbps (6 GHz high, BW320) / 8.6 Gbps (5 GHz, BW240) |
| Uplink          | (1) 10 GbE RJ45 + (1) 1 GbE RJ45 redundant                                             |
| Power           | PoE++, 51 W max, 42.5–57 V DC                                                          |
| Coverage        | 465 m² (5,000 ft²)                                                                     |
| Max clients     | 1500+                                                                                  |
| Antennas        | Directional internal, selectable 15 dBi @ 50°×50° or 11 dBi @ 90°×90°                  |
| Environmental   | IP68, −40 to 60 °C                                                                     |
| Dimensions      | 474.1 × 265.5 × 42.2 mm (18.7 × 10.5 × 1.7")                                           |
| Weight          | 3.3 kg (4.3 kg with mount)                                                             |

### E7 Audience Indoor

The same twelve-stream radio platform as the E7 Audience, re-certified for indoor 6 GHz operation and shipped without the outdoor sealing. Slightly narrower and a hundred grams lighter. For arenas, lecture halls and conference centres where the AP hangs inside the envelope, this is the model to order — the outdoor variant buys nothing indoors.

- **Price:** $1,999 USD / not listed in the UK store
- **Images:** [front](https://cdn.ecomm.ui.com/products/dc297e03-3266-43ee-963f-01c2234657a1/2fd1fc2f-08c6-4029-9e54-c6a1f5ef51ac.png) · [angle](https://cdn.ecomm.ui.com/products/dc297e03-3266-43ee-963f-01c2234657a1/c17a8797-d727-46c5-92a5-e0425d23efaf.png) · [rear](https://cdn.ecomm.ui.com/products/dc297e03-3266-43ee-963f-01c2234657a1/16f5e890-9ac1-4efa-991f-d903f137fd64.png)

| Property        | Value                                                      |
|-----------------|------------------------------------------------------------|
| WiFi generation | WiFi 7 (802.11be)                                          |
| Radios          | 6 GHz low 4×4, 6 GHz high 4×4, 5 GHz 4×4 — no 2.4 GHz      |
| Spatial streams | 12                                                         |
| Max PHY rates   | 11.5 / 11.5 / 8.6 Gbps                                     |
| Uplink          | (1) 10 GbE RJ45 + (1) 1 GbE RJ45 redundant                 |
| Power           | PoE++, 51 W max, 42.5–57 V DC                              |
| Coverage        | 465 m² (5,000 ft²)                                         |
| Max clients     | 1500+                                                      |
| Antennas        | Directional internal, 15 dBi @ 50°×50° or 11 dBi @ 90°×90° |
| Dimensions      | 474.1 × 239.4 × 42.2 mm (18.7 × 9.4 × 1.7")                |
| Weight          | 3.2 kg (4.2 kg with mount)                                 |

### E7 Campus

A ten-stream tri-band WiFi 7 AP built for outdoor quads, parking structures and long corridors, with PRISM™ active RF filtering and an articulating mount for precise aiming. Unlike the omnidirectional E7, the Campus is directional — 12 dBi on 5 and 6 GHz, 9 dBi on 2.4 GHz — which is how it reaches 465 m² on the same silicon that the indoor E7 uses to cover 185 m². IPX6 as shipped, IP67 with the included waterproof door kit. Supports AFC for standard-power 6 GHz in FCC/IC regions.

- **Price:** $799 USD / £635 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/67bc019d-eb07-4427-8989-b15bfd43cb5f/a5ca92f1-1d8c-48cf-b60b-2859a9adee41.png) · [angle](https://cdn.ecomm.ui.com/products/67bc019d-eb07-4427-8989-b15bfd43cb5f/69448d6e-a7d0-4dab-bb10-2ab983821868.png) · [mount](https://cdn.ecomm.ui.com/products/67bc019d-eb07-4427-8989-b15bfd43cb5f/fdb315d0-a71f-4c0e-81e0-ec0fd60b0038.png)

| Property        | Value                                                                                        |
|-----------------|----------------------------------------------------------------------------------------------|
| WiFi generation | WiFi 7 (802.11be)                                                                            |
| Radios          | 6 GHz 4×4, 5 GHz 4×4, 2.4 GHz 2×2                                                            |
| Spatial streams | 10                                                                                           |
| Max PHY rates   | 11.5 Gbps (6 GHz, BW320) / 8.6 Gbps (5 GHz, BW240) / 688 Mbps (2.4 GHz, BW40)                |
| Uplink          | (1) 10 GbE RJ45 + (1) 1 GbE RJ45 redundant                                                   |
| Power           | PoE++, 44 W max, 42.5–57 V DC                                                                |
| Coverage        | 465 m² (5,000 ft²)                                                                           |
| Max clients     | 1000+                                                                                        |
| Antenna gain    | 12 dBi (6 GHz), 12 dBi (5 GHz), 9 dBi (2.4 GHz) — directional                                |
| Max TX power    | 30 dBm (5/6 GHz; up to 36 dBm EIRP with 6 GHz Extended Range, FCC/IC only), 23 dBm (2.4 GHz) |
| Environmental   | IPX6; IP67 with included waterproof door kit                                                 |
| Mounting        | Wall and pole (1–2.5" poles), articulating; optional VESA                                    |
| Dimensions      | 250 × 250 × 45.5 mm (9.8 × 9.8 × 1.8")                                                       |

### E7 Campus Indoor

The Campus radio and directional antenna array without the weatherproofing, certified for the expanded indoor 6 GHz spectrum. Identical streams, coverage and client capacity; a kilogram lighter. The right choice for warehouses, atria and long indoor spans where you want a directional beam but no ingress rating.

- **Price:** $799 USD / not listed in the UK store
- **Images:** [front](https://cdn.ecomm.ui.com/products/996cbaaa-198b-4f11-92a3-78ffea4c4787/f6b3c5c7-000a-4aff-9cb1-f31010ff86af.png) · [angle](https://cdn.ecomm.ui.com/products/996cbaaa-198b-4f11-92a3-78ffea4c4787/b9029523-44b1-4aac-af67-5b61afca0dd9.png) · [mount](https://cdn.ecomm.ui.com/products/996cbaaa-198b-4f11-92a3-78ffea4c4787/f1302684-5f02-4d13-a9b3-48e66527eac8.png)

| Property        | Value                                                                          |
|-----------------|--------------------------------------------------------------------------------|
| WiFi generation | WiFi 7 (802.11be)                                                              |
| Radios          | 6 GHz 4×4, 5 GHz 4×4, 2.4 GHz 2×2                                              |
| Spatial streams | 10                                                                             |
| Max PHY rates   | 11.5 / 8.6 Gbps / 688 Mbps                                                     |
| Uplink          | (1) 10 GbE RJ45 + (1) GbE RJ45 redundant                                       |
| Power           | PoE++, 44 W max, 42.5–57 V DC                                                  |
| Coverage        | 465 m² (5,000 ft²)                                                             |
| Max clients     | 1000+                                                                          |
| Antenna gain    | 12 dBi @ 90°×50° (6 GHz), 12 dBi @ 100°×55° (5 GHz), 9 dBi @ 90°×80° (2.4 GHz) |
| Dimensions      | 250 × 250 × 45.5 mm (9.8 × 9.8 × 1.8")                                         |
| Weight          | 2.2 kg (3.1 kg with mount)                                                     |

### E7

The indoor omnidirectional enterprise AP and the entry point to the E7 feature set — ten streams, dedicated spectral analyser, AFC, redundant GbE uplink and 1000+ clients for $499. At roughly 1.8× the price of a U7 Pro XGS it doubles the client ceiling and adds 4×4 on 6 GHz; it is the first Ubiquiti AP with official AFC support and remains the best-value entry into the tier.

- **Price:** $499 USD / £395 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/93ae773e-8969-4889-8591-2c227a31ac3f/7157e85e-2dba-47c1-9ac4-d7e27fd68742.png) · [angle](https://cdn.ecomm.ui.com/products/93ae773e-8969-4889-8591-2c227a31ac3f/db27ed43-0ef4-41bc-bbb3-ceca2a260975.png) · [rear](https://cdn.ecomm.ui.com/products/93ae773e-8969-4889-8591-2c227a31ac3f/e92508dc-b369-4553-bfe3-5003925a15f5.png)

| Property        | Value                                                                         |
|-----------------|-------------------------------------------------------------------------------|
| WiFi generation | WiFi 7 (802.11be)                                                             |
| Radios          | 6 GHz 4×4, 5 GHz 4×4, 2.4 GHz 2×2                                             |
| Spatial streams | 10                                                                            |
| Max PHY rates   | 11.5 Gbps (6 GHz, BW320) / 8.6 Gbps (5 GHz, BW240) / 688 Mbps (2.4 GHz, BW40) |
| Uplink          | (1) 10 GbE RJ45 + (1) 1 GbE RJ45 redundant                                    |
| Power           | PoE++, 43 W max                                                               |
| Coverage        | 185 m² (2,000 ft²)                                                            |
| Max clients     | 1000+                                                                         |
| Antenna gain    | 6 dBi (6 GHz), 6 dBi (5 GHz), 5 dBi (2.4 GHz) — omnidirectional               |
| Special         | Dedicated spectral analyser radio; AFC (FCC/IC)                               |
| Dimensions      | 250 × 250 × 43.5 mm (9.8 × 9.8 × 1.7")                                        |
| Weight          | 1.8 kg (4 lb)                                                                 |

## Flagship WiFi 7 with 10 GbE Uplink (Pro XG)

The Pro XG family came out of the E7 development programme — Ubiquiti reused the enterprise thermal design to fit a 10 GbE uplink into a ceiling puck 30% thinner than the U7 Pro. All three require UniFi Network 9.0.114 or later.

### U7 Pro XGS

The most capable non-enterprise AP Ubiquiti sells. Eight streams with 4×4 on 5 GHz, a dedicated spectral scanning radio, Zero-Wait DFS, real-time spectral analysis, and a single multi-gig port that negotiates 1/2.5/5/10 GbE. Its practical advantage over the $20-cheaper U7 Pro Max is entirely the uplink and the PoE++ requirement that comes with it; the radios are the same.

- **Price:** $299 USD / £239 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/1604d78c-6e51-4fe8-a8e5-0110cc332ba0/6b4a3bc9-9bc2-4a02-905b-3d621729a161.png) · [angle](https://cdn.ecomm.ui.com/products/1604d78c-6e51-4fe8-a8e5-0110cc332ba0/73d680d3-c54b-48fb-a5f5-51c31c97b5d6.png) · [rear](https://cdn.ecomm.ui.com/products/1604d78c-6e51-4fe8-a8e5-0110cc332ba0/fff39309-3e0c-4216-bec1-d734f047b1df.png)

| Property        | Value                                                                        |
|-----------------|------------------------------------------------------------------------------|
| WiFi generation | WiFi 7 (802.11be / ax / ac / n)                                              |
| Radios          | 6 GHz 2×2, 5 GHz 4×4, 2.4 GHz 2×2                                            |
| Spatial streams | 8                                                                            |
| Max PHY rates   | 5.8 Gbps (6 GHz, BW320) / 8.6 Gbps (5 GHz, BW240) / 688 Mbps (2.4 GHz, BW40) |
| Uplink          | (1) 10 GbE RJ45 (1/2.5/5/10 GbE)                                             |
| Power           | PoE++, 29 W max                                                              |
| Coverage        | 160 m² (1,750 ft²)                                                           |
| Max clients     | 500+                                                                         |
| Antenna gain    | 6 dBi (6 GHz), 6 dBi (5 GHz), 4 dBi (2.4 GHz)                                |
| Special         | Dedicated spectral scanning radio; Zero-Wait DFS; RadSec; Passpoint          |
| Dimensions      | ⌀215 × 32.5 mm (⌀8.5 × 1.3")                                                 |
| Weight          | 800 g (1.8 lb)                                                               |

### U7 Pro XG

The XGS minus the fourth radio and two 5 GHz streams: a 6-stream tri-band AP on a 10 GbE port for $199. It runs on PoE+ rather than PoE++, which matters when you are filling a switch. What it gives up against the similarly priced U7 Pro Wall and U7 Outdoor is nothing; what it gives up against the $80-dearer XGS is the spectral scanner, Zero-Wait DFS and half the 5 GHz capacity.

- **Price:** $199 USD / £159 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/a433b9e5-4dd1-4784-adf8-9d12bcba4c87/110382a7-62f1-431e-a967-a547b096d376.png) · [angle](https://cdn.ecomm.ui.com/products/a433b9e5-4dd1-4784-adf8-9d12bcba4c87/b5d0e179-270f-468d-beeb-9675e36d44b3.png) · [rear](https://cdn.ecomm.ui.com/products/a433b9e5-4dd1-4784-adf8-9d12bcba4c87/6f2ab691-905c-46f2-95ba-66683e7da839.png)

| Property        | Value                                                                        |
|-----------------|------------------------------------------------------------------------------|
| WiFi generation | WiFi 7 (802.11be)                                                            |
| Radios          | 6 GHz 2×2, 5 GHz 2×2, 2.4 GHz 2×2                                            |
| Spatial streams | 6                                                                            |
| Max PHY rates   | 5.8 Gbps (6 GHz, BW320) / 4.3 Gbps (5 GHz, BW240) / 688 Mbps (2.4 GHz, BW40) |
| Uplink          | (1) 10 GbE RJ45 (1/2.5/5/10 GbE)                                             |
| Power           | PoE+, 22 W max, 42.5–57 V DC                                                 |
| Coverage        | 140 m² (1,500 ft²)                                                           |
| Max clients     | 300+                                                                         |
| Antenna gain    | 6 dBi (6 GHz), 5 dBi (5 GHz), 4 dBi (2.4 GHz)                                |
| Dimensions      | ⌀206 × 32.5 mm (⌀8.1 × 1.3")                                                 |
| Weight          | 750 g (1.7 lb)                                                               |

### U7 Pro XG Wall

The wall-mounted U7 Pro XG: identical radios, identical 10 GbE multi-gig uplink, identical PoE+ draw, in a 155 × 108 mm rectangle intended for corridor and above-desk mounting. Note that unlike the In-Wall models it has no downstream switch ports — it is a wall-*mounted* AP, not an in-wall one, and it costs $80 more than the ceiling version for the form factor alone.

- **Price:** $279 USD / £220 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/c7f7e7f9-b357-44ea-8d30-1191140face2/d4a5da46-a9c6-47ec-96e1-63a6b2123fae.png) · [angle](https://cdn.ecomm.ui.com/products/c7f7e7f9-b357-44ea-8d30-1191140face2/6b5ff780-41ef-4a90-abce-de42f21a700d.png) · [mounted](https://cdn.ecomm.ui.com/products/c7f7e7f9-b357-44ea-8d30-1191140face2/c4f4520e-c188-4704-8968-d43056be9bd4.png)

| Property        | Value                                                        |
|-----------------|--------------------------------------------------------------|
| WiFi generation | WiFi 7 (802.11be)                                            |
| Radios          | 6 GHz 2×2, 5 GHz 2×2, 2.4 GHz 2×2                            |
| Spatial streams | 6                                                            |
| Max PHY rates   | 5.8 / 4.3 Gbps / 688 Mbps                                    |
| Ports           | (1) 10 GbE RJ45 uplink (1/2.5/5/10 GbE); no downstream ports |
| Power           | PoE+, 22 W max, 42.5–57 V DC                                 |
| Coverage        | 140 m² (1,500 ft²)                                           |
| Max clients     | 300+                                                         |
| Dimensions      | 155 × 108 × 33.5 mm (6.1 × 4.3 × 1.3")                       |
| Weight          | 505 g (1.1 lb)                                               |

## Mainstream WiFi 7 Ceiling APs (U7)

The volume products: 2.5 GbE uplinks, PoE or PoE+, ceiling mounts, and the price band where most deployments actually buy.

### U7 Pro Max

Eight streams with 4×4 on 5 GHz, a dedicated spectral scanning engine and Zero-Wait DFS — the full flagship radio package — on a 2.5 GbE port and PoE+. For any site whose switching is gigabit or 2.5G, this is the same AP as the U7 Pro XGS for $20 less and 4 W less draw. It is the sweet spot of the current lineup.

- **Price:** $279 USD / £220 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/350070a0-ae43-431b-b052-8e849c3b0a75/d56932e3-1769-4c51-941d-778215daecf9.png) · [angle](https://cdn.ecomm.ui.com/products/350070a0-ae43-431b-b052-8e849c3b0a75/fb717b70-4c94-45fd-9d61-92caec3bd026.png) · [rear](https://cdn.ecomm.ui.com/products/350070a0-ae43-431b-b052-8e849c3b0a75/ef42b249-5911-4a95-9a57-318d1025b909.png)

| Property        | Value                                                                        |
|-----------------|------------------------------------------------------------------------------|
| WiFi generation | WiFi 7 (802.11be)                                                            |
| Radios          | 6 GHz 2×2, 5 GHz 4×4, 2.4 GHz 2×2                                            |
| Spatial streams | 8                                                                            |
| Max PHY rates   | 5.8 Gbps (6 GHz, BW320) / 8.6 Gbps (5 GHz, BW240) / 688 Mbps (2.4 GHz, BW40) |
| Uplink          | (1) 1/2.5 GbE RJ45                                                           |
| Power           | PoE+, 25 W max, 44–57 V DC                                                   |
| Coverage        | 160 m² (1,750 ft²)                                                           |
| Max clients     | 500+                                                                         |
| Antenna gain    | 5.9 dBi (6 GHz), 6 dBi (5 GHz), 4 dBi (2.4 GHz)                              |
| Special         | Dedicated spectral scanning radio; Zero-Wait DFS                             |
| Dimensions      | ⌀206 × 46 mm (⌀8.1 × 1.8")                                                   |
| Weight          | 680 g (1.5 lb)                                                               |

### U7 Pro

Six streams, tri-band, 2.5 GbE, PoE+ — the default WiFi 7 ceiling AP for offices and large homes. No spectral scanning radio and no Zero-Wait DFS, which is the honest reason it sits $90 below the Pro Max rather than the stream count alone.

- **Price:** $189 USD / from £140 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/fa8dd4e4-36c8-4c79-a928-22c7bff2ce29/7cbd3c8a-42c9-46b9-b9f4-b08c9bc35437.png) · [angle](https://cdn.ecomm.ui.com/products/fa8dd4e4-36c8-4c79-a928-22c7bff2ce29/80c7b3d6-8db3-4978-9c17-fef6c8c7f4a8.png) · [rear](https://cdn.ecomm.ui.com/products/fa8dd4e4-36c8-4c79-a928-22c7bff2ce29/e86e2d52-0d88-4059-94e2-e9aff3052ac2.png)

| Property        | Value                                           |
|-----------------|-------------------------------------------------|
| WiFi generation | WiFi 7 (802.11be)                               |
| Radios          | 6 GHz 2×2, 5 GHz 2×2, 2.4 GHz 2×2               |
| Spatial streams | 6                                               |
| Max PHY rates   | 5.8 / 4.3 Gbps / 688 Mbps                       |
| Uplink          | (1) 1/2.5 GbE RJ45                              |
| Power           | PoE+, 21 W max, 44–57 V DC                      |
| Coverage        | 140 m² (1,500 ft²)                              |
| Max clients     | 300+                                            |
| Antenna gain    | 5.8 dBi (6 GHz), 6 dBi (5 GHz), 4 dBi (2.4 GHz) |
| Dimensions      | ⌀206 × 46 mm (⌀8.1 × 1.8")                      |
| Weight          | 680 g (1.5 lb)                                  |

### U7 Long-Range

A dual-band WiFi 7 AP — no 6 GHz radio — that spends its budget on a 3×3 5 GHz array and 6 dBi gain instead. It covers 160 m² on 14 W of plain 802.3af PoE, which makes it the cheapest way to light a large open area from an unbudgeted switch port. The absence of 6 GHz is deliberate: at range, the band contributes nothing.

- **Price:** $159 USD / £125 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/7455fa2b-3074-47a0-b82f-a2cd701d4a8f/86b360cd-b8b0-4bb9-b4cb-cc11bc1a76b7.png) · [angle](https://cdn.ecomm.ui.com/products/7455fa2b-3074-47a0-b82f-a2cd701d4a8f/1ade42c7-b70d-4af6-892d-5174c8d7f2d3.png) · [rear](https://cdn.ecomm.ui.com/products/7455fa2b-3074-47a0-b82f-a2cd701d4a8f/9d75fc29-da2a-4ddd-adb8-7229958e6461.png)

| Property        | Value                                              |
|-----------------|----------------------------------------------------|
| WiFi generation | WiFi 7 (802.11be), dual-band                       |
| Radios          | 5 GHz 3×3, 2.4 GHz 2×2 — no 6 GHz                  |
| Spatial streams | 5                                                  |
| Max PHY rates   | 4.3 Gbps (5 GHz, BW160) / 688 Mbps (2.4 GHz, BW40) |
| Uplink          | (1) 2.5 GbE RJ45                                   |
| Power           | PoE, 14 W max, 42.5–57 V DC                        |
| Coverage        | 160 m² (1,750 ft²)                                 |
| Max clients     | 300+                                               |
| Antenna gain    | 6 dBi (5 GHz), 4 dBi (2.4 GHz)                     |
| Dimensions      | ⌀175.7 × 43 mm (⌀6.9 × 1.7")                       |
| Weight          | 448 g (15.8 oz)                                    |

### U7 Lite

The $99 entry to WiFi 7: four streams, dual-band, 2.5 GbE uplink, 13 W on plain PoE. It is the only sub-$100 AP in the lineup with a multi-gig port, and at £79 in the UK it undercuts every WiFi 6 model there. Small rooms, dense ceiling grids, and anywhere the client mix is phones rather than workstations.

- **Price:** $99 USD / £79 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/253cc208-4b09-4b2e-9d1a-7aa1e8f93507/431f1c32-3cb2-4cda-b271-89ffd27f885a.png) · [angle](https://cdn.ecomm.ui.com/products/253cc208-4b09-4b2e-9d1a-7aa1e8f93507/5f17f552-9fde-4b92-9700-c258801e6a25.png) · [rear](https://cdn.ecomm.ui.com/products/253cc208-4b09-4b2e-9d1a-7aa1e8f93507/3184b20e-3da4-483d-a8d6-8eb05678ecf0.png)

| Property        | Value                                              |
|-----------------|----------------------------------------------------|
| WiFi generation | WiFi 7 (802.11be), dual-band                       |
| Radios          | 5 GHz 2×2, 2.4 GHz 2×2                             |
| Spatial streams | 4                                                  |
| Max PHY rates   | 4.3 Gbps (5 GHz, BW240) / 688 Mbps (2.4 GHz, BW40) |
| Uplink          | (1) 2.5 GbE RJ45                                   |
| Power           | PoE, 13 W max, 42.5–57 V DC                        |
| Coverage        | 115 m² (1,250 ft²)                                 |
| Max clients     | 200+                                               |
| Antenna gain    | 5 dBi (5 GHz), 4 dBi (2.4 GHz)                     |
| Dimensions      | ⌀171.5 × 33 mm (⌀6.8 × 1.3")                       |
| Weight          | 313 g (11 oz)                                      |

## WiFi 7 Wall and In-Wall

### U7 Pro Wall

The U7 Pro's radios in a wall-mount body aimed at home builders and residential integrators — six streams, tri-band, 2.5 GbE, PoE+. Mounts flush to a single-gang box or a low-voltage bracket, and covers a room from eye level rather than from the ceiling.

- **Price:** $199 USD / £159 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/7dacb4f6-b703-4154-9264-784f2eb0dbda/2486a933-70ec-4697-8c33-03e70ab9cbbc.png) · [angle](https://cdn.ecomm.ui.com/products/7dacb4f6-b703-4154-9264-784f2eb0dbda/58e233c0-6bf1-4199-b998-b2c63b806098.png) · [mounted](https://cdn.ecomm.ui.com/products/7dacb4f6-b703-4154-9264-784f2eb0dbda/736cbf6d-3b12-4de5-a7c0-8c7ac0c2aee3.png)

| Property        | Value                                |
|-----------------|--------------------------------------|
| WiFi generation | WiFi 7 (802.11be)                    |
| Radios          | 6 GHz 2×2, 5 GHz 2×2, 2.4 GHz 2×2    |
| Spatial streams | 6                                    |
| Max PHY rates   | 5.8 / 4.3 Gbps / 688 Mbps            |
| Ports           | (1) 1/2.5 GbE RJ45                   |
| Power           | PoE+, 22 W max, 44–57 V DC           |
| Coverage        | 140 m² (1,500 ft²)                   |
| Max clients     | 300+                                 |
| Dimensions      | 150 × 103 × 36 mm (5.9 × 4.1 × 1.4") |
| Weight          | 580 g (1.3 lb)                       |

### U7 In-Wall

The hospitality AP: dual-band WiFi 7 plus a built-in three-port 2.5 GbE switch with one PoE output, so a hotel room or dorm gets wireless, wired drops and a powered device from a single cable. The 13 W figure excludes whatever the PoE output passes through, and PoE+ input is required for that output to work at all.

- **Price:** $149 USD / £119 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/9ea6158e-bc26-4ac7-946a-55eca465b059/fae96dde-daa7-4cc3-98b8-ffb23a88d31a.png) · [angle](https://cdn.ecomm.ui.com/products/9ea6158e-bc26-4ac7-946a-55eca465b059/da7a93e7-f4d6-47de-8455-0f1666d30605.png) · [ports](https://cdn.ecomm.ui.com/products/9ea6158e-bc26-4ac7-946a-55eca465b059/dc214301-3faa-45a9-bcc2-022bf6fc7073.png)

| Property        | Value                                                             |
|-----------------|-------------------------------------------------------------------|
| WiFi generation | WiFi 7 (802.11be), dual-band                                      |
| Radios          | 5 GHz 2×2, 2.4 GHz 2×2                                            |
| Spatial streams | 4                                                                 |
| Max PHY rates   | 4.3 Gbps (5 GHz, BW240) / 688 Mbps (2.4 GHz, BW40)                |
| Ports           | (3) 2.5 GbE RJ45 with built-in switch; (1) PoE output             |
| Power           | PoE; PoE+ required for PoE output; 13 W max excluding passthrough |
| Coverage        | 115 m² (1,250 ft²)                                                |
| Max clients     | 200+                                                              |
| Dimensions      | 137 × 98.7 × 30.2 mm (5.4 × 3.9 × 1.2")                           |
| Weight          | 400 g (14.1 oz)                                                   |

## WiFi 7 Outdoor and Mesh

### U7 Pro Outdoor

The outdoor flagship below the E7 Campus: six streams, tri-band, an integrated directional "super antenna" reaching 11 dBi on 5 GHz and 10 dBi on 6 GHz, plus detachable external omnis for when you want a broader pattern instead. It is the only non-E7 UniFi AP with AFC, so in FCC/IC regions it runs standard-power 6 GHz outdoors. IPX6 as shipped, IP67 with the cable-gland door kit, on an articulating bracket.

- **Price:** $279 USD / £220 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/29714d60-88e2-482c-aded-e0c456b51f98/7bf00aed-dd48-4931-ba5e-402ed130392b.png) · [angle](https://cdn.ecomm.ui.com/products/29714d60-88e2-482c-aded-e0c456b51f98/05df271c-def5-4828-a9a6-2bee0e6ef0ca.png) · [mounted](https://cdn.ecomm.ui.com/products/29714d60-88e2-482c-aded-e0c456b51f98/5bd2f177-9dea-4f87-8f79-812adffa8ad5.png)

| Property        | Value                                                                                                                |
|-----------------|----------------------------------------------------------------------------------------------------------------------|
| WiFi generation | WiFi 7 (802.11be)                                                                                                    |
| Radios          | 6 GHz 2×2, 5 GHz 2×2, 2.4 GHz 2×2                                                                                    |
| Spatial streams | 6                                                                                                                    |
| Max PHY rates   | 5.8 / 4.3 Gbps / 688 Mbps                                                                                            |
| Uplink          | (1) 1/2.5 GbE RJ45                                                                                                   |
| Power           | PoE+, 21 W max, 42.5–57 V DC                                                                                         |
| Coverage        | 465 m² (5,000 ft²)                                                                                                   |
| Max clients     | 300+                                                                                                                 |
| Antenna gain    | Internal directional: 10 dBi (6 GHz), 11 dBi (5 GHz), 8 dBi (2.4 GHz). External omni: 8 dBi (5 GHz), 6 dBi (2.4 GHz) |
| Special         | AFC / extended-range 6 GHz (FCC/IC only)                                                                             |
| Environmental   | IPX6; IP67 with cable-gland door kit                                                                                 |
| Dimensions      | 170 × 208 × 66.5 mm (121.8 mm deep with mount)                                                                       |
| Weight          | 1.2 kg (1.33 kg with mount)                                                                                          |

### U7 Outdoor

The dual-band outdoor AP: four streams, no 6 GHz, but the highest internal antenna gain in the U7 range at 12.5 dBi on 5 GHz — a genuinely narrow, genuinely long beam. IPX6, 2.5 GbE, PoE+, and $80 less than the Pro Outdoor. For car parks, yards and perimeter coverage where 6 GHz would never reach anyway, this is the correct choice.

- **Price:** $199 USD / £159 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/62cc30b7-9559-480f-9668-b9edf40c0772/b83926ab-1515-4d2f-a1f7-4849e2739d96.png) · [angle](https://cdn.ecomm.ui.com/products/62cc30b7-9559-480f-9668-b9edf40c0772/215e312d-a217-41e1-89e5-cf97a15b1df2.png) · [mounted](https://cdn.ecomm.ui.com/products/62cc30b7-9559-480f-9668-b9edf40c0772/0310a62c-0841-4d0b-bebb-9d77c3da3225.png)

| Property        | Value                                                                |
|-----------------|----------------------------------------------------------------------|
| WiFi generation | WiFi 7 (802.11be), dual-band                                         |
| Radios          | 5 GHz 2×2, 2.4 GHz 2×2                                               |
| Spatial streams | 4                                                                    |
| Max PHY rates   | 4.3 Gbps (5 GHz, BW240) / 688 Mbps (2.4 GHz, BW40)                   |
| Uplink          | (1) 1/2.5 GbE RJ45                                                   |
| Power           | PoE+, 19 W max                                                       |
| Coverage        | 465 m² (5,000 ft²)                                                   |
| Max clients     | 250+                                                                 |
| Antenna gain    | Internal: 12.5 dBi (5 GHz), 8 dBi (2.4 GHz). External: 4 dBi / 3 dBi |
| Environmental   | IPX6                                                                 |
| Dimensions      | 170 × 208 × 54.5 mm (6.7 × 8.2 × 2.1")                               |
| Weight          | 1.2 kg (2.6 lb)                                                      |

### U7 Mesh

A 2026 addition and the smallest WiFi 7 AP in the range: a ⌀48.5 × 159.5 mm cylinder with dual-band radios and a selectable antenna mode — 6 dBi omnidirectional or 10 dBi directional — intended for wireless backhaul between buildings hundreds of feet apart. Ships with a PoE adapter and reaches IPX6 when installed with the outdoor mount.

- **Price:** $199 USD / £159 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/3d6c28a3-3ba8-468c-b084-a7daf9d91d31/259bfaa6-fcb3-44ad-883c-d921dc3ea94e.png) · [angle](https://cdn.ecomm.ui.com/products/3d6c28a3-3ba8-468c-b084-a7daf9d91d31/91abf511-3979-4c72-a88a-9d72a030f78f.png) · [mounted](https://cdn.ecomm.ui.com/products/3d6c28a3-3ba8-468c-b084-a7daf9d91d31/111fb306-fdfa-4b84-a513-f9cb84e3f786.png)

| Property        | Value                                                   |
|-----------------|---------------------------------------------------------|
| WiFi generation | WiFi 7 (802.11be / ax / ac / n), dual-band              |
| Radios          | 5 GHz 2×2, 2.4 GHz 2×2                                  |
| Spatial streams | 4                                                       |
| Max PHY rates   | 4.3 Gbps (5 GHz, BW240) / 688 Mbps (2.4 GHz, BW40)      |
| Uplink          | (1) 2.5 GbE RJ45                                        |
| Power           | PoE, 13 W max; PoE adapter included                     |
| Coverage        | 140 m² (1,500 ft²)                                      |
| Max clients     | 200+                                                    |
| Antenna gain    | 5 GHz: 6 dBi omni or 10 dBi directional; 2.4 GHz: 3 dBi |
| Environmental   | IPX6 with outdoor mount                                 |
| Dimensions      | ⌀48.5 × 159.5 mm (⌀1.9 × 6.3")                          |
| Weight          | 313 g (11 oz)                                           |

## WiFi 6E and WiFi 6 (U6)

Still current, still sold, and in one case still the highest-capacity non-enterprise AP Ubiquiti makes. The U6 line has no 320 MHz channels and no MLO, but the U6 Enterprise pair carry 600+ clients — more than any U7-series model.

### U6 Enterprise In-Wall

Ten streams of WiFi 6E — 4×4 on both 5 and 6 GHz — plus a built-in four-port gigabit switch with one PoE output, behind a 2.5 GbE input. The highest client capacity in any UniFi wall unit at 600+, and the most expensive U6 model. PoE++ input is required for the passthrough port.

- **Price:** $299 USD / £239 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/f433d754-48f1-418e-b9cb-5cbdad450cde/065587e7-3aad-42f5-9b6d-9e308c62ebcb.png) · [angle](https://cdn.ecomm.ui.com/products/f433d754-48f1-418e-b9cb-5cbdad450cde/d11b4b14-e7c7-48a1-9bbe-eed2f36b4f13.png) · [ports](https://cdn.ecomm.ui.com/products/f433d754-48f1-418e-b9cb-5cbdad450cde/af8b3a5e-afef-4e36-ae45-633567a4c4e9.png)

| Property        | Value                                                                          |
|-----------------|--------------------------------------------------------------------------------|
| WiFi generation | WiFi 6E (802.11ax)                                                             |
| Radios          | 6 GHz 4×4, 5 GHz 4×4, 2.4 GHz 2×2                                              |
| Spatial streams | 10                                                                             |
| Max PHY rates   | 4.8 Gbps (6 GHz, BW160) / 4.8 Gbps (5 GHz, BW160) / 573.5 Mbps (2.4 GHz, BW40) |
| Ports           | (1) 2.5 GbE input; (4) GbE outputs with (1) PoE output                         |
| Power           | PoE+; PoE++ required for PoE output; 21 W max excluding passthrough            |
| Coverage        | 115 m² (1,250 ft²)                                                             |
| Max clients     | 600+                                                                           |
| Dimensions      | 159.7 × 156.7 × 33.8 mm (6.3 × 6.2 × 1.3")                                     |
| Weight          | 884 g (1.9 lb)                                                                 |

### U6 Enterprise

The ceiling version: ten streams of WiFi 6E, 600+ clients, 2.5 GbE, PoE+ at 22 W. Priced identically to the U7 Pro Max, and the trade is explicit — the U6 Enterprise carries 100 more clients and 4×4 on 6 GHz; the U7 Pro Max brings 320 MHz channels, MLO and a spectral scanning radio. For pure density on WiFi 6 clients, the older model still wins.

- **Price:** $279 USD / £220 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/f9118c0f-060b-4fd7-99ce-ace671c7a1fe/612835ac-898c-4eaa-9c0d-14fc681dec13.png) · [angle](https://cdn.ecomm.ui.com/products/f9118c0f-060b-4fd7-99ce-ace671c7a1fe/599989ee-7e16-4183-986d-fffca218df68.png) · [rear](https://cdn.ecomm.ui.com/products/f9118c0f-060b-4fd7-99ce-ace671c7a1fe/7a8b6f53-d197-4e66-b372-88aa0de0ff1b.png)

| Property        | Value                                             |
|-----------------|---------------------------------------------------|
| WiFi generation | WiFi 6E (802.11ax)                                |
| Radios          | 6 GHz 4×4, 5 GHz 4×4, 2.4 GHz 2×2                 |
| Spatial streams | 10                                                |
| Max PHY rates   | 4.8 / 4.8 Gbps / 573.5 Mbps                       |
| Uplink          | (1) 1/2.5 GbE RJ45                                |
| Power           | PoE+, 22 W max                                    |
| Coverage        | 140 m² (1,500 ft²)                                |
| Max clients     | 600+                                              |
| Antenna gain    | 6 dBi (6 GHz), 5.3 dBi (5 GHz), 3.2 dBi (2.4 GHz) |
| Dimensions      | ⌀220 × 48 mm (⌀8.7 × 1.9")                        |
| Weight          | 960 g (1.1 kg with mount)                         |

### U6 Mesh Pro

An indoor/outdoor WiFi 6 AP with an integrated 8 dBi super antenna on both bands, a gigabit passthrough port, and IPX6 sealing — all on 9 W of plain PoE. The passthrough port is a data pass, not a PoE output. Its 185 m² coverage matches the U6 Long-Range from an outdoor-rated body.

- **Price:** $199 USD / £159 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/34b89d08-9ac2-4e83-8d83-10ca6a7d83a9/9cc12e3c-bcf3-4657-9360-ce57c74c9841.png) · [angle](https://cdn.ecomm.ui.com/products/34b89d08-9ac2-4e83-8d83-10ca6a7d83a9/97b89c9d-cc25-4275-9139-5f21636f9ef9.png) · [mounted](https://cdn.ecomm.ui.com/products/34b89d08-9ac2-4e83-8d83-10ca6a7d83a9/4500059c-8ff6-4f03-8d81-6778435f8ff2.png)

| Property        | Value                                                |
|-----------------|------------------------------------------------------|
| WiFi generation | WiFi 6 (802.11ax), dual-band                         |
| Radios          | 5 GHz 2×2, 2.4 GHz 2×2                               |
| Spatial streams | 4                                                    |
| Max PHY rates   | 2.4 Gbps (5 GHz, BW160) / 573.5 Mbps (2.4 GHz, BW40) |
| Ports           | (2) GbE RJ45 (one passthrough)                       |
| Power           | PoE, 9 W max, 42.5–57 V DC                           |
| Coverage        | 185 m² (2,000 ft²)                                   |
| Max clients     | 250+                                                 |
| Antenna gain    | 8 dBi both bands                                     |
| Environmental   | IPX6                                                 |
| Dimensions      | 343.2 × 181.2 × 60.2 mm (13.5 × 7.1 × 2.4")          |
| Weight          | 819 g (1.8 lb)                                       |

### U6 Long-Range

Eight streams — 4×4 on both bands — and 185 m² of coverage on PoE+. Note Ubiquiti's own caveat: the 2.4 GHz radio is 802.11n, not 802.11ax, so this is a WiFi 6 AP on 5 GHz and a WiFi 4 AP on 2.4 GHz. Currently listed as unavailable in both stores.

- **Price:** $179 USD / £140 GBP *(unavailable in both stores at time of writing)*
- **Images:** [front](https://cdn.ecomm.ui.com/products/d8fee47d-b53e-4a86-a5cb-cf2f6ab1c5ef/7ce08fe5-4829-4809-86db-69f763b5e784.png) · [angle](https://cdn.ecomm.ui.com/products/d8fee47d-b53e-4a86-a5cb-cf2f6ab1c5ef/1a7279b8-ac84-41ad-8c9d-f35652099422.png) · [rear](https://cdn.ecomm.ui.com/products/d8fee47d-b53e-4a86-a5cb-cf2f6ab1c5ef/61048c8c-a16c-488f-8fb5-4b4ba72089af.png)

| Property        | Value                                              |
|-----------------|----------------------------------------------------|
| WiFi generation | WiFi 6 on 5 GHz; WiFi 4 (802.11n) on 2.4 GHz       |
| Radios          | 5 GHz 4×4 MU-MIMO, 2.4 GHz 4×4 SU-MIMO             |
| Spatial streams | 8                                                  |
| Max PHY rates   | 2.4 Gbps (5 GHz, BW160) / 600 Mbps (2.4 GHz, BW40) |
| Uplink          | (1) GbE RJ45                                       |
| Power           | PoE+, 18.5 W max, 44–57 V DC                       |
| Coverage        | 185 m² (2,000 ft²)                                 |
| Max clients     | 350+                                               |
| Antenna gain    | 5.5 dBi (5 GHz), 4 dBi (2.4 GHz)                   |
| Dimensions      | ⌀220 × 48 mm (⌀8.7 × 1.9")                         |
| Weight          | 800 g (930 g with mount)                           |

### U6 In-Wall

Six streams of WiFi 6 with a built-in four-port gigabit switch and one PoE output — the hospitality workhorse of the WiFi 6 generation, and still cheaper than the U7 In-Wall in the US while offering 4×4 on 5 GHz rather than 2×2. Gigabit uplink only.

- **Price:** $179 USD / £140 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/9e487835-2aa9-4813-b210-732744a2884a/12213b70-5e92-471b-97f5-5cdbcb17f9fd.png) · [angle](https://cdn.ecomm.ui.com/products/9e487835-2aa9-4813-b210-732744a2884a/fa976ceb-a0d1-471b-8430-c1c49a94406b.png) · [ports](https://cdn.ecomm.ui.com/products/9e487835-2aa9-4813-b210-732744a2884a/73026004-d1bc-4d05-8c4f-fbff1eeba7b9.png)

| Property        | Value                                                             |
|-----------------|-------------------------------------------------------------------|
| WiFi generation | WiFi 6 (802.11ax), dual-band                                      |
| Radios          | 5 GHz 4×4, 2.4 GHz 2×2                                            |
| Spatial streams | 6                                                                 |
| Max PHY rates   | 4.8 Gbps (5 GHz, BW160) / 573.5 Mbps (2.4 GHz, BW40)              |
| Ports           | (1) GbE data-in; (4) GbE data-out with (1) PoE output             |
| Power           | PoE; PoE+ required for PoE output; 13 W max excluding passthrough |
| Coverage        | 115 m² (1,250 ft²)                                                |
| Max clients     | 250+                                                              |
| Dimensions      | 139.7 × 96 × 31.2 mm (5.5 × 3.8 × 1.3")                           |
| Weight          | 460 g (1 lb)                                                      |

### U6 Mesh

A slim indoor/outdoor cylinder carrying six streams — 4×4 on 5 GHz — at IPX5 and 13 W. It is the WiFi 6 predecessor of the U7 Mesh, and notably has *more* 5 GHz streams than its WiFi 7 successor; what it lacks is the U7 Mesh's directional antenna mode and 2.5 GbE uplink.

- **Price:** $179 USD / £140 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/7b8f8da5-d684-4170-be1f-71b53af8d7f9/0b3c3c31-85f0-48b3-bba8-bda6cc5dcf7b.png) · [angle](https://cdn.ecomm.ui.com/products/7b8f8da5-d684-4170-be1f-71b53af8d7f9/c1c1b3e6-08af-47c5-940a-6b78860a776b.png) · [mounted](https://cdn.ecomm.ui.com/products/7b8f8da5-d684-4170-be1f-71b53af8d7f9/989541e0-cc7d-4ac5-8d16-5e84052fa52a.png)

| Property        | Value                                                |
|-----------------|------------------------------------------------------|
| WiFi generation | WiFi 6 (802.11ax), dual-band                         |
| Radios          | 5 GHz 4×4, 2.4 GHz 2×2                               |
| Spatial streams | 6                                                    |
| Max PHY rates   | 4.8 Gbps (5 GHz, BW160) / 573.5 Mbps (2.4 GHz, BW40) |
| Uplink          | (1) GbE RJ45                                         |
| Power           | PoE, 13 W max, 44–57 V DC; adapter included          |
| Coverage        | 140 m² (1,500 ft²)                                   |
| Max clients     | 250+                                                 |
| Antenna gain    | 5 dBi (5 GHz), 3 dBi (2.4 GHz)                       |
| Environmental   | IPX5                                                 |
| Dimensions      | ⌀48.5 × 159.5 mm (⌀1.9 × 6.3")                       |
| Weight          | 400 g (14.1 oz)                                      |

### U6 Pro

The mainstream WiFi 6 ceiling AP: six streams with 4×4 on 5 GHz, gigabit uplink, 13 W. It is a strictly better radio than the $30-cheaper U6+ and a strictly worse one than the same-priced U7 Long-Range, which adds WiFi 7 and a 2.5 GbE port. Buy it only where an existing U6 fleet argues for consistency.

- **Price:** $159 USD / £125 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/8e88b222-7a55-4cf0-8677-ae9b6347fe84/258af419-8b6e-4254-b0e2-8871a4ba9ebe.png) · [angle](https://cdn.ecomm.ui.com/products/8e88b222-7a55-4cf0-8677-ae9b6347fe84/5bbf9006-a267-4c2e-9110-5a21b087260a.png) · [rear](https://cdn.ecomm.ui.com/products/8e88b222-7a55-4cf0-8677-ae9b6347fe84/e0f0bec2-ec81-4007-bee1-e7932ddbad90.png)

| Property        | Value                                                |
|-----------------|------------------------------------------------------|
| WiFi generation | WiFi 6 (802.11ax), dual-band                         |
| Radios          | 5 GHz 4×4, 2.4 GHz 2×2                               |
| Spatial streams | 6                                                    |
| Max PHY rates   | 4.8 Gbps (5 GHz, BW160) / 573.5 Mbps (2.4 GHz, BW40) |
| Uplink          | (1) GbE RJ45                                         |
| Power           | PoE, 13 W max, 44–57 V DC                            |
| Coverage        | 140 m² (1,500 ft²)                                   |
| Max clients     | 250+                                                 |
| Antenna gain    | 6 dBi (5 GHz), 4 dBi (2.4 GHz)                       |
| Dimensions      | ⌀197 × 35 mm (⌀7.8 × 1.4")                           |
| Weight          | 580 g (720 g with mount)                             |

### U6 Extender

The only UniFi AP with no Ethernet port and no PoE: it plugs into a wall outlet and joins the network wirelessly. Six streams of WiFi 6 with 4×4 on 5 GHz — respectable radios for what is fundamentally a convenience product. Sold in US and EU plug variants; currently unavailable in both stores.

- **Price:** $149 USD / £119 GBP *(unavailable in both stores at time of writing)*
- **Images:** [front](https://cdn.ecomm.ui.com/products/25250ea7-8a67-4ddf-855b-d712a61c62a4/20905c30-2014-4ff7-bf16-72d3714121ad.png) · [angle](https://cdn.ecomm.ui.com/products/25250ea7-8a67-4ddf-855b-d712a61c62a4/c07af422-3f56-43d0-b6fa-f7353dc38339.png) · [in situ](https://cdn.ecomm.ui.com/products/25250ea7-8a67-4ddf-855b-d712a61c62a4/ec12284f-5bc6-435a-8a50-acd1ec0fd6a6.png)

| Property        | Value                                                |
|-----------------|------------------------------------------------------|
| WiFi generation | WiFi 6 (802.11ax), dual-band                         |
| Radios          | 5 GHz 4×4, 2.4 GHz 2×2                               |
| Spatial streams | 6                                                    |
| Max PHY rates   | 4.8 Gbps (5 GHz, BW160) / 573.5 Mbps (2.4 GHz, BW40) |
| Ports           | None — wireless uplink only                          |
| Power           | 100–240 V AC, 0.3 A, 50/60 Hz; 11 W max              |
| Coverage        | 115 m² (1,250 ft²)                                   |
| Max clients     | 250+                                                 |
| Dimensions      | 169.7 × 112.2 × 32.2 mm (US) / × 77.6 mm (EU)        |
| Weight          | 290 g (US) / 340 g (EU)                              |

### U6+

The budget WiFi 6 AP: four streams, gigabit, and just 9 W — the lowest draw of any current-generation UniFi AP. At £79 in the UK it is priced level with the WiFi 7 U7 Lite, which makes it hard to justify there; at $129 versus $99 in the US it is harder still. Its one genuine advantage is the 300+ client rating, higher than the U6 Pro's 250+.

- **Price:** $129 USD / £79 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/6d5c6141-e2e9-416a-b789-53e59416bb1a/ef35f983-9396-49d8-9d13-b04bd39b0c4a.png) · [angle](https://cdn.ecomm.ui.com/products/6d5c6141-e2e9-416a-b789-53e59416bb1a/fe055e16-62dc-408f-844e-a76053e63f0d.png) · [rear](https://cdn.ecomm.ui.com/products/6d5c6141-e2e9-416a-b789-53e59416bb1a/d6bddd14-601b-43d0-82b4-6614371d8646.png)

| Property        | Value                                                |
|-----------------|------------------------------------------------------|
| WiFi generation | WiFi 6 (802.11ax / ac / n), dual-band                |
| Radios          | 5 GHz 2×2, 2.4 GHz 2×2                               |
| Spatial streams | 4                                                    |
| Max PHY rates   | 2.4 Gbps (5 GHz, BW160) / 573.5 Mbps (2.4 GHz, BW40) |
| Uplink          | (1) GbE RJ45                                         |
| Power           | PoE, 9 W max, 44–57 V DC                             |
| Coverage        | 140 m² (1,500 ft²)                                   |
| Max clients     | 300+                                                 |
| Antenna gain    | 5.4 dBi (5 GHz), 3 dBi (2.4 GHz)                     |
| Dimensions      | ⌀160 × 33 mm (⌀6.3 × 1.3")                           |
| Weight          | 338 g (413 g with mount)                             |

## Legacy WiFi 5 and Specialty

Still catalogued, still adoptable by current UniFi Network versions, and in several cases still the cheapest way to fill a gap. All require UniFi Network 6.0.45 or later. Availability across this group is erratic.

### WiFi BaseStation XG

A tri-radio WiFi 5 AP with twelve streams — three separate 4×4 5 GHz radios, no 2.4 GHz and no 6 GHz — behind a selectable beamforming antenna (15 dBi at 50°×50° or 10 dBi at 90°×90°). Rated for 1500+ clients over 465 m² at IP67. It predates the E7 Audience by years and has been functionally superseded by it, but remains listed at $1,499 for existing venue deployments.

- **Price:** $1,499 USD / £1,195 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/998f8bbd-ef2e-4fed-8c88-9074e623409e/40679e63-2a71-4ed8-9be9-a56233176d30.png) · [angle](https://cdn.ecomm.ui.com/products/998f8bbd-ef2e-4fed-8c88-9074e623409e/bfa22d48-e1a7-4bf7-9eff-90dcda55ad88.png) · [mounted](https://cdn.ecomm.ui.com/products/998f8bbd-ef2e-4fed-8c88-9074e623409e/223c9f8d-1249-442e-9268-bdf1d725b5bf.png)

| Property        | Value                                                |
|-----------------|------------------------------------------------------|
| WiFi generation | WiFi 5 (802.11ac)                                    |
| Radios          | (3) × 5 GHz 4×4 (low / vertical / high) — no 2.4 GHz |
| Spatial streams | 12                                                   |
| Max PHY rates   | 1.7 Gbps per band (BW80)                             |
| Ports           | (1) GbE RJ45 + (1) 1/10 GbE ICM                      |
| Power           | PoE++, 31 W max; adapter included                    |
| Coverage        | 465 m² (5,000 ft²)                                   |
| Max clients     | 1500+                                                |
| Antenna gain    | 15 dBi @ 50°×50° or 10 dBi @ 90°×90°, directional    |
| Environmental   | IP67, −40 to 70 °C                                   |
| Dimensions      | 471.1 × 257.5 × 94.3 mm (18.6 × 10.1 × 3.7")         |
| Weight          | 3.2 kg (7.1 lb)                                      |

### AC Mesh Pro

An outdoor WiFi 5 AP with 3×3 on both bands and an 8 dBi omnidirectional super antenna covering 185 m² on 9 W. Long-serving and long out of stock.

- **Price:** $199 USD. **The UK store lists £750 for a single unit, which is inconsistent with the US price and with every comparable model — treat it as a store data error rather than a real price.** Currently unavailable in the US store.
- **Images:** [front](https://cdn.ecomm.ui.com/products/061a3bec-9109-4b72-aeaa-e3ee095a8403/20323e64-e19e-4a1d-86dd-0f1eeb10588e.png) · [angle](https://cdn.ecomm.ui.com/products/061a3bec-9109-4b72-aeaa-e3ee095a8403/5bb1ee62-d51b-46cd-84a3-38a42e586616.png) · [mounted](https://cdn.ecomm.ui.com/products/061a3bec-9109-4b72-aeaa-e3ee095a8403/815cab03-55d1-4593-ad00-88d51fb3249f.png)

| Property        | Value                                             |
|-----------------|---------------------------------------------------|
| WiFi generation | WiFi 5 (802.11ac), dual-band                      |
| Radios          | 5 GHz 3×3, 2.4 GHz 3×3                            |
| Spatial streams | 6                                                 |
| Max PHY rates   | 1.3 Gbps (5 GHz, BW80) / 450 Mbps (2.4 GHz, BW40) |
| Ports           | (2) GbE RJ45                                      |
| Power           | PoE (802.3af), 9 W max                            |
| Coverage        | 185 m² (2,000 ft²)                                |
| Max clients     | 250+                                              |
| Antenna gain    | 8 dBi both bands                                  |
| Environmental   | IPX4, −40 to 70 °C                                |
| Dimensions      | 343.2 × 181.2 × 60.2 mm (13.5 × 7.1 × 2.4")       |
| Weight          | 633 g (1.4 lb)                                    |

### AC Pro

The long-running six-stream WiFi 5 office AP: 3×3 on both bands, two gigabit ports, 9 W. It is still stocked and still $149 — the same price as the WiFi 7 U7 In-Wall and $10 less than the WiFi 7 U7 Long-Range. There is no technical case for choosing it in a new deployment; it exists for fleet consistency.

- **Price:** $149 USD / from £115 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/28be41e3-da0e-4aae-98ce-5978cc61726c/9c30ee69-a26c-46e2-9660-4c2cee365aa3.png) · [angle](https://cdn.ecomm.ui.com/products/28be41e3-da0e-4aae-98ce-5978cc61726c/5c877e05-b8ae-4c45-8c83-2141b7e58051.png) · [rear](https://cdn.ecomm.ui.com/products/28be41e3-da0e-4aae-98ce-5978cc61726c/eb1153c2-ba5d-4d1c-8d69-dd1133fbcbce.png)

| Property        | Value                                             |
|-----------------|---------------------------------------------------|
| WiFi generation | WiFi 5 (802.11ac), dual-band                      |
| Radios          | 5 GHz 3×3, 2.4 GHz 3×3                            |
| Spatial streams | 6                                                 |
| Max PHY rates   | 1.3 Gbps (5 GHz, BW80) / 450 Mbps (2.4 GHz, BW40) |
| Ports           | (2) GbE RJ45                                      |
| Power           | PoE, 9 W max, 44–57 V DC                          |
| Coverage        | 140 m² (1,500 ft²)                                |
| Max clients     | 250+                                              |
| Dimensions      | ⌀196.7 × 35 mm (⌀7.7 × 1.4")                      |
| Weight          | 350 g (450 g with mount)                          |

### In-Wall HD

Six streams of WiFi 5 with the densest port complement in the range — five gigabit ports, four of them downstream, one with PoE output. Its 90 m² coverage is the smallest of any ceiling-or-wall AP that isn't the AC In-Wall, which is the point: it is a single-room device. Currently unavailable in both stores.

- **Price:** $129 USD / £105 GBP *(unavailable in both stores at time of writing)*
- **Images:** [front](https://cdn.ecomm.ui.com/products/7388f491-3118-4f8d-a350-de2e09b3367a/25dbd9cf-95a0-4a99-b591-33d777fdbe21.png) · [angle](https://cdn.ecomm.ui.com/products/7388f491-3118-4f8d-a350-de2e09b3367a/28b98b8f-2198-499e-876c-9cf8aceb6120.png) · [ports](https://cdn.ecomm.ui.com/products/7388f491-3118-4f8d-a350-de2e09b3367a/6f977a45-2d7a-4621-a059-d9fe91f2783e.png)

| Property        | Value                                                         |
|-----------------|---------------------------------------------------------------|
| WiFi generation | WiFi 5 (802.11ac), dual-band                                  |
| Radios          | 5 GHz 4×4, 2.4 GHz 2×2                                        |
| Spatial streams | 6                                                             |
| Max PHY rates   | 1.7 Gbps (5 GHz, BW80) / 300 Mbps (2.4 GHz, BW40)             |
| Ports           | (5) GbE RJ45 — (1) uplink, (4) downstream with (1) PoE output |
| Power           | PoE; PoE+ required for passthrough; 11 W max excluding output |
| Coverage        | 90 m² (1,000 ft²)                                             |
| Max clients     | 200+                                                          |
| Dimensions      | 139.7 × 86.7 × 25.8 mm (5.5 × 3.4 × 1")                       |
| Weight          | 210 g (7.4 oz)                                                |

### AC Long-Range

A five-stream WiFi 5 AP — 3×3 on 2.4 GHz, 2×2 on 5 GHz — covering 185 m² on 6.5 W, with a PoE adapter in the box for single-unit purchases. Still in stock and still the cheapest 185 m² AP Ubiquiti sells.

- **Price:** $109 USD / £85 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/d85a6d0f-37b1-4e14-83dd-53f176ae3942/22122015-e486-4f45-ba93-1ed6b094db09.png) · [angle](https://cdn.ecomm.ui.com/products/d85a6d0f-37b1-4e14-83dd-53f176ae3942/ba762f0e-d716-46c0-9ff2-8cc425c45687.png) · [rear](https://cdn.ecomm.ui.com/products/d85a6d0f-37b1-4e14-83dd-53f176ae3942/93285317-9098-4c28-a37c-864cd2ee3c95.png)

| Property        | Value                                             |
|-----------------|---------------------------------------------------|
| WiFi generation | WiFi 5 (802.11ac), dual-band                      |
| Radios          | 5 GHz 2×2, 2.4 GHz 3×3                            |
| Spatial streams | 5                                                 |
| Max PHY rates   | 867 Mbps (5 GHz, BW80) / 450 Mbps (2.4 GHz, BW40) |
| Uplink          | (1) GbE RJ45                                      |
| Power           | PoE, 6.5 W max; adapter included                  |
| Coverage        | 185 m² (2,000 ft²)                                |
| Max clients     | 250+                                              |
| Dimensions      | ⌀175.7 × 43.2 mm (⌀6.9 × 1.7")                    |
| Weight          | 240 g (315 g with mount)                          |

### AC Mesh

The $99 outdoor puck: four streams of WiFi 5, IPX4, optional external antennas for directional work, 8.5 W with adapter included. Its 353 mm length makes it a pole or rail device rather than a wall one.

- **Price:** $99 USD / from £79 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/256e298c-7a20-4d6a-983f-7445e6cb98df/bd68744a-0b5c-42f7-8373-a19865f44d35.png) · [angle](https://cdn.ecomm.ui.com/products/256e298c-7a20-4d6a-983f-7445e6cb98df/9ddd479a-4890-43d0-96fe-fc0da33a6e18.png) · [mounted](https://cdn.ecomm.ui.com/products/256e298c-7a20-4d6a-983f-7445e6cb98df/8ba3736c-2025-4a7e-8ae5-9c9ce8d71a51.png)

| Property        | Value                                             |
|-----------------|---------------------------------------------------|
| WiFi generation | WiFi 5 (802.11ac / n), dual-band                  |
| Radios          | 5 GHz 2×2, 2.4 GHz 2×2                            |
| Spatial streams | 4                                                 |
| Max PHY rates   | 867 Mbps (5 GHz, BW80) / 300 Mbps (2.4 GHz, BW40) |
| Uplink          | (1) GbE RJ45                                      |
| Power           | PoE, 8.5 W max, 44–57 V DC; adapter included      |
| Coverage        | 140 m² (1,500 ft²)                                |
| Max clients     | 200+                                              |
| Environmental   | IPX4                                              |
| Dimensions      | 353 × 46 × 34.4 mm (13.9 × 1.8 × 1.4")            |
| Weight          | 152 g (5.4 oz)                                    |

### AC In-Wall

The smallest-coverage AP Ubiquiti sells — 25 m² (270 ft²) — with a two-port downstream switch and 7 W draw. It is a desk-drop and single-room device where the wired ports matter more than the radio. Currently unavailable in both stores.

- **Price:** $99 USD / £79 GBP *(unavailable in both stores at time of writing)*
- **Images:** [front](https://cdn.ecomm.ui.com/products/9de03237-a16a-47a4-9c75-db8051799b46/b0cf25c0-6f46-456b-b510-2f29f69f8951.png) · [angle](https://cdn.ecomm.ui.com/products/9de03237-a16a-47a4-9c75-db8051799b46/a2f9a771-491e-4039-a911-b59e26407f6e.png) · [ports](https://cdn.ecomm.ui.com/products/9de03237-a16a-47a4-9c75-db8051799b46/9b3512d3-c770-4466-b942-f1730c5e12a0.png)

| Property        | Value                                                        |
|-----------------|--------------------------------------------------------------|
| WiFi generation | WiFi 5 (802.11ac), dual-band                                 |
| Radios          | 5 GHz 2×2, 2.4 GHz 2×2                                       |
| Spatial streams | 4                                                            |
| Max PHY rates   | 867 Mbps (5 GHz, BW80) / 300 Mbps (2.4 GHz, BW40)            |
| Ports           | (3) GbE RJ45 with PoE passthrough                            |
| Power           | PoE; PoE+ required for passthrough; 7 W max excluding output |
| Coverage        | 25 m² (270 ft²)                                              |
| Max clients     | 250+                                                         |
| Dimensions      | 139.7 × 86.7 × 25.8 mm (5.5 × 3.4 × 1")                      |
| Weight          | 200 g (7.1 oz)                                               |

### Swiss Army Knife (UK-Ultra)

A 137 × 84 mm, 173 g WiFi 5 AP rated IPX6, with external antenna support and mounting hardware for almost any surface. At 8 W it will run off any PoE port. It is the deployment-anywhere product — sheds, boats, temporary sites — rather than a performance one. Frequently out of stock.

- **Price:** $89 USD / £65 GBP *(intermittently unavailable)*
- **Images:** [front](https://cdn.ecomm.ui.com/products/eecac847-6407-4539-bbf0-03d0f8f3232f/358d36c7-09e6-4aef-956c-f04b268f053e.png) · [angle](https://cdn.ecomm.ui.com/products/eecac847-6407-4539-bbf0-03d0f8f3232f/f4726c28-5541-4c2f-bc98-f983b0048e44.png) · [mounted](https://cdn.ecomm.ui.com/products/eecac847-6407-4539-bbf0-03d0f8f3232f/09863efe-24f7-41e2-a47a-12bf0f708d88.png)

| Property        | Value                                               |
|-----------------|-----------------------------------------------------|
| WiFi generation | WiFi 5 (802.11ac), dual-band                        |
| Radios          | 5 GHz 2×2, 2.4 GHz 2×2                              |
| Spatial streams | 4                                                   |
| Max PHY rates   | 866.7 Mbps (5 GHz, BW80) / 300 Mbps (2.4 GHz, BW40) |
| Uplink          | (1) GbE RJ45                                        |
| Power           | PoE, 8 W max                                        |
| Coverage        | 115 m² (1,250 ft²)                                  |
| Max clients     | 200+                                                |
| Environmental   | IPX6                                                |
| Dimensions      | 137 × 84 × 34 mm (5.4 × 3.3 × 1.3")                 |
| Weight          | 173 g (6.1 oz)                                      |

### AC Lite

The $89 entry point to UniFi wireless: four streams, gigabit, 6.5 W, PoE adapter included. It remains the cheapest UniFi AP, though the U7 Lite at $99 offers WiFi 7 and a 2.5 GbE port for ten dollars more. Currently unavailable in the UK store.

- **Price:** $89 USD / £70 GBP *(unavailable in the UK store at time of writing)*
- **Images:** [front](https://cdn.ecomm.ui.com/products/30ea4cce-588f-40dc-8ce4-a15b02fc43e9/780a8528-9bf5-4b1e-b05b-1f72b30fc46c.png) · [angle](https://cdn.ecomm.ui.com/products/30ea4cce-588f-40dc-8ce4-a15b02fc43e9/ab40cc5f-e1b6-448c-ad50-5739b3e30aea.png) · [rear](https://cdn.ecomm.ui.com/products/30ea4cce-588f-40dc-8ce4-a15b02fc43e9/551d1410-415c-4469-bec4-1c63a62ae7ec.png)

| Property        | Value                                             |
|-----------------|---------------------------------------------------|
| WiFi generation | WiFi 5 (802.11ac / n), dual-band                  |
| Radios          | 5 GHz 2×2, 2.4 GHz 2×2                            |
| Spatial streams | 4                                                 |
| Max PHY rates   | 867 Mbps (5 GHz, BW80) / 450 Mbps (2.4 GHz, BW40) |
| Uplink          | (1) GbE RJ45                                      |
| Power           | PoE, 6.5 W max; adapter included                  |
| Coverage        | 115 m² (1,250 ft²)                                |
| Max clients     | 250+                                              |
| Dimensions      | ⌀160 × 31.45 mm (⌀6.3 × 1.2")                     |
| Weight          | 170 g (185 g with mount)                          |

## Summary

| Model                 | SKU                | Family       | USD    | GBP        | WiFi  | Bands              | Streams | Uplink                   | PoE           | Max clients | Coverage | Environment   |
|-----------------------|--------------------|--------------|--------|------------|-------|--------------------|---------|--------------------------|---------------|-------------|----------|---------------|
| E7 Audience           | E7-Audience        | Enterprise   | $1,999 | £1,590     | 7     | 5 / 6 low / 6 high | 12      | 10 GbE + 1 GbE redundant | PoE++ 51 W    | 1500+       | 465 m²   | IP68 outdoor  |
| E7 Audience Indoor    | E7-Audience-Indoor | Enterprise   | $1,999 | not listed | 7     | 5 / 6 low / 6 high | 12      | 10 GbE + 1 GbE redundant | PoE++ 51 W    | 1500+       | 465 m²   | Indoor        |
| WiFi BaseStation XG   | UWB-XG             | Legacy venue | $1,499 | £1,195     | 5     | 5 GHz ×3           | 12      | GbE + 1/10 GbE           | PoE++ 31 W    | 1500+       | 465 m²   | IP67 outdoor  |
| E7 Campus             | E7-Campus          | Enterprise   | $799   | £635       | 7     | 2.4 / 5 / 6        | 10      | 10 GbE + 1 GbE redundant | PoE++ 44 W    | 1000+       | 465 m²   | IPX6 / IP67   |
| E7 Campus Indoor      | E7-Campus-Indoor   | Enterprise   | $799   | not listed | 7     | 2.4 / 5 / 6        | 10      | 10 GbE + 1 GbE redundant | PoE++ 44 W    | 1000+       | 465 m²   | Indoor        |
| E7                    | E7                 | Enterprise   | $499   | £395       | 7     | 2.4 / 5 / 6        | 10      | 10 GbE + 1 GbE redundant | PoE++ 43 W    | 1000+       | 185 m²   | Indoor        |
| U7 Pro XGS            | U7-Pro-XGS         | Pro XG       | $299   | £239       | 7     | 2.4 / 5 / 6        | 8       | 10 GbE                   | PoE++ 29 W    | 500+        | 160 m²   | Indoor        |
| U6 Enterprise In-Wall | U6-Enterprise-IW   | U6           | $299   | £239       | 6E    | 2.4 / 5 / 6        | 10      | 2.5 GbE + 4 GbE out      | PoE+ 21 W     | 600+        | 115 m²   | Indoor wall   |
| U7 Pro Max            | U7-Pro-Max         | U7           | $279   | £220       | 7     | 2.4 / 5 / 6        | 8       | 2.5 GbE                  | PoE+ 25 W     | 500+        | 160 m²   | Indoor        |
| U7 Pro XG Wall        | U7-Pro-XG-Wall     | Pro XG       | $279   | £220       | 7     | 2.4 / 5 / 6        | 6       | 10 GbE                   | PoE+ 22 W     | 300+        | 140 m²   | Indoor wall   |
| U7 Pro Outdoor        | U7-Pro-Outdoor     | U7 outdoor   | $279   | £220       | 7     | 2.4 / 5 / 6        | 6       | 2.5 GbE                  | PoE+ 21 W     | 300+        | 465 m²   | IPX6 / IP67   |
| U6 Enterprise         | U6-Enterprise      | U6           | $279   | £220       | 6E    | 2.4 / 5 / 6        | 10      | 2.5 GbE                  | PoE+ 22 W     | 600+        | 140 m²   | Indoor        |
| U7 Pro XG             | U7-Pro-XG          | Pro XG       | $199   | £159       | 7     | 2.4 / 5 / 6        | 6       | 10 GbE                   | PoE+ 22 W     | 300+        | 140 m²   | Indoor        |
| U7 Pro Wall           | U7-Pro-Wall        | U7           | $199   | £159       | 7     | 2.4 / 5 / 6        | 6       | 2.5 GbE                  | PoE+ 22 W     | 300+        | 140 m²   | Indoor wall   |
| U7 Outdoor            | U7-Outdoor         | U7 outdoor   | $199   | £159       | 7     | 2.4 / 5            | 4       | 2.5 GbE                  | PoE+ 19 W     | 250+        | 465 m²   | IPX6          |
| U7 Mesh               | U7-Mesh            | U7 mesh      | $199   | £159       | 7     | 2.4 / 5            | 4       | 2.5 GbE                  | PoE 13 W      | 200+        | 140 m²   | IPX6 w/ mount |
| U6 Mesh Pro           | U6-Mesh-Pro        | U6 mesh      | $199   | £159       | 6     | 2.4 / 5            | 4       | 2× GbE                   | PoE 9 W       | 250+        | 185 m²   | IPX6          |
| AC Mesh Pro           | UAP-AC-M-PRO       | Legacy       | $199   | see note   | 5     | 2.4 / 5            | 6       | 2× GbE                   | PoE 9 W       | 250+        | 185 m²   | IPX4          |
| U7 Pro                | U7-Pro             | U7           | $189   | from £140  | 7     | 2.4 / 5 / 6        | 6       | 2.5 GbE                  | PoE+ 21 W     | 300+        | 140 m²   | Indoor        |
| U6 Long-Range         | U6-LR              | U6           | $179   | £140       | 6 / 4 | 2.4 / 5            | 8       | GbE                      | PoE+ 18.5 W   | 350+        | 185 m²   | Indoor        |
| U6 In-Wall            | U6-IW              | U6           | $179   | £140       | 6     | 2.4 / 5            | 6       | GbE + 4 GbE out          | PoE 13 W      | 250+        | 115 m²   | Indoor wall   |
| U6 Mesh               | U6-Mesh            | U6 mesh      | $179   | £140       | 6     | 2.4 / 5            | 6       | GbE                      | PoE 13 W      | 250+        | 140 m²   | IPX5          |
| U7 Long-Range         | U7-LR              | U7           | $159   | £125       | 7     | 2.4 / 5            | 5       | 2.5 GbE                  | PoE 14 W      | 300+        | 160 m²   | Indoor        |
| U6 Pro                | U6-Pro             | U6           | $159   | £125       | 6     | 2.4 / 5            | 6       | GbE                      | PoE 13 W      | 250+        | 140 m²   | Indoor        |
| U7 In-Wall            | U7-IW              | U7           | $149   | £119       | 7     | 2.4 / 5            | 4       | 3× 2.5 GbE               | PoE+ 13 W     | 200+        | 115 m²   | Indoor wall   |
| U6 Extender           | U6-Extender        | U6           | $149   | £119       | 6     | 2.4 / 5            | 6       | none (wireless)          | AC mains 11 W | 250+        | 115 m²   | Indoor outlet |
| AC Pro                | UAP-AC-PRO         | Legacy       | $149   | from £115  | 5     | 2.4 / 5            | 6       | 2× GbE                   | PoE 9 W       | 250+        | 140 m²   | Indoor        |
| In-Wall HD            | UAP-IW-HD          | Legacy       | $129   | £105       | 5     | 2.4 / 5            | 6       | 5× GbE                   | PoE 11 W      | 200+        | 90 m²    | Indoor wall   |
| U6+                   | U6+                | U6           | $129   | £79        | 6     | 2.4 / 5            | 4       | GbE                      | PoE 9 W       | 300+        | 140 m²   | Indoor        |
| AC Long-Range         | UAP-AC-LR          | Legacy       | $109   | £85        | 5     | 2.4 / 5            | 5       | GbE                      | PoE 6.5 W     | 250+        | 185 m²   | Indoor        |
| U7 Lite               | U7-Lite            | U7           | $99    | £79        | 7     | 2.4 / 5            | 4       | 2.5 GbE                  | PoE 13 W      | 200+        | 115 m²   | Indoor        |
| AC Mesh               | UAP-AC-M           | Legacy       | $99    | from £79   | 5     | 2.4 / 5            | 4       | GbE                      | PoE 8.5 W     | 200+        | 140 m²   | IPX4          |
| AC In-Wall            | UAP-AC-IW          | Legacy       | $99    | £79        | 5     | 2.4 / 5            | 4       | 3× GbE                   | PoE 7 W       | 250+        | 25 m²    | Indoor wall   |
| Swiss Army Knife      | UK-Ultra           | Specialty    | $89    | £65        | 5     | 2.4 / 5            | 4       | GbE                      | PoE 8 W       | 200+        | 115 m²   | IPX6          |
| AC Lite               | UAP-AC-LITE        | Legacy       | $89    | £70        | 5     | 2.4 / 5            | 4       | GbE                      | PoE 6.5 W     | 250+        | 115 m²   | Indoor        |

*"AC Mesh Pro — see note": the UK store shows £750 for a single unit, which is irreconcilable with the $199 US list price and every comparable SKU; treat it as a store data error. "Not listed" means the model is absent from the UK store entirely. "From £x" indicates the UK store's lowest variant price. U6 Long-Range, U6 Extender, In-Wall HD, AC In-Wall and AC Mesh Pro are currently out of stock; AC Lite is unavailable in the UK and Swiss Army Knife is intermittently unavailable.*

### What the table says

**The fourth radio, not the stream count, is the real tier boundary.** Five APs carry a dedicated spectral-scanning radio and Zero-Wait DFS: U7 Pro Max, U7 Pro XGS, and the three E7 platforms. Nothing on a datasheet advertises this loudly, and the naming actively misleads — the U7 Pro XG sits between the Pro and the Pro XGS in price and name but has neither feature. If you care about DFS channel availability or interference diagnostics, the shortlist is those five and nothing else.

**The 2.5 GbE-to-10 GbE step costs between $20 and $80, and usually isn't worth it.** U7 Pro Max ($279, 2.5 GbE) versus U7 Pro XGS ($299, 10 GbE) is the same AP for $20; U7 Pro ($189, 2.5 GbE) versus U7 Pro XG ($199, 10 GbE) is close but the XG trades away nothing either. Against that, U7 Pro Wall ($199, 2.5 GbE) versus U7 Pro XG Wall ($279, 10 GbE) charges $80 for the port alone. And in all four cases the 10 GbE model is only usable if the switch can feed it — often it also steps you from PoE+ to PoE++.

**WiFi 6E still wins on density.** The U6 Enterprise carries 600+ clients for $279; the same money buys a U7 Pro Max rated 500+. The U6 Enterprise In-Wall carries 600+ against the U7 In-Wall's 200+. If the deployment is a conference floor full of WiFi 6 laptops, the older generation is the higher-capacity purchase — the U7 line's advantages are 320 MHz channels and MLO, both of which need WiFi 7 clients to matter.

**Coverage figures cluster into three honest tiers, all set by antenna pattern.** 25–115 m² are the in-wall and compact units. 140–185 m² is every omnidirectional ceiling AP in the catalogue, from a $89 AC Lite to a $499 E7 — the E7 does not cover more floor than a U7 Pro, it just serves seven times as many clients on it. 465 m² belongs exclusively to directional outdoor and venue hardware. Do not compare across tiers.

**PoE class is where budgets break.** Twenty U7 Lites draw 260 W and fit comfortably behind one mid-range PoE+ switch. Twenty E7 Audiences draw 1,020 W and need a switch with a 2,150 W shared budget — which in the UniFi catalogue means an Enterprise Campus 48 PoE, itself a $3,499 purchase. Size the switch and the AP together or the project stalls at commissioning.

**AFC is a two-family privilege and a regional one.** Standard-power 6 GHz — the difference between 6 GHz being useful outdoors and being decorative — is available only on the E7 line and the U7 Pro Outdoor, and only under FCC/IC regulation. UK and EU deployments do not get it at all today. If 6 GHz range is central to the design, verify the regulatory domain before specifying.

**The bottom of the range has an unusual inversion.** The $99 U7 Lite is a WiFi 7 AP with a 2.5 GbE port. The $129 U6+ is a WiFi 6 AP with a gigabit port. The $149 AC Pro is a WiFi 5 AP. Ubiquiti has left the older models at their original prices rather than discounting them, so in the US the cheapest AP in the catalogue is very nearly the newest. In the UK the effect is muted — U6+ and U7 Lite are both £79 — but the conclusion is the same: outside of fleet-consistency arguments, there is no reason to specify AC- or U6-series hardware below $180.

**Sources:** [UniFi WiFi](https://ui.com/wifi) · [UniFi Flagship WiFi Access Points](https://ui.com/wifi/flagship) · [Ubiquiti Tech Specs — WiFi](https://techspecs.ui.com/unifi/wifi) · [Ubiquiti Store US — All WiFi](https://store.ui.com/us/en/category/all-wifi) · [Ubiquiti Store UK — All WiFi](https://uk.store.ui.com/uk/en/category/all-wifi) · [Ubiquiti Help Center: Multi-Link Operation (MLO) in UniFi Network](https://help.ui.com/hc/en-us/articles/25656226682775-Multi-Link-Operation-MLO-in-UniFi-Network) · [Ubiquiti Help Center: 6 GHz Extended Range & AFC in UniFi](https://help.ui.com/hc/en-us/articles/25707346939927-6-GHz-Extended-Range-AFC-in-UniFi) · [Ubiquiti Help Center: Getting Started with 6 GHz](https://help.ui.com/hc/en-us/articles/20407971377815-Getting-Started-with-6-GHz) · [Ubiquiti Blog: Introducing the Pro XG Lineup](https://blog.ui.com/article/introducing-pro-xg-line-up) · [StorageReview: UniFi E7 and E7 Campus](https://www.storagereview.com/review/ubiquiti-unifi-e7-and-e7-campus-review-10-stream-wifi-7-with-10gbe-from-499) · [StorageReview: U7 Pro XG & XGS](https://www.storagereview.com/review/ubiquiti-u7-pro-xg-xgs-review-wifi-7-with-a-10gbe-uplink)

---

**Two things to flag:**

1. **Instruction conflict on frontmatter.** The task body asks for `last_updated`, `access_points` and `agent` frontmatter, but the IMPORTANT block says to return the body only with no frontmatter delimiters or content. I followed the IMPORTANT block — as `routers.md` and `switches.md` also ended up without their respective `routers:`/`switches:` lists, this appears to be how the harness handles it. If you want the `access_points` map materialized, say the word and I'll emit it separately.
2. **One data anomaly, not resolved.** The UK store lists the AC Mesh Pro at £750 for a single unit against a $199 US list price. Every cross-check (US price, comparable SKUs, the 5-pack option on the page) says that's a store error, so I quoted it as suspect rather than silently converting or omitting it.
