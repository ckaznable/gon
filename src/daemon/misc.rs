use std::net::Ipv4Addr;

use anyhow::Result;

pub fn get_preferred_local_ip() -> Result<Ipv4Addr> {
    let interfaces = if_addrs::get_if_addrs()?;

    for iface in &interfaces {
        if !iface.is_loopback() {
            if let if_addrs::IfAddr::V4(ref addr) = iface.addr {
                let ip = addr.ip;
                if (ip.octets()[0] == 192 && ip.octets()[1] == 168) ||                     // 192.168.x.x
                   (ip.octets()[0] == 10) ||                                               // 10.x.x.x
                   (ip.octets()[0] == 172 && ip.octets()[1] >= 16 && ip.octets()[1] <= 31) // 172.16.x.x - 172.31.x.x
                {
                    return Ok(ip);
                }
            }
        }
    }

    for iface in &interfaces {
        if !iface.is_loopback() {
            if let if_addrs::IfAddr::V4(ref addr) = iface.addr {
                return Ok(addr.ip);
            }
        }
    }

    Ok(Ipv4Addr::new(127, 0, 0, 1))
}
