//! 同義語辞書による表記ゆれ解消。
//!
//! エントリ追加時は `HashMap` に保持し、`build()` で
//! [`daachorse::CharwiseDoubleArrayAhoCorasick`] をコンパイルすることで
//! longest-match leftmost 置換を高速に実行する。

use daachorse::MatchKind;
use daachorse::charwise::{CharwiseDoubleArrayAhoCorasick, CharwiseDoubleArrayAhoCorasickBuilder};
use std::collections::HashMap;
use std::fmt;

/// 同義語辞書ロード時のエラー。
#[derive(Debug)]
#[non_exhaustive]
pub enum SynonymDictError {
    /// CSV/TSV の列数が期待と異なる。
    InvalidRow {
        /// 1始まりの行番号。
        line: usize,
    },
    /// JSON の構文または構造が不正。
    InvalidJson {
        /// 先頭からの文字オフセット(0始まり)。
        offset: usize,
        /// 何が期待されていたか。
        message: &'static str,
    },
    /// I/O エラー。
    Io(std::io::Error),
}

impl fmt::Display for SynonymDictError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRow { line } => write!(f, "invalid row at line {line}"),
            Self::InvalidJson { offset, message } => {
                write!(f, "invalid json at char {offset}: {message}")
            }
            Self::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for SynonymDictError {}

impl From<std::io::Error> for SynonymDictError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// 同義語辞書。キー(表記ゆれ)→正規形(canonical)のマップ。
///
/// 追加後は `build()` で内部的に Aho-Corasick オートマトンが構築され、
/// 以降の `apply()` は線形時間で longest-match leftmost 置換を実行する。
#[derive(Default)]
pub struct SynonymDict {
    map: HashMap<String, String>,
    /// 最長キー文字数(char 単位)。参考用に保持。
    max_key_chars: usize,
    /// 構築済み Aho-Corasick。`apply()` 呼び出し時に遅延構築する。
    automaton: std::sync::OnceLock<Automaton>,
    /// パターンID → canonical 値の並び(automaton と対応)。
    values: std::sync::OnceLock<Vec<String>>,
}

/// 内部の daachorse 型エイリアス。
type Automaton = CharwiseDoubleArrayAhoCorasick<u32>;

impl fmt::Debug for SynonymDict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SynonymDict")
            .field("len", &self.map.len())
            .field("max_key_chars", &self.max_key_chars)
            .finish()
    }
}

impl Clone for SynonymDict {
    fn clone(&self) -> Self {
        // Automaton は再構築すれば良いので、生データだけ複製する。
        Self {
            map: self.map.clone(),
            max_key_chars: self.max_key_chars,
            automaton: std::sync::OnceLock::new(),
            values: std::sync::OnceLock::new(),
        }
    }
}

impl SynonymDict {
    /// 空の辞書を作成する。
    pub fn new() -> Self {
        Self::default()
    }

    /// エントリ数。
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// 空かどうか。
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// 1件追加する。以前にビルドされたオートマトンは破棄される。
    pub fn insert(&mut self, variant: impl Into<String>, canonical: impl Into<String>) {
        let variant = variant.into();
        let canonical = canonical.into();
        // 空文字キーは Aho-Corasick で扱えないので無視する。
        if variant.is_empty() {
            return;
        }
        let key_chars = variant.chars().count();
        if key_chars > self.max_key_chars {
            self.max_key_chars = key_chars;
        }
        self.map.insert(variant, canonical);
        // 辞書が変化したのでキャッシュを無効化する。
        self.automaton = std::sync::OnceLock::new();
        self.values = std::sync::OnceLock::new();
    }

    /// 全エントリ `(表記ゆれ, 正規形)` を返す(順序は不定)。
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.map.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// 既存の辞書をマージする。
    pub fn extend(&mut self, other: SynonymDict) {
        for (k, v) in other.map {
            self.insert(k, v);
        }
    }

