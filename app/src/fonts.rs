//! Text uses the system's own UI font, like the Electron app's `-apple-system,
//! 'Segoe UI', Roboto, ...` stack, with fallbacks for every other script
//! (Japanese first) read from the system too. Fonts are memory-mapped, so the big CJK collections only cost
//! the pages that actually get touched.

use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use eframe::egui::{self, FontData, FontDefinitions, FontFamily, FontTweak};
use eframe::epaint::text::VariationCoords;

/// Families the UI asks for by weight.
pub const MEDIUM: &str = "medium";
pub const BOLD: &str = "bold";
/// Bold at headline sizes, which SF Pro draws with its Display cut.
pub const DISPLAY: &str = "display";

pub fn medium(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name(MEDIUM.into()))
}

pub fn bold(size: f32) -> egui::FontId {
    let family = if size >= 20.0 { DISPLAY } else { BOLD };
    egui::FontId::new(size, FontFamily::Name(family.into()))
}

fn map(path: &str) -> Option<Arc<memmap2::Mmap>> {
    let file = File::open(Path::new(path)).ok()?;
    // Safety: system font files aren't modified while we run.
    let mmap = unsafe { memmap2::Mmap::map(&file) }.ok()?;
    Some(Arc::new(mmap))
}

fn first(paths: &[&str]) -> Option<Arc<memmap2::Mmap>> {
    paths.iter().find_map(|p| map(p))
}

#[cfg(target_os = "macos")]
const UI: &[&str] = &["/System/Library/Fonts/SFNS.ttf", "/System/Library/Fonts/Helvetica.ttc"];
#[cfg(target_os = "windows")]
const UI: &[&str] = &["C:\\Windows\\Fonts\\segoeui.ttf", "C:\\Windows\\Fonts\\arial.ttf"];
#[cfg(all(unix, not(target_os = "macos")))]
const UI: &[&str] = &[
    "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/TTF/DejaVuSans.ttf",
];

#[cfg(target_os = "windows")]
const UI_BOLD: &[&str] = &["C:\\Windows\\Fonts\\segoeuib.ttf"];
#[cfg(target_os = "windows")]
const UI_MEDIUM: &[&str] = &["C:\\Windows\\Fonts\\seguisb.ttf", "C:\\Windows\\Fonts\\segoeuib.ttf"];
#[cfg(all(unix, not(target_os = "macos")))]
const UI_BOLD: &[&str] = &[
    "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf",
    "/usr/share/fonts/noto/NotoSans-Bold.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
];
#[cfg(all(unix, not(target_os = "macos")))]
const UI_MEDIUM: &[&str] = &[
    "/usr/share/fonts/truetype/noto/NotoSans-Medium.ttf",
    "/usr/share/fonts/noto/NotoSans-Medium.ttf",
    "/usr/share/fonts/truetype/noto/NotoSans-SemiBold.ttf",
];

/// Fonts that come in the UI's four weights (regular, medium, bold,
/// display), as (path, index in the collection). Japanese goes first so kana
/// and kanji match each other, then Korean; bold titles stay bold in both.
#[cfg(target_os = "macos")]
const WEIGHTED: &[[(&str, u32); 4]] = &[
    [
        ("/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc", 0),
        ("/System/Library/Fonts/ヒラギノ角ゴシック W5.ttc", 0),
        ("/System/Library/Fonts/ヒラギノ角ゴシック W6.ttc", 0),
        ("/System/Library/Fonts/ヒラギノ角ゴシック W7.ttc", 0),
    ],
    [
        ("/System/Library/Fonts/AppleSDGothicNeo.ttc", 0),
        ("/System/Library/Fonts/AppleSDGothicNeo.ttc", 2),
        ("/System/Library/Fonts/AppleSDGothicNeo.ttc", 4),
        ("/System/Library/Fonts/AppleSDGothicNeo.ttc", 6),
    ],
];
#[cfg(target_os = "windows")]
const WEIGHTED: &[[(&str, u32); 4]] = &[
    [
        ("C:\\Windows\\Fonts\\YuGothR.ttc", 0),
        ("C:\\Windows\\Fonts\\YuGothM.ttc", 0),
        ("C:\\Windows\\Fonts\\YuGothB.ttc", 0),
        ("C:\\Windows\\Fonts\\YuGothB.ttc", 0),
    ],
    [
        ("C:\\Windows\\Fonts\\malgun.ttf", 0),
        ("C:\\Windows\\Fonts\\malgun.ttf", 0),
        ("C:\\Windows\\Fonts\\malgunbd.ttf", 0),
        ("C:\\Windows\\Fonts\\malgunbd.ttf", 0),
    ],
];
// Noto Sans CJK covers Korean too. Debian and Arch keep it in different places.
#[cfg(all(unix, not(target_os = "macos")))]
const WEIGHTED: &[[(&str, u32); 4]] = &[
    [
        ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", 0),
        ("/usr/share/fonts/opentype/noto/NotoSansCJK-Medium.ttc", 0),
        ("/usr/share/fonts/opentype/noto/NotoSansCJK-Bold.ttc", 0),
        ("/usr/share/fonts/opentype/noto/NotoSansCJK-Bold.ttc", 0),
    ],
    [
        ("/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc", 0),
        ("/usr/share/fonts/noto-cjk/NotoSansCJK-Medium.ttc", 0),
        ("/usr/share/fonts/noto-cjk/NotoSansCJK-Bold.ttc", 0),
        ("/usr/share/fonts/noto-cjk/NotoSansCJK-Bold.ttc", 0),
    ],
];

