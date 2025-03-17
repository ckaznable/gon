use std::{collections::HashMap, net::{IpAddr, Ipv4Addr}};
use anyhow::{anyhow, Result};
use mdns_sd::{Receiver, ServiceDaemon, ServiceEvent, ServiceInfo};

const DOMAIN: &str = "_gon._tcp.local.";
const SERVICE_NAME: &str = "Gate of Notification";
const HOSTNAME: &str = "gon.local.";

pub enum AppServiceEvent {
    None,
    NewNode(Ipv4Addr, u16),
}

pub struct AppService {
    mdns_daemon: ServiceDaemon,
    mdns_rx: Receiver<ServiceEvent>,
    addr: Ipv4Addr,
    port: u16,
}

impl AppService {
    pub fn new(addr: Ipv4Addr, port: u16) -> Result<Self> {
        let mdns = ServiceDaemon::new()?;

        let service_info = ServiceInfo::new(
            DOMAIN,
            SERVICE_NAME,
            HOSTNAME,
            addr.to_string(),
            port,
            Some(HashMap::new()),
        )?;

        mdns.register(service_info)?;
        let mdns_rx = mdns.browse(DOMAIN)?;
        println!("services are registered on mdns and start browse other gon service on LAN");

        Ok(Self {
            addr,
            port,
            mdns_rx,
            mdns_daemon: mdns,
        })
    }

    pub async fn next(&mut self) -> Result<AppServiceEvent> {
        let mut event = AppServiceEvent::None;
        if let Ok(ServiceEvent::ServiceResolved(info)) = self.mdns_rx.recv_async().await {
            if info.get_type().eq(DOMAIN) {
                let addr = info.get_addresses().iter().next().ok_or(anyhow!("empty address recive from mdns"))?;
                let port = info.get_port();

                if let IpAddr::V4(addr) = *addr {
                    if addr != self.addr {
                        event = AppServiceEvent::NewNode(addr, port);
                    }
                }
            }
        };

        Ok(event)
    }
}
