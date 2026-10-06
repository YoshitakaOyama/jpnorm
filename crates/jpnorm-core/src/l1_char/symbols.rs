//! ハイフン/チルダ/長音/引用符など記号類の統一。

/// 記号類を統一する。
///
/// - `hyphens=true` の場合、各種ハイフン/マイナス/ダッシュを `-` に揃える。
/// - `tildes=true` の場合、各種チルダ/波ダッシュを `〜` に揃える。
/// - `prolonged=true` の場合、長音符バリエーションを `ー` に揃える。
///   罫線 `─` `━` は長音の代用として打たれることがある一方、「日本全史─ジャパン」の
///   ようにダッシュとしても使われる。そこで直前がかな (または `ー`) のときだけ `ー` にし、
///   それ以外は `hyphens=true` なら `-`、そうでなければそのまま残す。
pub fn unify(input: &str, hyphens: bool, tildes: bool, prolonged: bool) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev: Option<char> = None;
    for c in input.chars() {
        let mapped = match c {
            // Hyphens / dashes / minus
            '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}' | '\u{2015}'
            | '\u{2212}' | '\u{FF0D}' | '\u{FE63}' | '\u{FE58}' | '\u{00AD}' | '\u{058A}'
                if hyphens =>
            {
                '-'
            }
            // Tildes / wave dashes (ASCII `~` included)
            '\u{007E}' | '\u{FF5E}' | '\u{301C}' | '\u{223C}' | '\u{223D}' | '\u{223E}'
            | '\u{2053}' | '\u{02DC}'
                if tildes =>
            {
                '〜'
            }
            // Prolonged sound marks
            '\u{FF70}' if prolonged => 'ー',
            // Box drawing: かなの直後だけ長音、それ以外はダッシュ扱い
            '\u{2500}' | '\u{2501}' if prolonged && prev.is_some_and(is_kana_or_prolonged) => 'ー',
            '\u{2500}' | '\u{2501}' if hyphens => '-',
            _ => c,
        };
        out.push(mapped);
        prev = Some(mapped);
    }
    out
}

fn is_kana_or_prolonged(c: char) -> bool {
    matches!(c as u32, 0x3041..=0x3096 | 0x30A1..=0x30FA | 0x30FC | 0xFF66..=0xFF9F)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hyphen_unify() {
        assert_eq!(unify("ab‐cd—ef−gh", true, false, false), "ab-cd-ef-gh");
    }

    #[test]
    fn tilde_unify() {
        assert_eq!(unify("a~b～c", true, true, false), "a〜b〜c");
    }

    #[test]
    fn box_drawing_is_prolonged_only_after_kana() {
        assert_eq!(unify("ハ─ト", false, false, true), "ハート");
        assert_eq!(unify("カ━ド", false, false, true), "カード");
        assert_eq!(unify("ハ──ト", false, false, true), "ハーート");
        // ダッシュとしての罫線
        assert_eq!(
            unify("日本全史─ジャパン", false, false, true),
            "日本全史─ジャパン"
        );
        assert_eq!(
            unify("日本全史─ジャパン", true, false, true),
            "日本全史-ジャパン"
        );
        assert_eq!(unify("Python ──Web", true, false, true), "Python --Web");
        assert_eq!(unify("A─B", false, false, false), "A─B");
    }
}