    /// 内部 Aho-Corasick を(まだなら)構築して参照を返す。
    fn automaton(&self) -> Option<(&Automaton, &[String])> {
        if self.map.is_empty() {
            return None;
        }
        let (aut, values) = match (self.automaton.get(), self.values.get()) {
            (Some(a), Some(v)) => (a, v.as_slice()),
            _ => {
                // 決定的な順序にするため、キーをソートする。
                let mut keys: Vec<&String> = self.map.keys().collect();
                keys.sort();
                let values: Vec<String> = keys.iter().map(|k| self.map[*k].clone()).collect();
                let built: Automaton = CharwiseDoubleArrayAhoCorasickBuilder::new()
                    .match_kind(MatchKind::LeftmostLongest)
                    .build(keys.iter().map(|k| k.as_str()))
                    .ok()?;
                let _ = self.automaton.set(built);
                let _ = self.values.set(values);
                (
                    self.automaton.get().unwrap(),
                    self.values.get().unwrap().as_slice(),
                )
            }
        };
        Some((aut, values))
    }

    /// `separator` 区切り (CSV は `,`, TSV は `\t`) の文字列から読み込む。
    /// 各行は `variant<sep>canonical` の2列。空行・`#` で始まる行はコメントとして無視。
    pub fn from_delimited(text: &str, separator: char) -> Result<Self, SynonymDictError> {
        let mut dict = Self::new();
        for (idx, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.splitn(2, separator);
            let variant = parts.next().map(str::trim);
            let canonical = parts.next().map(str::trim);
            match (variant, canonical) {
                (Some(v), Some(c)) if !v.is_empty() => dict.insert(v, c),
                _ => return Err(SynonymDictError::InvalidRow { line: idx + 1 }),
            }
        }
        Ok(dict)
    }

    /// JSON オブジェクト `{"variant": "canonical", ...}` 形式の文字列から読み込む。
    ///
    /// 依存を避けるため最小限の JSON パーサを内蔵している。文字列・配列・オブジェクト・
    /// 標準エスケープ(`\uXXXX` とサロゲートペアを含む)をサポートする。
    pub fn from_json(text: &str) -> Result<Self, SynonymDictError> {
        let mut dict = Self::new();
        for (variant, value) in json::parse_object(text)? {
            let canonical = value.into_string("expected string value")?;
            dict.insert(variant, canonical);
        }
        Ok(dict)
    }

    /// カスタム辞書 JSON `{"canonical": ["variant1", "variant2", ...], ...}` 形式から読み込む。
    ///
    /// 正規形(canonical)をキーに、その表記ゆれ(variants)を配列で与える
    /// 「カスタム辞書」向けのフォーマット。内部的には各 variant → canonical の
    /// エントリに展開され、`from_json` と同じ longest-match 置換で動作する。
    ///
    /// 例:
    /// ```
    /// use jpnorm_core::SynonymDict;
    /// let j = r#"{"幽遊白書": ["幽白", "ゆうはく", "幽☆遊☆白書"]}"#;
    /// let d = SynonymDict::from_json_grouped(j).unwrap();
    /// assert_eq!(d.apply("昨日ゆうはくを読んだ"), "昨日幽遊白書を読んだ");
    /// assert_eq!(d.apply("幽☆遊☆白書は名作"), "幽遊白書は名作");
    /// ```
    pub fn from_json_grouped(text: &str) -> Result<Self, SynonymDictError> {
        let mut dict = Self::new();
        for (canonical, value) in json::parse_object(text)? {
            for item in value.into_array("expected array of strings")? {
                let variant = item.into_string("expected string in array")?;
                // canonical 自身は置換対象にしない(恒等変換のため不要)。
                if variant != canonical {
                    dict.insert(variant, canonical.clone());
                }
            }
        }
        Ok(dict)
    }

