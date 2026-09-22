//! カタカナ外来語の表記ゆれ統一。
//!
//! 「コンピューター / コンピュータ」「ヴァイオリン / バイオリン」「ウェブ / ウエブ」の
//! ように、同じ語が複数の書き方をされる問題を、辞書なしのルールで潰す。
//! 正しい表記を決めるのが目的ではなく、索引側と検索語側で同じ形に落とすのが目的。
//!
//! - [`unify_loanword_kana`][]: ヴ系・ヰヱ・ヂヅ・小書き母音の統一
//! - [`strip_trailing_prolonged`][]: 一定長以上のカタカナ語の末尾長音を落とす
//!   (JIS Z 8301 / Lucene `kuromoji_stemmer` と同じ発想)

/// カタカナ外来語のゆれをルールで統一する。
///
/// 適用ルール (カタカナに対してのみ):
/// - `ヴァ ヴィ ヴ ヴェ ヴォ` → `バ ビ ブ ベ ボ`、`ヴュ` → `ビュ`
/// - `ヰ ヱ` → `イ エ`
/// - `ヂ ヅ` → `ジ ズ`
/// - `ウィ ウェ ウォ` → `ウイ ウエ ウオ`
/// - `ティ ディ` → `テイ デイ`
/// - `トゥ ドゥ` → `ツ ズ`
pub fn unify_loanword_kana(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        // 2 文字で決まるもの
        let two = match (c, next) {
            ('ヴ', Some('ァ')) => Some('バ'),
            ('ヴ', Some('ィ')) => Some('ビ'),
            ('ヴ', Some('ェ')) => Some('ベ'),
            ('ヴ', Some('ォ')) => Some('ボ'),
            ('ウ', Some('ィ')) => Some('イ'),
            ('ウ', Some('ェ')) => Some('エ'),
            ('ウ', Some('ォ')) => Some('オ'),
            ('ト', Some('ゥ')) => Some('ツ'),
            ('ド', Some('ゥ')) => Some('ズ'),
            _ => None,
        };
        if let Some(mapped) = two {
            // ウィ→ウイ のように先頭を残すもの
            if matches!(c, 'ウ') {
                out.push('ウ');
            }
            out.push(mapped);
            i += 2;
            continue;
        }
        if c == 'ヴ' && matches!(next, Some('ュ' | 'ャ' | 'ョ')) {
            out.push('ビ');
            i += 1;
            continue;
        }
        if c == 'テ' && next == Some('ィ') {
            out.push_str("テイ");
            i += 2;
            continue;
        }
        if c == 'デ' && next == Some('ィ') {
            out.push_str("デイ");
            i += 2;
            continue;
        }
        out.push(match c {
            'ヴ' => 'ブ',
            'ヰ' => 'イ',
            'ヱ' => 'エ',
            'ヂ' => 'ジ',
            'ヅ' => 'ズ',
            other => other,
        });
        i += 1;
    }
    out
}

/// `min_len` 文字以上のカタカナ語の末尾長音 `ー` を落とす。
///
/// 「コンピューター → コンピュータ」「サーバー → サーバ」。JIS Z 8301 の
/// 「3 音以上の語では末尾の長音符号を省く」に相当し、Lucene の
/// `kuromoji_stemmer` (既定 4 文字) と同じ挙動。「コーヒー → コーヒ」のように
/// 慣用と異なる形になる語もあるが、索引と検索語の両方に掛ければ一致には影響しない。
pub fn strip_trailing_prolonged(input: &str, min_len: usize) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < chars.len() {
        if !is_katakana_char(chars[i]) {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && is_katakana_char(chars[i]) {
            i += 1;
        }
        let run = &chars[start..i];
        let strip = run.len() >= min_len && run.last() == Some(&'ー') && run.len() > 1;
        let end = if strip { run.len() - 1 } else { run.len() };
        out.extend(&run[..end]);
    }
    out
}

fn is_katakana_char(c: char) -> bool {
    matches!(c as u32, 0x30A1..=0x30FA | 0x30FC | 0x31F0..=0x31FF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vu_series() {
        assert_eq!(unify_loanword_kana("ヴァイオリン"), "バイオリン");
        assert_eq!(unify_loanword_kana("ヴィヴァルディ"), "ビバルデイ");
        assert_eq!(unify_loanword_kana("ヴェネツィア"), "ベネツィア");
        assert_eq!(unify_loanword_kana("ヴォルテール"), "ボルテール");
        assert_eq!(unify_loanword_kana("ヴ"), "ブ");
        assert_eq!(unify_loanword_kana("デジャヴュ"), "デジャビュ");
    }

    #[test]
    fn small_vowels_and_old_kana() {
        assert_eq!(unify_loanword_kana("ウェブサイト"), "ウエブサイト");
        assert_eq!(unify_loanword_kana("ウィキ"), "ウイキ");
        assert_eq!(unify_loanword_kana("パーティー"), "パーテイー");
        assert_eq!(unify_loanword_kana("ディジタル"), "デイジタル");
        assert_eq!(
            unify_loanword_kana("ヰタ・セクスアリス"),
            "イタ・セクスアリス"
        );
        assert_eq!(unify_loanword_kana("ヅラ"), "ズラ");
    }

    #[test]
    fn hiragana_and_others_untouched() {
        assert_eq!(unify_loanword_kana("ゔぁ ぢ づ"), "ゔぁ ぢ づ");
        assert_eq!(unify_loanword_kana("hello 日本"), "hello 日本");
    }

    #[test]
    fn trailing_prolonged() {
        assert_eq!(
            strip_trailing_prolonged("コンピューター", 4),
            "コンピュータ"
        );
        assert_eq!(strip_trailing_prolonged("サーバー", 4), "サーバ");
        assert_eq!(strip_trailing_prolonged("キー", 4), "キー");
        assert_eq!(strip_trailing_prolonged("カー", 4), "カー");
        assert_eq!(
            strip_trailing_prolonged("サーバーとキー", 4),
            "サーバとキー"
        );
        assert_eq!(strip_trailing_prolonged("ー", 4), "ー");
        assert_eq!(strip_trailing_prolonged("ラーメン屋", 4), "ラーメン屋");
    }
}
