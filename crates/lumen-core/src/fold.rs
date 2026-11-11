//! Search folding, the same as `fold()` in the Android app: decompose, drop
//! accents from Latin, Greek and Cyrillic letters, lower-case. So `кино`
//! finds `Кино`, `ё` matches `е` and `é` matches `e`. Marks in other scripts
//! are part of the letter (Japanese dakuten, Hindi and Thai vowel signs), so
//! they stay, and katakana folds to hiragana so `カル` finds `かる`.

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

pub fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut accented = false; // the last letter takes droppable accents
    for c in s.nfkd() {
        if is_combining_mark(c) {
            if !accented {
                out.push(c);
            }
            continue;
        }
        accented = matches!(c as u32, 0..=0x052F | 0x1E00..=0x1FFF);
        out.extend(c.to_lowercase());
    }
    out.nfc().map(hiragana).collect()
}

/// Katakana → hiragana; ヴ and the small forms all have a hiragana twin.
fn hiragana(c: char) -> char {
    match c as u32 {
        k @ 0x30A1..=0x30F6 => char::from_u32(k - 0x60).unwrap_or(c),
        _ => c,
    }
}

#[cfg(test)]
mod tests {
    use super::fold;

    #[test]
    fn folds_case_and_accents_in_any_script() {
        assert_eq!(fold("Кино"), "кино");
        assert_eq!(fold("Ёлка"), "елка");
        assert_eq!(fold("Beyoncé"), "beyonce");
        assert_eq!(fold("ＡＢＣ"), "abc"); // full-width, via NFKD
        assert_eq!(fold("Rób Co Chcesz"), "rob co chcesz");
        assert_eq!(fold("Ελένη"), "ελενη");
    }

    #[test]
    fn keeps_letters_whole_outside_latin() {
        assert_eq!(fold("ガ"), fold("が"));
        assert_ne!(fold("が"), fold("か"));
        assert_eq!(fold("ｶﾞﾙ"), fold("ガル")); // half-width kana
        assert_eq!(fold("ヒカル"), "ひかる");
        assert_eq!(fold("米津玄師"), "米津玄師");
        assert_eq!(fold("아이유"), "아이유");
        assert_ne!(fold("शंकर"), fold("शकर"));
        assert!(fold("อินไตย์").contains('\u{E34}'));
    }
}
