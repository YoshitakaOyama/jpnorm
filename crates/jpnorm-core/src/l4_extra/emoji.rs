//! 絵文字の検出と除去・置換。
//!
//! Unicode の主要な絵文字ブロックと ZWJ シーケンス、バリエーションセレクタ、
//! フラグ(Regional Indicator)を簡易的に扱う。

/// 絵文字の処理方法。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmojiAction {
    /// そのまま残す。
    Keep,
    /// 完全に削除する。
    Remove,
    /// 指定文字列で置換する(プレースホルダ)。
    Replace(&'static str),
}

/// `action` に従ってテキスト中の絵文字を処理する。
///
/// 連続する絵文字(ZWJ シーケンス等を含む)は1つの塊として扱う。
pub fn process(input: &str, action: EmojiAction) -> String {
    if matches!(action, EmojiAction::Keep) {
        return input.to_owned();
    }
    let mut out = String::with_capacity(input.len());
    let mut in_emoji = false;
    for c in input.chars() {
        if is_emoji_component(c) {
            if !in_emoji {
                if let EmojiAction::Replace(s) = action {
                    out.push_str(s);
                }
                in_emoji = true;
            }
        } else {
            out.push(c);
            in_emoji = false;
        }
    }
    out
}

/// 絵文字/その修飾文字として扱う文字か。
fn is_emoji_component(c: char) -> bool {
    let cp = c as u32;
    matches!(cp,
        // Miscellaneous Symbols and Pictographs
        0x1F300..=0x1F5FF |
        // Emoticons
        0x1F600..=0x1F64F |
        // Transport and Map
        0x1F680..=0x1F6FF |
        // Supplemental Symbols and Pictographs
        0x1F900..=0x1F9FF |
        // Symbols and Pictographs Extended-A
        0x1FA70..=0x1FAFF |
        // Dingbats
        0x2700..=0x27BF |
        // Misc Symbols
        0x2600..=0x26FF |
        // Regional Indicator (国旗)
        0x1F1E6..=0x1F1FF |
        // Variation Selectors (FE0F 等)
        0xFE00..=0xFE0F |
        // Zero Width Joiner
        0x200D |
        // Skin tone modifiers
        0x1F3FB..=0x1F3FF
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_basic() {
        assert_eq!(process("hello 😀 world", EmojiAction::Remove), "hello  world");
    }

    #[test]
    fn replace_basic() {
        assert_eq!(
            process("hi 🎉!!", EmojiAction::Replace("[emoji]")),
            "hi [emoji]!!"
        );
    }

    #[test]
    fn zwj_sequence_as_one() {
        // 👨‍👩‍👧 は ZWJ で繋がる一家族の絵文字
        let s = "family 👨\u{200D}👩\u{200D}👧 end";
        assert_eq!(
            process(s, EmojiAction::Replace("X")),
            "family X end"
        );
    }

    #[test]
    fn keep_passthrough() {
        assert_eq!(process("a😀b", EmojiAction::Keep), "a😀b");
    }
}
