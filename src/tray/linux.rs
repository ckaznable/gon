use std::io::Cursor;

use tray_item::{IconSource, TrayItem};

pub fn sys_tray() -> TrayItem {
    let icon = Cursor::new(include_bytes!("../../resources/icon.png"));
    let decoder_red = png::Decoder::new(icon);
    let mut reader = decoder_red.read_info().unwrap();
    let mut buf_icon = vec![0;reader.output_buffer_size()];
    reader.next_frame(&mut buf_icon).unwrap();

    let icon = IconSource::Data{data: buf_icon, height: 32, width: 32};

    TrayItem::new("Gon", icon).unwrap()
}
