fn main() {
    println!("cargo:rerun-if-changed=tray.rc");
    println!("cargo:rerun-if-changed=icons/empty.ico");
    println!("cargo:rerun-if-changed=icons/fill.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("tray.rc", embed_resource::NONE);
    }
}
