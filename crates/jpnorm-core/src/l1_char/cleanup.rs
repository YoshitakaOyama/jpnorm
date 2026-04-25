//! 不可視文字・制御文字・改行コードのクリーンアップ。
//!
//! 実運用で最もハマりやすい「見えない混入」を取り除くための処理群。
//! 各関数はフラグで個別に有効化される想定で、単体でも組み合わせても使える。

/// 改行コードを LF (`\n`) に統一する。
///
/// - CRLF (`\r\n`) → LF
/// - 残った CR (`\r`) → LF
/// - U+0085 (NEL), U+2028 (LS), U+2029 (PS) も LF に寄せる
pub fn normalize_newlines(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push('\n');
            }
            '\u{0085}' | '\u{2028}' | '\u{2029}' => out.push('\n'),
            _ => out.push(c),
        }
    }
    out
}

/// ゼロ幅・BOM 系の不可視文字を削除する。
///
/// 対象:
/// - U+200B ZERO WIDTH SPACE
/// - U+200C ZERO WIDTH NON-JOINER
/// - U+2060 WORD JOINER
/// - U+FEFF ZERO WIDTH NO-BREAK SPACE / BOM
///
/// 注: U+200D ZERO WIDTH JOINER は絵文字 ZWJ シーケンスで使われるため
/// ここでは削除しない。必要なら絵文字処理側で扱う。
pub fn remove_zero_width(input: &str) -> String {
    input
        .chars()
        .filter(|c| !matches!(*c, '\u{200B}' | '\u{200C}' | '\u{2060}' | '\u{FEFF}'))
        .collect()
}

/// 制御文字 (C0/C1) を削除する。
///
/// `\t` (U+0009) と `\n` (U+000A) は残す。改行コード正規化を先に掛けた前提で
/// CR は残さない方が安全だが、`normalize_newlines` を通していない場合に備えて
/// CR も削除する。
///
/// - U+0000–U+0008, U+000B–U+001F (C0 の一部)
/// - U+007F DEL
/// - U+0080–U+009F (C1)
pub fn remove_control(input: &str) -> String {
    input
        .chars()
        .filter(|c| {
            let cp = *c as u32;
            if *c == '\t' || *c == '\n' {
                return true;
            }
            !(cp <= 0x1F || cp == 0x7F || (0x80..=0x9F).contains(&cp))
        })
        .collect()
}

/// Bidi 制御文字を削除する (Trojan Source 攻撃対策)。
///
/// - U+200E LRM, U+200F RLM
/// - U+202A–U+202E (LRE/RLE/PDF/LRO/RLO)
/// - U+2066–U+2069 (LRI/RLI/FSI/PDI)
pub fn remove_bidi_control(input: &str) -> String {
    input
        .chars()
        .filter(|c| {
            let cp = *c as u32;
            !matches!(cp,
                0x200E | 0x200F |
                0x202A..=0x202E |
                0x2066..=0x2069
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newlines_crlf() {
        assert_eq!(normalize_newlines("a\r\nb\rc\nd"), "a\nb\nc\nd");
    }

    #[test]
    fn newlines_unicode_breaks() {
        assert_eq!(normalize_newlines("a\u{2028}b\u{2029}c\u{0085}d"), "a\nb\nc\nd");
    }

    #[test]
    fn zero_width_strip() {
        let s = "a\u{200B}b\u{FEFF}c\u{2060}d\u{200C}e";
        assert_eq!(remove_zero_width(s), "abcde");
    }

    #[test]
    fn zwj_preserved_for_emoji() {
        // 絵文字 ZWJ シーケンスが壊れないこと
        let s = "x\u{200D}y";
        assert_eq!(remove_zero_width(s), "x\u{200D}y");
    }

    #[test]
    fn control_strip() {
        let s = "a\x00b\x07c\x1Fd\x7Fe\u{0085}f\tg\nh";
        assert_eq!(remove_control(s), "abcdef\tg\nh");
    }

    #[test]
    fn bidi_strip() {
        // Trojan Source 風: RLO で逆向き表示を強制するサンプル
        let s = "admin\u{202E}reverse\u{202C}";
        assert_eq!(remove_bidi_control(s), "adminreverse");
    }
}
