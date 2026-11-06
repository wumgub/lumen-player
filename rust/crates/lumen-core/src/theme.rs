//! Theme tokens: the same 14 keys, presets and `{ "lumenTheme": 1, ... }`
//! share format as the Electron app (theme/tokens.ts) and the Android app, so
//! a theme exported from one imports into the others.

use std::collections::BTreeMap;

pub type Tokens = BTreeMap<String, String>;

pub const TOKEN_KEYS: [&str; 14] = [
    "bg",
    "bg-2",
    "bg-3",
    "popover-bg",
    "fg",
    "fg-dim",
    "accent",
    "accent-2",
    "border",
    "viz-bg",
    "viz-fg",
    "viz-accent",
    "viz-accent-2",
    "font",
];

/// What the theme editor shows for each token, grouped like the web editor.
pub const TOKEN_LABELS: [(&str, &str, &str); 14] = [
    ("bg", "App background", "Backgrounds"),
    ("bg-2", "Panels & bars", "Backgrounds"),
    ("bg-3", "Hover & elevated", "Backgrounds"),
    ("popover-bg", "Menus & modals", "Backgrounds"),
    ("fg", "Text", "Text"),
    ("fg-dim", "Muted text", "Text"),
    ("accent", "Primary accent", "Accents"),
    ("accent-2", "Secondary accent", "Accents"),
    ("border", "Borders", "Lines"),
    ("viz-bg", "Visualizer background", "Visualizer"),
    ("viz-fg", "Visualizer foreground", "Visualizer"),
    ("viz-accent", "Visualizer accent", "Visualizer"),
    ("viz-accent-2", "Visualizer accent 2", "Visualizer"),
    ("font", "Font family", "Typography"),
];

pub const SYS_FONT: &str =
    "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif";
pub const MONO_FONT: &str = "'Courier New', ui-monospace, monospace";

pub const FONT_PRESETS: [(&str, &str); 6] = [
    ("System", SYS_FONT),
    ("Helvetica", "Helvetica, Arial, sans-serif"),
    ("Rounded", "'Avenir Next', 'Segoe UI', system-ui, sans-serif"),
    ("Serif", "Georgia, 'Times New Roman', serif"),
    ("Monospace", "'SF Mono', 'Courier New', ui-monospace, monospace"),
    ("Condensed", "'Oswald', 'Arial Narrow', sans-serif"),
];

pub struct ThemeInfo {
    pub id: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
}

pub const THEMES: [ThemeInfo; 7] = [
    ThemeInfo { id: "neon", label: "Neon", hint: "Dark cyberpunk glow" },
    ThemeInfo { id: "minimal", label: "Flashbang", hint: "Blinding light mode" },
    ThemeInfo { id: "minimal-dark", label: "Minimal Dark", hint: "Clean & muted, dark" },
    ThemeInfo { id: "retro", label: "Retro", hint: "Classic winamp VU" },
    ThemeInfo { id: "glass", label: "Glass", hint: "Frosted, art-tinted" },
    ThemeInfo { id: "blackout", label: "Blackout", hint: "Pure black for OLED" },
    ThemeInfo { id: "custom", label: "Custom", hint: "Your own palette" },
];

/// Themes the editor can start a custom theme from.
pub const START_FROM: [&str; 6] = ["neon", "minimal", "minimal-dark", "retro", "glass", "blackout"];

