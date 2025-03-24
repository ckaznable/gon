use std::fmt::Display;

use tokio::sync::mpsc::{self, Receiver, Sender};
use tray_item::{TrayItem, IconSource};

#[derive(Debug, Clone, Copy)]
pub enum TrayEvent {
    BecomeHost,
    BecomeClient,
    Quit,
}

pub enum TrayIcon {
    Default,
    Host,
}

impl TrayIcon {
    #[cfg(target_os = "windows")]
    pub fn icon_source(&self) -> IconSource {
        match self {
            TrayIcon::Default => IconSource::Resource("tray-default"),
            TrayIcon::Host => IconSource::Resource("tray-host"),
        }
    }

    #[cfg(target_os = "linux")]
    pub fn icon_source(&self) -> IconSource {
        use std::io::Cursor;

        let data = match self {
            TrayIcon::Default => include_bytes!("../resources/icon.png").as_slice(),
            TrayIcon::Host => include_bytes!("../resources/tray-host.png").as_slice(),
        };

        let icon = Cursor::new(data);
        let decoder_red = png::Decoder::new(icon);
        let mut reader = decoder_red.read_info().unwrap();
        let mut buf_icon = vec![0;reader.output_buffer_size()];
        reader.next_frame(&mut buf_icon).unwrap();

        IconSource::Data{data: buf_icon, height: 32, width: 32}
    }
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
    let mut tray = TrayItem::new("Gon", TrayIcon::Default.icon_source()).unwrap();

    add_menu_item(&mut tray, tx.clone(), TrayEvent::BecomeHost);
    add_menu_item(&mut tray, tx.clone(), TrayEvent::BecomeClient);
    add_menu_item(&mut tray, tx.clone(), TrayEvent::Quit);

    (tray, rx)
}

pub fn set_icon(tray: &mut TrayItem, icon: TrayIcon) {
    tray.set_icon(icon.icon_source()).unwrap();
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
