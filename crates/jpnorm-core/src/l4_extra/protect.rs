//! 正規化対象外の領域(URL, email, @mention, #hashtag)をスキャンする。
//!
//! 依存を増やさないためのミニマムなスキャナで、完璧な RFC 準拠ではない。
//! 実用上よく現れる形だけをカバーする。

use std::ops::Range;

/// 保護領域の種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `http://` または `https://` で始まる URL。
    Url,
    /// メールアドレス。
    Email,
    /// `@username` 形式のメンション。
    Mention,
    /// `#tag` 形式のハッシュタグ。
    Hashtag,
}

/// 保護領域(入力のバイト範囲と種別)。
#[derive(Debug, Clone)]
pub struct Span {
    /// 入力テキストの **バイト** 範囲。
    pub range: Range<usize>,
    /// 保護領域の種別。
    pub kind: Kind,
}

/// どの種別を保護するかの設定。
#[derive(Debug, Clone, Copy, Default)]
pub struct ProtectConfig {
    /// URL を保護する。
    pub urls: bool,
    /// メールアドレスを保護する。
    pub emails: bool,
    /// `@mention` を保護する。
    pub mentions: bool,
    /// `#hashtag` を保護する。
    pub hashtags: bool,
}

impl ProtectConfig {
    /// 全てを保護する設定。
    pub const fn all() -> Self {
        Self {
            urls: true,
            emails: true,
            mentions: true,
            hashtags: true,
        }
    }

    /// 何か1つでも保護対象があるか。
    pub const fn any(&self) -> bool {
        self.urls || self.emails || self.mentions || self.hashtags
    }
}

/// 入力テキストから保護領域を検出して、開始位置昇順に返す。
/// 範囲は重ならず、重複がある場合は先に見つかった方を優先する。
pub fn scan(input: &str, cfg: ProtectConfig) -> Vec<Span> {
    if !cfg.any() {
        return Vec::new();
    }
    let mut spans: Vec<Span> = Vec::new();
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        // マルチバイト文字の途中ではマッチしない。
        if !input.is_char_boundary(i) {
            i += 1;
            continue;
        }

        if cfg.urls {
            if let Some(end) = match_url(input, i) {
                spans.push(Span {
                    range: i..end,
                    kind: Kind::Url,
                });
                i = end;
                continue;
            }
        }
        if cfg.emails {
            if let Some((start, end)) = match_email(input, i) {
                spans.push(Span {
                    range: start..end,
                    kind: Kind::Email,
                });
                i = end;
                continue;
            }
        }
        if cfg.mentions {
            if let Some(end) = match_prefixed(input, i, b'@') {
                spans.push(Span {
                    range: i..end,
                    kind: Kind::Mention,
                });
                i = end;
                continue;
            }
        }
        if cfg.hashtags {
            if let Some(end) = match_prefixed(input, i, b'#') {
                spans.push(Span {
                    range: i..end,
                    kind: Kind::Hashtag,
                });
                i = end;
                continue;
            }
        }
        i += next_char_width(bytes, i);
    }
    spans
}

fn next_char_width(bytes: &[u8], i: usize) -> usize {
    // UTF-8 の先頭バイトから文字長を算出。
    let b = bytes[i];
    if b < 0x80 {
        1
    } else if b < 0xC0 {
        1 // 続きバイト(不正): 1バイト進める
    } else if b < 0xE0 {
        2
    } else if b < 0xF0 {
        3
    } else {
        4
    }
}

fn match_url(input: &str, i: usize) -> Option<usize> {
    let rest = &input[i..];
    let lower_prefix = rest.as_bytes().get(..8).map(|b| b.to_ascii_lowercase());
    let (prefix_len, found) = if let Some(p) = lower_prefix.as_ref() {
        if p.starts_with(b"https://") {
            (8, true)
        } else if p.starts_with(b"http://") {
            (7, true)
        } else {
            (0, false)
        }
    } else {
        (0, false)
    };
    if !found {
        return None;
    }
    // URL として許容する文字: ASCII 英数 + `-._~:/?#[]@!$&'()*+,;=%`
    let mut end = i + prefix_len;
    let b = input.as_bytes();
    while end < b.len() {
        let c = b[end];
        if c.is_ascii_alphanumeric()
            || matches!(
                c,
                b'-' | b'.'
                    | b'_'
                    | b'~'
                    | b':'
                    | b'/'
                    | b'?'
                    | b'#'
                    | b'['
                    | b']'
                    | b'@'
                    | b'!'
                    | b'$'
                    | b'&'
                    | b'\''
                    | b'('
                    | b')'
                    | b'*'
                    | b'+'
                    | b','
                    | b';'
                    | b'='
                    | b'%'
            )
        {
            end += 1;
        } else {
            break;
        }
    }
    // 末尾の句読点(`.`, `,`, `)`, `?`, `!`)は URL から外すのが実用的。
    while end > i + prefix_len {
        let last = b[end - 1];
        if matches!(last, b'.' | b',' | b')' | b'?' | b'!') {
            end -= 1;
        } else {
            break;
        }
    }
    if end > i + prefix_len {
        Some(end)
    } else {
        None
    }
}

