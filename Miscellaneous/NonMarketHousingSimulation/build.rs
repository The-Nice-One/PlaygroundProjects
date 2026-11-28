// Embed the Windows icon resource (build/windows/icon.rc → icon.ico) into the
// release executable so it shows the game icon in Explorer / the taskbar.
// On non-Windows targets this is a no-op.
extern crate embed_resource;
use std::env;

fn main() {
    let target = env::var("TARGET").unwrap();
    if target.contains("windows") {
        embed_resource::compile("build/windows/icon.rc");
    }
}
