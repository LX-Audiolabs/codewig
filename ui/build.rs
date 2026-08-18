fn main() {
    println!("cargo:rerun-if-changed=ui/app.slint");
    println!("cargo:rerun-if-changed=../assets/icon.ico");
    slint_build::compile("ui/app.slint").unwrap();

    // Two gates needed: #[cfg(windows)] matches where the winresource *crate* is
    // available (build scripts compile for the HOST), CARGO_CFG_TARGET_OS decides
    // whether resources should actually be embedded (skip when cross-compiling
    // from Windows to Linux).
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../assets/icon.ico");
        res.set("ProductName", "codewig-live");
        res.set("FileDescription", "Live-coding UI for Bitwig Studio");
        res.set("CompanyName", "Codewig");
        res.set("LegalCopyright", "Copyright (C) 2026 LX AudioLabs");
        res.compile().expect("embed Windows icon/resources");
    }
}
