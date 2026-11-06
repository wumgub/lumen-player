//! Open a library.db (use a copy!) and print what the native app would see.
//! `cargo run -p lumen-core --example check_library -- <library.db> [--rescan]`
use lumen_core::{Library, Query, scanner, stats};

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("path to a library.db copy");
    let rescan = args.next().as_deref() == Some("--rescan");
    let lib = Library::open(path.as_ref()).expect("open");
    let c = lib.counts().unwrap();
    println!("counts: {} tracks, {} artists, {} albums", c.tracks, c.artists, c.albums);
    let all = lib.query(&Query::default()).unwrap();
    println!("query all: {}", all.len());
    for s in ["кино", "КИНО", "rob co", "deftones"] {
        let n = lib.query(&Query { search: Some(s.into()), ..Default::default() }).unwrap().len();
        println!("search {s:?}: {n}");
    }
    let albums = lib.albums().unwrap();
    let with_art = albums.iter().filter(|a| lib.album_art(&a.album).unwrap().is_some()).count();
    println!("albums: {} ({} with a cover)", albums.len(), with_art);
    println!("artists: {}", lib.artists().unwrap().len());
    for p in lib.playlists().unwrap() {
        println!("playlist {:?}: {} tracks", p.name, lib.playlist_tracks(p.id).unwrap().len());
    }
    let h = lib.play_history().unwrap();
    let ids: std::collections::HashSet<i64> = all.iter().map(|t| t.id).collect();
    let s = stats::summarise(&h, |id| ids.contains(&id), chrono_now());
    println!("plays: {} sessions, {} counted, {}s total, top: {:?}", h.len(), s.total_plays, s.total_seconds,
        s.top.iter().take(3).map(|r| (lib.track(r.track_id).unwrap().unwrap().title, r.plays)).collect::<Vec<_>>());
    println!("settings: {:?}", lib.app_settings().unwrap());
    println!("folders: {:?}", lib.folders().unwrap());
    let edited: Vec<_> = all.iter().filter(|t| !t.note.is_empty()).map(|t| (&t.title, &t.note)).collect();
    println!("notes: {edited:?}");
    if rescan {
        let before: Vec<_> = all.iter().map(|t| (t.id, t.title.clone(), t.album.clone(), t.artist.clone())).collect();
        let t0 = std::time::Instant::now();
        let r = scanner::scan(&lib, &lib.folders().unwrap(), |_| {}).unwrap();
        println!("rescan: {r:?} in {:?}", t0.elapsed());
        let after = lib.query(&Query::default()).unwrap();
        let changed: Vec<_> = before.iter().filter_map(|(id, title, album, artist)| {
            let t = after.iter().find(|t| t.id == *id)?;
            (t.title != *title || t.album != *album || t.artist != *artist)
                .then(|| format!("{id}: {title:?}/{album:?}/{artist:?} -> {:?}/{:?}/{:?}", t.title, t.album, t.artist))
        }).collect();
        println!("after rescan: {} tracks, {} changed", after.len(), changed.len());
        for c in changed.iter().take(10) { println!("  {c}"); }
    }
}

fn chrono_now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64
}
