use anyhow::Result;
use tokio::{select, sync::mpsc::unbounded_channel};
use std::sync::Arc;
use crate::notification::Notification;

mod notification;

#[tokio::main]
async fn main() -> Result<()> {
    let (tx, mut rx) = unbounded_channel::<Arc<Notification>>();

    loop {
        select! {
            _ = notification::notification_listener(tx.clone()) => {}
            Some(notif) = rx.recv() => {
                println!("Received notification: {:?}", notif);
            }
        }
    }
}
