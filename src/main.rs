use std::{net::SocketAddr, sync::Arc};

use crate::notification::SystemNotificationListener;
use anyhow::Result;
use client::Client;
use daemon::{
    node::Node,
    protocol::Response,
    service::{AppService, AppServiceEvent},
};
use tokio::{select, sync::RwLock};
use tray::TrayEvent;

mod client;
mod daemon;
mod notification;
mod tray;

pub enum AppMode<T> {
    Host,
    Client(Option<T>),
}

impl<T> AppMode<T> {
    pub fn is_client(&self) -> bool {
        matches!(self, AppMode::Client(_))
    }

    pub fn is_client_and_found_host(&self) -> bool {
        matches!(self, AppMode::Client(Some(_)))
    }

    pub fn is_host(&self) -> bool {
        matches!(self, AppMode::Host)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let pass = std::env::var("GON_PASS")
        .ok()
        .unwrap_or(String::from("pass"));

    let (_tray, mut tray_rx) = tray::init_tray();

    let mut listener = SystemNotificationListener::default();
    listener.listen();

    let (mut node, addr) = Node::new(pass.as_bytes()).await?;
    let mut messaeg_rx = node.listen().await?;
    let node = Arc::new(node);
    let mut service = AppService::new(addr)?;

    let host: Arc<RwLock<AppMode<SocketAddr>>> = Arc::new(RwLock::new(AppMode::Client(None)));
    let client = Client::new(node.clone(), host.clone());

    loop {
        select! {
            Some(event) = tray_rx.recv() => {
                match event {
                    TrayEvent::BecomeHost => {
                        *host.write().await = AppMode::Host;
                    }
                    TrayEvent::BecomeClient => {
                        *host.write().await = AppMode::Client(None);
                    }
                    TrayEvent::Quit => {
                        break;
                    }
                }
            }
            Ok(event) = service.next() => {
                match event {
                    AppServiceEvent::NodeDiscoverd(socket_addr) => {
                        println!("discoverd {}", socket_addr);
                        let Ok(mut stream) = client.connect(socket_addr).await else {
                            continue;
                        };

                        if stream.ping().await {
                            println!("ping pong sucess");
                        }
                    },
                    AppServiceEvent::None => continue,
                };
            }
            Some(notif) = listener.next_notify() => {
                println!("Received notification: {:?}", notif);
                let AppMode::Client(Some(host)) = *host.read().await else {
                    continue;
                };

                let mut stream = client.connect(host).await?;
                stream.send_notification(Arc::into_inner(notif).unwrap()).await?;
            }
            Some((mut stream, msg)) = messaeg_rx.recv() => {
                println!("Received new Message {:?}", msg);
                if msg.is_done() {
                    continue;
                }

                // if not host
                let res = if let AppMode::Client(Some(host)) = *host.read().await {
                    Response::host_changed(host)
                } else {
                    let handler = client.handle();
                    handler.handle(msg).await
                };

                let _ = node.reply(&mut stream, res).await;
            }
        }
    }

    Ok(())
}
