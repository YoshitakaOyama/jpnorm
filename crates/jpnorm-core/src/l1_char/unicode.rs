//! Unicode 正規化 (NFKC など)。

use unicode_normalization::UnicodeNormalization;

/// NFKC を適用する。
///
/// 素の NFKC は、単独で使われる記号の一部を「空白 + 結合文字」に分解する
/// (`´` → ` ́`、`゜` → ` ゚`)。顔文字 `(*´ω`*)` や OCR 由来の `ハ゜ソコン` に
/// 余計な空白が入り、結合文字が前の文字に重なって表示が崩れるため、次のように扱う。
///
/// - かなの直後の濁点・半濁点 (`゛゜` や結合用・半角の `ﾞﾟ`) は前のかなと合成する
///   (`ハ゜` → `パ`)。
/// - それ以外の位置の濁点・半濁点は全角の `゛` `゜` にそろえる (`(ﾟ∀ﾟ)` → `(゜∀゜)`)。
/// - 空白 + 結合文字に分解される記号 (`´` `¨` `¯` `￣` など) は分解せずに残す。
pub fn nfkc(input: &str) -> String {
    let mut prepared = String::with_capacity(input.len());
    let mut kept: Vec<(usize, char)> = Vec::new();
    let mut prev: Option<char> = None;
    for c in input.chars() {
        let mark = match c {
            '\u{309B}' | '\u{3099}' | '\u{FF9E}' => Some(('\u{3099}', '\u{309B}')),
            '\u{309C}' | '\u{309A}' | '\u{FF9F}' => Some(('\u{309A}', '\u{309C}')),
            _ => None,
        };
        if let Some((combining, spacing)) = mark {
            if prev.is_some_and(is_kana) {
                // 半角カナ + 半角濁点は NFKC がそのまま合成するので手を付けない
                let halfwidth = prev.is_some_and(|p| ('\u{FF66}'..='\u{FF9D}').contains(&p));
                prepared.push(if halfwidth && c >= '\u{FF9E}' {
                    c
                } else {
                    combining
                });
            } else {
                kept.push((prepared.len(), spacing));
            }
        } else if decomposes_to_space(c) {
            kept.push((prepared.len(), c));
        } else {
            prepared.push(c);
        }
        prev = Some(c);
    }
    if kept.is_empty() {
        return prepared.nfkc().collect();
    }
    // 残す記号の位置で区切り、その間だけ NFKC を掛ける。
    let mut out = String::with_capacity(prepared.len());
    let mut start = 0;
    for (at, c) in kept {
        out.extend(prepared[start..at].nfkc());
        out.push(c);
        start = at;
    }
    out.extend(prepared[start..].nfkc());
    out
}

fn is_kana(c: char) -> bool {
    matches!(c as u32, 0x3041..=0x3096 | 0x309D..=0x309E | 0x30A1..=0x30FA | 0x30FD..=0x30FE | 0xFF66..=0xFF9D)
}

/// NFKC で「空白 + 結合文字」に分解される記号。
fn decomposes_to_space(c: char) -> bool {
    matches!(
        c as u32,
        0x00A8
            | 0x00AF
            | 0x00B4
            | 0x00B8
            | 0x02D8..=0x02DD
            | 0x037A
            | 0x0384..=0x0385
            | 0x1FBD
            | 0x1FBF..=0x1FC1
            | 0x1FCD..=0x1FCF
            | 0x1FDD..=0x1FDF
            | 0x1FED..=0x1FEE
            | 0x1FFD..=0x1FFE
            | 0x2017
            | 0x203E
            | 0xFC5E..=0xFC63
            | 0xFE49..=0xFE4C
            | 0xFE70
            | 0xFE72
            | 0xFE74
            | 0xFE76..=0xFE7E
            | 0xFFE3
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_nfkc() {
        // 全角英数は半角に、合成済みの濁点は単一コードポイントに。
        assert_eq!(nfkc("ＡＢＣ１２３"), "ABC123");
        assert_eq!(nfkc("ﾊﾟｿｺﾝ"), "パソコン");
    }

    #[test]
    fn spacing_marks_are_not_split_into_space_and_combining() {
        // 顔文字
        assert_eq!(nfkc("(*´ω`*)"), "(*´ω`*)");
        assert_eq!(nfkc("(´；ω；｀)"), "(´;ω;`)");
        assert_eq!(nfkc("￣▽￣"), "￣▽￣");
        // 単独の半角濁点・半濁点は全角の記号に
        assert_eq!(nfkc("(ﾟ∀ﾟ)"), "(゜∀゜)");
        assert_eq!(nfkc("゛"), "゛");
    }

    #[test]
    fn voiced_marks_after_kana_are_composed() {
        // OCR などで濁点が分離した文字
        assert_eq!(nfkc("ハ゜ソコン と ハ゛ス"), "パソコン と バス");
        assert_eq!(nfkc("か゛き"), "がき");
        assert_eq!(nfkc("カ\u{3099}ラス"), "ガラス");
        // 合成済みの形がないかなは結合文字のまま
        assert_eq!(nfkc("ア゛"), "ア\u{3099}");
    }
}
