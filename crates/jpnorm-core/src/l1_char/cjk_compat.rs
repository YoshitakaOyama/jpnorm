//! 機種依存文字や CJK 互換文字の展開。
//!
//! 例: `㈱` → `(株)`, `①` → `1`, `髙` → `高`, `㌔` → `キロ`
//!
//! NFKC である程度吸収されるが、NFKC を適用しない設定でも最低限の展開を
//! したいケースがあるため独立した関数として提供する。

/// 代表的な機種依存文字を展開する。
pub fn expand(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            // 括弧付き文字
            '㈱' => out.push_str("(株)"),
            '㈲' => out.push_str("(有)"),
            '㈳' => out.push_str("(社)"),
            '㈶' => out.push_str("(財)"),
            '㈵' => out.push_str("(特)"),
            '㈹' => out.push_str("(代)"),
            '㈻' => out.push_str("(学)"),
            // 丸付き数字 1..20
            '①' => out.push('1'),
            '②' => out.push('2'),
            '③' => out.push('3'),
            '④' => out.push('4'),
            '⑤' => out.push('5'),
            '⑥' => out.push('6'),
            '⑦' => out.push('7'),
            '⑧' => out.push('8'),
            '⑨' => out.push('9'),
            '⑩' => out.push_str("10"),
            '⑪' => out.push_str("11"),
            '⑫' => out.push_str("12"),
            '⑬' => out.push_str("13"),
            '⑭' => out.push_str("14"),
            '⑮' => out.push_str("15"),
            '⑯' => out.push_str("16"),
            '⑰' => out.push_str("17"),
            '⑱' => out.push_str("18"),
            '⑲' => out.push_str("19"),
            '⑳' => out.push_str("20"),
            // 異体字(旧字→新字の代表例)
            '髙' => out.push('高'),
            '﨑' => out.push('崎'),
            '德' => out.push('徳'),
            '濵' => out.push('浜'),
            '龍' => out.push('竜'),
            '邊' => out.push('辺'),
            '邉' => out.push('辺'),
            '澤' => out.push('沢'),
            '齋' => out.push('斎'),
            '齊' => out.push('斉'),
            // 単位系(代表例)
            '㌔' => out.push_str("キロ"),
            '㍍' => out.push_str("メートル"),
            '㌘' => out.push_str("グラム"),
            '㌢' => out.push_str("センチ"),
            '㍉' => out.push_str("ミリ"),
            '㍑' => out.push_str("リットル"),
            '㌫' => out.push_str("パーセント"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn company_marks() {
        assert_eq!(expand("㈱kanon"), "(株)kanon");
    }

    #[test]
    fn circled_digits() {
        assert_eq!(expand("①②⑩"), "1210");
    }

    #[test]
    fn variant_forms() {
        assert_eq!(expand("髙橋"), "高橋");
    }
}