    /// 入力テキストに対して longest-match leftmost で置換を実行する。
    ///
    /// 初回呼び出し時に Aho-Corasick を構築し、以降は構築済みのものを再利用する。
    pub fn apply(&self, input: &str) -> String {
        let Some((aut, values)) = self.automaton() else {
            return input.to_owned();
        };
        let mut out = String::with_capacity(input.len());
        let mut cursor = 0usize;
        for m in aut.leftmost_find_iter(input) {
            let start = m.start();
            let end = m.end();
            if start > cursor {
                out.push_str(&input[cursor..start]);
            }
            let canonical = &values[m.value() as usize];
            out.push_str(canonical);
            cursor = end;
        }
        if cursor < input.len() {
            out.push_str(&input[cursor..]);
        }
        out
    }
}

// ---- 最小 JSON パーサ(依存追加を避けるため) ----

mod json {
    use super::SynonymDictError;

    /// 辞書ロードに必要な範囲の JSON 値。数値/真偽値/null は `Other` にまとめる。
    pub(super) enum Value {
        Str(String),
        Arr(Vec<Value>),
        Obj(Vec<(String, Value)>),
        Other,
    }

    impl Value {
        pub(super) fn into_string(self, message: &'static str) -> Result<String, SynonymDictError> {
            match self {
                Value::Str(s) => Ok(s),
                _ => Err(SynonymDictError::InvalidJson { offset: 0, message }),
            }
        }

        pub(super) fn into_array(
            self,
            message: &'static str,
        ) -> Result<Vec<Value>, SynonymDictError> {
            match self {
                Value::Arr(v) => Ok(v),
                _ => Err(SynonymDictError::InvalidJson { offset: 0, message }),
            }
        }
    }

    /// トップレベルがオブジェクトであることを要求してパースする。
    pub(super) fn parse_object(text: &str) -> Result<Vec<(String, Value)>, SynonymDictError> {
        let chars: Vec<char> = text.chars().collect();
        let mut p = Parser {
            chars: &chars,
            i: 0,
        };
        p.skip_ws();
        let value = p.parse_value()?;
        p.skip_ws();
        if p.i != p.chars.len() {
            return Err(p.err("trailing characters after top-level value"));
        }
        match value {
            Value::Obj(entries) => Ok(entries),
            _ => Err(SynonymDictError::InvalidJson {
                offset: 0,
                message: "expected top-level object",
            }),
        }
    }

