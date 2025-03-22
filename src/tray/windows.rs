use tray_item::{TrayItem, IconSource};

pub fn sys_tray() -> TrayItem {
    TrayItem::new(
        "Gon",
        IconSource::Resource("tray-default"),
    )
    .unwrap()
}

