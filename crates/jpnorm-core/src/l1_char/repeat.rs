//! 同一文字の過剰な連続を短縮する。

/// 同じ文字が `limit` 回を超えて連続する場合、`limit` 回に短縮する。
///
/// `limit == 0` は意味をなさないので 1 に丸める。
///
/// ASCII 英数字は短縮対象から除外する(`hello` の `ll` や `1200` の `00` を
/// 潰さないため)。短縮が狙うのは「ウェーーーい」「!!!」のような感情表現や
/// 長音/記号の連続であって、通常の単語・数値は触らない。
pub fn shorten(input: &str, limit: usize) -> String {
    let limit = limit.max(1);
    let mut out = String::with_capacity(input.len());
    let mut last: Option<char> = None;
    let mut run = 0usize;
    for c in input.chars() {
        if c.is_ascii_alphanumeric() {
            // ASCII 英数字はランを壊さずそのまま通す。
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
}
