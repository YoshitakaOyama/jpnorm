//! 空白の畳み込み。
//!
//! 方針: **ユーザーが意図的に入れた空白は保存する**。
//! - 連続する空白(半角 / タブ / 全角 `　`)を **単一の半角スペース**に統一する
//! - 文字種(日本語 / ASCII)にかかわらず、1 つは必ず残す
//! - 改行 (`\n`, `\r`) は空白扱いしない(段落の区切りとして保存)
//!
//! 先頭・末尾の空白は呼び出し側の `trim` フラグで処理する前提。

/// 空白を畳み込む。
pub fn collapse(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_space = false;
    for c in input.chars() {
        if is_space(c) {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(c);
            prev_space = false;
        }
    }
    out
}

fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\u{3000}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_space_preserved() {
        // 意図的な 1 つの空白は残す。
        assert_eq!(collapse("日本語 の 文章"), "日本語 の 文章");
        assert_eq!(collapse("日本語 text 混在"), "日本語 text 混在");
    }

    #[test]
    fn runs_collapsed_to_one() {
        assert_eq!(collapse("hello   world"), "hello world");
        assert_eq!(collapse("日本語   です"), "日本語 です");
    }

    #[test]
    fn fullwidth_space_becomes_single_halfwidth() {
        assert_eq!(collapse("日本語　と　英語"), "日本語 と 英語");
    }

    #[test]
    fn mixed_space_types() {
        assert_eq!(collapse("a \t　 b"), "a b");
    }

    #[test]
    fn newlines_preserved() {
        assert_eq!(collapse("line1\nline2"), "line1\nline2");
    }
}
