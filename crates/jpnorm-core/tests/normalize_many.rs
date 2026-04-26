//! 100+ ケースの正規化スナップショットテスト。
//!
//! 入力リストは `tests/normalize_cases_data.rs` (const `CASES`) と共有。
//! 期待値は現在の実装の出力を一度ダンプして確定したもの。
//! 挙動を変える PR では、まず `cargo run --example dump_normalize_cases -p jpnorm-core`
//! を再実行して差分を確認し、意図した変更であればここを更新する。

use jpnorm_core::test_fixtures::NORMALIZE_CASES as CASES;
use jpnorm_core::{Normalizer, Preset};

/// 期待値。`CASES` と同じ順序・同じ長さ。
const EXPECTED: &[&str] = &[
    // None
    "hello world",
    "日本語のテキスト",
    "   前後空白   ",
    "ＡＢＣ１２３",
    "ｶﾀｶﾅ",
    // NeologdnCompat (基本)
    "hello world",
    "ABC123",
    "カタカナ",
    "ハンカク カナ",
    "前後空白",
    "連続 空白",
    "日本語 と 英語",
    "Python と Rust",
    "ー長音ー",
    "〜チルダ〜",
    "a-b-c-d-e-f",
    "!!!!!!???",
    "100%オフ",
    "改行\nと\nCR",
    "ゼロ幅削除",
    "BOMつき",
    "制御文字",
    "Bidi攻撃",
    "タブ は残る",
    "改行\nは残る",
    "",
    "a",
    "abcABC",
    "12345",
    "(株)髙キロ1",
    "〜",
    "ー",
    "あーい",
    "スーパー〜",
    "“hello”",
    "‘world’",
    "あ い う",
    "ABC ABC",
    "3.14",
    "カレーライス",
    "ガギグゲゴ",
    "パピプペポ",
    // ForSearch
    "hello world",
    "ABC123",
    "カタカナ テスト",
    "(株)高キロ",
    "絵文字テスト",
    "http://example.com/path を参照",
    "mail@example.com まで",
    "@user さん",
    "#タグ付き",
    "\"quotes\" 'apos'",
    "連続ー長音",
    "連続!!",
    "連続ああ",
    "改行\n混在\nCR",
    "トリム",
    // ForDisplay
    "カタカナ",
    "ＡＢＣ",
    "前後",
    "a-b‐c",
    "ーーー",
    "連続   空白",
    "日本語 text",
    "絵文字😀残る",
    "㈱そのまま",
    "“quotes”",
    // ForCompare
    "hello world",
    "ABC123",
    "カタカナ テスト",
    "1234",
    "350000000",
    "第1章",
    "2025年",
    "1000000000000円",
    // 京/兆/東京 等の固有名詞は変換しない
    "京都",
    "東京",
    "兆し",
    "京王線",
    "万歳",
    "億万長者",
    "こんにちは世界",
    "A B C",
    "これはテストです",
    "株テスト",
    "あいう",
    "前後",
    "日本語 と 英語 の 混在",
    "スーパー",
    "",
    "abcdef",
    "ABC ABC",
    "あ",
    "わーい",
    "",
    "株高キロ",
    "123",
    "平成昭和大正",
    "ハンカクカナ",
    "ガギグ",
    "httpexamplecom",
    "mailexamplecom",
    "abcdef",
    "",
    "a",
    "速報東京で第1位",
    "2025年1月1日新年あけましておめでとう",
    "価格 ¥1200税込", // 1,200 → 1200 (canonicalize_numbers)
    "Python と Rust で実装",
    "グッド 絵文字 混じり",
    "ハイフンの統一", // 単独の 一 は固有語扱いで変換しない
    "連続 空白の 畳み込み",
    // NeologdnCompat 追加境界
    "abc def",
    "タブ 残す",
    "全角 スペース",
    "¥1,200",
    "𩸽",
    "😀",
    "〒100-0001",
    "TEL:03-1234-5678",
    "1+1=2",
    "(株)テスト",
    // ForSearch 追加
    "平成30年",
    "I章とII章",
    "ABCABC",
    "CO2排出",
    "H2O",
    // ForCompare 追加
    "東京と京都は違う都市です",
    "202511", // 2025/01/01 → 各数値トークンが正規化 (01→1) された後、記号除去で連結
    "一二三", // 単独の 一/二/三 は固有語扱いで変換しない
    "第1回から第10回まで",
];

#[test]
fn numeric_equivalence_for_compare() {
    // 1,200 / 1200 / 1200.00 / 一千二百 は比較用で全て同じ表現に畳める。
    let n = Normalizer::preset(Preset::ForCompare);
    let canonical = n.normalize("1200");
    assert_eq!(canonical, "1200");
    assert_eq!(n.normalize("1,200"), canonical);
    assert_eq!(n.normalize("1200.00"), canonical);
    assert_eq!(n.normalize("01200"), canonical);
    assert_eq!(n.normalize("一千二百"), canonical);

    // 小数もゼロ末尾が揃う。
    let pi = n.normalize("3.14");
    assert_eq!(n.normalize("3.1400"), pi);
    assert_eq!(n.normalize("03.14"), pi);
}

#[test]
fn cases_and_expected_lengths_match() {
    assert_eq!(
        CASES.len(),
        EXPECTED.len(),
        "CASES と EXPECTED の要素数を揃えてください (CASES={}, EXPECTED={})",
        CASES.len(),
        EXPECTED.len()
    );
    assert!(
        CASES.len() >= 100,
        "テストケースは 100 件以上を維持すること (現在 {})",
        CASES.len()
    );
}

#[test]
fn normalize_snapshot() {
    let mut failures: Vec<String> = Vec::new();
    for (i, ((preset, input), expected)) in CASES.iter().zip(EXPECTED.iter()).enumerate() {
        let n = Normalizer::preset(*preset);
        let actual = n.normalize(input);
        if actual != *expected {
            failures.push(format!(
                "[{i}] preset={preset:?}\n  input   = {input:?}\n  expected= {expected:?}\n  actual  = {actual:?}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} / {} ケース失敗:\n{}",
        failures.len(),
        CASES.len(),
        failures.join("\n")
    );
}
