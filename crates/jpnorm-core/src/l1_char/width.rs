//! 半角/全角カナの相互変換。
//!
//! M0 では「半角カナ→全角カナ」を提供する。濁点・半濁点の合成も行う。

/// 半角カナ(U+FF61..U+FF9F)を全角カナへ変換する。
/// 後続の濁点 `ﾞ` / 半濁点 `ﾟ` を合成する。
pub fn halfwidth_kana_to_fullwidth(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let mapped = map_halfwidth(c);
        if let Some(base) = mapped {
            // 次が濁点/半濁点なら合成を試みる。
            let next = chars.get(i + 1).copied();
            if next == Some('ﾞ') {
                if let Some(v) = voiced(base) {
                    out.push(v);
                    i += 2;
                    continue;
                }
            } else if next == Some('ﾟ') {
                if let Some(v) = semi_voiced(base) {
                    out.push(v);
                    i += 2;
                    continue;
                }
            }
            out.push(base);
            i += 1;
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

fn map_halfwidth(c: char) -> Option<char> {
    // 0xFF61 '｡' から 0xFF9F 'ﾟ' までをテーブルで対応させる。
    // 濁点付きは別途 voiced() で合成するので、ここでは基底のみ。
    Some(match c {
        '｡' => '。',
        '｢' => '「',
        '｣' => '」',
        '､' => '、',
        '･' => '・',
        'ｦ' => 'ヲ',
        'ｧ' => 'ァ',
        'ｨ' => 'ィ',
        'ｩ' => 'ゥ',
        'ｪ' => 'ェ',
        'ｫ' => 'ォ',
        'ｬ' => 'ャ',
        'ｭ' => 'ュ',
        'ｮ' => 'ョ',
        'ｯ' => 'ッ',
        'ｰ' => 'ー',
        'ｱ' => 'ア',
        'ｲ' => 'イ',
        'ｳ' => 'ウ',
        'ｴ' => 'エ',
        'ｵ' => 'オ',
        'ｶ' => 'カ',
        'ｷ' => 'キ',
        'ｸ' => 'ク',
        'ｹ' => 'ケ',
        'ｺ' => 'コ',
        'ｻ' => 'サ',
        'ｼ' => 'シ',
        'ｽ' => 'ス',
        'ｾ' => 'セ',
        'ｿ' => 'ソ',
        'ﾀ' => 'タ',
        'ﾁ' => 'チ',
        'ﾂ' => 'ツ',
        'ﾃ' => 'テ',
        'ﾄ' => 'ト',
        'ﾅ' => 'ナ',
        'ﾆ' => 'ニ',
        'ﾇ' => 'ヌ',
        'ﾈ' => 'ネ',
        'ﾉ' => 'ノ',
        'ﾊ' => 'ハ',
        'ﾋ' => 'ヒ',
        'ﾌ' => 'フ',
        'ﾍ' => 'ヘ',
        'ﾎ' => 'ホ',
        'ﾏ' => 'マ',
        'ﾐ' => 'ミ',
        'ﾑ' => 'ム',
        'ﾒ' => 'メ',
        'ﾓ' => 'モ',
        'ﾔ' => 'ヤ',
        'ﾕ' => 'ユ',
        'ﾖ' => 'ヨ',
        'ﾗ' => 'ラ',
        'ﾘ' => 'リ',
        'ﾙ' => 'ル',
        'ﾚ' => 'レ',
        'ﾛ' => 'ロ',
        'ﾜ' => 'ワ',
        'ﾝ' => 'ン',
        _ => return None,
    })
}

fn voiced(base: char) -> Option<char> {
    Some(match base {
        'カ' => 'ガ', 'キ' => 'ギ', 'ク' => 'グ', 'ケ' => 'ゲ', 'コ' => 'ゴ',
        'サ' => 'ザ', 'シ' => 'ジ', 'ス' => 'ズ', 'セ' => 'ゼ', 'ソ' => 'ゾ',
        'タ' => 'ダ', 'チ' => 'ヂ', 'ツ' => 'ヅ', 'テ' => 'デ', 'ト' => 'ド',
        'ハ' => 'バ', 'ヒ' => 'ビ', 'フ' => 'ブ', 'ヘ' => 'ベ', 'ホ' => 'ボ',
        'ウ' => 'ヴ',
        _ => return None,
    })
}

fn semi_voiced(base: char) -> Option<char> {
    Some(match base {
        'ハ' => 'パ', 'ヒ' => 'ピ', 'フ' => 'プ', 'ヘ' => 'ペ', 'ホ' => 'ポ',
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        assert_eq!(halfwidth_kana_to_fullwidth("ﾊﾝｶｸｶﾅ"), "ハンカクカナ");
    }

    #[test]
    fn voiced_compose() {
        assert_eq!(halfwidth_kana_to_fullwidth("ｶﾞｷﾞｸﾞ"), "ガギグ");
        assert_eq!(halfwidth_kana_to_fullwidth("ﾊﾟﾋﾟﾌﾟ"), "パピプ");
    }

    #[test]
    fn mixed() {
        assert_eq!(
            halfwidth_kana_to_fullwidth("ｷｮｳﾄｼ ABC"),
            "キョウトシ ABC"
        );
    }
}
