#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::SystemTime};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            let _ = tokio::spawn(async move {
                let _ = linux::notification_listener(tx).await;
            });
        }
    }

    pub async fn next_notify(&mut self) -> Option<Arc<Notification>> {
        self.rx.recv().await
    }
}

pub fn send_notification(notify: Notification) {
    #[cfg(target_os = "windows")]
    {
        if let Err(e) = windows::send_notification(&notify.title, &notify.message, false) {
            eprintln!("Windows notification send error: {:?}", e);
        }
    }

    #[cfg(target_os = "linux")]
    {
        let _ = notify_rust::Notification::new()
            .summary(&notif.title)
            .body(&notif.message)
            .show();
    }
}
