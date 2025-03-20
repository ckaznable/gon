use std::{net::{IpAddr, Ipv4Addr, SocketAddr}, sync::Arc};

use crate::notification::SystemNotificationListener;
use anyhow::Result;
use client::Client;
use daemon::{
    node::Node,
    protocol::{handle_message, Method, Payload},
    service::{AppService, AppServiceEvent},
};
use tokio::{select, sync::RwLock};

mod client;
mod daemon;
mod notification;

#[tokio::main]
async fn main() -> Result<()> {
    let pass = std::env::var("GON_PASS")
        .ok()
        .unwrap_or(String::from("pass"));

    let mut listener = SystemNotificationListener::default();
    listener.listen();

    let (mut node, addr) = Node::new(pass.as_bytes()).await?;
    let mut messaeg_rx = node.listen().await?;
    let node = Arc::new(node);

    let client = Client::new(node.clone());

    let mut service = AppService::new(addr)?;

    let host: Arc<RwLock<Option<SocketAddr>>> = Arc::new(RwLock::new(None));

    loop {
        select! {
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
                let Some(host) = *host.read().await else {
                    continue;
                };

                let mut stream = client.connect(host).await?;
                stream.send_notification(notif.as_ref().clone()).await?;
            }
            Some((mut stream, msg)) = messaeg_rx.recv() => {
                println!("Received new Message {:?}", msg);
                if msg.is_done() {
                    continue;
                }

                // host changed
                if let Method::HostChanged = msg.method {
                    if let Payload::Address(a, b, c, d, port) = msg.payload {
                        *host.write().await = Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(a, b, c, d)), port));
                    }

                    continue;
                }

                let res = handle_message(msg);
                let _ = node.reply(&mut stream, res).await;
            }
        }
    }
}
