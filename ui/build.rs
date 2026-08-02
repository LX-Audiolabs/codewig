fn main() {
    println!("cargo:rerun-if-changed=ui/app.slint");
    println!("cargo:rerun-if-changed=../assets/icon.ico");
    slint_build::compile("ui/app.slint").unwrap();

    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../assets/icon.ico");
        res.set("ProductName", "codewig-live");
        res.set("FileDescription", "Live-coding UI for Bitwig Studio");
        res.set("CompanyName", "Codewig");
        res.set("LegalCopyright", "MIT");
        res.compile().expect("embed Windows icon/resources");
    }
}
