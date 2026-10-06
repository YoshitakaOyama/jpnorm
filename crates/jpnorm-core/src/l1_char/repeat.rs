//! 同一文字の過剰な連続を短縮する。

/// 同じ文字が `limit` 回を超えて連続する場合、`limit` 回に短縮する。
///
/// `limit == 0` は意味をなさないので 1 に丸める。
///
/// ASCII 英数字は短縮対象から除外する(`hello` の `ll` や `1200` の `00` を
/// 潰さないため)。短縮が狙うのは「ウェーーーい」「!!!」のような感情表現や
/// 長音/記号の連続であって、通常の単語・数値は触らない。
///
/// ピリオドと括弧類も対象外。`...` (NFKC 後の `…`) や数式・コードの `}}}` `))` は
/// 個数に意味があり、感情表現として連打されるものではないため。
pub fn shorten(input: &str, limit: usize) -> String {
    let limit = limit.max(1);
    let mut out = String::with_capacity(input.len());
    let mut last: Option<char> = None;
    let mut run = 0usize;
    for c in input.chars() {
        if c.is_ascii_alphanumeric() || is_structural(c) {
            // ASCII 英数字・括弧類はランを壊さずそのまま通す。
            last = None;
            run = 0;
            out.push(c);
            continue;
        }
        if Some(c) == last {
            run += 1;
            if run <= limit {
                out.push(c);
            }
        } else {
            last = Some(c);
            run = 1;
            out.push(c);
        }
    }
    out
}

/// 個数に意味がある記号 (ピリオド・括弧類)。
fn is_structural(c: char) -> bool {
    matches!(
        c,
        '.' | '('
            | ')'
            | '['
            | ']'
            | '{'
            | '}'
            | '<'
            | '>'
            | '（'
            | '）'
            | '「'
            | '」'
            | '『'
            | '』'
            | '【'
            | '】'
            | '〈'
            | '〉'
            | '《'
            | '》'
            | '［'
            | '］'
            | '｛'
            | '｝'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shrink_basic() {
        assert_eq!(shorten("ウェーーーーイ", 2), "ウェーーイ");
        assert_eq!(shorten("abc", 2), "abc");
    }

    #[test]
    fn ascii_alnum_is_never_shortened() {
        // ASCII は保護されるので hello/1200/wwwww は原形維持。
        assert_eq!(shorten("hello", 1), "hello");
        assert_eq!(shorten("1200", 1), "1200");
        assert_eq!(shorten("wwwww", 3), "wwwww");
    }

    #[test]
    fn periods_and_brackets_are_never_shortened() {
        // Wikipedia 記事で見つかった誤変換: … (NFKC で ...) と LaTeX の }}}
        assert_eq!(shorten("でしょう...」", 2), "でしょう...」");
        assert_eq!(shorten("{2\\pi }}}}", 2), "{2\\pi }}}}");
        assert_eq!(shorten("f(g(x)))", 1), "f(g(x)))");
        assert_eq!(shorten("『「」』", 1), "『「」』");
        // 感情表現の記号は従来どおり短縮する
        assert_eq!(shorten("!!!!??", 2), "!!??");
    }
}
