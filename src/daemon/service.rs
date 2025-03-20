use std::{collections::HashMap, net::SocketAddr};
use anyhow::{anyhow, Result};
use mdns_sd::{Receiver, ServiceDaemon, ServiceEvent, ServiceInfo};

const DOMAIN: &str = "_gon._tcp.local.";
const SERVICE_NAME: &str = "Gate of Notification";
const HOSTNAME: &str = "gon.local.";

pub enum AppServiceEvent {
    None,
    NodeDiscoverd(SocketAddr),
}

pub struct AppService {
    _mdns_daemon: ServiceDaemon,
    mdns_rx: Receiver<ServiceEvent>,
    addr: SocketAddr,
}

impl AppService {
    pub fn new(addr: SocketAddr) -> Result<Self> {
        let mdns = ServiceDaemon::new()?;

        let service_info = ServiceInfo::new(
            DOMAIN,
            SERVICE_NAME,
            HOSTNAME,
            addr.ip().to_string(),
            addr.port(),
            Some(HashMap::new()),
        )?;

        mdns.register(service_info)?;
        let mdns_rx = mdns.browse(DOMAIN)?;
        println!("services are registered on mdns and start browse other gon service on {}", addr);

        Ok(Self {
            addr,
            mdns_rx,
            _mdns_daemon: mdns,
        })
    }

    pub async fn next(&mut self) -> Result<AppServiceEvent> {
        let mut event = AppServiceEvent::None;
        if let Ok(ServiceEvent::ServiceResolved(info)) = self.mdns_rx.recv_async().await {
            if info.get_type().eq(DOMAIN) {
                let addr = info.get_addresses().iter().next().ok_or(anyhow!("empty address recive from mdns"))?;
                let port = info.get_port();

                if *addr != self.addr.ip() || (*addr == self.addr.ip() && port != self.addr.port()) {
                    event = AppServiceEvent::NodeDiscoverd(SocketAddr::new(*addr, port));
                }
            }
        };

        Ok(event)
    }
}
