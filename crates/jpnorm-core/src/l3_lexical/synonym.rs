//! 同義語辞書による表記ゆれ解消。
//!
//! エントリ追加時は `HashMap` に保持し、`build()` で
//! [`daachorse::CharwiseDoubleArrayAhoCorasick`] をコンパイルすることで
//! longest-match leftmost 置換を高速に実行する。

use daachorse::charwise::{CharwiseDoubleArrayAhoCorasick, CharwiseDoubleArrayAhoCorasickBuilder};
use daachorse::MatchKind;
use std::collections::HashMap;
use std::fmt;

/// 同義語辞書ロード時のエラー。
#[derive(Debug)]
pub enum SynonymDictError {
    /// CSV/TSV の列数が期待と異なる。
    InvalidRow {
        /// 1始まりの行番号。
        line: usize,
    },
    /// I/O エラー。
    Io(std::io::Error),
}

impl fmt::Display for SynonymDictError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRow { line } => write!(f, "invalid row at line {line}"),
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
                let values: Vec<String> =
                    keys.iter().map(|k| self.map[*k].clone()).collect();
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
    /// 依存を避けるために超最小のパーサを使う。文字列キー/値はバックスラッシュ
    /// エスケープ `\"` `\\` `\n` `\t` のみサポート。
    pub fn from_json(text: &str) -> Result<Self, SynonymDictError> {
        let mut dict = Self::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = skip_ws(&chars, 0);
        if chars.get(i) != Some(&'{') {
            return Err(SynonymDictError::InvalidRow { line: 0 });
        }
        i += 1;
        loop {
            i = skip_ws(&chars, i);
            if chars.get(i) == Some(&'}') {
                break;
            }
            let (k, ni) = parse_string(&chars, i)?;
            i = skip_ws(&chars, ni);
            if chars.get(i) != Some(&':') {
                return Err(SynonymDictError::InvalidRow { line: 0 });
            }
            i += 1;
            i = skip_ws(&chars, i);
            let (v, ni) = parse_string(&chars, i)?;
            i = skip_ws(&chars, ni);
            dict.insert(k, v);
            match chars.get(i) {
                Some(',') => i += 1,
                Some('}') => {
                    break;
                }
                _ => return Err(SynonymDictError::InvalidRow { line: 0 }),
            }
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
        let chars: Vec<char> = text.chars().collect();
        let mut i = skip_ws(&chars, 0);
        if chars.get(i) != Some(&'{') {
            return Err(SynonymDictError::InvalidRow { line: 0 });
        }
        i += 1;
        loop {
            i = skip_ws(&chars, i);
            if chars.get(i) == Some(&'}') {
                break;
            }
            let (canonical, ni) = parse_string(&chars, i)?;
            i = skip_ws(&chars, ni);
            if chars.get(i) != Some(&':') {
                return Err(SynonymDictError::InvalidRow { line: 0 });
            }
            i += 1;
            i = skip_ws(&chars, i);
            if chars.get(i) != Some(&'[') {
                return Err(SynonymDictError::InvalidRow { line: 0 });
            }
            i += 1;
            loop {
                i = skip_ws(&chars, i);
                if chars.get(i) == Some(&']') {
                    i += 1;
                    break;
                }
                let (variant, ni2) = parse_string(&chars, i)?;
                i = skip_ws(&chars, ni2);
                // variant → canonical として登録。canonical 自身は置換対象にしない
                // (恒等変換のため不要)。
                if variant != canonical {
                    dict.insert(variant, canonical.clone());
                }
                match chars.get(i) {
                    Some(',') => i += 1,
                    Some(']') => {
                        i += 1;
                        break;
                    }
                    _ => return Err(SynonymDictError::InvalidRow { line: 0 }),
                }
            }
            i = skip_ws(&chars, i);
            match chars.get(i) {
                Some(',') => i += 1,
                Some('}') => break,
                _ => return Err(SynonymDictError::InvalidRow { line: 0 }),
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

// ---- 最小 JSON 文字列パーサ ----

fn skip_ws(chars: &[char], mut i: usize) -> usize {
    while let Some(&c) = chars.get(i) {
        if c.is_whitespace() {
            i += 1;
        } else {
            break;
        }
    }
    i
}

fn parse_string(chars: &[char], mut i: usize) -> Result<(String, usize), SynonymDictError> {
    if chars.get(i) != Some(&'"') {
        return Err(SynonymDictError::InvalidRow { line: 0 });
    }
    i += 1;
    let mut out = String::new();
    while let Some(&c) = chars.get(i) {
        match c {
            '"' => return Ok((out, i + 1)),
            '\\' => {
                i += 1;
                match chars.get(i) {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('/') => out.push('/'),
                    _ => return Err(SynonymDictError::InvalidRow { line: 0 }),
                }
                i += 1;
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    Err(SynonymDictError::InvalidRow { line: 0 })
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
    fn json_grouped_load() {
        let j = r#"{
            "幽遊白書": ["幽白", "ゆうはく", "幽☆遊☆白書"],
            "パソコン": ["PC", "パーコン"]
        }"#;
        let d = SynonymDict::from_json_grouped(j).unwrap();
        assert_eq!(d.apply("幽白とゆうはくと幽☆遊☆白書"), "幽遊白書と幽遊白書と幽遊白書");
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
