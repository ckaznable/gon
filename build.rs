#[cfg(target_os = "windows")]
extern crate embed_resource;

fn main() {
    #[cfg(target_os = "windows")]
    embed_resource::compile("resources/tray.rc", embed_resource::NONE);
}
