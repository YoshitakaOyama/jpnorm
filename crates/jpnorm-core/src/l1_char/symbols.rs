//! ハイフン/チルダ/長音/引用符など記号類の統一。

/// 記号類を統一する。
///
/// - `hyphens=true` の場合、各種ハイフン/マイナス/ダッシュを `-` に揃える。
/// - `tildes=true` の場合、各種チルダ/波ダッシュを `〜` に揃える。
/// - `prolonged=true` の場合、長音符バリエーションを `ー` に揃える。
pub fn unify(input: &str, hyphens: bool, tildes: bool, prolonged: bool) -> String {
    let mut out = String::with_capacity(input.len());
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
            '\u{2500}' | '\u{2501}' | '\u{FF70}' if prolonged => 'ー',
            _ => c,
        };
        out.push(mapped);
    }
    out
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
}
