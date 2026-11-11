//! Embeds the Mizu executable to install (`MIZU_SETUP_PAYLOAD=<path to mizu.exe>`), plus the
//! icon and application manifest. Without a payload the installer builds but refuses to run.

fn main() {
    println!("cargo:rerun-if-env-changed=MIZU_SETUP_PAYLOAD");
    println!("cargo:rerun-if-changed=resources/setup.rc");
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let payload = std::env::var("MIZU_SETUP_PAYLOAD").ok().filter(|p| !p.is_empty());
    match payload {
        Some(path) => {
            println!("cargo:rerun-if-changed={path}");
            std::fs::copy(&path, out.join("payload.bin")).expect("copy MIZU_SETUP_PAYLOAD");
        }
        None => std::fs::write(out.join("payload.bin"), b"").expect("write empty payload"),
    }
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("resources/setup.rc", embed_resource::NONE)
            .manifest_optional()
            .expect("compile installer resources");
    }
}
