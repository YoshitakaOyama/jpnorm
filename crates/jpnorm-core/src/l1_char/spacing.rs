//! 日本語 (CJK) と英数字の間の空白の扱い。
//!
//! neologdn は「日本語 の 文章」の空白を削除するが、jpnorm の `collapse_spaces` は
//! 畳むだけで残す。どちらが正しいかは用途次第なので、ここで選べるようにする。
//!
//! - [`CjkSpacing::Remove`][]: 片側でも CJK なら空白を削除する (比較・名寄せ向け)
//! - [`CjkSpacing::Insert`][]: CJK と英数字の境界に半角スペースを 1 つ入れる (表示向け、pangu 方式)

/// CJK と英数字の間の空白の扱い。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CjkSpacing {
    /// 何もしない。
    #[default]
    Keep,
    /// CJK に隣接する空白を削除する。
    Remove,
    /// CJK と英数字の境界に半角スペースを挿入する。
    Insert,
}

/// `mode` に従って CJK まわりの空白を整える。
pub fn process(input: &str, mode: CjkSpacing) -> String {
    match mode {
        CjkSpacing::Keep => input.to_owned(),
        CjkSpacing::Remove => remove(input),
        CjkSpacing::Insert => insert(input),
    }
}

/// CJK 文字 (漢字・かな・CJK 記号・全角形) か。
pub fn is_cjk(c: char) -> bool {
    matches!(
        c as u32,
        0x3000..=0x303F   // CJK 記号・句読点 (〜 ・ 「」 など)
        | 0x3040..=0x30FF // ひらがな・カタカナ (ー を含む)
        | 0x31F0..=0x31FF // 小書きカタカナ拡張
        | 0x3400..=0x4DBF // CJK 統合漢字拡張 A
        | 0x4E00..=0x9FFF // CJK 統合漢字
        | 0xF900..=0xFAFF // CJK 互換漢字
        | 0xFF01..=0xFF60 // 全角形 (英数・記号)
        | 0xFF66..=0xFF9F // 半角カナ
        | 0x20000..=0x2FFFF
    )
}

fn remove(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    for (i, &c) in chars.iter().enumerate() {
        if c == ' ' {
            let prev = chars[..i].iter().rev().find(|c| **c != ' ').copied();
            let next = chars[i + 1..].iter().find(|c| **c != ' ').copied();
            let touches_cjk = prev.is_some_and(is_cjk) || next.is_some_and(is_cjk);
            if touches_cjk && prev.is_some() && next.is_some() {
                continue;
            }
        }
        out.push(c);
    }
    out
}

fn insert(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 8);
    let mut prev: Option<char> = None;
    for c in input.chars() {
        if let Some(p) = prev {
            let boundary = (is_cjk(p) && c.is_ascii_alphanumeric())
                || (p.is_ascii_alphanumeric() && is_cjk(c));
            // 記号や句読点の隣には入れない (「日本語(text)」の括弧など)
            if boundary && !is_cjk_punct(p) && !is_cjk_punct(c) {
                out.push(' ');
            }
        }
        out.push(c);
        prev = Some(c);
    }
    out
}

fn is_cjk_punct(c: char) -> bool {
    matches!(c as u32, 0x3000..=0x303F | 0xFF01..=0xFF0F | 0xFF1A..=0xFF20 | 0xFF3B..=0xFF40 | 0xFF5B..=0xFF65)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_between_cjk() {
        assert_eq!(remove("日本語 の 文章"), "日本語の文章");
        assert_eq!(remove("日本語 text 混在"), "日本語text混在");
        assert_eq!(remove("Python と Rust"), "PythonとRust");
        assert_eq!(remove("hello world"), "hello world");
        assert_eq!(remove(" 前後 "), " 前後 ");
        assert_eq!(remove("日本語  と  英語"), "日本語と英語");
    }

    #[test]
    fn insert_at_boundaries() {
        assert_eq!(insert("日本語text混在"), "日本語 text 混在");
        assert_eq!(insert("PythonとRust3"), "Python と Rust3");
        assert_eq!(insert("日本語 text"), "日本語 text");
        assert_eq!(insert("日本語(text)"), "日本語(text)");
        assert_eq!(insert("価格は1200円"), "価格は 1200 円");
    }

    #[test]
    fn keep_is_identity() {
        assert_eq!(process("日本語 text", CjkSpacing::Keep), "日本語 text");
    }
}
