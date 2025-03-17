use anyhow::Result;
use tokio::select;
use crate::notification::SystemNotificationListener;

mod notification;

#[tokio::main]
async fn main() -> Result<()> {
    let mut listener = SystemNotificationListener::default();
    listener.listen().await?;

    loop {
        select! {
            Some(notif) = listener.recv() => {
                println!("Received notification: {:?}", notif);
            }
        }
    }
}
