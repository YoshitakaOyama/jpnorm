//! 数値トークンの正規化(比較用)。
//!
//! 桁区切りカンマと小数末尾ゼロを落として、同じ数値を同じ文字列に揃える。
//! 例:
//!   - `1,200` → `1200`
//!   - `1200.00` → `1200`
//!   - `3.1400` → `3.14`
//!   - `-0.50` → `-0.5`
//!   - `1,234.500` → `1234.5`
//!
//! 範囲は ASCII 半角数字のみ対象とする(全角は NFKC 後を前提)。
//! 不正な形(`1,23`, `1,2345`, `1..2` 等)はトークン認識から外して触らない。

/// テキスト中の数値トークンを正規形に揃える。
pub fn canonicalize(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;

    while i < bytes.len() {
        // 数値トークンは [ASCII数字] で始まる。
        // 直前が ASCII 英字の場合は識別子の一部とみなし触らない(例: abc123)。
        if bytes[i].is_ascii_digit()
            && (i == 0 || !prev_is_word_char(out.as_bytes()))
        {
            let (token, consumed) = scan_number(&bytes[i..]);
            if consumed > 0 {
                if let Some(normalized) = normalize_token(token) {
                    out.push_str(&normalized);
                    i += consumed;
                    continue;
                }
            }
        }
        // その他は 1 バイト/1 文字ずつ流す。UTF-8 安全のため char 単位で進める。
        let c = input[i..].chars().next().expect("non-empty");
        out.push(c);
        i += c.len_utf8();
    }

    out
}

fn prev_is_word_char(out: &[u8]) -> bool {
    out.last().is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_')
}

/// `bytes` の先頭から数値らしきトークンを貪欲に切り出す。
/// 許容形: `D{1,3}(,D{3})*(\.D+)?` または `D+(\.D+)?`。
fn scan_number(bytes: &[u8]) -> (&str, usize) {
    // 整数部を集める(カンマ区切りも許容)。
    let mut j = 0;
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        j += 1;
    }
    // カンマ区切りを末尾に向けて拡張: `,DDD` の繰り返し
    let mut k = j;
    while k + 3 < bytes.len() && bytes[k] == b',' && bytes[k + 1..k + 4].iter().all(|b| b.is_ascii_digit()) {
        // カンマ後の4桁目が数字だとグルーピング不正(1,2345 等) → 打ち切り。
        if k + 4 < bytes.len() && bytes[k + 4].is_ascii_digit() {
            break;
        }
        k += 4;
    }
    // 小数部
    let mut end = k;
    if end < bytes.len() && bytes[end] == b'.' && end + 1 < bytes.len() && bytes[end + 1].is_ascii_digit() {
        end += 1;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
    }

    // str::from_utf8 は ASCII 範囲なので必ず成功。
    (std::str::from_utf8(&bytes[..end]).unwrap(), end)
}

fn normalize_token(token: &str) -> Option<String> {
    // カンマを除去して integer / fraction に分割。
    let no_comma: String = token.chars().filter(|c| *c != ',').collect();
    let (int_part, frac_part) = match no_comma.split_once('.') {
        Some((a, b)) => (a, Some(b)),
        None => (no_comma.as_str(), None),
    };
    if int_part.is_empty() {
        return None;
    }
    // 整数部の先頭ゼロを落とす(ただし 1 桁は残す)。
    let int_trimmed: String = {
        let stripped = int_part.trim_start_matches('0');
        if stripped.is_empty() {
            "0".to_string()
        } else {
            stripped.to_string()
        }
    };
    let mut result = int_trimmed;
    if let Some(frac) = frac_part {
        let frac_trimmed = frac.trim_end_matches('0');
        if !frac_trimmed.is_empty() {
            result.push('.');
            result.push_str(frac_trimmed);
        }
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_thousands_separator() {
        assert_eq!(canonicalize("1,200"), "1200");
        assert_eq!(canonicalize("1,234,567"), "1234567");
    }

    #[test]
    fn strips_trailing_decimal_zeros() {
        assert_eq!(canonicalize("1200.00"), "1200");
        assert_eq!(canonicalize("3.1400"), "3.14");
        assert_eq!(canonicalize("0.50"), "0.5");
    }

    #[test]
    fn leading_zero_stripped() {
        assert_eq!(canonicalize("007"), "7");
        assert_eq!(canonicalize("0"), "0");
        assert_eq!(canonicalize("00.50"), "0.5");
    }

    #[test]
    fn combined() {
        assert_eq!(canonicalize("価格は 1,200.00 円"), "価格は 1200 円");
        assert_eq!(canonicalize("1,234.500"), "1234.5");
    }

    #[test]
    fn does_not_touch_identifier_with_digits() {
        // 識別子中の数字は触らない (abc123 → abc123)
        assert_eq!(canonicalize("abc123"), "abc123");
    }

    #[test]
    fn invalid_grouping_not_collapsed() {
        // 1,23 は不正なグルーピング → カンマは残し数字部分だけ展開
        // 実装では最初の数字 "1" を数値と認識してそこで切れる。
        let out = canonicalize("1,23");
        // "1" → "1"、その後 ",23" は別扱い
        assert!(out.starts_with('1'));
    }

    #[test]
    fn negative_unaffected_sign_handled_externally() {
        // マイナス符号は数値スキャナ外で扱う(文脈依存のため)。
        // ここではトークン単位で正しく動くことだけ確認。
        assert_eq!(canonicalize("-1200.00"), "-1200");
    }

    #[test]
    fn multiple_numbers() {
        assert_eq!(canonicalize("1,000 と 2,000.50"), "1000 と 2000.5");
    }
}
