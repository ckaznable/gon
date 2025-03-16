#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

use anyhow::Result;
use std::{sync::Arc, time::SystemTime};
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone)]
pub struct Notification {
    pub app_name: String,
    pub app_icon: Option<Vec<u8>>,
    pub title: String,
    pub message: String,
    pub timestamp: SystemTime,
}

pub async fn notification_listener(tx: UnboundedSender<Arc<Notification>>) -> Result<()> {
    #[cfg(target_os = "windows")]
    windows::notification_listener(tx).await?;

    #[cfg(target_os = "linux")]
    linux::notification_listener(tx).await?;

    Ok(())
}
