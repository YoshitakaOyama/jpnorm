//! neologdn とのゴールデン比較テスト。
//!
//! `tests/golden/neologdn.jsonl`(リポジトリルート直下)を読み込み、
//! 各 `input` に対して `Normalizer::preset(NeologdnCompat)` の出力が
//! neologdn の `expected` と一致するかを確認する。
//!
//! 一致しなくても良い「意図的な差分」は `INTENTIONAL_DIFFS` に列挙する。
//! それ以外の差分が出たら失敗する。
//!
//! ゴールデン生成: `scripts/gen-neologdn-golden.py`

use jpnorm_core::{Normalizer, Preset};
use std::path::PathBuf;

/// jpnorm が neologdn と**意図的に異なる**入力一覧。
/// 各エントリには「なぜ違うか」をコメントで残す。
const INTENTIONAL_DIFFS: &[&str] = &[
    // neologdn は `~`/`～` を削除、jpnorm は `〜` に統一する(記号を保存する方針)
    "a~b～c",
    "ｵｰﾙｲﾝﾜﾝ ＡＢＣ 〜〜〜 ！！！",
    // 末尾のチルダ類 〜〜〜 を残す jpnorm vs 削除する neologdn
    "これは ｶﾀｶﾅ と 漢字 の 混在 テスト です〜〜〜",
    // neologdn は `—` (U+2014 EM DASH) を `ー` 扱い、jpnorm は `-` 扱い
    "ab‐cd—ef−gh",
    // neologdn は日本語/ASCII 間の空白も削除するが、
    // jpnorm はユーザーが意図的に入れた空白を保存する(連続空白は1つに畳むだけ)
    "日本語 の 文章",
    "日本語 text 混在",
    "ﾊﾝｶｸｶﾅ\u{3000}と  全角  ！！ーーー",
    "Python と Rust",
    "URLは http://example.com/path を参照",
    // neologdn は `‘` を ``` ` ``` (backtick) に変換する独自挙動。
    // jpnorm は引用符統一を preset off にしているので触らない。
    "“hello” ‘world’",
    // neologdn は `¥` (U+00A5) を `\` にする(半角記号扱い)。jpnorm はそのまま。
    "価格 ¥1,200 (税込)",
];

#[derive(Debug)]
struct Case {
    input: String,
    expected: String,
}

/// jsonl を serde 無しで読む簡易パーサ。
/// 各行は `{"input": "...", "expected": "..."}` を仮定する。
fn load_golden() -> Vec<Case> {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // crates/jpnorm-core → リポジトリルート
    path.pop();
    path.pop();
    path.push("tests");
    path.push("golden");
    path.push("neologdn.jsonl");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("golden not found at {path:?}: {e}"));
    let mut cases = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let input = extract_field(line, "input").expect("input field");
        let expected = extract_field(line, "expected").expect("expected field");
        cases.push(Case { input, expected });
    }
    cases
}

/// 素朴な JSON 文字列フィールド抽出。`"<key>": "<value>"` を見つけ、
/// 値の `\"` `\\` `\n` `\t` をデコードする。
fn extract_field(line: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":");
    let k = line.find(&needle)?;
    let after = &line[k + needle.len()..];
    let q = after.find('"')?;
    let bytes = &after[q + 1..];
    let mut out = String::new();
    let mut chars = bytes.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                'n' => out.push('\n'),
                't' => out.push('\t'),
                '/' => out.push('/'),
                'u' => {
                    // \uXXXX
                    let mut hex = String::new();
                    for _ in 0..4 {
                        hex.push(chars.next()?);
                    }
                    let cp = u32::from_str_radix(&hex, 16).ok()?;
                    if let Some(ch) = char::from_u32(cp) {
                        out.push(ch);
                    }
                }
                other => out.push(other),
            },
            _ => out.push(c),
        }
    }
    None
}

#[test]
fn compat_with_neologdn() {
    let cases = load_golden();
    assert!(!cases.is_empty(), "golden is empty");

    let n = Normalizer::preset(Preset::NeologdnCompat);
    let mut unexpected_diffs: Vec<(String, String, String)> = Vec::new();
    let mut matched = 0usize;
    let mut intentional = 0usize;

    for case in &cases {
        let got = n.normalize(&case.input);
        if got == case.expected {
            matched += 1;
        } else if INTENTIONAL_DIFFS.contains(&case.input.as_str()) {
            intentional += 1;
        } else {
            unexpected_diffs.push((case.input.clone(), case.expected.clone(), got));
        }
    }

    eprintln!(
        "compat summary: matched={matched} intentional_diffs={intentional} \
         unexpected={} total={}",
        unexpected_diffs.len(),
        cases.len()
    );

    if !unexpected_diffs.is_empty() {
        for (input, expected, got) in &unexpected_diffs {
            eprintln!(
                "DIFF:\n  input    = {input:?}\n  neologdn = {expected:?}\n  jpnorm   = {got:?}"
            );
        }
        panic!(
            "{} unexpected diffs vs neologdn. Either fix jpnorm or add to INTENTIONAL_DIFFS with rationale.",
            unexpected_diffs.len()
        );
    }
}