/// Every other script, in the order they're tried. Variable fonts among
/// these get the UI weights too.
#[cfg(target_os = "macos")]
const SCRIPTS: &[&str] = &[
    "/System/Library/Fonts/Hiragino Sans GB.ttc",
    "/System/Library/Fonts/SFArabic.ttf",
    "/System/Library/Fonts/SFHebrew.ttf",
    "/System/Library/Fonts/SFArmenian.ttf",
    "/System/Library/Fonts/SFGeorgian.ttf",
    "/System/Library/Fonts/ThonburiUI.ttc",
    "/System/Library/Fonts/Kohinoor.ttc",
    "/System/Library/Fonts/KohinoorBangla.ttc",
    "/System/Library/Fonts/KohinoorGujarati.ttc",
    "/System/Library/Fonts/KohinoorTelugu.ttc",
    "/System/Library/Fonts/Supplemental/Gurmukhi Sangam MN.ttc",
    "/System/Library/Fonts/Supplemental/Tamil Sangam MN.ttc",
    "/System/Library/Fonts/Supplemental/Kannada Sangam MN.ttc",
    "/System/Library/Fonts/Supplemental/Malayalam Sangam MN.ttc",
    "/System/Library/Fonts/Supplemental/Sinhala Sangam MN.ttc",
    "/System/Library/Fonts/Supplemental/Khmer Sangam MN.ttf",
    "/System/Library/Fonts/Supplemental/Lao Sangam MN.ttf",
    "/System/Library/Fonts/NotoSansMyanmar.ttc",
    "/System/Library/Fonts/Supplemental/KefaIII.ttf",
    "/System/Library/Fonts/Apple Symbols.ttf",
    "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
];
#[cfg(target_os = "windows")]
const SCRIPTS: &[&str] = &[
    "C:\\Windows\\Fonts\\msyh.ttc",
    "C:\\Windows\\Fonts\\Nirmala.ttc",
    "C:\\Windows\\Fonts\\Nirmala.ttf",
    "C:\\Windows\\Fonts\\LeelawUI.ttf",
    "C:\\Windows\\Fonts\\ebrima.ttf",
    "C:\\Windows\\Fonts\\KhmerUI.ttf",
    "C:\\Windows\\Fonts\\LaoUI.ttf",
    "C:\\Windows\\Fonts\\mmrtext.ttf",
    "C:\\Windows\\Fonts\\seguisym.ttf",
];

#[cfg(not(all(unix, not(target_os = "macos"))))]
fn scripts() -> Vec<String> {
    SCRIPTS.iter().map(|s| s.to_string()).collect()
}

/// Linux has no fixed list: take every Noto Sans script font that's
/// installed (NotoSansArabic-Regular.ttf, NotoSansThai-Regular.ttf, ...).
#[cfg(all(unix, not(target_os = "macos")))]
fn scripts() -> Vec<String> {
    fn walk(dir: &Path, depth: u32, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for e in entries.flatten() {
            let path = e.path();
            if path.is_dir() {
                if depth > 0 {
                    walk(&path, depth - 1, out);
                }
                continue;
            }
            let name = e.file_name().to_string_lossy().into_owned();
            let script = name.strip_prefix("NotoSans").and_then(|n| n.split_once("-Regular.")).map(|(s, _)| s);
            if script.is_some_and(|s| !s.is_empty() && !s.contains("Mono") && !s.starts_with("CJK")) {
                out.push(path.to_string_lossy().into_owned());
            }
        }
    }
    let mut out = Vec::new();
    walk(Path::new("/usr/share/fonts"), 3, &mut out);
    out.sort();
    for p in ["/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", "/usr/share/fonts/TTF/DejaVuSans.ttf"] {
        out.push(p.into());
    }
    out
}

