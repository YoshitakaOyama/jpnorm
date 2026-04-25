//! 長音符 `ー` の連続を 1 に畳み込む。
//!
//! neologdn 互換挙動の一部。通常文字の繰り返し短縮 (`repeat::shorten`) とは
//! 別に扱う。対象は `ー` (U+30FC) と、互換用に `〜` (U+301C) のみ。

/// 連続した長音符を 1 つに畳み込む。
pub fn collapse(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev: Option<char> = None;
    for c in input.chars() {
        let is_target = matches!(c, 'ー' | '〜');
        if is_target && prev == Some(c) {
            continue;
        }
        out.push(c);
        prev = Some(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapse_prolonged() {
        assert_eq!(collapse("ーーーー"), "ー");
        assert_eq!(collapse("ウェーーーーイ"), "ウェーイ");
        assert_eq!(collapse("すごーーーーい"), "すごーい");
    }

    #[test]
    fn preserves_other_repeats() {
        assert_eq!(collapse("wwwww"), "wwwww");
        assert_eq!(collapse("あああ"), "あああ");
    }

    #[test]
    fn collapse_tilde_like_prolonged() {
        assert_eq!(collapse("〜〜〜"), "〜");
    }
}
