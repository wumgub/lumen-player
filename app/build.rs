// Give the Windows .exe Lumen's icon. Nothing to do elsewhere.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../build/icon.ico");
        res.set("ProductName", "Lumen");
        res.set("FileDescription", "Lumen");
        if let Err(e) = res.compile() {
            println!("cargo:warning=couldn't embed the icon: {e}");
        }
    }
}
