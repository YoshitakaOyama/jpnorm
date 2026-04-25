//! Sudachi 同義語辞書 `synonyms.txt` のパーサ。
//!
//! フォーマット(カンマ区切り、Shift_JIS ではなく UTF-8):
//!
//! | 列 | 内容 |
//! |----|------|
//! | 0  | グループID (6桁) |
//! | 1  | 体言/用言フラグ |
//! | 2  | 展開制御フラグ |
//! | 3  | グループ内の語彙素番号 |
//! | 4  | 語形種別 (0=代表語, 1=対訳, 2=別称, 3=旧称, 4=誤用) |
//! | 5  | 略語情報 |
//! | 6  | 表記揺れ情報 (0=代表表記) |
//! | 7  | 分野情報 |
//! | 8  | 見出し |
//!
//! グループ内で col4=0 かつ col6=0 の行を代表表記とし、
//! 同一グループの他の見出しをその代表にマップする。
//! 代表が見つからなければ、グループ内で最初に現れた見出しを代表として使う。
//!
//! 空行や `#` で始まる行はコメントとしてスキップする。

use jpnorm_core::SynonymDict;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;

/// Sudachi 同義語辞書パース時のエラー。
#[derive(Debug)]
pub enum SudachiParseError {
    /// 列数が不足している行。
    TooFewColumns {
        /// 1始まりの行番号。
        line: usize,
    },
}

impl fmt::Display for SudachiParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewColumns { line } => {
                write!(f, "sudachi synonyms: too few columns at line {line}")
            }
        }
    }
}

impl std::error::Error for SudachiParseError {}

/// 1行分のパース結果。
#[derive(Debug)]
struct Row<'a> {
    group_id: &'a str,
    form_type: u8,   // col 4
    variant_type: u8, // col 6
    headword: &'a str, // col 8
}

fn parse_row(line: &str) -> Option<Row<'_>> {
    let cols: Vec<&str> = line.split(',').collect();
    if cols.len() < 9 {
        return None;
    }
    Some(Row {
        group_id: cols[0].trim(),
        form_type: cols[4].trim().parse().unwrap_or(0),
        variant_type: cols[6].trim().parse().unwrap_or(0),
        headword: cols[8].trim(),
    })
}

/// `synonyms.txt` の内容文字列をパースして `SynonymDict` を返す。
///
/// 代表表記は「(見出し)」の括弧で囲まれているケースがあるため、
/// 先頭・末尾の括弧類は剥がしてから辞書に登録する。
pub fn load_sudachi_synonyms(text: &str) -> Result<SynonymDict, SudachiParseError> {
    // グループID → (代表候補, 全見出し)
    let mut groups: HashMap<String, GroupAcc> = HashMap::new();

    for (idx, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(row) = parse_row(line) else {
            // 末尾に空行等がある場合もあるので、列不足は無視して続行する。
            // 極端に壊れている場合のみエラーにする。
            if line.matches(',').count() > 0 {
                continue;
            }
            return Err(SudachiParseError::TooFewColumns { line: idx + 1 });
        };
        if row.headword.is_empty() || row.group_id.is_empty() {
            continue;
        }
        let headword = strip_brackets(row.headword);
        if headword.is_empty() {
            continue;
        }
        let headword = headword.into_owned();

        let acc = groups
            .entry(row.group_id.to_string())
            .or_insert_with(GroupAcc::default);
        // 代表表記の判定: form_type=0 (代表語) かつ variant_type=0 (代表表記)
        if row.form_type == 0 && row.variant_type == 0 && acc.canonical.is_none() {
            acc.canonical = Some(headword.clone());
        }
        acc.headwords.push(headword);
    }

    let mut dict = SynonymDict::new();
    for (_id, acc) in groups {
        let canonical = match acc.canonical {
            Some(c) => c,
            None => {
                // フォールバック: 最初の見出しを代表とする。
                match acc.headwords.first() {
                    Some(h) => h.clone(),
                    None => continue,
                }
            }
        };
        for h in acc.headwords {
            if h != canonical {
                dict.insert(h, canonical.clone());
            }
        }
    }
    Ok(dict)
}

#[derive(Default)]
struct GroupAcc {
    canonical: Option<String>,
    headwords: Vec<String>,
}

/// 先頭末尾の `(...)` / `（...）` を剥がす。
fn strip_brackets(s: &str) -> Cow<'_, str> {
    let trimmed = s.trim();
    if let Some(inner) = trimmed.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
        return Cow::Borrowed(inner);
    }
    if let Some(inner) = trimmed
        .strip_prefix('（')
        .and_then(|s| s.strip_suffix('）'))
    {
        return Cow::Owned(inner.to_string());
    }
    Cow::Borrowed(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sudachi フォーマットの最小フィクスチャ。
    /// グループ 000001 は「パーソナルコンピュータ」を代表、
    /// 「パソコン」「PC」が別名。
    const FIXTURE: &str = "\
000001,1,0,1,0,0,0,IT,パーソナルコンピュータ,,
000001,1,0,1,2,0,0,IT,パソコン,,
000001,1,0,1,2,1,0,IT,PC,,
000002,1,0,1,0,0,0,鉄道,東日本旅客鉄道,,
000002,1,0,1,2,1,0,鉄道,JR東日本,,
000002,1,0,1,2,1,0,鉄道,JR東,,
";

    #[test]
    fn parse_fixture() {
        let dict = load_sudachi_synonyms(FIXTURE).unwrap();
        assert!(dict.len() >= 4);
        assert_eq!(
            dict.apply("パソコンを買った"),
            "パーソナルコンピュータを買った"
        );
        assert_eq!(
            dict.apply("JR東に乗る"),
            "東日本旅客鉄道に乗る"
        );
    }

    #[test]
    fn ignores_comments_and_blanks() {
        let text = "# header comment\n\n000001,1,0,1,0,0,0,x,代表,,\n000001,1,0,1,2,0,0,x,別名,,\n";
        let dict = load_sudachi_synonyms(text).unwrap();
        assert_eq!(dict.apply("別名"), "代表");
    }
}
