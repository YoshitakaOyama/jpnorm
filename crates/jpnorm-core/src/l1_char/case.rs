//! 大文字・小文字の統一。
//!
//! 検索索引や比較では英字の大文字小文字を揃えるのが定番だが、表示用途では
//! 触りたくないので、用途に応じて選べるようにする。

/// 英字の大文字・小文字の扱い。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaseAction {
    /// 変換しない。
    #[default]
    Keep,
    /// 小文字に揃える (Unicode の lowercase mapping)。
    Lower,
    /// 大文字に揃える。
    Upper,
}

/// `action` に従って大文字小文字を変換する。
pub fn process(input: &str, action: CaseAction) -> String {
    match action {
        CaseAction::Keep => input.to_owned(),
        CaseAction::Lower => input.to_lowercase(),
        CaseAction::Upper => input.to_uppercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_and_upper() {
        assert_eq!(
            process("Python と Rust", CaseAction::Lower),
            "python と rust"
        );
        assert_eq!(
            process("Python と Rust", CaseAction::Upper),
            "PYTHON と RUST"
        );
        assert_eq!(process("Straße", CaseAction::Lower), "straße");
        assert_eq!(process("日本語", CaseAction::Lower), "日本語");
    }
}
