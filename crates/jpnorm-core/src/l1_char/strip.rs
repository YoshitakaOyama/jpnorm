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
pub fn remove_symbols(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        if !is_symbol(c) {
            out.push(c);
        }
    }
    out
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
