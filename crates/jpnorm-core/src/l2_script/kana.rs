//! ひらがな ↔ カタカナ の双方向変換。
//!
//! Unicode では平仮名(U+3041..U+3096)と片仮名(U+30A1..U+30F6)が
//! `0x60` オフセットで一対一対応する(一部例外あり)。

/// ひらがな/カタカナ間の変換方法。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KanaAction {
    /// 変換しない。
    #[default]
    Keep,
    /// ひらがなをカタカナに揃える。
    HiraToKata,
    /// カタカナをひらがなに揃える。
    KataToHira,
}

/// `action` に従ってかなを変換する。
pub fn process(input: &str, action: KanaAction) -> String {
    match action {
        KanaAction::Keep => input.to_owned(),
        KanaAction::HiraToKata => hira_to_kata(input),
        KanaAction::KataToHira => kata_to_hira(input),
    }
}

/// ひらがなをカタカナに変換する。
pub fn hira_to_kata(input: &str) -> String {
    input
        .chars()
        .map(|c| match c as u32 {
            0x3041..=0x3096 => char::from_u32(c as u32 + 0x60).unwrap_or(c),
            // U+309D ゝ → U+30FD ヽ, U+309E ゞ → U+30FE ヾ
            0x309D | 0x309E => char::from_u32(c as u32 + 0x60).unwrap_or(c),
            _ => c,
        })
        .collect()
}

/// カタカナをひらがなに変換する。
pub fn kata_to_hira(input: &str) -> String {
    input
        .chars()
        .map(|c| match c as u32 {
            0x30A1..=0x30F6 => char::from_u32(c as u32 - 0x60).unwrap_or(c),
            0x30FD | 0x30FE => char::from_u32(c as u32 - 0x60).unwrap_or(c),
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn h2k() {
        assert_eq!(hira_to_kata("きょうとし"), "キョウトシ");
        assert_eq!(hira_to_kata("がっこう"), "ガッコウ");
    }

    #[test]
    fn k2h() {
        assert_eq!(kata_to_hira("キョウトシ"), "きょうとし");
        assert_eq!(kata_to_hira("ガッコウ"), "がっこう");
    }

    #[test]
    fn mixed_passthrough() {
        assert_eq!(hira_to_kata("あa漢"), "アa漢");
    }

    #[test]
    fn roundtrip() {
        let s = "きょうはいいてんきだ";
        assert_eq!(kata_to_hira(&hira_to_kata(s)), s);
    }
}
