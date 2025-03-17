use anyhow::Result;
use daemon::{node::Node, service::AppService};
use tokio::select;
use crate::notification::SystemNotificationListener;

mod daemon;
mod notification;

#[tokio::main]
async fn main() -> Result<()> {
    let mut listener = SystemNotificationListener::default();
    listener.listen();

    let node = Node::new().await?;
    let mut service = AppService::new(node.addr, node.port)?;

    loop {
        select! {
            _ = service.next() => {},
            notif = listener.next_notify() => {
                println!("Received notification: {:?}", notif);
            }
        }
    }
}
