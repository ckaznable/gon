#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::{collections::HashSet, net::SocketAddr, sync::{Arc, LazyLock}, time::Duration};

use crate::notification::SystemNotificationListener;
use anyhow::Result;
use client::Client;
use daemon::{
    node::Node,
    protocol::Response,
    service::{AppService, AppServiceEvent},
};
use directories::ProjectDirs;
use tokio::{select, sync::Mutex};
use tray::{set_icon, TrayEvent, TrayIcon};

mod client;
mod daemon;
mod notification;
mod tray;

pub static DIRS: LazyLock<ProjectDirs> = LazyLock::new(|| {
    ProjectDirs::from("", "", "gon").unwrap()
});

#[derive(Clone, Debug)]
pub enum AppMode<T> {
    Host,
    Client(Option<T>),
}

impl<T> AppMode<T> {
    pub fn is_client(&self) -> bool {
        matches!(self, AppMode::Client(_))
    }

    pub fn is_client_and_not_found_host(&self) -> bool {
        matches!(self, AppMode::Client(None))
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
    let (mut tray, mut tray_rx) = tray::init_tray();

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
                let host = host.lock().await;
                if !host.is_client_and_not_found_host() || host.is_host() {
                    continue;
                }

                println!("try to get host addr in lan");
                'a: for addr in addr_book.lock().await.iter() {
                    if let Ok(mut stream) = client.connect(*addr).await {
                        if let Ok(()) = stream.get_addr().await {
                            break 'a;
                        }
                    }
                }
            }
            Some(event) = tray_rx.recv() => {
                match event {
                    TrayEvent::BecomeHost => {
                        println!("become host");
                        let mut host = host.lock().await;
                        let _host = host.clone();
                        let origin_host = _host.get_host();

                        *host = AppMode::Host;
                        set_icon(&mut tray, TrayIcon::Host);

                        // if host exist, send im_host to host
                        if let Some(host) = origin_host {
                            println!("tell {} i'm host", host);
                            if let Ok(mut stream) = client.connect(*host).await {
                                let _ = stream.im_host().await;
                            }
                        }
                    }
                    TrayEvent::BecomeClient => {
                        println!("become client");
                        *host.lock().await = AppMode::Client(None);
                        set_icon(&mut tray, TrayIcon::Default);
                    }
                    TrayEvent::Quit => {
                        break;
                    }
                }
            }
            Ok(event) = service.next() => {
                match event {
                    AppServiceEvent::NodeDiscoverd(socket_addr) => {
                        if host.lock().await.is_host() {
                            continue;
                        }

                        println!("discoverd {}", socket_addr);
                        let Ok(mut stream) = client.connect(socket_addr).await else {
                            continue;
                        };

                        if stream.ping().await {
                            println!("ping pong sucess");
                            let mut addr_book = addr_book.lock().await;
                            addr_book.insert(socket_addr);

                            if let Ok(mut stream) = client.connect(socket_addr).await {
                                let _ = stream.get_addr().await;
                            }
                        }
                    },
                    AppServiceEvent::None => continue,
                };
            }
            Some(notif) = listener.next_notify() => {
                // skip notification from self
                if notif.app_id == "gon" {
                    continue;
                }

                let host = host.lock().await;
                if host.is_host() || host.is_client_and_not_found_host() {
                    continue;
                }

                let AppMode::Client(Some(host)) = *host else {
                    continue;
                };

                println!("send notification to {}", host);
                if let Ok(mut stream) = client.connect(host).await {
                    let _ = stream.send_notification(Arc::into_inner(notif).unwrap()).await;
                }
            }
            Some((mut stream, msg)) = messaeg_rx.recv() => {
                println!("Received new Message {:?}", msg);
                if msg.is_done() {
                    continue;
                }

                // if not host
                let res = if let AppMode::Client(Some(host)) = *host.lock().await {
                    println!("i'm not host, host changed to {}", host);
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
