//! 特定カテゴリの文字を削除する処理。
//!
//! フラグで明示的に有効化された場合のみ、`記号` や `機種依存文字` を
//! 取り除く。ユースケースは検索インデックスや比較キー生成で、文字種を
//! 積極的に落としたいとき。

/// 記号(句読点・各種シンボル類)を削除する。
///
/// ここで言う「記号」は、英数字・空白・日本語文字(ひらがな/カタカナ/漢字)
/// 以外で、以下の Unicode ブロックに含まれる文字を指す。絵文字や機種依存文字
/// は別フラグで扱うため、ここでは対象外とする。
///
/// - ASCII 句読点 (`!"#$%&'()*+,-./:;<=>?@[\]^_`{|}~` と backtick)
/// - General Punctuation (U+2000–U+206F)
/// - CJK Symbols and Punctuation (U+3001–U+303F。U+3000 は空白系なので除外)
/// - Halfwidth and Fullwidth Forms の記号部分 (U+FF01–U+FF0F, U+FF1A–U+FF20,
///   U+FF3B–U+FF40, U+FF5B–U+FF65)
///
/// ASCII 数字に挟まれた記号だけは数値の区切りとして扱う。単純に消すと別の数値が
/// 同じ文字列になってしまうため (`3.14` と `314`、`2025/1/1` と `2025/11`)。
///
/// - 小数点 `.` は残す (`3.14`, `192.168.0.1`)
/// - 桁区切りらしい `,` (直後がちょうど 3 桁) は消して数字を繋ぐ (`1,200` → `1200`)
/// - それ以外の記号は空白に置き換える (`2025/1/1` → `2025 1 1`, `03-1234` → `03 1234`)
pub fn remove_symbols(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    for (i, &c) in chars.iter().enumerate() {
        if !is_symbol(c) {
            out.push(c);
            continue;
        }
        let between_digits = i > 0
            && chars[i - 1].is_ascii_digit()
            && chars.get(i + 1).is_some_and(char::is_ascii_digit);
        if !between_digits {
            continue;
        }
        match c {
            '.' => out.push('.'),
            ',' if is_grouping_comma(&chars[i + 1..]) => {}
            _ => out.push(' '),
        }
    }
    out
}

/// `,` の直後がちょうど 3 桁の数字なら桁区切りとみなす。
fn is_grouping_comma(rest: &[char]) -> bool {
    rest.len() >= 3
        && rest[..3].iter().all(char::is_ascii_digit)
        && !rest.get(3).is_some_and(char::is_ascii_digit)
}

fn is_symbol(c: char) -> bool {
    let cp = c as u32;
    // ASCII punctuation
    if c.is_ascii() && !c.is_ascii_alphanumeric() && !c.is_ascii_whitespace() {
        return true;
    }
    // 〇 (U+3007 IDEOGRAPHIC NUMBER ZERO) は漢数字ゼロなので記号扱いしない。
    if cp == 0x3007 {
        return false;
    }
    matches!(cp,
        // General Punctuation
        0x2000..=0x206F |
        // CJK Symbols and Punctuation (U+3000 ideographic space は除外)
        0x3001..=0x303F |
        // Fullwidth punctuation 群
        0xFF01..=0xFF0F |
        0xFF1A..=0xFF20 |
        0xFF3B..=0xFF40 |
        0xFF5B..=0xFF65
    )
}

/// 機種依存文字を削除する。
///
/// `cjk_compat::expand` が展開対象としている代表的な文字群に加え、
/// より広い CJK 互換/囲み文字ブロックをまとめて削除する。
///
/// - Enclosed Alphanumerics (U+2460–U+24FF)
/// - Enclosed CJK Letters and Months (U+3200–U+32FF)
/// - CJK Compatibility (U+3300–U+33FF, ㌔㍉ など)
/// - CJK Compatibility Ideographs (U+F900–U+FAFF, 髙﨑 など)
pub fn remove_cjk_compat(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        if !is_cjk_compat(c) {
            out.push(c);
        }
    }
    out
}

fn is_cjk_compat(c: char) -> bool {
    let cp = c as u32;
    matches!(cp,
        0x2460..=0x24FF |
        0x3200..=0x32FF |
        0x3300..=0x33FF |
        0xF900..=0xFAFF
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbols_removed() {
        assert_eq!(remove_symbols("hello, world!"), "hello world");
        assert_eq!(remove_symbols("これは「テスト」です。"), "これはテストです");
    }

    #[test]
    fn symbols_between_digits_keep_numbers_apart() {
        // 消すと別の数値と衝突するもの
        assert_eq!(remove_symbols("3.14"), "3.14");
        assert_eq!(remove_symbols("v1.2.3"), "v1.2.3");
        assert_eq!(remove_symbols("2025/1/1"), "2025 1 1");
        assert_eq!(remove_symbols("2025/11"), "2025 11");
        assert_eq!(remove_symbols("03-1234-5678"), "03 1234 5678");
        assert_eq!(remove_symbols("第1,2巻"), "第1 2巻");
        assert_eq!(remove_symbols("1,200円"), "1200円");
        // 数字に挟まれていない記号は従来どおり消す
        assert_eq!(remove_symbols("終わり。3."), "終わり3");
        assert_eq!(remove_symbols("(1)"), "1");
        assert_eq!(remove_symbols("-5"), "5");
    }

    #[test]
    fn symbols_keep_japanese_letters() {
        let s = "ひらがなカタカナ漢字ABC123";
        assert_eq!(remove_symbols(s), s);
    }

    #[test]
    fn cjk_compat_removed() {
        assert_eq!(remove_cjk_compat("①②③kanon"), "kanon");
        assert_eq!(remove_cjk_compat("㈱テスト"), "テスト");
        assert_eq!(remove_cjk_compat("㌔メートル"), "メートル");
    }
}
