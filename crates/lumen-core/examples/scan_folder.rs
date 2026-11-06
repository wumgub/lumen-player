//! Scan a folder into a library file, without the app:
//! `cargo run -p lumen-core --example scan_folder -- <library.db> <folder>`.
//! scripts/screenshots.sh uses it to build its demo library.

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(db), Some(folder)) = (args.next(), args.next()) else {
        eprintln!("usage: scan_folder <library.db> <folder>");
        std::process::exit(2);
    };
    let lib = lumen_core::Library::open(std::path::Path::new(&db)).expect("open library");
    lib.add_folder(&folder).expect("add folder");
    let result = lumen_core::scanner::scan(&lib, &[folder], |_| {}).expect("scan");
    println!("{result:?}");
}
