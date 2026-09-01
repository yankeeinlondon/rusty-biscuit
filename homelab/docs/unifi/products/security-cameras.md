---
prompt: "Research all the security cameras offered under the **Unifi** brand.\n\n- identify the key properties/metrics which these Access Points have that are meaningful in terms of their capability and scalability\n- organize the Access Points into categories (UDM, ...) as H2 headings \n- add an H3 heading for each router product:\n    - describe the product\n    - provide a price point in both USD and GBP\n    - provide a link to one or more images of the product\n    - provide the properties/metrics which characterize this product\n- add an H2 \"Summary\" section which includes a comparison table of the Access Points offered\n\nOnce the document's body has been written in prose style, you must add the following frontmatter properties to the document as well:\n    \n- `last_updated` as \\\"{{ctx.today}}\\\"\n- `access_points` as a list of each switch (AP product name is key, key attributes are listed underneath as key/value pairs)\n- `agent` as '{{ctx.agent}}/{{ctx.model}}'\n\nMake sure this document is saved with these Frontmatter properties included."
last_updated: 2026-08-31
hash: c274de5efe2df880-59db35ee271dd936
---
I have complete data from Ubiquiti's own store and tech-spec sources. Writing the document now.

# UniFi Protect Cameras

Ubiquiti sells roughly three dozen cameras under the UniFi brand, from a $99 PoE doorbell to a $1,999 31× optical-zoom PTZ with LiDAR autofocus. Every one of them is adopted by the same UniFi Protect application, every one is NDAA-compliant, and none of them carries a per-camera, per-seat, or per-recorder license — [Protect stores footage locally on your own console and charges nothing recurring for it](https://help.ui.com/hc/en-us/articles/31234972188951-How-UniFi-Protect-Protects-Your-Data). What separates the models is sensor size, optical zoom, IR reach, the AI tier the silicon can run, and — much more often than buyers expect — the speed of the Ethernet port on the back.

This document covers every camera currently listed on [store.ui.com](https://store.ui.com), grouped by product family, with the specifications that actually determine what a given camera can see and what it costs you in switch budget.

> **Scope note.** Ubiquiti files sensors, sirens, speakers, alarm hubs, and the SuperLink family under the same "Physical Security" store category. Those are alarm and detection products, not cameras, and are excluded here. Recorders (UNVR, ENVR, Cloud Gateways) appear only where camera capacity is relevant to scale.

## How to Read the Numbers

Ubiquiti publishes about thirty specifications per camera. Roughly nine of them change a buying decision.

**Sensor size matters more than megapixel count, and the lineup is 8MP almost everywhere.** Nearly every current 4K UniFi camera resolves the same 3840 × 2160 at 8MP. What differs is the physical sensor gathering the light: the G6 Pro and G6 Edge families use a 1/1.2" sensor, the standard G6 family and the AI Turret/Dome use 1/1.8", the AI Multi Sensor pair uses 1/2.8" per lens, and the older G5 Pro uses 1/2". A 1/1.2" sensor has roughly 2.2× the area of a 1/1.8", which is the entire reason a G6 Pro Turret ($479) exists next to a G6 Turret ($199) at identical output resolution. Buy sensor area, not pixels.

**The Ethernet port is the most-missed specification in the range, and it splits the G6 family in half.** The G6 Pro, G6 Edge, and every AI-series camera carry a **GbE** port. The standard G6 Turret, G6 Dome, G6 Bullet, G6 Mini Dome, G6 180, G6 Pro 360, and G6 PTZ carry a **10/100 MbE** port — as does the entire G5 range. All of them are 4K cameras. A 100 Mbps port is genuinely sufficient for a single H.265 4K stream plus a substream, but it constrains high-bitrate configurations and it forecloses future headroom on exactly the models most likely to be deployed in bulk.

**PoE class is the switch-budget constraint, and the spread is enormous.** UniFi cameras run from 4 W (G5 Bullet, G5 Flex, G5 Turret Ultra) to 51 W (both AI PTZ models). Standard PoE (802.3af) covers most of the G5 and standard-G6 range; PoE+ (802.3at) covers the G6 Pro, G6 Edge, G6 PTZ, AI Turret, AI LPR, and AI Multi Sensor 2; PoE++ (802.3bt) is required by the AI Multi Sensor 4 and both AI PTZ cameras. Twenty AI PTZ Industrials need a 1,020 W budget before you count anything else on the switch.

**The AI tier is a two-value field with real consequences.** Ubiquiti grades every camera as either **"AI Detections"** or **"Enhanced AI."** Both tiers do smart detections — people, vehicles, animals. Only Enhanced AI cameras carry on-board face recognition and license plate recognition. The G5 range is uniformly AI Detections; the G6 and AI ranges are uniformly Enhanced AI, with two instructive exceptions: the **G6 Pro 360** is AI Detections despite its $499 price, and the **AI LPR** is graded AI Detections because it does plates but not faces. If facial recognition is a requirement, the tier field is the only thing to read.

**IR range is a floodlight-free reach number, and accessories move it.** The published figures run 5 m (Doorbell Lite) → 6 m (G5 Flex, G6 Instant) → 9 m → 20 m → 30 m → 40 m (G6 Pro family, AI Turret/Dome) → 60 m (G6 Edge Bullet) → 100 m (both AI PTZ models). Three cameras extend it with a paid accessory: the G6 Pro Bullet reaches 60 m with the Pro Bullet Enhancer, and the AI Pro and G5 Pro reach 40 m with their respective Enhancers. The Enhancer also pushes the AI Pro from PoE to PoE+.

**Optical zoom is scarce, and "hybrid" is not optical.** Only nine cameras have any optical zoom at all: 2.33× (AI Multi Sensor 2 and 4), 2.36× (all six G6 Pro and G6 Edge models), 2× (G5 PTZ), 3× (AI Pro, AI LPR, G5 Pro), 22× (AI PTZ Industrial), and 31× (AI PTZ Precision). The G6 PTZ's headline "10× hybrid" is 5× optical plus 2× digital — it is a shorter-reach instrument than the number suggests.

**Weatherproofing has four grades and they are not interchangeable.** IP66 covers most outdoor models. IP65 applies to the AI Pro and G5 Pro. IPX4/IPX5 (G5 Dome, G5 Flex, G6 Instant, AI DSLR with its case, Doorbell Lite, G4 Doorbell Pro) means splash resistance only — "outdoor covered," not "outdoor exposed." Three models publish no ingress rating at all and are indoor-only: the G6 Mini Dome, the G5 Dome Ultra, and both AI Theta systems. Impact ratings run separately from IK04 through IK10; IK10 is what you want anywhere the camera is reachable by hand.

**On-board storage is the difference between a camera and a system.** Most G6 and AI cameras have a MicroSD slot for buffering during recorder outages. The **G6 Edge** family is the structural exception: it ships with 128 GB pre-installed and runs standalone with no NVR at all, which is why it costs roughly 45% more than the otherwise-identical G6 Pro. The AI Multi Sensor 4 goes further with two MicroSD slots plus an M.2 2280 SATA SSD bay.

**Recorder capacity is the real scalability ceiling.** No camera has a client limit; the recorder does. The published 4K camera counts run: UNVR Instant 6 → UNVR 18 → UNVR Pro 24 → UNVR G2 30 → UNVR G2 Pro 50 → Enterprise NVR 70 → Enterprise NVR Core 300. Plan the recorder before the cameras — a $299 UNVR caps out at eighteen 4K streams regardless of how many cameras you buy.

> **Pricing basis.** All prices below are Ubiquiti's own list prices on [store.ui.com](https://store.ui.com) (USD) and [uk.store.ui.com](https://uk.store.ui.com) (GBP) as of **30 August 2026**. The US store separately displays a higher "surcharge included" figure; the base list price is quoted here. Regional taxes are added at checkout. Stock status is noted per product where a model is currently sold out or unreleased.

## G6 Pro Series

The flagship fixed-lens tier. All three share the same platform: a 1/1.2" 8MP sensor, 2.36× optical zoom, adaptive IR to 40 m, a GbE port, PoE+ at 15 W, Enhanced AI with face and plate recognition, and a MicroSD slot. What you choose between is the enclosure — turret for aim-anywhere flexibility, dome for vandal resistance, bullet for reach and the Enhancer accessory.

### G6 Pro Turret

The default choice in the premium tier. A three-axis manually adjustable turret in an aluminum-alloy housing, with the large 1/1.2" sensor and a wide-to-tele optical range that covers 134° diagonal at the wide end and 52° at the tele end. Rated to −30 °C, which is the widest cold tolerance of any fixed G6.

- **Price:** $479 USD / £380 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/26e646b0-09d9-4f83-b9a2-398f55f31f28/959cdb9c-6699-4be9-b445-a8f7bd6d3e7f.png) · [angle](https://cdn.ecomm.ui.com/products/26e646b0-09d9-4f83-b9a2-398f55f31f28/bcd030d7-abb5-455a-bf5a-5335f5d57880.png) · [detail](https://cdn.ecomm.ui.com/products/26e646b0-09d9-4f83-b9a2-398f55f31f28/e479a641-e796-4cb1-a10c-cc3972d72152.png)

| Property        | Value                                                             |
|-----------------|-------------------------------------------------------------------|
| Resolution      | 8MP 3840 × 2160 (16:9) @ 30 FPS                                   |
| Sensor          | 1/1.2" 8MP                                                        |
| Lens            | F 5.85–13.8 mm, ƒ/1.5–ƒ/2.9 — 2.36× optical                       |
| Field of view   | Wide 113.8° H / 61.9° V / 134° D · Tele 45.5° H / 25.8° V / 52° D |
| IR night vision | 40 m (131 ft), adaptive IR LED                                    |
| AI tier         | Enhanced AI — face recognition, license plate recognition         |
| Uplink          | GbE RJ45                                                          |
| Power           | PoE+, 15 W max, 42.5–57 V DC                                      |
| Storage         | MicroSD slot                                                      |
| Environmental   | IP66, IK04, −30 to 50 °C                                          |
| Dimensions      | ⌀117.2 × 116.5 mm (⌀4.61 × 4.59"), 1.2 kg                         |

### G6 Pro Dome

The same optics and sensor in an **IK10** vandal-resistant dome with a stainless-steel mount. The trade is audio: the Dome has a microphone only, where the Turret and Bullet have two-way audio. Twenty dollars more than the Turret buys the impact rating and loses the speaker.

- **Price:** $499 USD / £395 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/ed2301d1-7bc1-4569-b257-0ed1df0ed4d5/a695c111-28c9-4b90-a73c-8352dc104bfd.png) · [angle](https://cdn.ecomm.ui.com/products/ed2301d1-7bc1-4569-b257-0ed1df0ed4d5/c7eb3894-5deb-45c2-9eae-289e9b85226b.png) · [mounted](https://cdn.ecomm.ui.com/products/ed2301d1-7bc1-4569-b257-0ed1df0ed4d5/68fbcf18-a372-4231-adb4-a8a047da5db7.png)

| Property        | Value                                                             |
|-----------------|-------------------------------------------------------------------|
| Resolution      | 8MP 3840 × 2160 (16:9) @ 30 FPS                                   |
| Sensor          | 1/1.2" 8MP                                                        |
| Lens            | F 5.9–13.8 mm, ƒ/1.5–ƒ/2.9 — 2.36× optical                        |
| Field of view   | Wide 113.8° H / 61.9° V / 134° D · Tele 45.5° H / 25.8° V / 52° D |
| IR night vision | 40 m (131 ft), adaptive IR LED                                    |
| Audio           | Microphone only                                                   |
| AI tier         | Enhanced AI — face recognition, license plate recognition         |
| Uplink          | GbE RJ45                                                          |
| Power           | PoE+, 15 W max                                                    |
| Environmental   | IP66, **IK10**, −20 to 50 °C                                      |
| Dimensions      | ⌀163.8 × 108.8 mm (⌀6.45 × 4.28"), 1.2 kg                         |

### G6 Pro Bullet

The reach model. Same sensor and optics, but the only G6 Pro that accepts the **Pro Bullet Enhancer** ($179 / £145) — a long-range IR, floodlight, and radar-detection accessory that lifts night vision from 40 m to 60 m. Ships with wall, ceiling, and pole mounts.

- **Price:** $479 USD / £380 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/c48d3ada-c223-4a2b-98f3-2bb55c3b1a11/69792922-3944-41ab-a386-2059b23a6b1b.png) · [angle](https://cdn.ecomm.ui.com/products/c48d3ada-c223-4a2b-98f3-2bb55c3b1a11/6a1aacd1-5147-4f2b-897a-07414cc490eb.png) · [with mount](https://cdn.ecomm.ui.com/products/c48d3ada-c223-4a2b-98f3-2bb55c3b1a11/01688523-6db9-428b-8ae5-0931e4642c22.png)

| Property        | Value                                                   |
|-----------------|---------------------------------------------------------|
| Resolution      | 8MP 3840 × 2160 (16:9) @ 30 FPS                         |
| Sensor          | 1/1.2" 8MP                                              |
| Lens            | F 5.9–13.8 mm, ƒ/1.5–ƒ/2.9 — 2.36× optical              |
| IR night vision | 40 m (131 ft); **60 m (197 ft) with Vision Enhancer**   |
| Audio           | Two-way                                                 |
| AI tier         | Enhanced AI                                             |
| Uplink          | GbE RJ45                                                |
| Power           | PoE+, 15 W max, 37–57 V DC                              |
| Environmental   | IP66, IK04, −20 to 50 °C                                |
| Mounting        | Wall, ceiling, pole (1.5–2") included                   |
| Dimensions      | ⌀85.8 × 106.2 mm bare; ⌀85.8 × 210 mm with mount; 755 g |

## G6 Edge Series

The structurally most interesting family in the lineup and the newest: same 1/1.2" sensor and 2.36× optics as the G6 Pro, but built around a **Dual-core Arm Cortex-A76** processor, a pre-installed 128 GB MicroSD card, and Bluetooth — and rated to run **standalone with no NVR at all**, with AI-powered search happening on the camera. For single-camera sites and small deployments, the Edge removes the recorder from the bill of materials entirely.

All three are listed **Coming Soon** as of 30 August 2026; only the Turret and Bullet carry a published price, and only in the UK store.

### G6 Edge Turret

The standalone counterpart to the G6 Pro Turret. Identical sensor, optics, field of view, IR reach, and enclosure; the differences are the A76 processor, the bundled 128 GB card, Bluetooth, and a 25 W PoE+ draw (up from 15 W) to feed the on-board analytics.

- **Price:** £555 GBP; **not yet priced in the US store** (Coming Soon)
- **Images:** [front](https://cdn.ecomm.ui.com/products/802a5185-1d4a-45d1-b4bb-6b1d4036c52d/e545457a-b588-4afc-94a5-ffd86fc50b8d.png) · [angle](https://cdn.ecomm.ui.com/products/802a5185-1d4a-45d1-b4bb-6b1d4036c52d/a5406c11-293e-48c1-88a7-0bd7ee7be0a4.png) · [detail](https://cdn.ecomm.ui.com/products/802a5185-1d4a-45d1-b4bb-6b1d4036c52d/27c9cc81-b1e5-4db3-91da-5f48eec8307e.png)

| Property        | Value                                            |
|-----------------|--------------------------------------------------|
| Resolution      | 8MP 3840 × 2160 (16:9) @ 30 FPS                  |
| Sensor          | 1/1.2" 8MP                                       |
| Lens            | F 5.85–13.8 mm, ƒ/1.5–ƒ/2.9 — 2.36× optical      |
| IR night vision | 40 m (131 ft), adaptive                          |
| Processor       | Dual-core Arm Cortex-A76                         |
| Storage         | MicroSD, **128 GB pre-installed** — NVR optional |
| Uplink          | GbE RJ45 + Bluetooth                             |
| Power           | PoE+, **25 W** max                               |
| Audio           | Two-way                                          |
| Environmental   | IP66, IK04, −30 to 50 °C                         |
| Dimensions      | ⌀117.2 × 116.5 mm, 1.2 kg                        |

### G6 Edge Dome

The vandal-resistant standalone. IK10 dome housing, microphone only, and a 15 W draw — notably lower than the Turret and Bullet despite the same processor and storage.

- **Price:** not yet published in either the US or UK store (Coming Soon)
- **Images:** [front](https://cdn.ecomm.ui.com/products/78af4fc9-c880-4c19-84fe-29adbecc0827/68533c5d-bf5c-41dd-826c-e665d22d2e6e.png) · [angle](https://cdn.ecomm.ui.com/products/78af4fc9-c880-4c19-84fe-29adbecc0827/86613bff-0917-46ef-afac-0b01927620f2.png) · [mounted](https://cdn.ecomm.ui.com/products/78af4fc9-c880-4c19-84fe-29adbecc0827/4f12687b-3653-4748-928f-5d7cd4186bcf.png)

| Property        | Value                                      |
|-----------------|--------------------------------------------|
| Resolution      | 8MP 3840 × 2160 (16:9) @ 30 FPS            |
| Sensor          | 1/1.2" 8MP                                 |
| Lens            | F 5.9–13.8 mm, ƒ/1.5–ƒ/2.9 — 2.36× optical |
| IR night vision | 40 m (131 ft)                              |
| Processor       | Dual-core Arm Cortex-A76                   |
| Storage         | MicroSD, 128 GB pre-installed              |
| Uplink          | GbE RJ45 + Bluetooth                       |
| Power           | PoE+, 15 W max                             |
| Audio           | Microphone only                            |
| Environmental   | IP66, **IK10**, −20 to 50 °C               |
| Dimensions      | ⌀163.8 × 108.8 mm, 1.2 kg                  |

### G6 Edge Bullet

The longest unassisted IR reach of any fixed UniFi camera: **60 m natively**, without an Enhancer accessory. Physically larger than the G6 Pro Bullet (⌀107 mm versus ⌀85.8 mm) to house the bigger IR array, and it draws 25 W.

- **Price:** £555 GBP; not yet priced in the US store (Coming Soon)
- **Images:** [front](https://cdn.ecomm.ui.com/products/408f5d67-d421-4cc7-9786-d96f44f5b718/da100d4d-3f1a-4bb0-8282-6a67eaccfe57.png) · [angle](https://cdn.ecomm.ui.com/products/408f5d67-d421-4cc7-9786-d96f44f5b718/f223a6a1-125f-4bd0-93e8-46a3d9e4651d.png) · [with mount](https://cdn.ecomm.ui.com/products/408f5d67-d421-4cc7-9786-d96f44f5b718/88e2d289-0fc0-4ee2-a19d-c8b75519eda6.png)

| Property        | Value                                                 |
|-----------------|-------------------------------------------------------|
| Resolution      | 8MP 3840 × 2160 (16:9) @ 30 FPS                       |
| Sensor          | 1/1.2" 8MP                                            |
| Lens            | F 5.9–13.8 mm, ƒ/1.5–ƒ/2.9 — 2.36× optical            |
| IR night vision | **60 m (196 ft)** — highest of any fixed UniFi camera |
| Processor       | Dual-core Arm Cortex-A76                              |
| Storage         | MicroSD, 128 GB pre-installed                         |
| Uplink          | GbE RJ45                                              |
| Power           | PoE+, **25 W** max                                    |
| Audio           | Two-way                                               |
| Environmental   | IP66, IK04, −20 to 50 °C                              |
| Dimensions      | ⌀107 × 113 mm bare; ⌀107 × 212 mm with mount; 780 g   |

## G6 Series

The volume tier, and the best price-per-4K-camera in the range. All four use a 1/1.8" 8MP sensor, a fixed focal length, Enhanced AI with face and plate recognition, and a **10/100 MbE** port. There is no optical zoom and no MicroSD slot on the Turret, Dome, Bullet, or Mini Dome — these are recorder-dependent cameras. At $199 for a 4K camera with face recognition, the G6 Turret and G6 Bullet are the value anchors of the whole UniFi Protect line.

### G6 Turret

The default bulk-deployment camera. Three-axis manual adjustment, 30 m IR, 134° diagonal field of view, and standard PoE at 12.5 W. Rated to −30 °C. Currently **sold out in the UK store**.

- **Price:** $199 USD / £159 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/cc0e6f23-b53b-45b5-bff5-faf69f4fbf30/40f697ee-d5c1-4e96-b418-1c642ab1ea19.png) · [angle](https://cdn.ecomm.ui.com/products/cc0e6f23-b53b-45b5-bff5-faf69f4fbf30/5b876027-a290-4886-9e8d-d0be83691112.png) · [detail](https://cdn.ecomm.ui.com/products/cc0e6f23-b53b-45b5-bff5-faf69f4fbf30/2a996915-9183-4c90-8168-2605b10c6d5e.png)

| Property        | Value                                  |
|-----------------|----------------------------------------|
| Resolution      | 8MP 3864 × 2160 (16:9) @ 30 FPS        |
| Sensor          | 1/1.8" 8MP, fixed focal length         |
| Field of view   | 109.9° H / 56.7° V / 134.1° D          |
| IR night vision | 30 m (98 ft)                           |
| AI tier         | Enhanced AI — face + plate recognition |
| Uplink          | **10/100 MbE** RJ45                    |
| Power           | PoE, 12.5 W max                        |
| Audio           | Microphone                             |
| Environmental   | IP66, IK04, −30 to 50 °C               |
| Dimensions      | ⌀100 × 95 mm (⌀3.9 × 3.7"), 550 g      |

### G6 Dome

The IK10 version of the same camera, at an $80 premium. Aluminum-alloy enclosure, polycarbonate mount, and the lowest power draw in the G6 family at 9.25 W.

- **Price:** $279 USD / £220 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/2f241968-e3be-42c2-98fc-7d879b360d25/1e661d71-79eb-4f79-9f17-2199b3655bb4.png) · [angle](https://cdn.ecomm.ui.com/products/2f241968-e3be-42c2-98fc-7d879b360d25/07eef437-6bab-4c19-80d4-206b5a5d0634.png) · [mounted](https://cdn.ecomm.ui.com/products/2f241968-e3be-42c2-98fc-7d879b360d25/4191c910-6535-47ac-a5ef-2cc889a0ff90.png)

| Property        | Value                           |
|-----------------|---------------------------------|
| Resolution      | 8MP 3840 × 2160 (16:9) @ 30 FPS |
| Sensor          | 1/1.8" 8MP, fixed focal length  |
| Field of view   | 109.9° H / 56.7° V / 134.1° D   |
| IR night vision | 30 m (98 ft)                    |
| AI tier         | Enhanced AI                     |
| Uplink          | 10/100 MbE RJ45                 |
| Power           | PoE, 9.25 W max                 |
| Environmental   | IP66, **IK10**, −20 to 50 °C    |
| Dimensions      | ⌀144.7 × 96.3 mm, 820 g         |

### G6 Bullet

Identical internals to the G6 Turret in a bullet housing with wall, ceiling, and pole mounts included. 9.9 W, and the cheapest outdoor-exposed 4K camera Ubiquiti sells alongside the Turret.

- **Price:** $199 USD / £159 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/5dc40311-0e08-4eaa-b901-472cc707b436/4dee8c08-78b7-418f-ab85-01329267c062.png) · [angle](https://cdn.ecomm.ui.com/products/5dc40311-0e08-4eaa-b901-472cc707b436/7769ee13-3a54-43bc-93d1-d26ad9d9d0d0.png) · [with mount](https://cdn.ecomm.ui.com/products/5dc40311-0e08-4eaa-b901-472cc707b436/40c08ca9-4577-44f5-92c8-2c5cde81c560.png)

| Property        | Value                                              |
|-----------------|----------------------------------------------------|
| Resolution      | 8MP 3840 × 2160 (16:9) @ 30 FPS                    |
| Sensor          | 1/1.8" 8MP, fixed focal length                     |
| IR night vision | 30 m (98 ft)                                       |
| AI tier         | Enhanced AI                                        |
| Uplink          | 10/100 MbE RJ45                                    |
| Power           | PoE, 9.9 W max                                     |
| Environmental   | IP66, IK04, −20 to 50 °C                           |
| Mounting        | Wall, ceiling, pole (1.5–2") included              |
| Dimensions      | ⌀82 × 88.8 mm bare; ⌀82 × 153 mm with mount; 587 g |

### G6 Mini Dome

The smallest 4K camera in the lineup at ⌀100.8 × 56 mm and 254 g — roughly a third the mass of a G6 Dome. **Indoor only**: it publishes no ingress rating, and its operating range stops at 40 °C. IK08 impact rating and two-way audio make it a lobby, corridor, and retail-floor camera.

- **Price:** $299 USD / £239 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/f49b5512-674f-4f96-89cb-8e8dcd6b1aa6/5f7ae61c-7c8c-4c2a-b54b-ba2a268ab23a.png) · [angle](https://cdn.ecomm.ui.com/products/f49b5512-674f-4f96-89cb-8e8dcd6b1aa6/0dea4834-9d26-43cc-bcd2-9a5ec214c595.png) · [mounted](https://cdn.ecomm.ui.com/products/f49b5512-674f-4f96-89cb-8e8dcd6b1aa6/a150d405-bf2c-4315-88a2-fb4699d54a5d.png)

| Property        | Value                                              |
|-----------------|----------------------------------------------------|
| Resolution      | 8MP 3840 × 2160 (16:9) @ 30 FPS                    |
| Sensor          | 1/1.8" 8MP, fixed focal length                     |
| IR night vision | 20 m (65 ft)                                       |
| Audio           | Two-way                                            |
| AI tier         | Enhanced AI                                        |
| Uplink          | 10/100 MbE RJ45                                    |
| Power           | PoE, 9 W max                                       |
| Environmental   | **Indoor only** (no IP rating), IK08, −20 to 40 °C |
| Dimensions      | ⌀100.8 × 56 mm (⌀4 × 2.2"), 254 g                  |

## AI Series

Ubiquiti's specialist fixed cameras. The Turret and Dome are the previous-generation flagships, now positioned below the G6 Pro; the Pro, LPR, and DSLR are purpose-built instruments with no equivalent elsewhere in the range. All carry GbE ports.

### AI Turret

A 4K turret with the deepest IR in the fixed non-Pro range — 40 m — plus visible LEDs for color night vision and two-way audio. IK08, GbE, and PoE+ at 20 W. Requires a MicroSD card of at least 64 GB if you use local buffering.

- **Price:** $399 USD / £315 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/995b6a91-fab1-4c15-b5b9-6dfdede19bab/781cbab2-de3b-4aa5-a2e1-3c6ac570d520.png) · [angle](https://cdn.ecomm.ui.com/products/995b6a91-fab1-4c15-b5b9-6dfdede19bab/dfe8dd47-a968-425e-8993-24dd94115d3a.png) · [mounted](https://cdn.ecomm.ui.com/products/995b6a91-fab1-4c15-b5b9-6dfdede19bab/507a8857-4c84-4fab-99db-a857a296ec7d.png)

| Property        | Value                            |
|-----------------|----------------------------------|
| Resolution      | 8MP 3840 × 2160 (16:9) @ 30 FPS  |
| Sensor          | 1/1.8" 8MP, fixed focal length   |
| IR night vision | 40 m (131 ft), IR + visible LEDs |
| Audio           | Two-way                          |
| AI tier         | Enhanced AI                      |
| Uplink          | GbE RJ45                         |
| Power           | PoE+, 20 W max                   |
| Storage         | MicroSD (64 GB minimum)          |
| Environmental   | IP66, IK08, −30 to 50 °C         |
| Dimensions      | ⌀118 × 111 mm, 990 g             |

### AI Dome

The same camera in an IK10 dome, at exactly the same price — and it drops to standard **PoE at 10 W**, half the Turret's draw. For dense ceiling deployments on a constrained switch budget, this is the most power-efficient Enhanced-AI 4K camera Ubiquiti sells.

- **Price:** $399 USD / £315 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/1d22f247-f5bb-40ce-a465-5d885760f335/97f18708-5a3c-413b-9616-7987771de399.png) · [angle](https://cdn.ecomm.ui.com/products/1d22f247-f5bb-40ce-a465-5d885760f335/c337ad24-87e4-464c-afbf-b3299f1af381.png) · [mounted](https://cdn.ecomm.ui.com/products/1d22f247-f5bb-40ce-a465-5d885760f335/1a2ee41c-2cd9-47b9-b1ce-fcacd19d1e0a.png)

| Property        | Value                           |
|-----------------|---------------------------------|
| Resolution      | 8MP 3840 × 2160 (16:9) @ 30 FPS |
| Sensor          | 1/1.8" 8MP                      |
| IR night vision | 40 m (131 ft)                   |
| Audio           | Microphone                      |
| AI tier         | Enhanced AI                     |
| Uplink          | GbE RJ45                        |
| Power           | **PoE, 10 W max**               |
| Environmental   | IP66, **IK10**, −30 to 50 °C    |
| Dimensions      | ⌀118 × 90.8 mm, 700 g           |

### AI Pro

A 3× optical zoom bullet with the widest accessory ecosystem in the range. On its own it draws 11 W over standard PoE and reaches 25 m of IR; with the **AI Pro Enhancer** ($179 / £145) it gains long-range IR to 40 m, a floodlight, and radar detection, and moves to PoE+ at 22 W. IP65 rather than IP66, and a −20 to 40 °C range — the narrowest of the outdoor AI cameras.

- **Price:** $499 USD / £395 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/3b2997e9-38a5-499b-9b01-28acaf9bf6b7/3b579101-bfac-4a57-987b-f06d5fb9a3fd.png) · [angle](https://cdn.ecomm.ui.com/products/3b2997e9-38a5-499b-9b01-28acaf9bf6b7/8889e279-c8b3-4d97-89fb-9559d498c6bd.png) · [with mount](https://cdn.ecomm.ui.com/products/3b2997e9-38a5-499b-9b01-28acaf9bf6b7/0ca663f2-f696-45d4-91eb-45e2ee91c434.png)

| Property        | Value                                                       |
|-----------------|-------------------------------------------------------------|
| Resolution      | 8MP 3840 × 2160 (16:9) @ 30 FPS                             |
| Sensor          | 1/1.8" 8MP                                                  |
| Lens            | F 4.1–12.3 mm, ƒ/1.53–ƒ/3.3 — **3× optical**                |
| Field of view   | Wide 109.9° H / 59.9° V / 127.7° D · Zoom 34.9° H / 19.7° V |
| IR night vision | 25 m (82 ft); 40 m (131 ft) with Enhancer                   |
| AI tier         | Enhanced AI                                                 |
| Uplink          | GbE RJ45                                                    |
| Power           | PoE 11 W; **PoE+ 22 W with Enhancer**                       |
| Environmental   | IP65, IK04, −20 to 40 °C                                    |
| Dimensions      | ⌀86 × 112.6 mm bare; ⌀86 × 175.3 mm with mount; 675 g       |

### AI LPR

A single-purpose license plate camera, and physically the largest fixed model at 302.9 mm long. 3× optical zoom, an LPR-specific night optimization filter that captures both reflective and non-reflective plates in darkness, and tuning for vehicles moving up to 90 km/h. Ubiquiti grades it **AI Detections** rather than Enhanced AI: it does plates and smart detections, but not facial recognition.

- **Price:** $499 USD / £395 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/ad7cda21-71d0-4d92-a97e-93a0050b15ff/1fc68e16-2c0a-42bc-91ed-2b9e01a15316.png) · [angle](https://cdn.ecomm.ui.com/products/ad7cda21-71d0-4d92-a97e-93a0050b15ff/047469b4-796e-413d-857d-218e1fa8ff9c.png) · [mounted](https://cdn.ecomm.ui.com/products/ad7cda21-71d0-4d92-a97e-93a0050b15ff/8227d13c-fcc5-45cb-a382-5aa967efa71b.png)

| Property        | Value                                                   |
|-----------------|---------------------------------------------------------|
| Resolution      | 8MP 3840 × 2160 (16:9) @ 30 FPS                         |
| Sensor          | 1/1.8" 8MP                                              |
| Lens            | F 4.1–12.3 mm, ƒ/1.53–ƒ/3.3 — 3× optical                |
| Field of view   | Wide 109.9° H / 60° V / 127.7° D · Tele 35° H / 19.8° V |
| IR night vision | 15 m (50 ft) + LPR night optimization filter            |
| Rated for       | Plates on vehicles up to 90 km/h                        |
| AI tier         | AI Detections + license plate recognition (no faces)    |
| Uplink          | GbE RJ45                                                |
| Power           | PoE+, 25.5 W max                                        |
| Environmental   | IP66, IK04, −20 to 50 °C                                |
| Dimensions      | 130 × 151.4 × 302.9 mm, 1.5 kg                          |

### AI DSLR

The most unusual product Ubiquiti makes: a PoE+ camera body with a **Four Thirds 10MP CMOS sensor** and interchangeable **M. Zuiko Digital ED PRO** lenses — a 17 mm ƒ/1.2 wide and a 45 mm ƒ/1.2 telephoto. Output is still 8MP 4K at 30 FPS; the point is the optics and the low-light performance a ƒ/1.2 prime on a Four Thirds sensor delivers. Weatherproofing is IPX5 and only with the optional outdoor case. Currently **sold out in the US store**.

- **Price:** $1,499 USD / £1,195 GBP
- **Images:** [body](https://cdn.ecomm.ui.com/products/33b6a065-bb4f-4776-a701-991332d3d9ab/e883b766-503e-446b-be0e-cdd2e1caa575.png) · [with lens](https://cdn.ecomm.ui.com/products/33b6a065-bb4f-4776-a701-991332d3d9ab/c12e985d-48ff-4d5a-8371-60021a0b849a.png) · [outdoor case](https://cdn.ecomm.ui.com/products/33b6a065-bb4f-4776-a701-991332d3d9ab/802ed025-6ccd-4cc8-8787-25aa8ee3f16d.png)

| Property      | Value                                                                     |
|---------------|---------------------------------------------------------------------------|
| Resolution    | 8MP 3840 × 2160 (16:9) @ 30 FPS                                           |
| Sensor        | **4/3" 10MP CMOS**                                                        |
| Lenses        | M. Zuiko Digital ED 17 mm ƒ/1.2 PRO · M. Zuiko Digital ED 45 mm ƒ/1.2 PRO |
| Field of view | 17 mm: 52° H / 39° V / 65° D · 45 mm: 21.6° H / 16.2° V / 27° D           |
| Audio         | Two-way                                                                   |
| AI tier       | Enhanced AI                                                               |
| Uplink        | GbE RJ45                                                                  |
| Power         | PoE+, 17.77 W with lens working                                           |
| Environmental | IPX5 **only with the outdoor case**, −20 to 40 °C                         |
| Dimensions    | ⌀80 × 89 mm body; 660 g body, 390–410 g per lens                          |

## Panoramic and Multi-Sensor Cameras

Cameras that replace two to four conventional units with one drop. The trade is consistently frame rate — panoramic and multi-sensor models run 20–24 FPS rather than 30 — and, on the Ubiquiti-branded panoramics, a lower AI tier.

### G6 Pro 360

A 12MP fisheye covering a full 180° in every axis, output as a 3504 × 3504 square image with digital pan-tilt-zoom applied in Protect. Smart IR divides the 15 m illumination into four independently controllable zones so a bright nearby surface does not wash out the rest of the frame. **AI Detections only** — no face or plate recognition, despite the $499 price.

- **Price:** $499 USD / £395 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/43852c61-28db-41fd-8907-c283fffa41af/7db1cc7a-d786-4e90-8dc7-511dcb5f40d1.png) · [angle](https://cdn.ecomm.ui.com/products/43852c61-28db-41fd-8907-c283fffa41af/11836f28-cc0a-4369-aea3-a666c889944b.png) · [mounted](https://cdn.ecomm.ui.com/products/43852c61-28db-41fd-8907-c283fffa41af/12031a95-8944-46da-ae7b-917d2d4cf7a3.png)

| Property        | Value                                            |
|-----------------|--------------------------------------------------|
| Resolution      | 12MP 3504 × 3504 (1:1) @ 24 FPS                  |
| Sensor          | 1/1.6" 12MP, fisheye lens                        |
| Field of view   | 180° H / 180° V / 180° D                         |
| IR night vision | 15 m (50 ft), Smart IR with 4 controllable zones |
| Audio           | Two-way                                          |
| AI tier         | **AI Detections only**                           |
| Uplink          | 10/100 MbE RJ45                                  |
| Power           | PoE+, 13.5 W max                                 |
| Environmental   | IP66, IK10, −30 to 50 °C                         |
| Dimensions      | ⌀147 × 65.5 mm, 610 g                            |

### G6 180

Two 1/1.8" 8MP sensors stitched into a single 7680 × 2160 panorama — a 3.5:1 image 180° wide and 56.7° tall, at 20 FPS. Unlike the Pro 360 it carries **Enhanced AI**, so faces and plates work across the full panorama. An optional G6 180 Enhancer ($129 / £105) adds long-range IR, a floodlight, and a buzzer.

- **Price:** $299 USD / £239 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/c2196251-739c-43d0-b840-567a732cd7ef/cc48eefc-2995-4d40-add0-7a36f9e2325e.png) · [angle](https://cdn.ecomm.ui.com/products/c2196251-739c-43d0-b840-567a732cd7ef/38c5b890-3886-43a9-9ca4-b4298eb7587e.png) · [mounted](https://cdn.ecomm.ui.com/products/c2196251-739c-43d0-b840-567a732cd7ef/4e7f3df7-b623-488d-ada1-c44305fda8f4.png)

| Property        | Value                                 |
|-----------------|---------------------------------------|
| Resolution      | 16MP 7680 × 2160 (3.5:1) @ 20 FPS     |
| Sensors         | **Dual** 1/1.8" 8MP, two fixed lenses |
| Field of view   | 180° H / 56.7° V                      |
| IR night vision | 20 m (65 ft)                          |
| Audio           | Two-way                               |
| AI tier         | Enhanced AI                           |
| Uplink          | 10/100 MbE RJ45                       |
| Power           | PoE+, 15 W max                        |
| Storage         | MicroSD slot                          |
| Environmental   | IP66, IK04, −20 to 50 °C              |
| Dimensions      | 136 × 64 × 92 mm, 839 g               |

### AI Multi Sensor 2

Two independently aimable 8MP lenses, each with 2.33× optical zoom, in one PoE+ housing. 16MP total across two 4K streams at 30 FPS, IK10, and a conduit-pipe adapter in the box. The use case is a corner or a gate where you need one cable and two genuinely different views.

- **Price:** $699 USD / £555 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/aee5feaf-475b-43cc-bfdf-df56b22bf48b/3b486708-3647-4583-ad15-5376af9f0464.png) · [angle](https://cdn.ecomm.ui.com/products/aee5feaf-475b-43cc-bfdf-df56b22bf48b/99857aa1-7306-43b3-9cd2-14591d6e81ac.png) · [mounted](https://cdn.ecomm.ui.com/products/aee5feaf-475b-43cc-bfdf-df56b22bf48b/e284ec3d-1de6-48f1-a51f-51aa5980e05e.png)

| Property        | Value                                                       |
|-----------------|-------------------------------------------------------------|
| Resolution      | (2) 8MP 3840 × 2160 @ 30 FPS — 16MP total                   |
| Sensors         | (2) 1/2.8" 8MP                                              |
| Lenses          | (2) F 3.18–7.42 mm, ƒ/1.8–ƒ/2.8 — **2.33× optical each**    |
| Field of view   | Wide 108.8° H / 57.6° V / 130.8° D · Tele 42.8° H / 24.1° V |
| IR night vision | 20 m (65 ft), adaptive                                      |
| AI tier         | Enhanced AI                                                 |
| Uplink          | GbE RJ45                                                    |
| Power           | PoE+, 25 W max                                              |
| Environmental   | IP66, IK10, −20 to 50 °C                                    |
| Dimensions      | 218 × 132 × 93.5 mm, 1.3 kg                                 |

### AI Multi Sensor 4

Four independently aimable 8MP lenses — 32MP total — with 360° IR coverage and sixteen adaptive IR LEDs. The most storage-capable camera in the range: two MicroSD slots plus an M.2 2280 SATA SSD bay. PoE++ at 34.6 W and 2.4 kg. One of these replaces four conventional cameras and four cable runs at a roughly comparable total cost, which is the entire argument for it.

- **Price:** $1,799 USD / £1,430 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/06bfe79f-2abf-46fd-8810-b12ed69f6375/d29dfaad-19d9-4126-86b6-f8dce04b7145.png) · [angle](https://cdn.ecomm.ui.com/products/06bfe79f-2abf-46fd-8810-b12ed69f6375/8b2f6523-09f1-4b7c-a50d-718218d35b74.png) · [mounted](https://cdn.ecomm.ui.com/products/06bfe79f-2abf-46fd-8810-b12ed69f6375/18c53348-067c-4c86-af9b-0bfe1aabd4b2.png)

| Property        | Value                                                |
|-----------------|------------------------------------------------------|
| Resolution      | (4) 8MP 3840 × 2160 @ 24 FPS — **32MP total**        |
| Sensors         | (4) 1/2.8" 8MP                                       |
| Lenses          | (4) F 3.18–7.42 mm, ƒ/1.8–ƒ/2.8 — 2.33× optical each |
| IR night vision | 20 m (65 ft), (16) LEDs, **360° coverage**           |
| AI tier         | Enhanced AI                                          |
| Uplink          | GbE RJ45                                             |
| Power           | **PoE++, 34.6 W max**                                |
| Storage         | (2) MicroSD + (1) M.2 2280 SATA SSD                  |
| Environmental   | IP66, IK10, −20 to 50 °C                             |
| Dimensions      | ⌀255 × 105 mm, 2.4 kg                                |

### AI Theta

A modular system rather than a single camera: a 140 × 70 × 38 mm processing hub with a 0.96" display, connected by thin cable to one or more tiny remote lens heads (⌀22.8 × 43.5 mm, 15 g). The kit ships with the hub plus two lenses. Lens options are wide-angle, long-distance, and 360°; face and plate recognition work with the wide-angle and long-distance lenses, not the 360°. Additional lenses are $59 / £47 each, two-way audio is a $69 / £55 module, and the hub alone is $199 / £159. Indoor only.

- **Price:** $299 USD / £229 GBP (hub + 2 lenses)
- **Images:** [system](https://cdn.ecomm.ui.com/products/81578fff-57b1-4c6c-88c9-facf57670f7f/4f3e760b-f857-49a0-a3ed-2a46dd6ae000.png) · [hub](https://cdn.ecomm.ui.com/products/81578fff-57b1-4c6c-88c9-facf57670f7f/5a06a6f2-1093-47cd-9d06-ae0e083c3aa4.png) · [lens](https://cdn.ecomm.ui.com/products/81578fff-57b1-4c6c-88c9-facf57670f7f/27ef30a6-8dad-42b1-b91e-99a91d08df50.png)

| Property      | Value                                                                           |
|---------------|---------------------------------------------------------------------------------|
| Resolution    | Wide lens: 8MP 3264 × 2448 (4:3) @ 24 FPS · 360° lens: 6MP 2560 × 2560 @ 20 FPS |
| Sensors       | Wide lens 8MP CMOS · 360° lens 12MP CMOS                                        |
| Field of view | 97.5° H / 79.4° V / 118.2° D (wide lens)                                        |
| Audio         | Optional AI Theta Audio module (two-way)                                        |
| AI tier       | Enhanced AI — face + plate with wide/long-distance lenses only                  |
| Uplink        | GbE RJ45 (on hub)                                                               |
| Power         | PoE, 12.5 W max; hub supplies 5 V 1 A per lens port                             |
| Environmental | **Indoor only**, −20 to 40 °C                                                   |
| Dimensions    | Hub 140 × 70 × 38 mm (330 g) · lens ⌀22.8 × 43.5 mm (15 g)                      |

### AI Theta Pro

The same hub architecture with substantially better optics: a 1/1.8" 8MP sensor per lens head and a true 180°-in-every-axis 360° lens, output as 2160 × 2160 at 24 FPS. The Pro lens heads are larger (⌀36.6 × 58.9 mm) and cost $129 / £105 each. Designed to be flush-mounted into a ceiling so that only the lens is visible.

- **Price:** $329 USD / £260 GBP (hub + 360° Pro lens)
- **Images:** [system](https://cdn.ecomm.ui.com/products/d90f6e37-084a-45e9-aa4e-d561daee15fd/03f2b854-925f-45e2-8f74-2296573ccbf2.png) · [hub](https://cdn.ecomm.ui.com/products/d90f6e37-084a-45e9-aa4e-d561daee15fd/8d711efd-6ebb-47da-852c-8825852ca551.png) · [flush mount](https://cdn.ecomm.ui.com/products/d90f6e37-084a-45e9-aa4e-d561daee15fd/d0d2926a-a0b3-4c13-9ac7-620c1689ad01.png)

| Property      | Value                                                             |
|---------------|-------------------------------------------------------------------|
| Resolution    | 4MP 2160 × 2160 (1:1) @ 24 FPS                                    |
| Sensor        | **1/1.8" 8MP CMOS** per lens head                                 |
| Field of view | 180° H / 180° V / 180° D                                          |
| Audio         | Optional AI Theta Audio module                                    |
| AI tier       | Enhanced AI — face + plate with wide-angle / long-distance lenses |
| Uplink        | GbE RJ45 (on hub)                                                 |
| Power         | PoE, 12.5 W max                                                   |
| Environmental | Indoor only, −20 to 40 °C                                         |
| Dimensions    | Hub 140 × 70 × 38 mm · Pro lens ⌀36.6 × 58.9 mm (58 g)            |

## PTZ Cameras

Four models spanning a 6.7× price range and a 15× difference in optical reach. The two AI PTZ models are industrial instruments: endless 360° pan, 100 m IR, PoE++ at 51 W, and −40 °C operation.

### G6 PTZ

A compact dual-lens PTZ: a wide 1/1.8" 8MP sensor and a separate tele 1/1.8" 8MP sensor, marketed as 10× hybrid (5× optical, 2× digital). 350° pan and 100° tilt with ultra-low-latency control and motion tracking. The catch is the **10/100 MbE** port on a $399 4K PTZ.

- **Price:** $399 USD / £315 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/8b0777b6-851e-45f8-b4d2-b98b598c1ebc/117bc92a-0154-43fe-9db4-59ba8be2ed3f.png) · [angle](https://cdn.ecomm.ui.com/products/8b0777b6-851e-45f8-b4d2-b98b598c1ebc/99ca2edb-f32a-405b-919e-572015028359.png) · [mounted](https://cdn.ecomm.ui.com/products/8b0777b6-851e-45f8-b4d2-b98b598c1ebc/4bb64270-b756-4e8a-b6e2-9f22d6e2411b.png)

| Property        | Value                                                                  |
|-----------------|------------------------------------------------------------------------|
| Resolution      | 8MP 3840 × 2160 (16:9) @ 30 FPS                                        |
| Sensors         | Wide 1/1.8" 8MP (F 4.46 mm ƒ/1.65) · Tele 1/1.8" 8MP (F 16.3 mm ƒ/2.4) |
| Zoom            | 10× hybrid = **5× optical + 2× digital**                               |
| Pan / tilt      | 350° / 100°                                                            |
| Field of view   | Wide 109.9° H / 56.7° V · Tele 26.6° H / 15.1° V                       |
| IR night vision | 30 m (98 ft)                                                           |
| AI tier         | Enhanced AI                                                            |
| Uplink          | **10/100 MbE** RJ45                                                    |
| Power           | PoE+, 24.5 W max                                                       |
| Environmental   | IP66, IK04, −30 to 50 °C                                               |
| Dimensions      | ⌀107.2 × 104.5 × 203.2 mm, 1 kg                                        |

### AI PTZ Industrial

A 22× optical zoom instrument reaching a 3° horizontal tele field of view, with 100 m IR and endless 360° pan. Rated −40 to 50 °C, PoE++ at 51 W, 3.8 kg. Ships with wall, pole, and desk mounts.

- **Price:** $1,299 USD / £1,035 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/be9d2241-a2db-4a66-bba5-f6cbe4703165/ea8cb7bc-08e1-45f4-9329-798fde380f91.png) · [angle](https://cdn.ecomm.ui.com/products/be9d2241-a2db-4a66-bba5-f6cbe4703165/c8a5e391-ec73-462c-be6e-4333b118995a.png) · [mounted](https://cdn.ecomm.ui.com/products/be9d2241-a2db-4a66-bba5-f6cbe4703165/581e3cb9-b0f9-47f4-85b3-d90cf25ab19d.png)

| Property        | Value                                            |
|-----------------|--------------------------------------------------|
| Resolution      | 8MP 3840 × 2160 (16:9) @ 30 FPS                  |
| Sensor          | 1/1.8" 8MP                                       |
| Lens            | F 6.36–138.5 mm, ƒ/1.5–ƒ/3.4 — **22× optical**   |
| Field of view   | Wide 59.8° H / 44.4° V · **Tele 3° H / 2.24° V** |
| Pan / tilt      | **360° endless** / 120°                          |
| IR night vision | 100 m (328 ft)                                   |
| AI tier         | Enhanced AI                                      |
| Uplink          | GbE RJ45                                         |
| Power           | **PoE++, 51 W max**                              |
| Environmental   | IP66, **−40 to 50 °C**                           |
| Dimensions      | 207 × 223.7 × 341.3 mm, 3.8 kg                   |

### AI PTZ Precision

The flagship. 31× optical zoom to a 1.98° horizontal tele field of view, plus **LiDAR** for faster autofocus — the only UniFi camera with a ranging sensor. Same 100 m IR, endless pan, PoE++ and −40 °C rating as the Industrial, in a 5.5 kg housing that takes 2–6" poles. Ubiquiti lists a "Lightning Zoom" capability as Coming Soon.

- **Price:** $1,999 USD / £1,590 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/188dd3fc-5988-4878-81c5-473f9dcdf017/d56c85a3-ff72-496f-a3fc-785ebad7a5b5.png) · [angle](https://cdn.ecomm.ui.com/products/188dd3fc-5988-4878-81c5-473f9dcdf017/52334e6f-8a8b-4a8f-8d37-18d15cc02929.png) · [mounted](https://cdn.ecomm.ui.com/products/188dd3fc-5988-4878-81c5-473f9dcdf017/4fc3616c-1a92-4b54-a353-99317b6a436a.png)

| Property        | Value                                             |
|-----------------|---------------------------------------------------|
| Resolution      | 8MP 3840 × 2160 (16:9) @ 30 FPS                   |
| Sensor          | 1/1.8" 8MP                                        |
| Lens            | F 6.91–214.64 mm, ƒ/1.36–ƒ/4.6 — **31× optical**  |
| Field of view   | Wide 59° H / 34.1° V · **Tele 1.98° H / 1.12° V** |
| Autofocus       | **LiDAR-assisted**                                |
| Pan / tilt      | 360° endless / 120°                               |
| IR night vision | 100 m (328 ft), adaptive                          |
| AI tier         | Enhanced AI                                       |
| Uplink          | GbE RJ45                                          |
| Power           | PoE++, 51 W max                                   |
| Environmental   | IP66, −40 to 50 °C                                |
| Dimensions      | ⌀241 × 349 mm, 5.5 kg                             |

### G5 PTZ

The entry PTZ: 2K output, 2× optical zoom, 350° pan and 100° tilt, and IR plus white LEDs for color night vision to 20 m. PoE+ at 14 W and 580 g, with the widest mounting-accessory range of the four (surface, in-ceiling, pendant, corner, conduit).

- **Price:** $299 USD / £239 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/e3cbecf3-07dc-4f09-82e1-b88dca942d7a/f853df6e-2252-4da1-b4a9-38dc327a67ef.png) · [angle](https://cdn.ecomm.ui.com/products/e3cbecf3-07dc-4f09-82e1-b88dca942d7a/8d38104c-c265-42dd-9266-8a2de2ad3e93.png) · [mounted](https://cdn.ecomm.ui.com/products/e3cbecf3-07dc-4f09-82e1-b88dca942d7a/3b66e069-8ea9-4814-86a3-3df70a43cd2c.png)

| Property        | Value                                     |
|-----------------|-------------------------------------------|
| Resolution      | 4MP 2688 × 1512 (16:9) @ 30 FPS           |
| Sensor          | 5MP 1/2.7" CMOS                           |
| Lens            | F 3.42–6.85 mm, ƒ/1.85–ƒ/2.4 — 2× optical |
| Pan / tilt      | 350° / 100°                               |
| IR night vision | 20 m (65 ft), IR + white LED              |
| AI tier         | AI Detections                             |
| Uplink          | 10/100 MbE RJ45                           |
| Power           | PoE+, 14 W max                            |
| Environmental   | IP66, IK04, −30 to 45 °C                  |
| Dimensions      | ⌀90 × 152.5 mm bare, 580 g                |

## G5 Series

The budget tier, and the price floor of the range. All six are 2K (4MP 2688 × 1512), all are **AI Detections** — smart detections without face or plate recognition — and all use 10/100 MbE ports. Power draw is 4–5 W across the board except the G5 Pro. These remain the cheapest way to add a camera to a UniFi Protect site.

### G5 Pro

The odd one out: 4K rather than 2K, 3× optical zoom, and an 8MP 1/2" sensor. Functionally an AI Pro with a smaller sensor, no two-way audio, a 10/100 port, and AI Detections instead of Enhanced AI — for $120 less. The G5 Pro Enhancer ($99 / £79) adds long-range IR to 40 m and a floodlight.

- **Price:** $379 USD / £300 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/38dd06cd-7c6b-4d6e-8e64-981d504d468f/53e6f4ff-bf2d-43f2-bc40-efce98a8046c.png) · [angle](https://cdn.ecomm.ui.com/products/38dd06cd-7c6b-4d6e-8e64-981d504d468f/c275c58c-a97a-444c-82ab-a468e1620713.png) · [mounted](https://cdn.ecomm.ui.com/products/38dd06cd-7c6b-4d6e-8e64-981d504d468f/a4db7c13-e7a3-466e-8e3c-74521526b3b3.png)

| Property        | Value                                            |
|-----------------|--------------------------------------------------|
| Resolution      | 8MP 3840 × 2160 (16:9) @ 30 FPS                  |
| Sensor          | 1/2" 8MP                                         |
| Lens            | F 4.1–12.3 mm, ƒ/1.53–ƒ/3.3 — 3× optical         |
| IR night vision | 25 m (82 ft); 40 m (131 ft) with Enhancer        |
| AI tier         | **AI Detections** — no face or plate recognition |
| Uplink          | 10/100 MbE RJ45                                  |
| Power           | PoE 10 W; PoE+ 12.95 W with Enhancer             |
| Environmental   | IP65, IK04, −20 to 50 °C                         |
| Dimensions      | ⌀86 × 154.3 mm, 650 g                            |

### G5 Turret Ultra

At $129 / £79 this is the cheapest outdoor-exposed camera Ubiquiti sells, and it carries 30 m of IR — more than the $199 G6 Turret's peers in its own family and equal to the G6 Turret itself. IP66, IK04, 4 W, 330 g, and rated to −30 °C.

- **Price:** $129 USD / £79 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/54f9bb29-40b8-4dac-9b1c-7ccf0e303aa5/4291a8f4-7390-4bb6-9190-8953739a0a92.png) · [angle](https://cdn.ecomm.ui.com/products/54f9bb29-40b8-4dac-9b1c-7ccf0e303aa5/c1eec67f-0b70-4c6b-af18-b19c05dbb841.png) · [mounted](https://cdn.ecomm.ui.com/products/54f9bb29-40b8-4dac-9b1c-7ccf0e303aa5/327ff0a7-1cfd-4a6b-b2f9-738e9c7279dd.png)

| Property        | Value                           |
|-----------------|---------------------------------|
| Resolution      | 4MP 2688 × 1512 (16:9) @ 30 FPS |
| Sensor          | 1/2.4" CMOS, fixed focal length |
| Field of view   | 102.4° H / 54.9° V / 120.6° D   |
| IR night vision | 30 m (98 ft)                    |
| AI tier         | AI Detections                   |
| Uplink          | 10/100 MbE RJ45                 |
| Power           | PoE, **4 W max**                |
| Environmental   | IP66, IK04, −30 to 50 °C        |
| Dimensions      | ⌀90 × 71.2 mm, 330 g            |

### G5 Dome Ultra

The smallest camera in the range at ⌀63.6 × 68.2 mm and 175 g, with an optional flush mount that leaves almost nothing visible. 20 m IR and 4.2 W, but **indoor only** — no ingress rating, IK06, and a 40 °C ceiling.

- **Price:** $129 USD / £79 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/c2eb7a6f-7d6a-4b84-b1bd-7dc54e26c4e2/6417969a-f091-4db6-a8dc-7f7226e5968c.png) · [angle](https://cdn.ecomm.ui.com/products/c2eb7a6f-7d6a-4b84-b1bd-7dc54e26c4e2/8f147aaf-b2fc-4ab0-932f-e92a4281801d.png) · [flush mount](https://cdn.ecomm.ui.com/products/c2eb7a6f-7d6a-4b84-b1bd-7dc54e26c4e2/62eb286c-a481-422e-83f6-46f14fc08155.png)

| Property        | Value                               |
|-----------------|-------------------------------------|
| Resolution      | 4MP 2688 × 1512 (16:9) @ 30 FPS     |
| Sensor          | 1/2.4" CMOS                         |
| IR night vision | 20 m (65 ft)                        |
| AI tier         | AI Detections                       |
| Uplink          | 10/100 MbE RJ45                     |
| Power           | PoE, 4.2 W max                      |
| Environmental   | **Indoor only**, IK06, −20 to 40 °C |
| Dimensions      | ⌀63.6 × 68.2 mm, **175 g**          |

### G5 Dome

A ceiling dome with two-way audio and IK08, but only 9 m of IR and IPX4 splash resistance — "outdoor covered," meaning under a soffit, not on an exposed wall.

- **Price:** $179 USD / £140 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/1c20460a-b90b-4430-bbf1-8adf26ec2122/46b73c63-f1fb-4b07-844e-fa3315bb0327.png) · [angle](https://cdn.ecomm.ui.com/products/1c20460a-b90b-4430-bbf1-8adf26ec2122/4f2c84e7-71ff-4560-b952-d5d7923b9738.png) · [mounted](https://cdn.ecomm.ui.com/products/1c20460a-b90b-4430-bbf1-8adf26ec2122/18dad804-ee68-4358-ba54-2557a90c7347.png)

| Property        | Value                                      |
|-----------------|--------------------------------------------|
| Resolution      | 4MP 2688 × 1512 (16:9) @ 30 FPS            |
| Sensor          | 5MP CMOS                                   |
| IR night vision | 9 m (30 ft)                                |
| Audio           | Two-way                                    |
| AI tier         | AI Detections                              |
| Uplink          | 10/100 MbE RJ45                            |
| Power           | PoE, 5 W max                               |
| Environmental   | **IPX4 while covered**, IK08, −20 to 40 °C |
| Dimensions      | ⌀109.2 × 64.5 mm, 370 g                    |

### G5 Bullet

The narrowest field of view in the G5 line (84.4° horizontal) in an IP55 bullet with a 3/4" pole mount. 9 m IR, 4 W, 225 g.

- **Price:** $129 USD / £105 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/48ca8dea-109e-4d35-af46-b7ad03764207/0cd9d5ff-af60-45bd-a01f-b0fb050e76bb.png) · [angle](https://cdn.ecomm.ui.com/products/48ca8dea-109e-4d35-af46-b7ad03764207/1263e514-5539-4069-8fab-b26156505c86.png) · [with mount](https://cdn.ecomm.ui.com/products/48ca8dea-109e-4d35-af46-b7ad03764207/1b655029-fbfd-48d2-b8d8-6b1beefefdb9.png)

| Property        | Value                           |
|-----------------|---------------------------------|
| Resolution      | 4MP 2688 × 1512 (16:9) @ 30 FPS |
| Sensor          | 5MP CMOS                        |
| Field of view   | 84.4° H / 45.4° V / 99.8° D     |
| IR night vision | 9 m (30 ft)                     |
| AI tier         | AI Detections                   |
| Uplink          | 10/100 MbE RJ45                 |
| Power           | PoE, 4 W max                    |
| Environmental   | IP55, IK04, −20 to 40 °C        |
| Dimensions      | ⌀75.5 × 74.4 mm bare, 225 g     |

### G5 Flex

A ⌀48 mm cylinder that stands on a desk, screws to a wall, clamps to a 1–1.5" pole, or drops into a ceiling. 170 g, 4 W, 6 m IR, IPX4 while covered. The most placement-flexible camera in the range and a common choice for temporary or interior monitoring.

- **Price:** $129 USD / £105 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/af685346-47c7-41cf-a3c8-71ab26608dcd/c8249de8-79af-4b9b-8566-fdaedf6917b9.png) · [angle](https://cdn.ecomm.ui.com/products/af685346-47c7-41cf-a3c8-71ab26608dcd/3f874e90-0cfe-4379-8bc0-1de6c95307b5.png) · [mounted](https://cdn.ecomm.ui.com/products/af685346-47c7-41cf-a3c8-71ab26608dcd/1785ad18-4f1e-4d10-8c74-db93a4efc671.png)

| Property        | Value                                               |
|-----------------|-----------------------------------------------------|
| Resolution      | 4MP 2688 × 1512 (16:9) @ 30 FPS                     |
| Sensor          | 5MP CMOS                                            |
| IR night vision | 6 m (20 ft)                                         |
| AI tier         | AI Detections                                       |
| Uplink          | 10/100 MbE RJ45                                     |
| Power           | PoE, 4 W max                                        |
| Environmental   | IPX4 while covered, IK04, −20 to 40 °C              |
| Mounting        | Desktop, wall, pole (1–1.5"), hard ceiling included |
| Dimensions      | ⌀48 × 107.5 mm, 170 g                               |

## Instant (WiFi) Cameras

The only UniFi cameras that are not primarily PoE. Both are USB-powered plug-and-play units on 802.11ac WiFi with Bluetooth for onboarding; a PoE-to-USB-C adapter is available separately if you want to wire them.

### G6 Instant

A 4K camera the size of a matchbox — 81.7 × 50.1 × 57.2 mm, 180 g — with the same 1/1.8" 8MP sensor as the G6 Turret, two-way audio, a MicroSD slot, and **Enhanced AI** including face and plate recognition. At $179 it is the cheapest camera in the entire lineup that does facial recognition. IR reach is only 6 m and it draws 7 W over 5 V USB.

- **Price:** $179 USD / £140 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/16382cd9-95ca-4c26-ae58-3f2cf02c8b75/ac0e2657-769a-4921-ab11-c4eaa9584c13.png) · [angle](https://cdn.ecomm.ui.com/products/16382cd9-95ca-4c26-ae58-3f2cf02c8b75/f8ec5cf8-7158-4910-aec0-4e3c03351154.png) · [mounted](https://cdn.ecomm.ui.com/products/16382cd9-95ca-4c26-ae58-3f2cf02c8b75/bf81edbb-a0f4-45f1-8df9-fd7318894cb3.png)

| Property        | Value                                                            |
|-----------------|------------------------------------------------------------------|
| Resolution      | 8MP 3840 × 2160 (16:9) @ 30 FPS                                  |
| Sensor          | 1/1.8" 8MP                                                       |
| IR night vision | 6 m (20 ft)                                                      |
| Audio           | Two-way                                                          |
| AI tier         | **Enhanced AI** — cheapest face-recognition camera in range      |
| Uplink          | 802.11a/b/g/n/ac WiFi + Bluetooth                                |
| Power           | 5 V 2 A USB adapter included; PoE-to-USB-C adapter optional; 7 W |
| Storage         | MicroSD slot                                                     |
| Environmental   | IPX5, IK04, −20 to 40 °C                                         |
| Dimensions      | 81.7 × 50.1 × 57.2 mm, 180 g                                     |

### G4 Instant

The previous-generation 2K version, still the cheapest camera Ubiquiti sells at $99 / £75. AI Detections only, 6 m IR, two-way audio, 6 W. Currently **sold out in the US store**.

- **Price:** $99 USD / £75 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/45dbdf62-8b0c-4283-aeb9-34d8597e0154/48d5f7c7-c1dc-4118-80ce-c9e226a36d04.png) · [angle](https://cdn.ecomm.ui.com/products/45dbdf62-8b0c-4283-aeb9-34d8597e0154/a835e607-ee8e-448a-8723-ab5590d1c51e.png) · [mounted](https://cdn.ecomm.ui.com/products/45dbdf62-8b0c-4283-aeb9-34d8597e0154/97f22769-f57c-4b32-bb75-ab4754c87137.png)

| Property        | Value                             |
|-----------------|-----------------------------------|
| Resolution      | 4MP 2688 × 1512 (16:9) @ 30 FPS   |
| Sensor          | 5MP CMOS                          |
| IR night vision | 6 m (20 ft)                       |
| Audio           | Two-way                           |
| AI tier         | AI Detections                     |
| Uplink          | 802.11a/b/g/n/ac WiFi + Bluetooth |
| Power           | 5 V 2 A USB adapter included; 6 W |
| Environmental   | IPX5, IK04                        |
| Dimensions      | 81.6 × 50 × 47.2 mm               |

## Doorbell Cameras

Three SKUs, but effectively two products: the current PoE Doorbell Lite, and the older G4 Doorbell Pro in AC/WiFi and PoE-kit forms. Both G4 variants are **sold out in both stores** as of 30 August 2026.

### Doorbell Lite

The current-generation doorbell and a straightforward PoE device — no transformer, no chime wiring, no WiFi. A 5MP sensor in a portrait 3:4 aspect ratio (1920 × 2560) with a 175.9° diagonal field of view that sees a package on the doorstep and a face at the same time. Two-way audio with noise cancellation, 137 mm tall, 142 g, 8 W.

- **Price:** $99 USD / £79 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/cc3293dc-3fe5-48a6-87c1-ea1ea940f33c/d0ccc0e4-64a3-4d4a-85f6-341cd76310ea.png) · [angle](https://cdn.ecomm.ui.com/products/cc3293dc-3fe5-48a6-87c1-ea1ea940f33c/06f7252d-1a97-4139-833f-2cfd4d2df10c.png) · [mounted](https://cdn.ecomm.ui.com/products/cc3293dc-3fe5-48a6-87c1-ea1ea940f33c/3728e86c-dec0-4ada-bb47-13e0d22fd17c.png)

| Property        | Value                                       |
|-----------------|---------------------------------------------|
| Resolution      | 5MP 1920 × 2560 (**3:4 portrait**) @ 24 FPS |
| Sensor          | 1/2.7" 5MP                                  |
| Field of view   | 95.8° H / 131.2° V / 175.9° D               |
| IR night vision | 5 m (16 ft)                                 |
| Audio           | Two-way with noise cancellation             |
| AI tier         | AI Detections                               |
| Uplink          | 10/100 MbE                                  |
| Power           | **PoE**, 8 W max                            |
| Environmental   | IPX5, −30 to 50 °C                          |
| Dimensions      | 137 × 40 × 26.4 mm, 142 g                   |

### G4 Doorbell Pro

The dual-camera doorbell: a main 5MP sensor at 138° and a second 8MP **package camera** aimed down at the doorstep, plus an integrated display for on-device messaging, NFC card and keyfob access, and two-way intercom that ties into UniFi Access. Powered by existing 16–24 V AC doorbell wiring or USB-C, with WiFi and Bluetooth 5.0; Gigabit is possible only via the UACC-Adapter-DBPOE. **Sold out in both stores.**

- **Price:** $299 USD / £229 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/09c53756-12c3-4f3c-b0e7-f4268a0ad88f/5ac1509e-dae1-44a8-ad34-f2c1b0b94119.png) · [angle](https://cdn.ecomm.ui.com/products/09c53756-12c3-4f3c-b0e7-f4268a0ad88f/2c5d5d1c-e747-4817-a343-1aeebd8f0239.png) · [mounted](https://cdn.ecomm.ui.com/products/09c53756-12c3-4f3c-b0e7-f4268a0ad88f/34416fec-4200-4ebe-afe3-7120ecc39837.png)

| Property        | Value                                                         |
|-----------------|---------------------------------------------------------------|
| Resolution      | Main 2MP 1600 × 1200 (4:3) @ 30 FPS · Package 2MP 1600 × 1200 |
| Sensors         | Main 5MP CMOS · Package 8MP CMOS                              |
| Field of view   | Main 138° H / 114° V / 155° D · Package 97.5° H / 79.4° V     |
| IR night vision | 6 m (20 ft)                                                   |
| Access features | NFC card and keyfob, two-way intercom, integrated display     |
| AI tier         | AI Detections                                                 |
| Uplink          | 802.11a/b/g/n/ac WiFi + Bluetooth 5.0; GbE via adapter        |
| Power           | 16–24 V AC 1.25 A, or USB-C 5 V 2 A; 10 W max                 |
| Environmental   | IPX4, −10 to 40 °C                                            |
| Dimensions      | 160.6 × 51.7 × 28.7 mm, 253 g                                 |

### G4 Doorbell Pro PoE Kit

The same doorbell re-engineered for PoE and bundled with a PoE Smart Chime. Removes the transformer and the WiFi dependency entirely, widens the main camera to 160° with lens distortion correction off, and extends the temperature range to −30 °C. The device draws 7 W and the chime 3 W. **Sold out in both stores.**

- **Price:** $379 USD / £300 GBP
- **Images:** [front](https://cdn.ecomm.ui.com/products/217c2677-ceb5-4908-8953-792cdc72fd32/53757abd-20a1-42c5-bc98-7f0ba210a2b2.png) · [angle](https://cdn.ecomm.ui.com/products/217c2677-ceb5-4908-8953-792cdc72fd32/3aff9427-296d-42d9-9378-aa990b529e51.png) · [with chime](https://cdn.ecomm.ui.com/products/217c2677-ceb5-4908-8953-792cdc72fd32/43198283-e23c-42ec-97ed-8e1f9a053904.png)

| Property        | Value                                                             |
|-----------------|-------------------------------------------------------------------|
| Resolution      | Main 2MP 1600 × 1200 (4:3) @ 30 FPS · Package 2MP 1600 × 1200     |
| Sensors         | Main 5MP CMOS · Package 8MP CMOS                                  |
| Field of view   | Main 138° D-corrected / **160° H with LDC off** · Package 97.5° H |
| IR night vision | 6 m (20 ft)                                                       |
| Access features | NFC card and keyfob, two-way intercom, integrated display         |
| Uplink          | 10/100 MbE                                                        |
| Power           | **PoE**, 7 W (doorbell) + 3 W (chime)                             |
| In the box      | Doorbell, PoE Smart Chime, wall / wedge / on-wall mounts          |
| Environmental   | IPX4, −30 to 40 °C                                                |
| Dimensions      | 160.6 × 51.7 × 35.1 mm, 264 g                                     |

## Legacy and Discontinued Models

These models still appear in Ubiquiti's technical-specification archive and remain supported by UniFi Protect, but they are no longer listed for sale in the Physical Security store category and carry no current price.

| Model      | Resolution                     | Sensor                               | Power       | Notes                                                                     | Image                                                                                                                   |
|------------|--------------------------------|--------------------------------------|-------------|---------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------|
| G4 Pro     | 8MP 3840 × 2160 @ 50 FPS       | 1/2" 8MP, F 4.1–12.3 mm ƒ/1.53–ƒ/3.3 | PoE, 12.5 W | GbE port; the fastest frame rate ever shipped on a UniFi camera           | [view](https://cdn.ecomm.ui.com/products/952638fe-6e9d-4e43-aa39-790b7712c11c/9ed82ebf-e315-4549-97dd-2391dbe6fd2c.png) |
| G4 Dome    | 4MP 2688 × 1512 @ 24 FPS       | 5MP CMOS                             | PoE, 5 W    | Superseded by the G5 Dome                                                 | [view](https://cdn.ecomm.ui.com/products/eb76b35c-fa0a-4418-a876-643e5c3a31b7/fb1fd2ac-8785-4019-a16f-e3bf374f76aa.png) |
| G4 Bullet  | 4MP 2688 × 1512 @ 24 FPS       | 5MP CMOS                             | PoE, 4 W    | GbE port; accepts the G4 Bullet IR Enhancer                               | [view](https://cdn.ecomm.ui.com/products/0152e6dc-5701-49e2-bff7-dc87321742a8/4b567c26-501f-4afb-8944-35e09609d096.png) |
| AI 360     | 4MP 1920 × 1920 (1:1) @ 30 FPS | 5MP CMOS fisheye                     | PoE, 8.64 W | 360° indoor fisheye, IPX4 while covered, IK08; replaced by the G6 Pro 360 | [view](https://cdn.ecomm.ui.com/products/509a3a6f-f6d1-4a62-962c-b42e33d4955d/6e6aaa5b-9687-4bde-ad71-36c1493e521c.png) |
| G3 Flex    | 2MP 1920 × 1080 @ 25 FPS       | 2MP HDR, EFL 4 mm ƒ/2.0              | PoE, 4 W    | Replaced by the G5 Flex                                                   | [view](https://cdn.ecomm.ui.com/products/41b1bff2-10cf-45d0-be7f-46180e95e60d/3ff54778-615d-4d4a-b609-8cca1823b7aa.png) |
| G3 Instant | 1080p                          | —                                    | 5 V 2 A USB | WiFi; replaced by the G4 and G6 Instant                                   | [view](https://cdn.ecomm.ui.com/products/1ca80f22-3393-4711-a48d-47fa6f4d5ad4/fd05e793-5be2-4f88-960c-c8c779c3cd8e.png) |

## Camera-Adjacent Hardware

Not cameras, but hardware that changes what the cameras in a deployment can do. Listed here because the per-model specifications above reference them.

| Product                 | Price (USD / GBP) | What it does                                                                                                                             |
|-------------------------|-------------------|------------------------------------------------------------------------------------------------------------------------------------------|
| **AI Key**              | $799 / £635       | Edge AI appliance that adds proactive AI threat detection to any Protect deployment; rated for **1,800 smart-detection events per hour** |
| **AI Port**             | $199 / £159       | Adds AI detection, classification, and recognition to any UniFi **or third-party** camera                                                |
| **AI Pro Enhancer**     | $179 / £145       | Long-range IR, floodlight, and radar detection for the AI Pro — IR 25 m → 40 m                                                           |
| **Pro Bullet Enhancer** | $179 / £145       | Long-range IR, floodlight, and radar for the G6 Pro Bullet and AI Pro — IR 40 m → 60 m                                                   |
| **G6 180 Enhancer**     | $129 / £105       | Long-range IR, floodlight, and buzzer for the G6 180                                                                                     |
| **G5 Pro Enhancer**     | $99 / £79         | Long-range IR and floodlight for the G5 Pro — IR 25 m → 40 m                                                                             |
| **Protect Viewport**    | $199 / £159       | HDMI hub for displaying Protect camera feeds on a monitor or TV                                                                          |
| **Protect Floodlight**  | $99 / £79         | Motion-triggered light, controllable from Protect                                                                                        |
| **AI Theta Hub**        | $199 / £159       | Standalone processing hub for AI Theta lens heads                                                                                        |
| **PoE Smart Chime**     | $79 / £65         | PoE chime for any UniFi doorbell                                                                                                         |
| **WiFi Smart Chime**    | $59 / —           | Plug-in chime; US store only                                                                                                             |

## Summary

Thirty-six cameras across nine families. The shape of the range: a $99–$179 entry tier of 2K and WiFi cameras, a $199–$299 volume tier of fixed 4K cameras with Enhanced AI on 100 Mbps ports, a $399–$499 premium tier that adds gigabit and larger sensors, and a $699–$1,999 specialist tier of multi-sensor, interchangeable-lens, and long-zoom PTZ instruments.

Three things decide most purchases: whether you need **face and plate recognition** (Enhanced AI vs AI Detections), whether **100 Mbps is acceptable** on the uplink, and whether the **PoE class** fits the switch budget you already own.

| Camera                  | Category     | Resolution @ FPS      | Sensor          | IR          | Optical zoom    | Uplink      | Power                    | Weather / impact | AI tier          | USD    | GBP    |
|-------------------------|--------------|-----------------------|-----------------|-------------|-----------------|-------------|--------------------------|------------------|------------------|--------|--------|
| G6 Pro Turret           | G6 Pro       | 8MP 4K @ 30           | 1/1.2" 8MP      | 40 m        | 2.36×           | GbE         | PoE+ 15 W                | IP66 / IK04      | Enhanced         | $479   | £380   |
| G6 Pro Dome             | G6 Pro       | 8MP 4K @ 30           | 1/1.2" 8MP      | 40 m        | 2.36×           | GbE         | PoE+ 15 W                | IP66 / IK10      | Enhanced         | $499   | £395   |
| G6 Pro Bullet           | G6 Pro       | 8MP 4K @ 30           | 1/1.2" 8MP      | 40 m (60 m) | 2.36×           | GbE         | PoE+ 15 W                | IP66 / IK04      | Enhanced         | $479   | £380   |
| G6 Edge Turret          | G6 Edge      | 8MP 4K @ 30           | 1/1.2" 8MP      | 40 m        | 2.36×           | GbE + BT    | PoE+ 25 W                | IP66 / IK04      | Enhanced         | —      | £555   |
| G6 Edge Dome            | G6 Edge      | 8MP 4K @ 30           | 1/1.2" 8MP      | 40 m        | 2.36×           | GbE + BT    | PoE+ 15 W                | IP66 / IK10      | Enhanced         | —      | —      |
| G6 Edge Bullet          | G6 Edge      | 8MP 4K @ 30           | 1/1.2" 8MP      | **60 m**    | 2.36×           | GbE         | PoE+ 25 W                | IP66 / IK04      | Enhanced         | —      | £555   |
| G6 Turret               | G6           | 8MP 4K @ 30           | 1/1.8" 8MP      | 30 m        | —               | 10/100      | PoE 12.5 W               | IP66 / IK04      | Enhanced         | $199   | £159   |
| G6 Dome                 | G6           | 8MP 4K @ 30           | 1/1.8" 8MP      | 30 m        | —               | 10/100      | PoE 9.25 W               | IP66 / IK10      | Enhanced         | $279   | £220   |
| G6 Bullet               | G6           | 8MP 4K @ 30           | 1/1.8" 8MP      | 30 m        | —               | 10/100      | PoE 9.9 W                | IP66 / IK04      | Enhanced         | $199   | £159   |
| G6 Mini Dome            | G6           | 8MP 4K @ 30           | 1/1.8" 8MP      | 20 m        | —               | 10/100      | PoE 9 W                  | Indoor / IK08    | Enhanced         | $299   | £239   |
| AI Turret               | AI           | 8MP 4K @ 30           | 1/1.8" 8MP      | 40 m        | —               | GbE         | PoE+ 20 W                | IP66 / IK08      | Enhanced         | $399   | £315   |
| AI Dome                 | AI           | 8MP 4K @ 30           | 1/1.8" 8MP      | 40 m        | —               | GbE         | **PoE 10 W**             | IP66 / IK10      | Enhanced         | $399   | £315   |
| AI Pro                  | AI           | 8MP 4K @ 30           | 1/1.8" 8MP      | 25 m (40 m) | 3×              | GbE         | PoE 11 W                 | IP65 / IK04      | Enhanced         | $499   | £395   |
| AI LPR                  | AI           | 8MP 4K @ 30           | 1/1.8" 8MP      | 15 m        | 3×              | GbE         | PoE+ 25.5 W              | IP66 / IK04      | Detections + LPR | $499   | £395   |
| AI DSLR                 | AI           | 8MP 4K @ 30           | **4/3" 10MP**   | —           | interchangeable | GbE         | PoE+ 17.8 W              | IPX5 (case)      | Enhanced         | $1,499 | £1,195 |
| G6 Pro 360              | Panoramic    | 12MP 3504² @ 24       | 1/1.6" 12MP     | 15 m        | —               | 10/100      | PoE+ 13.5 W              | IP66 / IK10      | **Detections**   | $499   | £395   |
| G6 180                  | Panoramic    | 16MP 7680 × 2160 @ 20 | Dual 1/1.8" 8MP | 20 m        | —               | 10/100      | PoE+ 15 W                | IP66 / IK04      | Enhanced         | $299   | £239   |
| AI Multi Sensor 2       | Multi-sensor | (2) 8MP 4K @ 30       | (2) 1/2.8" 8MP  | 20 m        | 2.33× ×2        | GbE         | PoE+ 25 W                | IP66 / IK10      | Enhanced         | $699   | £555   |
| AI Multi Sensor 4       | Multi-sensor | (4) 8MP 4K @ 24       | (4) 1/2.8" 8MP  | 20 m, 360°  | 2.33× ×4        | GbE         | **PoE++ 34.6 W**         | IP66 / IK10      | Enhanced         | $1,799 | £1,430 |
| AI Theta                | Modular      | 8MP 4:3 @ 24          | 8MP CMOS        | —           | —               | GbE         | PoE 12.5 W               | Indoor           | Enhanced         | $299   | £229   |
| AI Theta Pro            | Modular      | 4MP 2160² @ 24        | 1/1.8" 8MP      | —           | —               | GbE         | PoE 12.5 W               | Indoor           | Enhanced         | $329   | £260   |
| G6 PTZ                  | PTZ          | 8MP 4K @ 30           | Dual 1/1.8" 8MP | 30 m        | 5× (10× hybrid) | 10/100      | PoE+ 24.5 W              | IP66 / IK04      | Enhanced         | $399   | £315   |
| AI PTZ Industrial       | PTZ          | 8MP 4K @ 30           | 1/1.8" 8MP      | 100 m       | **22×**         | GbE         | PoE++ 51 W               | IP66             | Enhanced         | $1,299 | £1,035 |
| AI PTZ Precision        | PTZ          | 8MP 4K @ 30           | 1/1.8" 8MP      | 100 m       | **31× + LiDAR** | GbE         | PoE++ 51 W               | IP66             | Enhanced         | $1,999 | £1,590 |
| G5 PTZ                  | PTZ          | 4MP 2K @ 30           | 1/2.7" 5MP      | 20 m        | 2×              | 10/100      | PoE+ 14 W                | IP66 / IK04      | Detections       | $299   | £239   |
| G5 Pro                  | G5           | 8MP 4K @ 30           | 1/2" 8MP        | 25 m (40 m) | 3×              | 10/100      | PoE 10 W                 | IP65 / IK04      | Detections       | $379   | £300   |
| G5 Turret Ultra         | G5           | 4MP 2K @ 30           | 1/2.4" CMOS     | 30 m        | —               | 10/100      | **PoE 4 W**              | IP66 / IK04      | Detections       | $129   | £79    |
| G5 Dome Ultra           | G5           | 4MP 2K @ 30           | 1/2.4" CMOS     | 20 m        | —               | 10/100      | PoE 4.2 W                | Indoor / IK06    | Detections       | $129   | £79    |
| G5 Dome                 | G5           | 4MP 2K @ 30           | 5MP CMOS        | 9 m         | —               | 10/100      | PoE 5 W                  | IPX4 / IK08      | Detections       | $179   | £140   |
| G5 Bullet               | G5           | 4MP 2K @ 30           | 5MP CMOS        | 9 m         | —               | 10/100      | PoE 4 W                  | IP55 / IK04      | Detections       | $129   | £105   |
| G5 Flex                 | G5           | 4MP 2K @ 30           | 5MP CMOS        | 6 m         | —               | 10/100      | PoE 4 W                  | IPX4 / IK04      | Detections       | $129   | £105   |
| G6 Instant              | Instant      | 8MP 4K @ 30           | 1/1.8" 8MP      | 6 m         | —               | WiFi 5 + BT | USB 7 W                  | IPX5 / IK04      | Enhanced         | $179   | £140   |
| G4 Instant              | Instant      | 4MP 2K @ 30           | 5MP CMOS        | 6 m         | —               | WiFi 5 + BT | USB 6 W                  | IPX5 / IK04      | Detections       | $99    | £75    |
| Doorbell Lite           | Doorbell     | 5MP 1920 × 2560 @ 24  | 1/2.7" 5MP      | 5 m         | —               | 10/100      | PoE 8 W                  | IPX5             | Detections       | $99    | £79    |
| G4 Doorbell Pro         | Doorbell     | 2MP + 2MP @ 30        | 5MP + 8MP CMOS  | 6 m         | —               | WiFi 5 + BT | 16–24 V AC / USB-C, 10 W | IPX4             | Detections       | $299   | £229   |
| G4 Doorbell Pro PoE Kit | Doorbell     | 2MP + 2MP @ 30        | 5MP + 8MP CMOS  | 6 m         | —               | 10/100      | PoE 7 W + 3 W chime      | IPX4             | Detections       | $379   | £300   |

**Reading the table.** Bracketed IR figures are the extended range available with the matching Enhancer accessory. "Enhanced" in the AI column means face and license plate recognition are supported on-camera; "Detections" means people, vehicle, and animal detection only. Dashes in the price columns mark models that are listed Coming Soon without a published price in that store. The G6 Edge family is the only one that operates without an NVR.

**Sources:** [store.ui.com](https://store.ui.com) · [uk.store.ui.com](https://uk.store.ui.com) · [techspecs.ui.com](https://techspecs.ui.com/unifi/physical-security) · [How UniFi Protect Protects Your Data](https://help.ui.com/hc/en-us/articles/31234972188951-How-UniFi-Protect-Protects-Your-Data)

---

Two things to flag about the request itself:

**Prompt drift.** The bullet list in `homelab/docs/unifi/security-cameras.md` was copied from the access-point research prompt — it says "Access Points," "router product," and "list of each switch." I wrote the document about **security cameras**, per the opening line and the file name, and used camera-appropriate categories and metrics throughout.

**Frontmatter.** Your instructions conflict: the body says to add `last_updated`, `access_points`, and `agent` and save the file, while the closing **IMPORTANT** block says to return body content only, with no frontmatter and no edits to the source file. I followed the closing block — nothing was written to disk. If you want the frontmatter applied, the `access_points` key is also the wrong name for this document; `cameras` would be correct. Tell me which you want and I'll write the file with a `cameras:` (or `access_points:`) list keyed by product name.