fn tokens(pairs: [(&str, &str); 14]) -> Tokens {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

/// The hex preset for a built-in theme (Neon for anything unknown).
pub fn preset(id: &str) -> Tokens {
    match id {
        "minimal" => tokens([
            ("bg", "#f7f7f9"), ("bg-2", "#ffffff"), ("bg-3", "#eeeef2"), ("popover-bg", "#ffffff"),
            ("fg", "#16161a"), ("fg-dim", "#70707c"), ("accent", "#5b6ccf"), ("accent-2", "#e0607e"),
            ("border", "#e2e2e8"), ("viz-bg", "#ffffff"), ("viz-fg", "#16161a"),
            ("viz-accent", "#5b6ccf"), ("viz-accent-2", "#e0607e"), ("font", SYS_FONT),
        ]),
        "minimal-dark" => tokens([
            ("bg", "#121316"), ("bg-2", "#191b1f"), ("bg-3", "#24272c"), ("popover-bg", "#191b1f"),
            ("fg", "#e8eaed"), ("fg-dim", "#898f98"), ("accent", "#7c8cff"), ("accent-2", "#f0879f"),
            ("border", "#2b2f35"), ("viz-bg", "#121316"), ("viz-fg", "#e8eaed"),
            ("viz-accent", "#7c8cff"), ("viz-accent-2", "#f0879f"), ("font", SYS_FONT),
        ]),
        "retro" => tokens([
            ("bg", "#10140f"), ("bg-2", "#1a2016"), ("bg-3", "#242b1e"), ("popover-bg", "#1a2016"),
            ("fg", "#c7f270"), ("fg-dim", "#7f8f5a"), ("accent", "#a3e635"), ("accent-2", "#f59e0b"),
            ("border", "#39421f"), ("viz-bg", "#10140f"), ("viz-fg", "#c7f270"),
            ("viz-accent", "#a3e635"), ("viz-accent-2", "#f59e0b"), ("font", MONO_FONT),
        ]),
        "glass" => tokens([
            ("bg", "#0c1018"), ("bg-2", "#182030"), ("bg-3", "#232d40"), ("popover-bg", "#101420"),
            ("fg", "#eef2f8"), ("fg-dim", "#9fb0c3"), ("accent", "#7dd3fc"), ("accent-2", "#f0abfc"),
            ("border", "#2a3647"), ("viz-bg", "#0c1018"), ("viz-fg", "#eef2f8"),
            ("viz-accent", "#7dd3fc"), ("viz-accent-2", "#f0abfc"), ("font", SYS_FONT),
        ]),
        // Same values as the Android app's Blackout.
        "blackout" => tokens([
            ("bg", "#000000"), ("bg-2", "#000000"), ("bg-3", "#0a0a0a"), ("popover-bg", "#000000"),
            ("fg", "#ffffff"), ("fg-dim", "#8a8a8a"), ("accent", "#22d3ee"), ("accent-2", "#22d3ee"),
            ("border", "#1b1b1b"), ("viz-bg", "#000000"), ("viz-fg", "#ffffff"),
            ("viz-accent", "#22d3ee"), ("viz-accent-2", "#22d3ee"), ("font", SYS_FONT),
        ]),
        _ => tokens([
            ("bg", "#0a0a12"), ("bg-2", "#11111d"), ("bg-3", "#1a1a2b"), ("popover-bg", "#11111d"),
            ("fg", "#f0f0ff"), ("fg-dim", "#8a8aa8"), ("accent", "#22d3ee"), ("accent-2", "#c026d3"),
            ("border", "#23233a"), ("viz-bg", "#0a0a12"), ("viz-fg", "#ffffff"),
            ("viz-accent", "#22d3ee"), ("viz-accent-2", "#e879f9"), ("font", SYS_FONT),
        ]),
    }
}

/// Serialise a custom theme into the shareable string (keys in the usual order).
pub fn export(t: &Tokens) -> String {
    let mut out = String::from("{\n  \"lumenTheme\": 1,\n  \"tokens\": {");
    let mut first = true;
    for key in TOKEN_KEYS {
        if let Some(v) = t.get(key).filter(|v| !v.is_empty()) {
            out.push_str(if first { "\n" } else { ",\n" });
            first = false;
            out.push_str(&format!(
                "    {}: {}",
                serde_json::to_string(key).unwrap_or_default(),
                serde_json::to_string(v).unwrap_or_default()
            ));
        }
    }
    out.push_str(if first { "}\n}" } else { "\n  }\n}" });
    out
}

/// Parse a shared theme back into tokens. Accepts the wrapped form or a bare
/// token map, shrugs off a markdown fence or stray backticks around it (a
/// common way imports break), keeps only known keys, and returns None if
/// nothing usable is there.
pub fn parse(text: &str) -> Option<Tokens> {
    let mut src = text.trim();
    if let Some(rest) = src.strip_prefix("```") {
        let rest = rest.trim_start_matches('`');
        let lang_end = rest.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(rest.len());
        src = rest[lang_end..].trim_start();
    }
    let src = src.trim_end().trim_end_matches('`').trim_matches('`').trim();
    let data: serde_json::Value = serde_json::from_str(src).ok().or_else(|| {
        let (a, b) = (src.find('{')?, src.rfind('}')?);
        (a < b).then(|| serde_json::from_str(&src[a..=b]).ok()).flatten()
    })?;
    let obj = data.as_object()?;
    let raw = obj.get("tokens").and_then(|t| t.as_object()).unwrap_or(obj);
    let out: Tokens = TOKEN_KEYS
        .iter()
        .filter_map(|k| {
            let v = raw.get(*k)?.as_str()?.trim();
            (!v.is_empty()).then(|| (k.to_string(), v.to_string()))
        })
        .collect();
    (!out.is_empty()).then_some(out)
}

/// Any CSS colour the editor writes (#rgb, #rrggbb, #rrggbbaa, rgb(), rgba())
/// as sRGB bytes with alpha.
pub fn parse_color(s: &str) -> Option<[u8; 4]> {
    let s = s.trim();
    if let Some(h) = s.strip_prefix('#') {
        let full: String = match h.len() {
            3 | 4 => h.chars().flat_map(|c| [c, c]).collect(),
            6 | 8 => h.to_owned(),
            _ => return None,
        };
        let byte = |i: usize| u8::from_str_radix(full.get(i..i + 2)?, 16).ok();
        return Some([byte(0)?, byte(2)?, byte(4)?, if full.len() == 8 { byte(6)? } else { 255 }]);
    }
    let inner = s.strip_prefix("rgba(").or_else(|| s.strip_prefix("rgb("))?.strip_suffix(')')?;
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    if parts.len() < 3 {
        return None;
    }
    let ch = |p: &str| p.parse::<f32>().ok().map(|v| v.clamp(0.0, 255.0).round() as u8);
    let a = match parts.get(3) {
        Some(p) => (p.parse::<f32>().ok()?.clamp(0.0, 1.0) * 255.0).round() as u8,
        None => 255,
    };
    Some([ch(parts[0])?, ch(parts[1])?, ch(parts[2])?, a])
}

/// Perceived-luminance darkness test, as the web app does it.
pub fn is_dark(c: [u8; 4]) -> bool {
    (0.299 * c[0] as f32 + 0.587 * c[1] as f32 + 0.114 * c[2] as f32) < 128.0
}

/// A readable text colour for on top of `c` (white on dark, near-black on light).
pub fn contrast_on(c: [u8; 4]) -> [u8; 4] {
    if is_dark(c) { [255, 255, 255, 255] } else { [0x14, 0x14, 0x14, 255] }
}

/// Whether a theme's font stack asks for a monospace face, like Retro's.
pub fn is_mono_font(stack: &str) -> bool {
    let s = stack.to_ascii_lowercase();
    s.trim_start_matches(['\'', '"']).starts_with("courier") || s.starts_with("'sf mono'") || s.starts_with("monospace")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_then_parse_round_trips() {
        let t = preset("retro");
        let text = export(&t);
        assert!(text.starts_with("{\n  \"lumenTheme\": 1,\n  \"tokens\": {\n    \"bg\": \"#10140f\""));
        assert_eq!(parse(&text), Some(t));
    }

    #[test]
    fn parse_tolerates_fences_and_bare_maps() {
        let fenced = "```json\n{\"lumenTheme\":1,\"tokens\":{\"bg\":\"#000000\",\"junk\":\"x\"}}\n```";
        let t = parse(fenced).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t["bg"], "#000000");
        assert_eq!(parse("{\"accent\": \" #ff0000 \"}").unwrap()["accent"], "#ff0000");
        assert_eq!(parse("here you go: {\"fg\":\"#fff\"} enjoy").unwrap()["fg"], "#fff");
        assert_eq!(parse("not a theme"), None);
        assert_eq!(parse("{\"nothing\": 1}"), None);
    }

    #[test]
    fn colours_parse() {
        assert_eq!(parse_color("#22d3ee"), Some([0x22, 0xd3, 0xee, 255]));
        assert_eq!(parse_color("#fff"), Some([255, 255, 255, 255]));
        assert_eq!(parse_color("rgba(255, 255, 255, 0.06)"), Some([255, 255, 255, 15]));
        assert_eq!(parse_color("rgb(1,2,3)"), Some([1, 2, 3, 255]));
        assert_eq!(parse_color("nope"), None);
        assert!(is_dark(parse_color("#0a0a12").unwrap()));
        assert!(!is_dark(parse_color("#f7f7f9").unwrap()));
        assert!(is_mono_font(MONO_FONT));
        assert!(!is_mono_font(SYS_FONT));
    }
}
