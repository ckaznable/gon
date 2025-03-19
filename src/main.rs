use anyhow::Result;
use daemon::{node::Node, service::AppService};
use tokio::select;
use crate::notification::SystemNotificationListener;

mod daemon;
mod notification;

#[tokio::main]
async fn main() -> Result<()> {
    let pass = std::env::var("GON_PASS").ok().unwrap_or(String::from("pass"));

    let mut listener = SystemNotificationListener::default();
    listener.listen();

    let (mut node, addr) = Node::new(pass.as_bytes()).await?;
    let mut service = AppService::new(addr)?;

    let mut messaeg_rx = node.listen().await?;

    loop {
        select! {
            _ = service.next() => {},
            notif = listener.next_notify() => {
                println!("Received notification: {:?}", notif);
            }
            Some((stream, msg)) = messaeg_rx.recv() => {
                println!("Received new Message {:?}", msg)
            }
        }
    }
}
