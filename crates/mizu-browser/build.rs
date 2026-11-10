//! Embeds the application icon, version information and the Windows application manifest
//! (per-monitor DPI v2, Common Controls 6, Windows 10/11 compatibility).

fn main() {
    println!("cargo:rerun-if-changed=resources/mizu.rc");
    println!("cargo:rerun-if-changed=resources/mizu.manifest");
    println!("cargo:rerun-if-changed=../../assets/icons/mizu.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("resources/mizu.rc", embed_resource::NONE)
            .manifest_optional()
            .expect("failed to compile Windows resources");
    }
}
