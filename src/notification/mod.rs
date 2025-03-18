#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

use anyhow::Result;
use std::{sync::Arc, time::SystemTime};
use tokio::{sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender}, task::JoinHandle};

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
    pub fn listen(&self) {
        let tx = self.tx.clone();

        #[cfg(target_os = "windows")]
        {
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();

                rt.block_on(async {
                    if let Err(e) = windows::notification_listener(tx).await {
                        eprintln!("Windows notification listener error: {:?}", e);
                    }
                });
            });
        }

        #[allow(clippy::let_underscore_future)]
        #[cfg(target_os = "linux")]
        {
            let _: JoinHandle<Result<()>> = tokio::spawn(async move {
                linux::notification_listener(tx).await?;
                Ok(())
            });
        }
    }

    pub async fn next_notify(&mut self) -> Option<Arc<Notification>> {
        self.rx.recv().await
    }
}
