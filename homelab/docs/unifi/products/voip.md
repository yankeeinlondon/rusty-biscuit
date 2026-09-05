---
prompt: |-
    [Unifi](https://ui.com)'s **Talk** service offering surrounds IP-based telephony and Unifi sells a number of VOIP based telephones.

    Your task is to research the full compliment of VOIP telephony products that the Talk service support:

    - you must identify what characterisitics help to distinguish each of the VOIP products from one another
    - when discussing a production be sure to:
        - describe the product in prose
        - specify the key characterisitics/metrics
        - indicate whether the product was introduced and whether it is still being sold

    Once the research content has been written up as prose in this document 
    you will be expected to add frontmatter metadata:

    - `$prompt` as a dictionary:
        
        ```yaml
        prompt: string(required) -> the agent prompt used to perform the research
        last_updated: date -> the date which the latest research on this topic was performed
        researched_by: string -> the agent which performed the research
        products: voip_product[]
        ```

        where `voip_product` is defined as (using SimpliedSchema):

        ```yaml
        id: string -> a unique dasherized identifier for a VOIP product
        name: string -> the friendly name of the product
        uk_price: number -> the UK price in GBP
        us_price: number -> the US price in USD
        description: string -> 1-2 sentence description of the product
        metrics: object -> the key metrics which the research has identified as being useful for describing the product's capabilities
        ```
        
    - `last_updated` as "{[ctx.today}}"
    - `researched_by` as "{ctx.agent}/{ctx.model}"

    Make sure all metadata you've set is saved to the document along with the prose research and then you are done.
hash: ca60fc771674c36b-cc6e7c6adc09c456
last_updated: 2026-09-05
---
The target document `.claude/skills/unifi/products/voip.md` is empty, so I'll research UniFi Talk's VoIP phone lineup from primary sources (Ubiquiti store and tech specs) and write the body content. Starting with the Talk marketing page and the US store listings.The category page rendered only the store shell. Fetching the specific product pages and tech specs directly.The marketing nav revealed the current path: Talk lives under "Integrations → Managed VoIP". Fetching that page and the store's Integrations category.The US store's Managed VoIP category gives the full lineup: G3 Touch Enterprise/Pro/Wall, G2 Touch/Touch Max, G3 Wireless Handset, and the Analog Telephone Adapter. Now getting UK pricing and per-product tech specs.UK prices captured. Now pulling detailed tech specs for each product from techspecs.ui.com.All specs and pricing captured. Let me check a sibling product doc for house style before writing.I have everything needed: full lineup, specs, US/UK pricing, and stock status. Following the IMPORTANT instruction — no "Allowed response frontmatter properties" list appeared, so I'm returning the Markdown body only (frontmatter is owned by the calling system).

# UniFi Talk Phones and VoIP Hardware

UniFi **Talk** is Ubiquiti's IP-telephony service: a PBX application that runs on a UniFi OS Console (or pairs with third-party SIP/PBX providers), plus subscription-based outbound calling in the US, Canada, and the UK. The hardware side of the service lives in the **Managed VoIP** category of the Ubiquiti store and currently consists of seven products: five touchscreen desk/wall phones split across two generations (**G3 Touch** and **G2 Touch**), a Bluetooth wireless handset, and an analog telephone adapter (ATA) for legacy phones and fax machines.

The phones are unusual in the VoIP market: each is a self-contained Android-style "smartphone" with its own octa-core processor, 32 GB of storage, integrated 5 MP camera (with privacy shutter), WiFi, and an app platform that reaches into the wider UniFi ecosystem — Talk phones can unlock doors adopted in UniFi Access, pull up live feeds from UniFi Protect cameras, and place internal one-touch video calls between each other. There are no per-device licensing fees; internal extension calling is free, and outbound calling is subscription-based (US$9.99–$24.99/month or £7.99/month per number, depending on plan and region).

> **Scope note.** This document covers the telephony hardware in the Managed VoIP store category: the phones themselves plus the two Talk-specific companion devices (G3 Wireless Handset, UT-ATA). Legacy, discontinued phone lines are summarized at the end for completeness. Consoles (Cloud Gateways, CloudKey+) that host the Talk application are out of scope.

> **Pricing basis.** Prices are Ubiquiti's list prices on [store.ui.com](https://store.ui.com/us/en/category/all-integrations) (USD) and [store.ui.com/uk](https://store.ui.com/uk/en/category/managed-voip) (GBP) as of **5 September 2026**. The US store separately displays a higher "surcharge included" figure; base list prices are quoted here. Stock status is noted per product.

## What Distinguishes the Phones From Each Other

Ubiquiti publishes a nearly identical spec block for every phone, which makes the handful of real differentiators easy to miss. Five axes separate the lineup:

**Generation (G2 vs G3) is the biggest divide.** The G3 series is the current generation, flagged "New" on Ubiquiti's Managed VoIP page. All three G3 models support **Talk Relay** (remote provisioning and management via UniFi Official Hosting), Zero Touch Provisioning, direct SIP configuration from the touchscreen with third-party PBXes (Zoom, RingCentral, 3CX, Asterisk, Nextiva, Voip.ms are verified), and **Bluetooth 5.2 LE audio** for the G3 Wireless Handset. The G2 phones carry only Bluetooth 4.2, are not listed for Talk Relay support, and cannot pair with the G3 Wireless Handset. Both generations work with the Talk PBX on a UniFi OS Console.

**Display size is the primary tier split within each generation.** A 7" 1280 × 800 panel (Enterprise, Wall, Touch Max) versus a 5" 720 × 1280 panel (G3 Touch Pro, G2 Touch). Screen size tracks the rest of the hardware tier almost perfectly.

**Audio hardware separates tiers as reliably as the screen.** The 7" models carry **dual 3 W hands-free speakers**; the 5" models carry a single 2 W speaker. All five phones share the same dual MEMS hands-free microphones, wideband handset audio, and dedicated mute button.

**Mounting orientation.** Four phones are desktop models with an ergonomic wired handset; the **G3 Touch Wall** is the only wall-mount model and ships without a wired handset or headset jack, operating as a hands-free speakerphone/video unit.

**Power draw and PoE class.** G2 Touch sips 8 W over standard PoE; G3 Touch Pro takes 10 W (PoE); the G3 Enterprise and Wall take 11 W; the G2 Touch Max tops out at 15 W and wants PoE+. All desktop phones expose dual GbE RJ45 ports so a PC can daisy-chain through the phone.

NFC is a minor differentiator (present on the two 7" G3s and the G2 Touch Max; absent on both 5" phones), as are colors (G3 ships black only; both G2 models ship in black or white).

## G3 Touch Series (Current Generation)

Introduced in 2026 and marked "New" across Ubiquiti's marketing, the G3 series is the flagship of the Talk line. All three share one platform: an octa-core ARM Cortex-A53 SoC, 2 GB RAM / 32 GB storage, a 5 MP camera with embedded privacy shutter, dual GbE RJ45 ports, 802.11a/b/g/n/ac WiFi, Bluetooth 4.2 (5.2 reserved for the G3 Wireless Handset), anti-glare multi-touch displays, Talk and Talk Relay support, and one-touch internal video calling.

### G3 Touch Enterprise

The premium desk phone and the flagship of the lineup. A 7" HD touchscreen sits above an ergonomic wideband handset with a dedicated mute button; behind the display are dual 3 W speakers, dual MEMS microphones, and the phone's 5 MP privacy-shuttered camera for internal video calls. It is the most connectivity-rich Talk device: NFC (including MIFARE and FeliCa families, useful for badge-based workflows), a 3.5 mm headset jack, dual GbE pass-through, and USB-C PoE+ power. At $299 it sits in the middle of the price range while carrying the fullest feature set.

- **Price:** $299 USD / £239 GBP
- **Status:** Current — introduced 2026 with the G3 generation; in stock in both the US and UK stores
- **Images:** [front](https://cdn.ecomm.ui.com/products/b6d45ef1-83e2-45cb-a745-4996d733bca5/96c35975-b20e-4130-9e49-eccf42048a9e.png) · [angle](https://cdn.ecomm.ui.com/products/b6d45ef1-83e2-45cb-a745-4996d733bca5/82f79c43-2891-43a5-86ab-91a4561db0d6.png)

| Property     | Value                                                         |
|--------------|---------------------------------------------------------------|
| Display      | 7" (178 mm) 1280 × 800 HD, multi-touch, anti-glare            |
| Camera       | 5 MP 2592 × 1944 with embedded privacy shutter                |
| Audio        | Dual 3 W speakers, dual MEMS mics, wideband handset           |
| Connectivity | (2) GbE RJ45, WiFi 5, BT 4.2 + BT 5.2 (G3 handset), NFC       |
| Power        | USB-C PoE+, 11 W max                                          |
| Processor    | Octa-core ARM Cortex-A53, 2 GB RAM, 32 GB storage             |
| Mounting     | Desktop                                                       |
| Extras       | 3.5 mm headset jack, mute button, RGBW LED, NFC 13.56 MHz     |
| Dimensions   | 256.5 × 139.5 × 185.3 mm (10.1 × 5.5 × 7.3"), 1.5 kg (3.3 lb) |

### G3 Touch Pro

The compact G3: the same current-generation platform, camera, and app experience as the Enterprise, scaled to a 5" display and a lighter 1 kg frame. The trade-offs are the single 2 W speaker, no NFC, standard PoE instead of PoE+, and slightly lower peak draw (10 W). It keeps the dual GbE ports, WiFi, Bluetooth 5.2 for the G3 Wireless Handset, the 3.5 mm headset jack, and full Talk Relay/third-party PBX support. At $199 / £159 it is the entry point into the G3 generation.

- **Price:** $199 USD / £159 GBP
- **Status:** Current — introduced 2026 with the G3 generation; in stock in both the US and UK stores
- **Images:** [front](https://cdn.ecomm.ui.com/products/5607e410-4003-498c-9670-d501e727ee71/082070fa-62ba-442d-a8bb-3c9ea2f0e2a4.png) · [angle](https://cdn.ecomm.ui.com/products/5607e410-4003-498c-9670-d501e727ee71/f774678d-4ccd-421e-8d31-07097612c246.png)

| Property     | Value                                                      |
|--------------|------------------------------------------------------------|
| Display      | 5" (127 mm) 720 × 1280 HD, multi-touch, anti-glare         |
| Camera       | 5 MP 2592 × 1944 with embedded privacy shutter             |
| Audio        | Single 2 W speaker, dual MEMS mics, wideband handset       |
| Connectivity | (2) GbE RJ45, WiFi 5, BT 4.2 + BT 5.2 (G3 handset)         |
| Power        | PoE, 10 W max                                              |
| Processor    | Octa-core ARM Cortex-A53, 2 GB RAM, 32 GB storage          |
| Mounting     | Desktop                                                    |
| Extras       | 3.5 mm headset jack, mute button, RGBW LED                 |
| Dimensions   | 165.4 × 139.5 × 185.3 mm (6.5 × 5.5 × 7.3"), 1 kg (2.2 lb) |

### G3 Touch Wall

The wall-mount Talk phone, and the only one oriented for common-area and corridor deployments: a 7" 1280 × 800 display in a slim 70 mm-deep enclosure that mounts flat to a wall. It shares the G3 platform and the Enterprise's audio muscle — dual 3 W speakers, dual MEMS microphones — plus NFC and a 5 MP camera, but drops the wired handset and headset jack entirely: this is a hands-free speakerphone and video-calling unit. It is the most expensive phone in the lineup at $399 / £315, reflecting its specialist form factor.

- **Price:** $399 USD / £315 GBP
- **Status:** Current — introduced 2026 with the G3 generation; **out of stock in the US store since 21 August 2026 with no restock date listed** as of this research, but in stock in the UK store
- **Images:** [front](https://cdn.ecomm.ui.com/products/7ce6ac31-d696-4130-ab20-77a2577f73e4/7fdc43b8-5bf8-4c09-828f-1f50f036519b.png) · [angle](https://cdn.ecomm.ui.com/products/7ce6ac31-d696-4130-ab20-77a2577f73e4/e4cdf78a-25f1-41d7-b0c0-63525ca0f42b.png)

| Property     | Value                                                            |
|--------------|------------------------------------------------------------------|
| Display      | 7" (178 mm) 1280 × 800 HD, multi-touch, anti-glare               |
| Camera       | 5 MP 2592 × 1944 with embedded privacy shutter                   |
| Audio        | Dual 3 W speakers, dual MEMS mics (hands-free; no wired handset) |
| Connectivity | (2) GbE RJ45, WiFi 5, BT 4.2 + BT 5.2 (G3 handset), NFC          |
| Power        | USB-C PoE, 11 W max                                              |
| Processor    | Octa-core ARM Cortex-A53, 2 GB RAM, 32 GB storage                |
| Mounting     | Wall                                                             |
| Extras       | Mute button, RGBW LED, NFC 13.56 MHz                             |
| Dimensions   | 254.4 × 202.5 × 70.1 mm (10 × 8 × 2.8"), 1.1 kg (2.5 lb)         |

## G2 Touch Series (Previous Generation)

The G2 phones launched with the modern Talk application generation (circa late 2024) and remain part of the supported lineup — Ubiquiti's plug-and-play Talk deployment path explicitly supports "G2, G3 phones." They run the same octa-core A53 / 2 GB / 32 GB hardware platform and carry the same 5 MP camera as the G3s, but lack Talk Relay support, NFC-capable Bluetooth 5.2, and the G3 Wireless Handset pairing. Both models are showing end-of-life signals in the US store as the G3 generation takes over.

### G2 Touch Max

The previous-generation flagship: the 7" 1280 × 800 display, dual 3 W speakers, NFC, and USB-C PoE+ power — on paper a close match to today's G3 Touch Enterprise, at a $50 lower list price. The gaps versus G3 are generational rather than material: Bluetooth 4.2 only (no G3 Wireless Handset support), no Talk Relay, and a plastic rather than polycarbonate/aluminum build in black or white. US availability has effectively ended — the listing is out of stock with no restock date — though the UK store still sells it.

- **Price:** $249 USD / £239 GBP
- **Status:** Previous generation — introduced circa late 2024; **out of stock in the US store with no restock date listed** (strong end-of-life signal), still orderable in the UK store as of this research
- **Images:** [front](https://cdn.ecomm.ui.com/products/52bbbbab-2435-4cbf-8d74-2e5d63b6e9b3/682d33ee-54b0-4ff2-8080-f613f16104ec.png) · [angle](https://cdn.ecomm.ui.com/products/52bbbbab-2435-4cbf-8d74-2e5d63b6e9b3/36d46f74-70ec-4100-a5c2-b11d21c5ab30.png)

| Property     | Value                                                         |
|--------------|---------------------------------------------------------------|
| Display      | 7" (178 mm) 1280 × 800 HD, multi-touch, fingerprint-resistant |
| Camera       | 5 MP 2592 × 1944 with embedded privacy shutter                |
| Audio        | Dual 3 W speakers, dual MEMS mics, wideband handset           |
| Connectivity | (2) GbE RJ45, WiFi 5, BT 4.2                                  |
| Power        | USB-C PoE+, 15 W max                                          |
| Processor    | Octa-core ARM Cortex-A53, 2 GB RAM, 32 GB storage             |
| Mounting     | Desktop                                                       |
| Extras       | 3.5 mm headset jack, mute button, NFC, black or white         |
| Dimensions   | 255 × 135 × 183 mm (10 × 5.3 × 7.2"), 1.5 kg (3.4 lb)         |

### G2 Touch

The value model of the line and still the cheapest Talk phone at $129. It is a 5" / single-2 W-speaker design in the same spirit as the G3 Touch Pro, with the lowest power draw of any phone (8 W, standard PoE), black or white enclosures, and dual GbE pass-through. The US store sells it in multiple selectable variants — reflecting Ubiquiti's **subscription-locked** (lower hardware cost, must be assigned a UniFi Talk number) versus **subscription-unlocked** (shares group numbers, no per-user number required) hardware SKUs. The UK store redirects purchasers to the EU store, so it has no direct UK availability.

- **Price:** $129 USD / £159 GBP (UK listing redirects to the EU store)
- **Status:** Previous generation — introduced circa late 2024; still sold in the US store (select variants, subscription-locked and unlocked), no longer sold directly in the UK store
- **Images:** [front](https://cdn.ecomm.ui.com/products/6bb607c4-58c5-440b-937e-4173ffd59905/75366d63-5795-47f4-8ba3-356ddae6afcc.png) · [angle](https://cdn.ecomm.ui.com/products/6bb607c4-58c5-440b-937e-4173ffd59905/98d7c630-5263-4bf4-a6b1-e503ab7761c1.png)

| Property     | Value                                                                                   |
|--------------|-----------------------------------------------------------------------------------------|
| Display      | 5" (127 mm) 720 × 1280 HD, multi-touch, fingerprint-resistant                           |
| Camera       | 5 MP 2592 × 1944 with embedded privacy shutter                                          |
| Audio        | Single 2 W speaker, dual MEMS mics, wideband handset                                    |
| Connectivity | (2) GbE RJ45, WiFi 5, BT 4.2                                                            |
| Power        | PoE, 8 W max                                                                            |
| Processor    | Octa-core ARM Cortex-A53, 2 GB RAM, 32 GB storage                                       |
| Mounting     | Desktop                                                                                 |
| Extras       | 3.5 mm headset jack, mute button, black or white; subscription-locked/unlocked variants |
| Dimensions   | 166 × 140 × 183 mm (6.5 × 5.5 × 7.2"), 1.1 kg (2.4 lb)                                  |

## Talk Companion Devices

Two non-phone products round out the Managed VoIP category. Neither places calls on its own — they extend the reach of the phone system to a wireless handset and to legacy analog equipment.

### G3 Wireless Handset

A cordless Bluetooth handset that pairs with any G3 Touch phone over Bluetooth 5.2 LE audio, giving the desk phone DECT-like mobility around a workspace. It carries a tiny 0.96" status display, pickup/end, mute, and volume buttons, a 1000 mAh Li-ion battery charged through two pogo pins in the phone's cradle, and the same wideband receiver audio as the desk phones' wired handsets. It does not pair with G2-generation phones and is not a standalone SIP device.

- **Price:** $99 USD / £79 GBP
- **Status:** Current — introduced 2026 alongside the G3 series; in stock in both the US and UK stores
- **Images:** [front](https://cdn.ecomm.ui.com/products/a20354f2-5e19-4e51-b3e5-98494f6a8b89/f34be5e7-fa87-400c-ab2c-d263b5850a48.png) · [detail](https://cdn.ecomm.ui.com/products/a20354f2-5e19-4e51-b3e5-98494f6a8b89/b885d182-e60a-4442-b6b1-aefe8f9266c5.png)

| Property   | Value                                                   |
|------------|---------------------------------------------------------|
| Pairing    | Bluetooth 5.2 LE audio — G3 Touch phones only           |
| Battery    | 1000 mAh Li-ion, pogo-pin charging                      |
| Display    | 0.96" (24 mm) status display                            |
| Controls   | Pickup/end, mute, volume up/down                        |
| Audio      | Omnidirectional electret mic, wideband receiver         |
| Dimensions | 186 × 53.8 × 54.7 mm (7.3 × 2.1 × 2.2"), 180 g (6.3 oz) |

### Analog Telephone Adapter (UT-ATA)

A small managed adapter that bridges two RJ11 FXS ports onto a UniFi Talk line, so existing analog phones and fax machines can participate in the Talk system. It is managed directly by the UniFi Talk application (v1.12.0+), exposes call features — voicemail, DND, caller ID, transfer, waiting, 3-way conference — across its two supported phone lines, and handles fax through G.711 pass-through. Power comes from included USB-C adapter or PoE (mode B), and it even includes WiFi and a 3.4" monochrome display for local status. It is listed only in the US store; the UK Managed VoIP category does not carry it.

- **Price:** $99 USD / not listed in the UK store
- **Status:** Current; in stock in the US store
- **Images:** [front](https://cdn.ecomm.ui.com/products/6ca19e85-3bc9-4e4f-991f-9944e3eef1a1/fbe031b1-2dd9-46f7-99e5-b38e8f64fa7a.png) · [angle](https://cdn.ecomm.ui.com/products/6ca19e85-3bc9-4e4f-991f-9944e3eef1a1/2a68c36a-a51d-4a8d-84b3-7cda6348a37a.png)

| Property      | Value                                                                    |
|---------------|--------------------------------------------------------------------------|
| Phone lines   | 2 (RJ11 FXS)                                                             |
| Codec / fax   | G.711 µ-law/A-law; fax pass-through over G.711                           |
| Call features | Voicemail, DND, caller ID, call transfer, call waiting, 3-way conference |
| Connectivity  | 100 MbE PoE, WiFi, USB-C power, 3.5 mm headset port                      |
| Display       | 3.4" (86 mm) 192 × 64 dot matrix, 3 control keys                         |
| Management    | UniFi Talk application v1.12.0+                                          |
| Dimensions    | 131.5 × 110.7 × 28.2 mm (5.2 × 4.4 × 1.1"), 300 g (10.6 oz)              |

## Retired Talk Hardware

Three earlier hardware generations predate today's lineup and none are sold any longer; they surface constantly on the secondhand market and none are supported by the current G3-era platform.

- **UVP series (UVP, UVP-Pro, UVP-Executive)** — Ubiquiti's original mid-2010s Android desk phones, built for the long-discontinued UniFi VoIP Controller. Introduced 2015–2016; discontinued.
- **UVP-Enterprise** — the phone announced alongside the original 2019 launch of UniFi Talk; retired with that first-generation service. Introduced 2019; discontinued.
- **UH-VoIP Phone and UH-VoIP+ Phone** — early reboot hardware listed circa 2023 ahead of the modern Talk application; both were delisted before the G2 Touch series arrived and never reached general availability at scale. Introduced 2023; discontinued (delisted by 2024).

## Summary

| Product                  | SKU                     | US   | UK       | Display       | Speakers          | NFC | G3 Handset / Talk Relay | Power            | Mount      | Status (Sep 2026)                       |
|--------------------------|-------------------------|------|----------|---------------|-------------------|-----|-------------------------|------------------|------------|-----------------------------------------|
| G3 Touch Enterprise      | UTP-G3-Touch-Enterprise | $299 | £239     | 7" 1280 × 800 | Dual 3 W          | Yes | Yes / Yes               | USB-C PoE+, 11 W | Desk       | Current, in stock                       |
| G3 Touch Pro             | UTP-G3-Touch-Pro        | $199 | £159     | 5" 720 × 1280 | Single 2 W        | No  | Yes / Yes               | PoE, 10 W        | Desk       | Current, in stock                       |
| G3 Touch Wall            | UTP-G3-Touch-Wall       | $399 | £315     | 7" 1280 × 800 | Dual 3 W          | Yes | Yes / Yes               | USB-C PoE, 11 W  | Wall       | Current; US out of stock, UK in stock   |
| G2 Touch Max             | UTP-TouchMax            | $249 | £239     | 7" 1280 × 800 | Dual 3 W          | Yes | No / No                 | USB-C PoE+, 15 W | Desk       | Prior gen; US out of stock, UK in stock |
| G2 Touch                 | UTP-Touch               | $129 | £159*    | 5" 720 × 1280 | Single 2 W        | No  | No / No                 | PoE, 8 W         | Desk       | Prior gen; US in stock, UK via EU store |
| G3 Wireless Handset      | UT-G3-Handset           | $99  | £79      | 0.96" status  | Wideband receiver | —   | Is the accessory        | 1000 mAh battery | —          | Current, in stock                       |
| Analog Telephone Adapter | UT-ATA                  | $99  | not sold | 3.4" 192 × 64 | —                 | —   | —                       | USB-C or PoE     | Flat/angle | Current, in stock (US only)             |

\* UK listing redirects to the EU store.

All seven products share the same management story: adoption into the UniFi Talk application, no per-device licensing, and free internal calling. For new deployments the practical guidance is straightforward — buy G3 (Enterprise for full-featured desks, Pro for compact ones, Wall for common areas), add the G3 Wireless Handset where mobility matters, and use the UT-ATA only to preserve analog or fax equipment. The G2 phones remain supported but are visibly being phased out of the US channel as the G3 generation ramps.
