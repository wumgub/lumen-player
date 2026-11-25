// Link libmpv. On macOS and Linux pkg-config finds it (Homebrew's mpv, or the
// distro's libmpv-dev). On Windows, or to point at a custom build, set
// MPV_LIB_DIR to the folder holding mpv.lib / libmpv.dylib / libmpv.so.
fn main() {
    println!("cargo:rerun-if-env-changed=MPV_LIB_DIR");
    if let Ok(dir) = std::env::var("MPV_LIB_DIR") {
        println!("cargo:rustc-link-search=native={dir}");
        println!("cargo:rustc-link-lib=dylib=mpv");
        return;
    }
    if pkg_config::Config::new().atleast_version("2.0").probe("mpv").is_err() {
        // Fall back to a plain -lmpv and let the linker look, in Homebrew's
        // folders too on a Mac (GitHub's macOS 26 runners have mpv there but
        // pkg-config can't see it).
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
            for dir in ["/opt/homebrew/lib", "/usr/local/lib"] {
                if std::path::Path::new(dir).join("libmpv.dylib").exists() {
                    println!("cargo:rustc-link-search=native={dir}");
                }
            }
        }
        println!("cargo:rustc-link-lib=dylib=mpv");
    }
}
