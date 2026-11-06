//! Search folding, the same as `foldText` in the Electron app: decompose,
//! drop combining marks, lower-case. So `кино` finds `Кино`, `ё` matches `е`
//! and `é` matches `e`, in any script.

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

pub fn fold(s: &str) -> String {
    let stripped: String = s.nfkd().filter(|c| !is_combining_mark(*c)).collect();
    stripped.to_lowercase()
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
    }
}