/// Build the font set. `mono_ui` swaps the UI font for Courier New, the way
/// the Retro theme does.
pub fn definitions(mono_ui: bool) -> FontDefinitions {
    let mut defs = FontDefinitions::default();
    let mut regular = Vec::new();
    let mut medium_names = Vec::new();
    let mut bold_names = Vec::new();
    let mut display_names = Vec::new();

    let add = |defs: &mut FontDefinitions, name: &str, data: FontData| {
        defs.font_data.insert(name.to_owned(), Arc::new(data));
        name.to_owned()
    };

    #[cfg(target_os = "macos")]
    const COURIER: &[&str] = &["/System/Library/Fonts/Supplemental/Courier New.ttf"];
    #[cfg(target_os = "macos")]
    const COURIER_BOLD: &[&str] = &["/System/Library/Fonts/Supplemental/Courier New Bold.ttf"];
    #[cfg(target_os = "windows")]
    const COURIER: &[&str] = &["C:\\Windows\\Fonts\\cour.ttf"];
    #[cfg(target_os = "windows")]
    const COURIER_BOLD: &[&str] = &["C:\\Windows\\Fonts\\courbd.ttf"];
    #[cfg(all(unix, not(target_os = "macos")))]
    const COURIER: &[&str] = &["/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf"];
    #[cfg(all(unix, not(target_os = "macos")))]
    const COURIER_BOLD: &[&str] = &["/usr/share/fonts/truetype/liberation/LiberationMono-Bold.ttf"];

    if mono_ui {
        if let Some(m) = first(COURIER) {
            let n = add(&mut defs, "ui-regular", FontData::from_blob(m, 0));
            regular.push(n.clone());
            medium_names.push(n);
        }
        if let Some(m) = first(COURIER_BOLD) {
            bold_names.push(add(&mut defs, "ui-bold", FontData::from_blob(m, 0)));
        }
    } else if let Some(m) = first(UI) {
        // SF Pro is one variable font: weight and optical size are axes.
        // Browsers pick the optical size from the text size, so body text
        // gets the "Text" cut (17) and big headings the "Display" one; the
        // font's own default is Display, which looks wrong at 13px.
        let variable = !FontData::from_blob(m.clone(), 0).variation_axes().is_empty();
        let cut = |w: f32, opsz: f32| FontTweak {
            coords: if variable { VariationCoords::new([("wght", w), ("opsz", opsz)]) } else { Default::default() },
            ..Default::default()
        };
        regular.push(add(&mut defs, "ui-regular", FontData::from_blob(m.clone(), 0).tweak(cut(400.0, 17.0))));
        if variable {
            medium_names.push(add(&mut defs, "ui-medium", FontData::from_blob(m.clone(), 0).tweak(cut(500.0, 17.0))));
            bold_names.push(add(&mut defs, "ui-bold", FontData::from_blob(m.clone(), 0).tweak(cut(650.0, 17.0))));
            display_names.push(add(&mut defs, "ui-display", FontData::from_blob(m, 0).tweak(cut(760.0, 30.0))));
        }
        #[cfg(not(target_os = "macos"))]
        {
            if let Some(m) = first(UI_MEDIUM) {
                medium_names.push(add(&mut defs, "ui-medium", FontData::from_blob(m, 0)));
            }
            if let Some(m) = first(UI_BOLD) {
                bold_names.push(add(&mut defs, "ui-bold", FontData::from_blob(m, 0)));
            }
        }
    }

    // Fallbacks, one chain per weight: Japanese and Korean, then every other script,
    // then egui's bundled fonts (Ubuntu, emoji, icons) catching anything left.
    const WEIGHTS: [f32; 4] = [400.0, 500.0, 650.0, 760.0];
    let mut fallbacks: [Vec<String>; 4] = Default::default();
    for (i, faces) in WEIGHTED.iter().enumerate() {
        for (w, (path, index)) in faces.iter().enumerate() {
            if let Some(m) = map(path) {
                fallbacks[w].push(add(&mut defs, &format!("weighted-{i}-{w}"), FontData::from_blob(m, *index)));
            }
        }
    }
    for (i, path) in scripts().iter().enumerate() {
        let Some(m) = map(path) else { continue };
        if FontData::from_blob(m.clone(), 0).variation_axes().iter().any(|a| a.tag == *b"wght") {
            for (w, weight) in WEIGHTS.iter().enumerate() {
                let tweak = FontTweak { coords: VariationCoords::new([("wght", *weight)]), ..Default::default() };
                fallbacks[w].push(add(&mut defs, &format!("script-{i}-{w}"), FontData::from_blob(m.clone(), 0).tweak(tweak)));
            }
        } else {
            let n = add(&mut defs, &format!("script-{i}"), FontData::from_blob(m, 0));
            for f in &mut fallbacks {
                f.push(n.clone());
            }
        }
    }
    let builtin = defs.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    for f in &mut fallbacks {
        f.extend(builtin.iter().cloned());
    }

    let chain = |head: &[String], or: &[String], w: usize| -> Vec<String> {
        let mut v: Vec<String> = if head.is_empty() { or.to_vec() } else { head.to_vec() };
        v.extend(fallbacks[w].iter().cloned());
        v
    };
    defs.families.insert(FontFamily::Proportional, chain(&regular, &[], 0));
    defs.families.insert(FontFamily::Name(MEDIUM.into()), chain(&medium_names, &regular, 1));
    defs.families.insert(FontFamily::Name(BOLD.into()), chain(&bold_names, &regular, 2));
    let display = if display_names.is_empty() { bold_names.clone() } else { display_names };
    defs.families.insert(FontFamily::Name(DISPLAY.into()), chain(&display, &regular, 3));
    defs
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    #[test]
    fn sf_pro_axes() {
        let m = super::map(super::UI[0]).unwrap();
        for a in eframe::egui::FontData::from_blob(m, 0).variation_axes() {
            println!("{a:?}");
        }
    }
}
