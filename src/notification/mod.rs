#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

use anyhow::Result;
use std::{sync::Arc, time::SystemTime};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender, UnboundedReceiver};

#[derive(Debug, Clone)]
pub struct Notification {
    pub app_name: String,
    pub app_icon: Option<Vec<u8>>,
    pub title: String,
    pub message: String,
    pub timestamp: SystemTime,
}

pub struct SystemNotificationListener {
    tx: UnboundedSender<Arc<Notification>>,
    rx: UnboundedReceiver<Arc<Notification>>,
}

impl Default for SystemNotificationListener {
    fn default() -> Self {
        let (tx, rx) = unbounded_channel();

        SystemNotificationListener {
            tx,
            rx,
        }
    }
}

impl SystemNotificationListener {
    pub async fn listen(&self) -> Result<()> {
        let tx = self.tx.clone();

        #[cfg(target_os = "windows")]
        windows::notification_listener(tx).await?;

        #[cfg(target_os = "linux")]
        linux::notification_listener(tx).await?;

        Ok(())
    }

    pub async fn recv(&mut self) -> Option<Arc<Notification>> {
        self.rx.recv().await
    }
}

