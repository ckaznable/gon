use std::fmt::Display;

use tokio::sync::mpsc::{self, Receiver, Sender};
use tray_item::TrayItem;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

#[derive(Debug, Clone, Copy)]
pub enum TrayEvent {
    BecomeHost,
    BecomeClient,
    Quit,
}

impl Display for TrayEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrayEvent::BecomeHost => write!(f, "Become Host"),
            TrayEvent::BecomeClient => write!(f, "Become Client"),
            TrayEvent::Quit => write!(f, "Quit"),
        }
    }
}

pub fn init_tray() -> (TrayItem, Receiver<TrayEvent>) {
    let (tx, rx) = mpsc::channel(1);

    #[cfg(target_os = "windows")]
    let mut tray = windows::sys_tray();
    #[cfg(target_os = "linux")]
    let mut tray = linux::sys_tray();

    add_menu_item(&mut tray, tx.clone(), TrayEvent::BecomeHost);
    add_menu_item(&mut tray, tx.clone(), TrayEvent::BecomeClient);
    add_menu_item(&mut tray, tx.clone(), TrayEvent::Quit);

    (tray, rx)
}


fn add_menu_item(tray: &mut TrayItem, tx: Sender<TrayEvent>,  event: TrayEvent) {
    tray.add_menu_item(event.to_string().as_str(), move || {
        let _tx = tx.clone();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.spawn(async move {
            if let Err(e) = _tx.send(event).await {
                println!("Error sending event: {:?}", e);
            }
        });
    }).unwrap();
}
