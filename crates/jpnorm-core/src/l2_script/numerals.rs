//! 漢数字 ↔ アラビア数字の変換。
//!
//! 単純な一対一置換だけでなく、位取り(十/百/千/万/億/兆)も解釈する。
//! 例: `一千二百三十四` → `1234`, `三億五千万` → `350000000`
//!
//! テキスト中の連続した漢数字列を検出し、それぞれをアラビア数字に置き換える。

/// テキスト中の漢数字列をアラビア数字に変換する。
///
/// 純粋な漢数字 (`一千二百三十四`) に加えて、アラビア数字と位取り漢字が混在する
/// 表現 (`1万2千`, `1.5億`, `12万3456`, `1,200万`, `3千円`) も 1 つの数値として解釈する。
pub fn kansuji_to_arabic(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if !(is_kansuji(c) || c.is_ascii_digit()) {
            out.push(c);
            i += 1;
            continue;
        }
        let start = i;
        let end = scan_numeric_run(&chars, start);
        let run = &chars[start..end];
        let has_kansuji = run.iter().any(|&c| is_kansuji(c));
        let has_arabic = run.iter().any(|c| c.is_ascii_digit());
        let replaced = if !has_kansuji {
            None
        } else if !has_arabic {
            let segment: String = run.iter().collect();
            let prev = if start == 0 {
                None
            } else {
                Some(chars[start - 1])
            };
            let next = chars.get(end).copied();
            parse_kansuji(&segment, prev, next)
        } else {
            parse_mixed(run)
        };
        match replaced {
            Some(v) => out.push_str(&v.to_string()),
            None => out.extend(run),
        }
        i = end;
    }
    out
}

/// `start` から、漢数字・ASCII 数字・数字に挟まれた `.`・桁区切りとして妥当な `,`
/// の連続を読み、その終端を返す。
fn scan_numeric_run(chars: &[char], start: usize) -> usize {
    let mut i = start;
    while i < chars.len() {
        let c = chars[i];
        if is_kansuji(c) || c.is_ascii_digit() {
            i += 1;
            continue;
        }
        let prev_digit = i > 0 && chars[i - 1].is_ascii_digit();
        if c == '.' && prev_digit && chars.get(i + 1).is_some_and(|n| n.is_ascii_digit()) {
            i += 1;
            continue;
        }
        // 桁区切り: 直前が数字で、直後にちょうど 3 桁の数字が続き、その後に数字が続かない
        if c == ','
            && prev_digit
            && chars[i + 1..]
                .iter()
                .take(3)
                .filter(|n| n.is_ascii_digit())
                .count()
                == 3
            && !chars.get(i + 4).is_some_and(|n| n.is_ascii_digit())
        {
            i += 1;
            continue;
        }
        break;
    }
    i
}

/// アラビア数字と位取り漢字が混在した数値を解釈する。
///
/// 万/億/兆/京 で区切った各ブロックを、アラビア数字 (小数・桁区切り可) または
/// 十/百/千 を含む漢数字として解釈し、倍率を掛けて合算する。小数は倍率を掛けた
/// 結果が整数になる場合だけ受け付ける (`1.5億` は可、`1.23456万` は不可)。
fn parse_mixed(run: &[char]) -> Option<u128> {
    let mut total: u128 = 0;
    let mut block: Vec<char> = Vec::new();
    let mut saw_unit = false;
    for &c in run {
        let mult = match c {
            '京' => Some(10u128.pow(16)),
            '兆' => Some(10u128.pow(12)),
            '億' => Some(10u128.pow(8)),
            '万' => Some(10u128.pow(4)),
            _ => None,
        };
        if let Some(mult) = mult {
            saw_unit = true;
            let value = if block.is_empty() {
                1
            } else {
                parse_mixed_block(&block, mult)?
            };
            total = total.checked_add(value)?;
            block.clear();
        } else {
            block.push(c);
        }
    }
    if !block.is_empty() {
        if block.iter().any(|c| matches!(c, '十' | '百' | '千')) {
            saw_unit = true;
        }
        total = total.checked_add(parse_mixed_block(&block, 1)?)?;
    }
    // 位取り漢字を含まない混在 (例: 1一) は数値とみなさない
    if !saw_unit {
        return None;
    }
    Some(total)
}

