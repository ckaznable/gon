use std::net::{Ipv4Addr, SocketAddr};

use anyhow::{anyhow, Result};
use tokio::net::TcpListener;

use crate::daemon::misc::get_preferred_local_ip;

#[derive(Copy, Clone, Default)]
pub enum NodeMode {
    #[default]
    Host,
    Client,
}

pub struct Node {
    pub mode: NodeMode,
    pub socket: TcpListener,
    pub addr: Ipv4Addr,
    pub port: u16,
}

impl Node {
    pub async fn new() -> Result<Self> {
        let socket = TcpListener::bind("0.0.0.0:0").await?;
        let Ok(SocketAddr::V4(addr)) = socket.local_addr() else {
            return Err(anyhow!("can't listener tcp socket"));
        };

        let port = addr.port();
        let addr = get_preferred_local_ip()?;
        println!("listen service on {}:{}", addr, port);

        Ok(Self {
            addr,
            port,
            socket,
            mode: NodeMode::Host,
        })
    }

    pub fn become_host(&mut self) {
        self.mode = NodeMode::Host;
    }

    pub fn become_client(&mut self) {
        self.mode = NodeMode::Client;
    }
}
