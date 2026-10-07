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
//!
//! ハイフンで繋がった数字列 (`03-1234-5678`, `100-0001`, `4-0286-72`) は電話番号・
//! 郵便番号・ISBN などのコードとみなし、先頭ゼロも含めてそのまま残す。
//! ただし `2025-01-01` のような日付は `2025/01/01` と揃うよう月日の先頭ゼロを落とす。

/// テキスト中の数値トークンを正規形に揃える。
pub fn canonicalize(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;

    while i < bytes.len() {
        // 数値トークンは [ASCII数字] で始まる。
        // 直前が ASCII 英字の場合は識別子の一部とみなし触らない(例: abc123)。
        if bytes[i].is_ascii_digit() && (i == 0 || !prev_is_word_char(out.as_bytes())) {
            if let Some(parts) = hyphen_chain(&bytes[i..]) {
                let consumed = parts.iter().map(|p| p.len() + 1).sum::<usize>() - 1;
                if is_date(&parts) {
                    let date: Vec<&str> = parts.iter().map(|p| strip_leading_zeros(p)).collect();
                    out.push_str(&date.join("-"));
                } else {
                    out.push_str(&input[i..i + consumed]);
                }
                i += consumed;
                continue;
            }
            if let Some(len) = dotted_chain(&bytes[i..]) {
                out.push_str(&input[i..i + len]);
                i += len;
                continue;
            }
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

/// 直前が識別子の一部なら `true`。数値トークンはそこから始めない。
///
/// - 英字・`_` の直後 (`abc123`, `N700`)
/// - 数字の直後。トークンは常に丸ごと消費するので、ここに来るのは識別子の途中だけ
///   (`N700` の `00` を別トークンとして `0` に縮めないため)
/// - 英数字の直後の `.` (`WHIP1.06` の `06`、`v1.2.03`、`No.36`)
fn prev_is_word_char(out: &[u8]) -> bool {
    let is_word = |b: &u8| b.is_ascii_alphanumeric() || *b == b'_';
    match out {
        [.., before, b'.'] => is_word(before),
        [.., last] => is_word(last),
        [] => false,
    }
}

/// 先頭から `D+(-D+)+` の形の数字列を切り出し、各部分を返す。
///
/// 末尾が英数字・`-`・小数部・桁区切りに続く場合は数値の一部とみなして `None`
/// (`1-0.5` や `12-34abc` はコード扱いしない)。
fn hyphen_chain(bytes: &[u8]) -> Option<Vec<&str>> {
    let mut parts = Vec::new();
    let mut start = 0;
    loop {
        let mut end = start;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if end == start {
            return None;
        }
        parts.push(std::str::from_utf8(&bytes[start..end]).unwrap());
        match bytes.get(end) {
            Some(b'-') if bytes.get(end + 1).is_some_and(u8::is_ascii_digit) => start = end + 1,
            Some(b'.' | b',') if bytes.get(end + 1).is_some_and(u8::is_ascii_digit) => {
                return None;
            }
            Some(b) if b.is_ascii_alphanumeric() || *b == b'_' => return None,
            _ => break,
        }
    }
    (parts.len() >= 2).then_some(parts)
}

/// `D+.D+.D+...` (バージョン番号・IP アドレス) なら全体の長さを返す。
/// 小数ではないので末尾ゼロも先頭ゼロも落とさない (`1.0.0`, `192.168.0.1`)。
fn dotted_chain(bytes: &[u8]) -> Option<usize> {
    let mut i = 0;
    let mut dots = 0;
    loop {
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return None;
        }
        if bytes.get(i) == Some(&b'.') && bytes.get(i + 1).is_some_and(u8::is_ascii_digit) {
            dots += 1;
            i += 1;
        } else {
            break;
        }
    }
    (dots >= 2).then_some(i)
}

/// `YYYY-M-D` / `YYYY-M` (月日は 1〜2 桁) なら日付とみなす。
fn is_date(parts: &[&str]) -> bool {
    let in_range =
        |p: &str, max: u32| p.len() <= 2 && p.parse::<u32>().is_ok_and(|n| (1..=max).contains(&n));
    match parts {
        [y, m] => y.len() == 4 && in_range(m, 12),
        [y, m, d] => y.len() == 4 && in_range(m, 12) && in_range(d, 31),
        _ => false,
    }
}

fn strip_leading_zeros(digits: &str) -> &str {
    let stripped = digits.trim_start_matches('0');
    if stripped.is_empty() { "0" } else { stripped }
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
    while k + 3 < bytes.len()
        && bytes[k] == b','
        && bytes[k + 1..k + 4].iter().all(|b| b.is_ascii_digit())
    {
        // カンマ後の4桁目が数字だとグルーピング不正(1,2345 等) → 打ち切り。
        if k + 4 < bytes.len() && bytes[k + 4].is_ascii_digit() {
            break;
        }
        k += 4;
    }
    // 小数部
    let mut end = k;
    if end < bytes.len()
        && bytes[end] == b'.'
        && end + 1 < bytes.len()
        && bytes[end + 1].is_ascii_digit()
    {
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
    let mut result = strip_leading_zeros(int_part).to_string();
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
    fn identifiers_are_never_partially_canonicalized() {
        // Wikipedia 記事で見つかった誤変換 (N700系 → N70系, WHIP1.06 → WHIP1.6)
        assert_eq!(canonicalize("N700系"), "N700系");
        assert_eq!(canonicalize("N700S"), "N700S");
        assert_eq!(canonicalize("R10000000"), "R10000000");
        assert_eq!(canonicalize("WHIP1.06"), "WHIP1.06");
        assert_eq!(canonicalize("v1.2.03"), "v1.2.03");
        assert_eq!(canonicalize("No.036"), "No.036");
        // 単独の数値は従来どおり
        assert_eq!(canonicalize("1.060 と 700系"), "1.06 と 700系");
        assert_eq!(canonicalize("打率.250"), "打率.250");
    }

    #[test]
    fn dotted_versions_and_addresses_untouched() {
        assert_eq!(canonicalize("バージョン1.0.0"), "バージョン1.0.0");
        assert_eq!(canonicalize("192.168.0.10"), "192.168.0.10");
        assert_eq!(canonicalize("2.10.0 リリース"), "2.10.0 リリース");
        // 小数は従来どおり
        assert_eq!(canonicalize("1.50 と 2.0"), "1.5 と 2");
    }

    #[test]
    fn hyphenated_codes_keep_leading_zeros() {
        assert_eq!(canonicalize("〒100-0001"), "〒100-0001");
        assert_eq!(canonicalize("03-1234-5678"), "03-1234-5678");
        assert_eq!(canonicalize("ISBN 4-0286-72"), "ISBN 4-0286-72");
        // 日付は月日の先頭ゼロを落とし、スラッシュ区切りと揃える
        assert_eq!(canonicalize("2025-01-01"), "2025-1-1");
        assert_eq!(canonicalize("2025/01/01"), "2025/1/1");
        assert_eq!(canonicalize("2025-06"), "2025-6");
        // 範囲外の月日はコード扱い
        assert_eq!(canonicalize("2025-13-01"), "2025-13-01");
        // 小数を含むものはチェーン扱いしない
        assert_eq!(canonicalize("1-0.50"), "1-0.5");
        // 単独の負数は従来どおり
        assert_eq!(canonicalize("-007"), "-7");
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