/// 万/億/兆/京 より下のブロックを解釈し、`mult` を掛けた値を返す。
fn parse_mixed_block(block: &[char], mult: u128) -> Option<u128> {
    let s: String = block.iter().filter(|c| **c != ',').collect();
    if let Some((int_part, frac_part)) = s.split_once('.') {
        // 小数はブロック全体がアラビア数字のときだけ
        if !int_part.chars().all(|c| c.is_ascii_digit())
            || !frac_part.chars().all(|c| c.is_ascii_digit())
            || int_part.is_empty()
            || frac_part.is_empty()
        {
            return None;
        }
        let scale = 10u128.checked_pow(frac_part.len() as u32)?;
        let digits: u128 = format!("{int_part}{frac_part}").parse().ok()?;
        let scaled = digits.checked_mul(mult)?;
        if scaled % scale != 0 {
            return None;
        }
        return Some(scaled / scale);
    }
    // 十/百/千 と数字 (アラビア or 漢数字) の組み合わせ
    let mut total: u128 = 0;
    let mut cur: u128 = 0;
    let mut cur_set = false;
    for c in s.chars() {
        match c {
            '千' | '百' | '十' => {
                let unit = match c {
                    '千' => 1000,
                    '百' => 100,
                    _ => 10,
                };
                total = total.checked_add(if cur_set {
                    cur.checked_mul(unit)?
                } else {
                    unit
                })?;
                cur = 0;
                cur_set = false;
            }
            _ => {
                let d = if c.is_ascii_digit() {
                    u128::from(c as u8 - b'0')
                } else {
                    digit_value(c)?
                };
                cur = cur.checked_mul(10)?.checked_add(d)?;
                cur_set = true;
            }
        }
    }
    total.checked_add(cur)?.checked_mul(mult)
}

