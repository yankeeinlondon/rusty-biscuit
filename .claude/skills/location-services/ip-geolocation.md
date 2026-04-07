# IP Geolocation

Two Rust crates read MaxMind `.mmdb` files: `maxminddb` (community standard) and `geoip2` (IncSW, performance-focused).

## `maxminddb`

[Repo](https://github.com/oschwald/maxminddb-rust) | [docs.rs](https://docs.rs/maxminddb/) | License: ISC

The definitive Rust `.mmdb` reader. Thread-safe, supports mmap and in-memory loading, with lazy decoding (v0.27+) and selective field extraction.

### Features

- **Reader modes**: `open_mmap` (zero-copy, unsafe if file modified) or `open_readfile` (safe, loads into memory)
- **Lazy decoding**: `LookupResult` defers deserialization; use `decode_path` for single fields
- **Built-in models**: `geoip2::City`, `Country`, `Asn`, `Isp`, `AnonymousIp` in the `geoip2` module
- **Network iteration**: Iterate all networks or query within a CIDR range
- **Performance flags**: `simdutf8` (SIMD UTF-8 validation), `unsafe-str-decode` (skip validation)

### City Lookup

```rust
use maxminddb::{geoip2, Reader};
use std::net::IpAddr;

let reader = Reader::open_readfile("GeoLite2-City.mmdb")?;
let ip: IpAddr = "1.1.1.1".parse()?;
let city: geoip2::City = reader.lookup(ip)?;

if let Some(country) = city.country {
    println!("ISO Code: {:?}", country.iso_code);
}
```

### Selective Field Extraction (High Throughput)

```rust
use maxminddb::{Reader, PathElement};

fn get_asn_only(reader: &Reader<Vec<u8>>, ip: std::net::IpAddr) -> Option<u32> {
    reader.lookup(ip).ok()?.decode_path(&[
        PathElement::Key("autonomous_system_number")
    ]).ok()
}
```

### Fraud Detection

```rust
use maxminddb::{geoip2, Reader};

fn is_risky(reader: &Reader<Vec<u8>>, ip: std::net::IpAddr) -> bool {
    if let Ok(anon) = reader.lookup::<geoip2::AnonymousIp>(ip) {
        return anon.is_anonymous_proxy.unwrap_or(false)
            || anon.is_vpn.unwrap_or(false)
            || anon.is_tor_exit_node.unwrap_or(false);
    }
    false
}
```

### Gotchas

| Issue | Detail | Workaround |
|-------|--------|------------|
| Mmap soundness | `open_mmap` is unsafe; file truncation during read causes SIGBUS | Use `open_readfile`, or ensure atomic rename updates |
| Thread sharing | Opening per-thread is expensive | Wrap `Reader` in `Arc` |
| Missing test data | Tests need `.mmdb` submodule files | `git submodule update --init` |
| Large record cost | Full City decode involves many allocations | Use `decode_path` for specific fields |

## `geoip2` (IncSW)

[Repo](https://github.com/IncSW/geoip2-rs) | [docs.rs](https://docs.rs/geoip2/)

Codegen-backed reader optimized for raw lookup speed. Pre-generated structs for each database type avoid serde overhead.

### Features

- **Codegen models**: Pre-generated structs for City, Country, ASN, ISP, Anonymous IP, Connection Type, Enterprise
- **Zero-allocation focus**: Minimizes heap allocations during tree traversal
- **`unsafe-str` flag**: Bypasses UTF-8 validation for ~10-20% speed boost
- **Loading**: `open_readfile` (into memory) or `from_bytes` (from buffer)

### City Lookup

```rust
use geoip2::Reader;
use std::net::IpAddr;

let reader = Reader::open_readfile("GeoLite2-City.mmdb")?;
let ip: IpAddr = "1.1.1.1".parse()?;
let city = reader.lookup::<geoip2::City>(ip)?;

if let Some(c) = city.city {
    println!("City: {:?}", c.names.get("en"));
}
```

### ASN Lookup

```rust
let reader = Reader::open_readfile("GeoLite2-ASN.mmdb")?;
let asn = reader.lookup::<geoip2::Asn>("8.8.8.8".parse()?)?;
println!("ASN: {:?}, Org: {:?}", asn.autonomous_system_number, asn.autonomous_system_organization);
```

### Gotchas

| Issue | Detail | Workaround |
|-------|--------|------------|
| Sparse docs | Must read source or GitHub examples | Refer to repo examples directory |
| `unsafe-str` risk | Bypasses UTF-8 checks | Only use with trusted MaxMind-produced databases |
| Model rigidity | Codegen structs; can't add custom fields easily | Use `maxminddb` for custom `.mmdb` schemas |
| No native mmap | Only `open_readfile` or `from_bytes` | Manually mmap and pass slice to `from_bytes` |

## Database Acquisition

The `.mmdb` database is **never bundled** with either crate.

1. Create account at [MaxMind.com](https://www.maxmind.com)
2. Generate a license key
3. Download via `geoipupdate` tool or manual download

| Database | License | Cost |
|----------|---------|------|
| GeoLite2 | GeoLite2 EULA (attribution required, no redistribution) | Free |
| GeoIP2 | Commercial | Paid subscription |

## Decision Guide

- **Most projects**: Use `maxminddb`. Lazy decoding (v0.27+) is fast, API is safer, docs are excellent, supports custom schemas.
- **Extreme throughput with full-record decode**: Benchmark `geoip2` against `maxminddb` with `decode_path`. Only switch if you measure a real difference.
