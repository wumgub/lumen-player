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
        // Fall back to a plain -lmpv and let the linker look.
        println!("cargo:rustc-link-lib=dylib=mpv");
    }
}
