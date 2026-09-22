//! 元号年 → 西暦年の変換。
//!
//! `令和6年` `令和六年` `令和元年` `R6年` `H30年` を `2024年` `2018年` のように
//! 西暦に揃える。比較・名寄せ・検索で「同じ年」を同じ文字列にするための処理で、
//! 元号を持たない `年` には触らない。
//!
//! アルファベット略記 (M/T/S/H/R) は、直前が英数字でなく直後に `年` が続く
//! 場合だけ変換する (`R6年` は変換、`PR6年` や `R6` 単独は変換しない)。

const ERAS: &[(&str, char, u32, u32)] = &[
    // (元号, 略記, 元年の西暦, 最終年)
    ("明治", 'M', 1868, 45),
    ("大正", 'T', 1912, 15),
    ("昭和", 'S', 1926, 64),
    ("平成", 'H', 1989, 31),
    ("令和", 'R', 2019, 99),
];

/// 元号年を西暦年に変換する。
pub fn era_to_western(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < chars.len() {
        if let Some((year, consumed)) = match_era(&chars, i) {
            out.push_str(&year.to_string());
            out.push('年');
            i += consumed;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// `chars[i..]` が元号年で始まっていれば `(西暦年, 消費文字数)` を返す。
fn match_era(chars: &[char], i: usize) -> Option<(u32, usize)> {
    for (name, abbr, base, max) in ERAS {
        let name_chars: Vec<char> = name.chars().collect();
        let mut pos = i;
        let matched_name = chars[i..].starts_with(&name_chars);
        if matched_name {
            pos += name_chars.len();
        } else {
            // 略記: 直前が英数字なら別の語の一部とみなす
            let prev_is_alnum = i > 0 && chars[i - 1].is_ascii_alphanumeric();
            if chars[i] != *abbr || prev_is_alnum {
                continue;
            }
            pos += 1;
        }
        let (n, after) = parse_year_number(chars, pos, matched_name)?;
        if chars.get(after) != Some(&'年') {
            continue;
        }
        if n == 0 || n > *max {
            continue;
        }
        return Some((base + n - 1, after + 1 - i));
    }
    None
}

/// `元` / ASCII 数字 1〜2 桁 / 漢数字 (十の位まで) を読む。
fn parse_year_number(chars: &[char], pos: usize, allow_kansuji: bool) -> Option<(u32, usize)> {
    if allow_kansuji && chars.get(pos) == Some(&'元') {
        return Some((1, pos + 1));
    }
    // ASCII 数字
    let mut end = pos;
    while end < chars.len() && chars[end].is_ascii_digit() && end - pos < 2 {
        end += 1;
    }
    if end > pos {
        // 3 桁以上の数字は年号としては不正 (R123年)
        if chars.get(end).is_some_and(|c| c.is_ascii_digit()) {
            return None;
        }
        let s: String = chars[pos..end].iter().collect();
        return s.parse().ok().map(|n| (n, end));
    }
    if !allow_kansuji {
        return None;
    }
    // 漢数字: 一〜九、十、十一〜十九、二十〜九十九
    let digit = |c: char| {
        "一二三四五六七八九"
            .chars()
            .position(|d| d == c)
            .map(|p| p as u32 + 1)
    };
    let mut n;
    let mut end = pos;
    if let Some(d) = chars.get(end).and_then(|&c| digit(c)) {
        if chars.get(end + 1) == Some(&'十') {
            n = d * 10;
            end += 2;
        } else {
            return Some((d, end + 1));
        }
    } else if chars.get(end) == Some(&'十') {
        n = 10;
        end += 1;
    } else {
        return None;
    }
    if let Some(d) = chars.get(end).and_then(|&c| digit(c)) {
        n += d;
        end += 1;
    }
    Some((n, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_names() {
        assert_eq!(era_to_western("令和6年3月"), "2024年3月");
        assert_eq!(era_to_western("令和元年"), "2019年");
        assert_eq!(era_to_western("平成30年度"), "2018年度");
        assert_eq!(era_to_western("昭和64年と平成元年"), "1989年と1989年");
        assert_eq!(era_to_western("明治45年"), "1912年");
    }

    #[test]
    fn kansuji_years() {
        assert_eq!(era_to_western("令和六年"), "2024年");
        assert_eq!(era_to_western("昭和二十三年"), "1948年");
        assert_eq!(era_to_western("平成十年"), "1998年");
        assert_eq!(era_to_western("大正十五年"), "1926年");
    }

    #[test]
    fn abbreviations() {
        assert_eq!(era_to_western("R6年"), "2024年");
        assert_eq!(era_to_western("H30年度"), "2018年度");
        assert_eq!(era_to_western("S60年"), "1985年");
        assert_eq!(era_to_western("PR6年"), "PR6年");
        assert_eq!(era_to_western("R6"), "R6");
        assert_eq!(era_to_western("R元年"), "R元年");
    }

    #[test]
    fn out_of_range_and_non_era_kept() {
        assert_eq!(era_to_western("令和0年"), "令和0年");
        assert_eq!(era_to_western("平成99年"), "平成99年");
        assert_eq!(era_to_western("2024年"), "2024年");
        assert_eq!(era_to_western("令和の時代"), "令和の時代");
        assert_eq!(era_to_western("R123年"), "R123年");
    }
}
