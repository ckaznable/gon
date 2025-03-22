use std::{collections::HashSet, net::SocketAddr, sync::Arc, time::Duration};

use crate::notification::SystemNotificationListener;
use anyhow::Result;
use client::Client;
use daemon::{
    node::Node,
    protocol::Response,
    service::{AppService, AppServiceEvent},
};
use tokio::{select, sync::Mutex};
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

    pub fn get_host(&self) -> Option<&T> {
        if let AppMode::Client(Some(host)) = self {
            Some(host)
        } else {
            None
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let (_tray, mut tray_rx) = tray::init_tray();

    let mut listener = SystemNotificationListener::default();
    listener.listen();

    let mut node = Node::new().await?;
    let mut messaeg_rx = node.listen().await?;
    let node = Arc::new(node);
    let mut service = AppService::new(node.addr)?;

    let host: Arc<Mutex<AppMode<SocketAddr>>> = Arc::new(Mutex::new(AppMode::Client(None)));
    let client = Client::new(node.clone(), host.clone());

    let mut check_interval = tokio::time::interval(Duration::from_secs(30)); 

    let addr_book: Arc<Mutex<HashSet<SocketAddr>>> = Arc::new(Mutex::new(HashSet::new()));

    loop {
        select! {
            // try to get host addr in lan per 30 seconds if node is client and not found host
            _ = check_interval.tick() => {
                if host.lock().await.is_client_and_found_host() {
                    continue;
                }
                
                println!("try to get host addr in lan");
                'a: for addr in addr_book.lock().await.iter() {
                    let mut stream = client.connect(*addr).await?;
                    if let Ok(()) = stream.get_addr().await {
                        break 'a;
                    }
                }
            }
            Some(event) = tray_rx.recv() => {
                match event {
                    TrayEvent::BecomeHost => {
                        println!("become host");
                        let mut host = host.lock().await;
                        *host = AppMode::Host;

                        // if host exist, send im_host to host
                        if let Some(host) = host.get_host() {
                            let mut stream = client.connect(*host).await?;
                            stream.im_host().await?;
                        }
                    }
                    TrayEvent::BecomeClient => {
                        println!("become client");
                        *host.lock().await = AppMode::Client(None);
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
                            let mut addr_book = addr_book.lock().await;
                            addr_book.insert(socket_addr);

                            let mut stream = client.connect(socket_addr).await?;
                            stream.get_addr().await?;
                        }
                    },
                    AppServiceEvent::None => continue,
                };
            }
            Some(notif) = listener.next_notify() => {
                println!("Received notification: {:?}", notif);
                let AppMode::Client(Some(host)) = *host.lock().await else {
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
                let res = if let AppMode::Client(Some(host)) = *host.lock().await {
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
