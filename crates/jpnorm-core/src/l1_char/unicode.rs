//! Unicode 正規化 (NFKC など)。

use unicode_normalization::UnicodeNormalization;

/// NFKC を適用する。
pub fn nfkc(input: &str) -> String {
    input.nfkc().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_nfkc() {
        // 全角英数は半角に、合成済みの濁点は単一コードポイントに。
        assert_eq!(nfkc("ＡＢＣ１２３"), "ABC123");
    }
}