/// アラビア数字を漢数字に変換する(位取りあり、4桁区切り)。
///
/// 例: `1234` → `一千二百三十四`, `0` → `〇`
pub fn arabic_to_kansuji(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let num: String = chars[start..i].iter().collect();
            if let Ok(n) = num.parse::<u128>() {
                out.push_str(&format_kansuji(n));
            } else {
                out.push_str(&num);
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn is_kansuji(c: char) -> bool {
    matches!(
        c,
        '〇' | '零'
            | '一'
            | '二'
            | '三'
            | '四'
            | '五'
            | '六'
            | '七'
            | '八'
            | '九'
            | '十'
            | '百'
            | '千'
            | '万'
            | '億'
            | '兆'
            | '京'
    )
}

fn digit_value(c: char) -> Option<u128> {
    Some(match c {
        '〇' | '零' => 0,
        '一' => 1,
        '二' => 2,
        '三' => 3,
        '四' => 4,
        '五' => 5,
        '六' => 6,
        '七' => 7,
        '八' => 8,
        '九' => 9,
        _ => return None,
    })
}

/// 漢数字文字列を u128 に解釈する。位取り表現と、単純な桁の並び(例: `一二三` → 123)の
/// 両方を受け付ける。
fn parse_kansuji(s: &str, prev: Option<char>, next: Option<char>) -> Option<u128> {
    let chars: Vec<char> = s.chars().collect();
    let has_digit = chars.iter().any(|&c| digit_value(c).is_some());
    let has_small_unit = chars.iter().any(|&c| matches!(c, '十' | '百' | '千'));
    let has_large_unit = chars
        .iter()
        .any(|&c| matches!(c, '万' | '億' | '兆' | '京'));

    // 「京」「兆」などが固有名詞(京都/東京/兆し等)として現れるケースを壊さないため、
    // 数字シグナル(各位の漢数字 or 十/百/千)が無い語は変換しない。
    if !has_digit && !has_small_unit {
        return None;
    }
    // 単独の 万/億/兆/京 も変換対象外。
    if chars.len() == 1 && has_large_unit {
        return None;
    }

    // 単独の 1 文字漢数字(一/二/... 九/〇) は、前後文脈に数字シグナルがない限り
    // 変換しない。固有語(統一/唯一/一緒 など)を壊さないための保守判定。
    // 変換を許すコンテキスト:
    //   - 直前が '第' (第一章→第1章)
    //   - 直前/直後が ASCII 数字 or 別の漢数字単位 (1,一,二 のような混在)
    //   - 直後がカウンタ的な文字 (章/回/位/番/月/日/年/人/個/枚/つ)
    if chars.len() == 1 && !has_large_unit && !has_small_unit {
        let counter_like = |c: char| {
            matches!(
                c,
                '章' | '回' | '位' | '番' | '月' | '日' | '年' | '人' | '個' | '枚' | 'つ' | '度'
            )
        };
        let context_ok = match (prev, next) {
            (Some('第'), _) => true,
            (_, Some(n)) if counter_like(n) => true,
            (Some(p), _) if p.is_ascii_digit() => true,
            (_, Some(n)) if n.is_ascii_digit() => true,
            _ => false,
        };
        if !context_ok {
            return None;
        }
    }

    if !has_large_unit && !has_small_unit {
        let mut n: u128 = 0;
        for c in chars {
            let d = digit_value(c)?;
            n = n.checked_mul(10)?.checked_add(d)?;
        }
        return Some(n);
    }

    // 位取り解釈: 京 > 兆 > 億 > 万 > (千百十).
    parse_with_large_units(s)
}

/// 京・兆・億・万 を含む位取り漢数字を解釈する。
fn parse_with_large_units(s: &str) -> Option<u128> {
    let chars: Vec<char> = s.chars().collect();
    let mut total: u128 = 0;
    let mut buf: Vec<char> = Vec::new();

    // 実装方針: 左から順にスキャンし、大きい単位(万/億/兆/京)に当たったら
    // それまでの `buf` を4桁以下として解釈し、対応する倍率をかけて total に足す。
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let big = match c {
            '京' => Some(10u128.pow(16)),
            '兆' => Some(10u128.pow(12)),
            '億' => Some(10u128.pow(8)),
            '万' => Some(10u128.pow(4)),
            _ => None,
        };
        if let Some(mult) = big {
            let block: String = buf.iter().collect();
            let value = if block.is_empty() {
                1
            } else {
                parse_small(&block)?
            };
            total = total.checked_add(value.checked_mul(mult)?)?;
            buf.clear();
            i += 1;
            continue;
        }
        buf.push(c);
        i += 1;
    }
    if !buf.is_empty() {
        let block: String = buf.iter().collect();
        total = total.checked_add(parse_small(&block)?)?;
    }
    Some(total)
}

/// 千百十を含む 4桁以下の漢数字を解釈する。
fn parse_small(s: &str) -> Option<u128> {
    let mut total: u128 = 0;
    let mut cur: u128 = 0;
    for c in s.chars() {
        match c {
            '千' => {
                total = total.checked_add(if cur == 0 {
                    1000
                } else {
                    cur.checked_mul(1000)?
                })?;
                cur = 0;
            }
            '百' => {
                total = total.checked_add(if cur == 0 { 100 } else { cur.checked_mul(100)? })?;
                cur = 0;
            }
            '十' => {
                total = total.checked_add(if cur == 0 { 10 } else { cur.checked_mul(10)? })?;
                cur = 0;
            }
            _ => {
                let d = digit_value(c)?;
                cur = cur.checked_mul(10)?.checked_add(d)?;
            }
        }
    }
    total.checked_add(cur)
}

fn format_kansuji(mut n: u128) -> String {
    if n == 0 {
        return "〇".to_string();
    }
    let units: [(u128, char); 5] = [
        (10u128.pow(16), '京'),
        (10u128.pow(12), '兆'),
        (10u128.pow(8), '億'),
        (10u128.pow(4), '万'),
        (1, '\0'),
    ];
    let mut out = String::new();
    for (mult, marker) in units {
        let block = n / mult;
        if block > 0 {
            out.push_str(&format_small(block));
            if marker != '\0' {
                out.push(marker);
            }
            n %= mult;
        }
    }
    out
}

fn format_small(n: u128) -> String {
    debug_assert!(n < 10_000);
    let digits = ['〇', '一', '二', '三', '四', '五', '六', '七', '八', '九'];
    let mut out = String::new();
    let thousands = n / 1000;
    let hundreds = (n % 1000) / 100;
    let tens = (n % 100) / 10;
    let ones = n % 10;
    if thousands > 0 {
        if thousands > 1 {
            out.push(digits[thousands as usize]);
        }
        out.push('千');
    }
    if hundreds > 0 {
        if hundreds > 1 {
            out.push(digits[hundreds as usize]);
        }
        out.push('百');
    }
    if tens > 0 {
        if tens > 1 {
            out.push(digits[tens as usize]);
        }
        out.push('十');
    }
    if ones > 0 {
        out.push(digits[ones as usize]);
    }
    out
}

// `units` は static 定義にすると [(u128, char); 5] が `Copy` なので参照不要。
// 上のコードで `&(mult, marker)` 分解のために参照表記を使っているが、
// 実際には値コピーで十分。コンパイルエラー回避のため配列を直接回す実装に下で差し替え済み。

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_sequence() {
        assert_eq!(kansuji_to_arabic("一二三"), "123");
    }

    #[test]
    fn with_units() {
        assert_eq!(kansuji_to_arabic("十"), "10");
        assert_eq!(kansuji_to_arabic("二十"), "20");
        assert_eq!(kansuji_to_arabic("一千二百三十四"), "1234");
        assert_eq!(kansuji_to_arabic("三億五千万"), "350000000");
        assert_eq!(kansuji_to_arabic("一兆"), "1000000000000");
    }

    #[test]
    fn embedded_in_text() {
        assert_eq!(kansuji_to_arabic("第一章と第二節"), "第1章と第2節");
    }

    #[test]
    fn arabic_to_kan_basic() {
        assert_eq!(arabic_to_kansuji("1234"), "千二百三十四");
        assert_eq!(arabic_to_kansuji("350000000"), "三億五千万");
        assert_eq!(arabic_to_kansuji("0"), "〇");
    }

    #[test]
    fn does_not_convert_common_words_containing_large_units() {
        assert_eq!(kansuji_to_arabic("京都"), "京都");
        assert_eq!(kansuji_to_arabic("東京"), "東京");
        assert_eq!(kansuji_to_arabic("兆し"), "兆し");
        assert_eq!(kansuji_to_arabic("京王線"), "京王線");
    }

    #[test]
    fn roundtrip_small() {
        // n=1 は format_kansuji → "一" (1文字) となり、context-aware ルールで
        // スタンドアロン変換されないため roundtrip 対象から外す。
        for n in [10u128, 99, 100, 1234, 9999, 10000, 350000000] {
            let k = format_kansuji(n);
            let back = parse_kansuji(&k, None, None).unwrap();
            assert_eq!(back, n, "roundtrip {n} via {k}");
        }
    }

    #[test]
    fn mixed_arabic_and_kanji_units() {
        assert_eq!(kansuji_to_arabic("1万2千"), "12000");
        assert_eq!(kansuji_to_arabic("3千円"), "3000円");
        assert_eq!(kansuji_to_arabic("2千5百"), "2500");
        assert_eq!(kansuji_to_arabic("12万3456人"), "123456人");
        assert_eq!(kansuji_to_arabic("1.5億"), "150000000");
        assert_eq!(kansuji_to_arabic("3.25万"), "32500");
        assert_eq!(kansuji_to_arabic("1,200万円"), "12000000円");
        assert_eq!(kansuji_to_arabic("100万"), "1000000");
        assert_eq!(kansuji_to_arabic("1億2000万"), "120000000");
    }

    #[test]
    fn overflow_leaves_input_untouched() {
        // u128 を超える桁数は変換せずそのまま (release で wrap して別の数値になるのを防ぐ)。
        let digits: String = "一二三四五六七八九".chars().cycle().take(37).collect();
        let s = format!("{digits}千");
        assert_eq!(kansuji_to_arabic(&s), s);
        let s = format!("{digits}京");
        assert_eq!(kansuji_to_arabic(&s), s);
    }

    #[test]
    fn mixed_edge_cases_untouched() {
        // 小数が整数に落ちないものはそのまま
        assert_eq!(kansuji_to_arabic("1.23456万"), "1.23456万");
        // 位取り漢字を含まない混在は数値扱いしない
        assert_eq!(kansuji_to_arabic("第1一"), "第1一");
        // 純粋なアラビア数字は触らない (canonicalize_numbers の担当)
        assert_eq!(
            kansuji_to_arabic("2024年3月 1,200円 3.14"),
            "2024年3月 1,200円 3.14"
        );
        // 桁区切りとして不正な , はトークンを切る
        assert_eq!(kansuji_to_arabic("1,20万"), "1,200000");
    }
}