fn match_email(input: &str, i: usize) -> Option<(usize, usize)> {
    // `user@host.tld` を検出する。`@` を起点に前後に伸ばす。
    let b = input.as_bytes();
    if b.get(i).copied() != Some(b'@') {
        return None;
    }
    // 後方: ローカル部
    let mut start = i;
    while start > 0 {
        let c = b[start - 1];
        if c.is_ascii_alphanumeric()
            || matches!(c, b'.' | b'_' | b'+' | b'-' | b'%')
        {
            start -= 1;
        } else {
            break;
        }
    }
    if start == i {
        return None;
    }
    // 前方: ドメイン部
    let mut end = i + 1;
    while end < b.len() {
        let c = b[end];
        if c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-') {
            end += 1;
        } else {
            break;
        }
    }
    // ドメインに少なくとも1つのドットを要求。
    let domain = &input[i + 1..end];
    if !domain.contains('.') || domain.ends_with('.') {
        return None;
    }
    Some((start, end))
}

fn match_prefixed(input: &str, i: usize, prefix: u8) -> Option<usize> {
    let b = input.as_bytes();
    if b.get(i).copied() != Some(prefix) {
        return None;
    }
    // 直前が英数なら @foo/#foo ではなく email の一部等なのでスキップ。
    if i > 0 {
        let prev = b[i - 1];
        if prev.is_ascii_alphanumeric() || prev == b'_' {
            return None;
        }
    }
    let mut end = i + 1;
    while end < b.len() {
        let c = b[end];
        if c.is_ascii_alphanumeric() || c == b'_' {
            end += 1;
        } else {
            break;
        }
    }
    if end > i + 1 {
        Some(end)
    } else {
        None
    }
}

/// 入力を「非保護セグメント」と「保護セグメント」に分割する。
///
/// 返すベクタは入力順で、`Ok(&str)` が非保護(正規化対象)、
/// `Err((&str, Kind))` が保護領域(そのまま保つ)を表す。
pub fn segment<'a>(
    input: &'a str,
    spans: &[Span],
) -> Vec<Result<&'a str, (&'a str, Kind)>> {
    let mut out = Vec::with_capacity(spans.len() * 2 + 1);
    let mut cursor = 0usize;
    for s in spans {
        if s.range.start > cursor {
            out.push(Ok(&input[cursor..s.range.start]));
        }
        out.push(Err((&input[s.range.clone()], s.kind)));
        cursor = s.range.end;
    }
    if cursor < input.len() {
        out.push(Ok(&input[cursor..]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_url() {
        let s = "詳しくは https://example.com/a?b=1 を見て。";
        let spans = scan(s, ProtectConfig::all());
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].kind, Kind::Url);
        assert_eq!(&s[spans[0].range.clone()], "https://example.com/a?b=1");
    }

    #[test]
    fn url_strips_trailing_punct() {
        let s = "here: https://example.com/path.";
        let spans = scan(s, ProtectConfig::all());
        assert_eq!(&s[spans[0].range.clone()], "https://example.com/path");
    }

    #[test]
    fn detect_email() {
        let s = "連絡先: foo.bar+test@example.co.jp まで";
        let spans = scan(s, ProtectConfig::all());
        let email: Vec<_> = spans.iter().filter(|s| s.kind == Kind::Email).collect();
        assert_eq!(email.len(), 1);
        assert_eq!(&s[email[0].range.clone()], "foo.bar+test@example.co.jp");
    }

    #[test]
    fn detect_mention_and_hashtag() {
        let s = "@alice と #rust_lang";
        let spans = scan(s, ProtectConfig::all());
        let kinds: Vec<_> = spans.iter().map(|s| s.kind).collect();
        assert_eq!(kinds, vec![Kind::Mention, Kind::Hashtag]);
    }

    #[test]
    fn segment_split() {
        let s = "見て https://example.com/a を";
        let spans = scan(s, ProtectConfig::all());
        let segs = segment(s, &spans);
        assert_eq!(segs.len(), 3);
        assert!(matches!(segs[1], Err((_, Kind::Url))));
    }
}
