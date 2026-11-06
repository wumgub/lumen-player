//! Text uses the system's own UI font, like the Electron app's `-apple-system,
//! 'Segoe UI', Roboto, ...` stack, with CJK and emoji fallbacks read from the
//! system too. Fonts are memory-mapped, so the big CJK collections only cost
//! the pages that actually get touched.

use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use eframe::egui::{self, FontData, FontDefinitions, FontFamily, FontTweak};
use eframe::epaint::text::VariationCoords;

/// Families the UI asks for by weight.
pub const MEDIUM: &str = "medium";
pub const BOLD: &str = "bold";

pub fn medium(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name(MEDIUM.into()))
}

pub fn bold(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name(BOLD.into()))
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

#[cfg(target_os = "macos")]
const CJK: &[(&str, u32)] = &[
    ("/System/Library/Fonts/Hiragino Sans GB.ttc", 0),
    ("/System/Library/Fonts/AppleSDGothicNeo.ttc", 0),
];
#[cfg(target_os = "windows")]
const CJK: &[(&str, u32)] = &[
    ("C:\\Windows\\Fonts\\msyh.ttc", 0),
    ("C:\\Windows\\Fonts\\YuGothR.ttc", 0),
    ("C:\\Windows\\Fonts\\malgun.ttf", 0),
];
#[cfg(all(unix, not(target_os = "macos")))]
const CJK: &[(&str, u32)] = &[
    ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", 0),
    ("/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc", 0),
];

/// Build the font set. `mono_ui` swaps the UI font for Courier New, the way
/// the Retro theme does.
pub fn definitions(mono_ui: bool) -> FontDefinitions {
    let mut defs = FontDefinitions::default();
    let mut regular = Vec::new();
    let mut medium_names = Vec::new();
    let mut bold_names = Vec::new();

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
        // SF Pro is one variable font; weights are points on its "wght" axis.
        let variable = !FontData::from_blob(m.clone(), 0).variation_axes().is_empty();
        let weight = |w: f32| FontTweak {
            coords: if variable { VariationCoords::new([("wght", w)]) } else { Default::default() },
            ..Default::default()
        };
        regular.push(add(&mut defs, "ui-regular", FontData::from_blob(m.clone(), 0).tweak(weight(400.0))));
        if variable {
            medium_names.push(add(&mut defs, "ui-medium", FontData::from_blob(m.clone(), 0).tweak(weight(560.0))));
            bold_names.push(add(&mut defs, "ui-bold", FontData::from_blob(m, 0).tweak(weight(700.0))));
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

    let mut fallbacks = Vec::new();
    for (i, (path, index)) in CJK.iter().enumerate() {
        if let Some(m) = map(path) {
            fallbacks.push(add(&mut defs, &format!("cjk-{i}"), FontData::from_blob(m, *index)));
        }
    }
    // egui's bundled fonts (Ubuntu, emoji, icons) go last, catching anything left.
    let builtin = defs.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    fallbacks.extend(builtin);

    let chain = |head: &[String], or: &[String]| -> Vec<String> {
        let mut v: Vec<String> = if head.is_empty() { or.to_vec() } else { head.to_vec() };
        v.extend(fallbacks.iter().cloned());
        v
    };
    defs.families.insert(FontFamily::Proportional, chain(&regular, &[]));
    defs.families.insert(FontFamily::Name(MEDIUM.into()), chain(&medium_names, &regular));
    defs.families.insert(FontFamily::Name(BOLD.into()), chain(&bold_names, &regular));
    defs
}