    struct Parser<'a> {
        chars: &'a [char],
        i: usize,
    }

    impl Parser<'_> {
        fn err(&self, message: &'static str) -> SynonymDictError {
            SynonymDictError::InvalidJson {
                offset: self.i,
                message,
            }
        }

        fn peek(&self) -> Option<char> {
            self.chars.get(self.i).copied()
        }

        fn skip_ws(&mut self) {
            while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
                self.i += 1;
            }
        }

        fn expect(&mut self, c: char, message: &'static str) -> Result<(), SynonymDictError> {
            if self.peek() == Some(c) {
                self.i += 1;
                Ok(())
            } else {
                Err(self.err(message))
            }
        }

        fn parse_value(&mut self) -> Result<Value, SynonymDictError> {
            match self.peek() {
                Some('"') => self.parse_string().map(Value::Str),
                Some('[') => self.parse_array(),
                Some('{') => self.parse_object(),
                Some(_) => {
                    self.skip_scalar();
                    Ok(Value::Other)
                }
                None => Err(self.err("unexpected end of input")),
            }
        }

        /// 数値/true/false/null を読み飛ばす。
        fn skip_scalar(&mut self) {
            while let Some(c) = self.peek() {
                if matches!(c, ',' | ']' | '}' | ' ' | '\t' | '\n' | '\r') {
                    break;
                }
                self.i += 1;
            }
        }

        fn parse_array(&mut self) -> Result<Value, SynonymDictError> {
            self.expect('[', "expected '['")?;
            let mut items = Vec::new();
            loop {
                self.skip_ws();
                if self.peek() == Some(']') {
                    self.i += 1;
                    return Ok(Value::Arr(items));
                }
                items.push(self.parse_value()?);
                self.skip_ws();
                match self.peek() {
                    Some(',') => self.i += 1,
                    Some(']') => {
                        self.i += 1;
                        return Ok(Value::Arr(items));
                    }
                    _ => return Err(self.err("expected ',' or ']'")),
                }
            }
        }

        fn parse_object(&mut self) -> Result<Value, SynonymDictError> {
            self.expect('{', "expected '{'")?;
            let mut entries = Vec::new();
            loop {
                self.skip_ws();
                if self.peek() == Some('}') {
                    self.i += 1;
                    return Ok(Value::Obj(entries));
                }
                let key = self.parse_string()?;
                self.skip_ws();
                self.expect(':', "expected ':' after object key")?;
                self.skip_ws();
                let value = self.parse_value()?;
                entries.push((key, value));
                self.skip_ws();
                match self.peek() {
                    Some(',') => self.i += 1,
                    Some('}') => {
                        self.i += 1;
                        return Ok(Value::Obj(entries));
                    }
                    _ => return Err(self.err("expected ',' or '}'")),
                }
            }
        }

        fn parse_string(&mut self) -> Result<String, SynonymDictError> {
            self.expect('"', "expected string")?;
            let mut out = String::new();
            loop {
                let Some(c) = self.peek() else {
                    return Err(self.err("unterminated string"));
                };
                self.i += 1;
                match c {
                    '"' => return Ok(out),
                    '\\' => {
                        let Some(e) = self.peek() else {
                            return Err(self.err("unterminated escape"));
                        };
                        self.i += 1;
                        match e {
                            '"' => out.push('"'),
                            '\\' => out.push('\\'),
                            '/' => out.push('/'),
                            'b' => out.push('\u{0008}'),
                            'f' => out.push('\u{000C}'),
                            'n' => out.push('\n'),
                            'r' => out.push('\r'),
                            't' => out.push('\t'),
                            'u' => out.push(self.parse_unicode_escape()?),
                            _ => return Err(self.err("invalid escape sequence")),
                        }
                    }
                    _ => out.push(c),
                }
            }
        }

        /// `\u` の直後から 4 桁を読む。サロゲートペアは結合して 1 文字にする。
        fn parse_unicode_escape(&mut self) -> Result<char, SynonymDictError> {
            let hi = self.parse_hex4()?;
            if (0xD800..=0xDBFF).contains(&hi) {
                // 上位サロゲート: 続く \uXXXX が下位サロゲートである必要がある。
                if self.peek() == Some('\\') && self.chars.get(self.i + 1) == Some(&'u') {
                    self.i += 2;
                    let lo = self.parse_hex4()?;
                    if (0xDC00..=0xDFFF).contains(&lo) {
                        let cp = 0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00);
                        return char::from_u32(cp).ok_or_else(|| self.err("invalid code point"));
                    }
                }
                return Err(self.err("lone high surrogate in \\u escape"));
            }
            char::from_u32(hi).ok_or_else(|| self.err("invalid \\u escape (lone surrogate)"))
        }

        fn parse_hex4(&mut self) -> Result<u32, SynonymDictError> {
            let mut v = 0u32;
            for _ in 0..4 {
                let Some(d) = self.peek().and_then(|c| c.to_digit(16)) else {
                    return Err(self.err("expected 4 hex digits after \\u"));
                };
                v = (v << 4) | d;
                self.i += 1;
            }
            Ok(v)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_apply() {
        let mut d = SynonymDict::new();
        d.insert("パソコン", "パーソナルコンピュータ");
        d.insert("JR東", "東日本旅客鉄道");
        let out = d.apply("昨日パソコンとJR東を使った");
        assert_eq!(out, "昨日パーソナルコンピュータと東日本旅客鉄道を使った");
    }

    #[test]
    fn longest_match() {
        let mut d = SynonymDict::new();
        d.insert("東京", "TOKYO");
        d.insert("東京都", "TOKYO-METRO");
        // "東京都" のほうが長いので優先される。
        assert_eq!(d.apply("東京都に住む"), "TOKYO-METROに住む");
        assert_eq!(d.apply("東京に行く"), "TOKYOに行く");
    }

    #[test]
    fn csv_load() {
        let csv = "# comment\nパソコン,パーソナルコンピュータ\nJR東,東日本旅客鉄道\n";
        let d = SynonymDict::from_delimited(csv, ',').unwrap();
        assert_eq!(d.len(), 2);
        assert_eq!(d.apply("パソコン"), "パーソナルコンピュータ");
    }

    #[test]
    fn tsv_load() {
        let tsv = "受付\t受け付け\n";
        let d = SynonymDict::from_delimited(tsv, '\t').unwrap();
        assert_eq!(d.apply("受付"), "受け付け");
    }

    #[test]
    fn json_load() {
        let j = r#"{"パソコン": "パーソナルコンピュータ", "旧字": "新字"}"#;
        let d = SynonymDict::from_json(j).unwrap();
        assert_eq!(d.len(), 2);
        assert_eq!(d.apply("パソコン"), "パーソナルコンピュータ");
    }

    #[test]
    fn json_unicode_escapes() {
        // Python の json.dumps() 既定 (ensure_ascii=True) の出力を読めること。
        let j = r#"{"\u5e7d\u904a\u767d\u66f8": ["\u5e7d\u767d", "\ud83d\ude00"]}"#;
        let d = SynonymDict::from_json_grouped(j).unwrap();
        assert_eq!(d.apply("幽白と😀"), "幽遊白書と幽遊白書");
    }

    #[test]
    fn json_all_escapes_and_whitespace() {
        let j = "{\r\n  \"a\\\"b\" : \"x\\/y\\n\" ,\n \"c\": \"\\b\\f\\r\\t\"\n}";
        let d = SynonymDict::from_json(j).unwrap();
        assert_eq!(d.apply("a\"b"), "x/y\n");
        assert_eq!(d.apply("c"), "\u{8}\u{c}\r\t");
    }

    #[test]
    fn json_errors_are_descriptive() {
        let e = SynonymDict::from_json("[1, 2]").unwrap_err();
        assert!(matches!(e, SynonymDictError::InvalidJson { .. }));
        assert!(e.to_string().contains("top-level object"), "{e}");
        let e = SynonymDict::from_json_grouped(r#"{"a": "b"}"#).unwrap_err();
        assert!(e.to_string().contains("array"), "{e}");
        let e = SynonymDict::from_json(r#"{"a": "b"} x"#).unwrap_err();
        assert!(e.to_string().contains("trailing"), "{e}");
        let e = SynonymDict::from_json(r#"{"a": "\ud800"}"#).unwrap_err();
        assert!(e.to_string().contains("surrogate"), "{e}");
    }

    #[test]
    fn json_tolerates_non_string_values_when_unused() {
        // 未使用の値型(数値/真偽値/null)は構造としては受理し、型不一致は明示エラー。
        let e = SynonymDict::from_json(r#"{"a": 1}"#).unwrap_err();
        assert!(e.to_string().contains("string"), "{e}");
    }

    #[test]
    fn json_grouped_load() {
        let j = r#"{
            "幽遊白書": ["幽白", "ゆうはく", "幽☆遊☆白書"],
            "パソコン": ["PC", "パーコン"]
        }"#;
        let d = SynonymDict::from_json_grouped(j).unwrap();
        assert_eq!(
            d.apply("幽白とゆうはくと幽☆遊☆白書"),
            "幽遊白書と幽遊白書と幽遊白書"
        );
        assert_eq!(d.apply("PCを買う"), "パソコンを買う");
        // canonical 自身は変化しない
        assert_eq!(d.apply("幽遊白書"), "幽遊白書");
    }

    #[test]
    fn json_grouped_empty_variants() {
        let d = SynonymDict::from_json_grouped(r#"{"正規": []}"#).unwrap();
        assert_eq!(d.len(), 0);
    }

    #[test]
    fn empty_dict_passthrough() {
        let d = SynonymDict::new();
        assert_eq!(d.apply("何も変わらない"), "何も変わらない");
    }
}
