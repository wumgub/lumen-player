//! Writing edits back into a file's own tags, for the Edit Metadata setting.
//! The tags are written to a copy next to the file, which then replaces it,
//! so a failed write never leaves a half-written song behind.

use std::path::{Path, PathBuf};

use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::prelude::*;
use lofty::tag::{Tag, TagType};

use crate::models::Art;

/// What Lumen shows for a song, to be written into its file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FileTags {
    pub title: String,
    /// "Unknown Artist" (the scanner's placeholder) clears the artist tag.
    pub artist: String,
    pub album_artist: String,
    pub album: String,
    /// The front cover to embed; None leaves the file's pictures alone.
    pub cover: Option<Art>,
}

fn set(tag: &mut Tag, key: ItemKey, value: &str) {
    if value.is_empty() {
        tag.remove_key(key);
    } else {
        tag.insert_text(key, value.to_owned());
    }
}

fn apply(tag: &mut Tag, t: &FileTags) {
    set(tag, ItemKey::TrackTitle, &t.title);
    set(tag, ItemKey::TrackArtist, if t.artist == "Unknown Artist" { "" } else { &t.artist });
    set(tag, ItemKey::AlbumArtist, &t.album_artist);
    set(tag, ItemKey::AlbumTitle, &t.album);
    if let Some(art) = &t.cover {
        tag.remove_picture_type(PictureType::CoverFront);
        let pic = Picture::unchecked(art.data.clone())
            .pic_type(PictureType::CoverFront)
            .mime_type(MimeType::from_str(&art.mime))
            .build();
        tag.push_picture(pic);
    }
}

/// `dir/.name.lumen-tmp.ext`: hidden, and keeps the extension so lofty
/// still knows what kind of file it is.
fn temp_path(path: &Path) -> Option<PathBuf> {
    let stem = path.file_stem()?.to_string_lossy();
    let name = match path.extension() {
        Some(ext) => format!(".{stem}.lumen-tmp.{}", ext.to_string_lossy()),
        None => format!(".{stem}.lumen-tmp"),
    };
    Some(path.with_file_name(name))
}

/// Write `t` into every tag the file has (adding its usual kind of tag if it
/// has none), then swap the result in for the original.
pub fn write(path: &Path, t: &FileTags) -> Result<(), String> {
    let tmp = temp_path(path).ok_or("not a file")?;
    let result = (|| {
        std::fs::copy(path, &tmp).map_err(|e| e.to_string())?;
        let mut file = lofty::read_from_path(&tmp).map_err(|e| e.to_string())?;
        if file.primary_tag().is_none() {
            file.insert_tag(Tag::new(file.primary_tag_type()));
        }
        let kinds: Vec<TagType> = file.tags().iter().map(|t| t.tag_type()).collect();
        for kind in kinds {
            if let Some(tag) = file.tag_mut(kind) {
                apply(tag, t);
            }
        }
        file.save_to_path(&tmp, WriteOptions::default()).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::read_track;

    // A real MP3 needs real frames for lofty to accept it, so build a
    // minimal WAV, which takes the same tag paths through lofty (ID3v2 in a
    // RIFF chunk) and needs no encoder.
    fn wav(path: &Path) {
        let samples = 4410u32;
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36 + samples * 2).to_le_bytes());
        b.extend_from_slice(b"WAVEfmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&44100u32.to_le_bytes());
        b.extend_from_slice(&88200u32.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&16u16.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&(samples * 2).to_le_bytes());
        b.resize(b.len() + samples as usize * 2, 0);
        std::fs::write(path, b).unwrap();
    }

    #[test]
    fn edits_land_in_the_file_and_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("song.wav");
        wav(&path);
        let cover = Art { mime: "image/png".into(), data: b"\x89PNG\r\n\x1a\nnot really".to_vec() };
        let t = FileTags {
            title: "ヒカル".into(),
            artist: "宇多田ヒカル".into(),
            album_artist: String::new(),
            album: "First Love".into(),
            cover: Some(cover.clone()),
        };
        write(&path, &t).unwrap();
        let back = read_track(path.to_str().unwrap());
        assert_eq!(back.title, "ヒカル");
        assert_eq!(back.artist, "宇多田ヒカル");
        assert_eq!(back.album, "First Love");
        assert_eq!(back.art.map(|a| a.data), Some(cover.data));
        // No temp file left over.
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);

        // The placeholder artist and an empty album clear those tags.
        write(&path, &FileTags { title: "ヒカル".into(), artist: "Unknown Artist".into(), ..Default::default() }).unwrap();
        let back = read_track(path.to_str().unwrap());
        assert_eq!(back.artist, "Unknown Artist");
        assert_eq!(back.album, "");
        assert!(back.art.is_some(), "no cover given, so the old one stays");
    }

    #[test]
    fn a_failed_write_leaves_the_file_alone() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("song.mp3");
        std::fs::write(&path, b"not audio at all").unwrap();
        assert!(write(&path, &FileTags { title: "x".into(), ..Default::default() }).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"not audio at all");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
