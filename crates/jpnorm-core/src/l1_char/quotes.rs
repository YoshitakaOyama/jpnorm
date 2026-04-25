//! 引用符の統一。
//!
//! 左右の曲線引用符を直線引用符に揃える。
//! 日本語の鉤括弧(`「」『』`)は触らない。

/// 各種引用符をASCII直線引用符に統一する。
pub fn unify(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            // 左右の二重引用符
            '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' | '\u{FF02}' => '"',
            // 左右の一重引用符
            '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' | '\u{FF07}' => '\'',
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smart_to_straight() {
        assert_eq!(unify("“hello” ‘world’"), "\"hello\" 'world'");
    }

    #[test]
    fn leaves_japanese_brackets() {
        assert_eq!(unify("「日本語」"), "「日本語」");
    }
}
