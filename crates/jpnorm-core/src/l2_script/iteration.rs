//! 繰り返し記号 (々 ゝ ゞ ヽ ヾ 〻) の展開。
//!
//! 検索索引では「人々」と「人人」、「いすゞ」と「いすず」を同一視したい。
//! Lucene の `JapaneseIterationMarkCharFilter` と同じ方針で、直前の文字を
//! 繰り返す形に展開する。
//!
//! - `々` `〻`: 直前が漢字ならその漢字を繰り返す
//! - `ゝ` `ヽ`: 直前がかななら、その清音を繰り返す
//! - `ゞ` `ヾ`: 直前がかななら、その濁音を繰り返す
//!
//! 直前の文字が対象外 (先頭、記号、英数など) の場合は記号をそのまま残す。

/// 繰り返し記号を展開する。
pub fn expand_iteration_marks(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev: Option<char> = None;
    for c in input.chars() {
        let expanded = match (c, prev) {
            ('々' | '〻', Some(p)) if is_kanji(p) => Some(p),
            ('ゝ', Some(p)) if is_hiragana(p) => Some(unvoice(p)),
            ('ゞ', Some(p)) if is_hiragana(p) => voice(unvoice(p)),
            ('ヽ', Some(p)) if is_katakana(p) => Some(unvoice(p)),
            ('ヾ', Some(p)) if is_katakana(p) => voice(unvoice(p)),
            _ => None,
        };
        let emitted = expanded.unwrap_or(c);
        out.push(emitted);
        prev = Some(emitted);
    }
    out
}

fn is_kanji(c: char) -> bool {
    matches!(
        c as u32,
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x2FFFF
    )
}

fn is_hiragana(c: char) -> bool {
    matches!(c as u32, 0x3041..=0x3096)
}

fn is_katakana(c: char) -> bool {
    matches!(c as u32, 0x30A1..=0x30FA)
}

/// 濁音・半濁音を清音に戻す (ひらがな・カタカナ共通のオフセットで扱う)。
fn unvoice(c: char) -> char {
    let cp = c as u32;
    // ひらがな (0x3041..) とカタカナ (0x30A1..) は 0x60 ずれで対応するので
    // ひらがな基準に寄せてから判定する。
    let (base, shift) = if is_katakana(c) {
        (cp - 0x60, 0x60)
    } else {
        (cp, 0)
    };
    let unvoiced = match char::from_u32(base) {
        Some(
            'が' | 'ぎ' | 'ぐ' | 'げ' | 'ご' | 'ざ' | 'じ' | 'ず' | 'ぜ' | 'ぞ' | 'だ' | 'ぢ'
            | 'づ' | 'で' | 'ど' | 'ば' | 'び' | 'ぶ' | 'べ' | 'ぼ',
        ) => base - 1,
        Some('ぱ' | 'ぴ' | 'ぷ' | 'ぺ' | 'ぽ') => base - 2,
        Some('ゔ') => 'う' as u32,
        _ => base,
    };
    char::from_u32(unvoiced + shift).unwrap_or(c)
}

/// 清音を濁音にする。濁音を持たない文字は `None`。
fn voice(c: char) -> Option<char> {
    let cp = c as u32;
    let (base, shift) = if is_katakana(c) {
        (cp - 0x60, 0x60)
    } else {
        (cp, 0)
    };
    let voiced = match char::from_u32(base)? {
        'か' | 'き' | 'く' | 'け' | 'こ' | 'さ' | 'し' | 'す' | 'せ' | 'そ' | 'た' | 'ち'
        | 'つ' | 'て' | 'と' | 'は' | 'ひ' | 'ふ' | 'へ' | 'ほ' => base + 1,
        'う' => 'ゔ' as u32,
        _ => return None,
    };
    char::from_u32(voiced + shift)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kanji_marks() {
        assert_eq!(expand_iteration_marks("人々と時々"), "人人と時時");
        assert_eq!(expand_iteration_marks("民々々"), "民民民");
        assert_eq!(expand_iteration_marks("佐々木"), "佐佐木");
    }

    #[test]
    fn kana_marks() {
        assert_eq!(expand_iteration_marks("こゝろ"), "こころ");
        assert_eq!(expand_iteration_marks("いすゞ"), "いすず");
        assert_eq!(expand_iteration_marks("つゞく"), "つづく");
        assert_eq!(expand_iteration_marks("ぶゝ"), "ぶふ");
        assert_eq!(expand_iteration_marks("バナヽ"), "バナナ");
        assert_eq!(expand_iteration_marks("ハヾ"), "ハバ");
    }

    #[test]
    fn marks_without_context_are_kept() {
        assert_eq!(expand_iteration_marks("々"), "々");
        assert_eq!(expand_iteration_marks("A々"), "A々");
        assert_eq!(expand_iteration_marks("あ々"), "あ々");
        assert_eq!(expand_iteration_marks("んゞ"), "んゞ");
    }
}
