fn main() {
    println!("cargo:rerun-if-changed=ui/app.slint");
    println!("cargo:rerun-if-changed=../assets/icon.ico");
    slint_build::compile("ui/app.slint").unwrap();

    // Gate on TARGET, not host: cfg(windows) is also true when cross-compiling
    // from Windows to Linux, and rc.exe cannot emit resources for ELF targets.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../assets/icon.ico");
        res.set("ProductName", "codewig-live");
        res.set("FileDescription", "Live-coding UI for Bitwig Studio");
        res.set("CompanyName", "Codewig");
        res.set("LegalCopyright", "MIT");
        res.compile().expect("embed Windows icon/resources");
    }
}
