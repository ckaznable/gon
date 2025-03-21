use std::fmt::Display;

use tokio::sync::mpsc::{self, Receiver, Sender};
use tray_item::{IconSource, TrayItem};

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
    let mut tray = TrayItem::new(
        "Gon",
        IconSource::Resource("tray-default"),
    )
    .unwrap();

    add_menu_item(&mut tray, tx.clone(), TrayEvent::BecomeHost);
    tray.inner_mut().add_separator().unwrap();
    add_menu_item(&mut tray, tx.clone(), TrayEvent::BecomeClient);
    tray.inner_mut().add_separator().unwrap();
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
