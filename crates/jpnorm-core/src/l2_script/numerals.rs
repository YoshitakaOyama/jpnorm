//! 漢数字 ↔ アラビア数字の変換。
//!
//! 単純な一対一置換だけでなく、位取り(十/百/千/万/億/兆)も解釈する。
//! 例: `一千二百三十四` → `1234`, `三億五千万` → `350000000`
//!
//! テキスト中の連続した漢数字列を検出し、それぞれをアラビア数字に置き換える。

/// テキスト中の漢数字列をアラビア数字に変換する。
pub fn kansuji_to_arabic(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < chars.len() {
        if is_kansuji(chars[i]) {
            // 連続する漢数字を集める
            let start = i;
            while i < chars.len() && is_kansuji(chars[i]) {
                i += 1;
            }
            let segment: String = chars[start..i].iter().collect();
            let prev = if start == 0 { None } else { Some(chars[start - 1]) };
            let next = chars.get(i).copied();
            match parse_kansuji(&segment, prev, next) {
                Some(v) => out.push_str(&v.to_string()),
                None => out.push_str(&segment),
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
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
    let has_large_unit = chars.iter().any(|&c| matches!(c, '万' | '億' | '兆' | '京'));

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
                total = total.checked_add(if cur == 0 { 1000 } else { cur * 1000 })?;
                cur = 0;
            }
            '百' => {
                total = total.checked_add(if cur == 0 { 100 } else { cur * 100 })?;
                cur = 0;
            }
            '十' => {
                total = total.checked_add(if cur == 0 { 10 } else { cur * 10 })?;
                cur = 0;
            }
            _ => {
                let d = digit_value(c)?;
                cur = cur.checked_mul(10)?.checked_add(d)?;
            }
        }
    }
    Some(total + cur)
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
}
