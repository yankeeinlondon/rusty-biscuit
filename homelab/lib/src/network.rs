use std::net::{Ipv4Addr, Ipv6Addr};

pub enum Host {
  V4(Ipv4Addr),
  V6(Ipv6Addr),
  Dns(String)
}


