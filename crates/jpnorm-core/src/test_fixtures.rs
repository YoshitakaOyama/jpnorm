//! テスト用の共有フィクスチャ。
//!
//! 本モジュールは「テスト支援 API」であり、製品コードから直接呼ぶことは
//! 想定していない。lib / 統合テスト / ベンチ / 下流クレートのどこからでも
//! `jpnorm_core::test_fixtures::NORMALIZE_CASES` として参照できる。
//!
//! 順序を変更するとスナップショットテストの期待値とずれるので注意。

use crate::Preset;

/// 正規化テストで使う共通入力ケース (131件, 全プリセット網羅)。
///
/// 利用例:
/// ```
/// use jpnorm_core::{Normalizer, test_fixtures::NORMALIZE_CASES};
/// for (preset, input) in NORMALIZE_CASES {
///     let _ = Normalizer::preset(*preset).normalize(input);
/// }
/// ```
pub const NORMALIZE_CASES: &[(Preset, &str)] = &[
    // ===== 1. None: 恒等変換 =====
    (Preset::None, "hello world"),
    (Preset::None, "日本語のテキスト"),
    (Preset::None, "   前後空白   "),
    (Preset::None, "ＡＢＣ１２３"),
    (Preset::None, "ｶﾀｶﾅ"),

    // ===== 2. NeologdnCompat: 基本 =====
    (Preset::NeologdnCompat, "hello world"),
    (Preset::NeologdnCompat, "ＡＢＣ１２３"),
    (Preset::NeologdnCompat, "ｶﾀｶﾅ"),
    (Preset::NeologdnCompat, "ﾊﾝｶｸ ｶﾅ"),
    (Preset::NeologdnCompat, "  前後空白  "),
    (Preset::NeologdnCompat, "連続   空白"),
    (Preset::NeologdnCompat, "日本語 と 英語"),
    (Preset::NeologdnCompat, "Python と Rust"),
    (Preset::NeologdnCompat, "ーーーー長音ーーー"),
    (Preset::NeologdnCompat, "〜〜〜チルダ〜〜〜"),
    (Preset::NeologdnCompat, "a-b‐c−d―e—f"),
    (Preset::NeologdnCompat, "!!!！！！？？？"),
    (Preset::NeologdnCompat, "100%オフ"),
    (Preset::NeologdnCompat, "改行\r\nと\rCR"),
    (Preset::NeologdnCompat, "ゼロ幅\u{200B}削除"),
    (Preset::NeologdnCompat, "BOM\u{FEFF}つき"),
    (Preset::NeologdnCompat, "制御\u{0001}文字"),
    (Preset::NeologdnCompat, "Bidi\u{202E}攻撃"),
    (Preset::NeologdnCompat, "タブ\tは残る"),
    (Preset::NeologdnCompat, "改行\nは残る"),
    (Preset::NeologdnCompat, ""),
    (Preset::NeologdnCompat, "a"),
    (Preset::NeologdnCompat, "ａｂｃＡＢＣ"),
    (Preset::NeologdnCompat, "１２３４５"),
    (Preset::NeologdnCompat, "㈱髙㌔①"),
    (Preset::NeologdnCompat, "〜"),
    (Preset::NeologdnCompat, "ーー"),
    (Preset::NeologdnCompat, "あーーーい"),
    (Preset::NeologdnCompat, "スーパー〜"),
    (Preset::NeologdnCompat, "“hello”"),
    (Preset::NeologdnCompat, "‘world’"),
    (Preset::NeologdnCompat, "あ  い  う"),
    (Preset::NeologdnCompat, "ＡＢＣ ＡＢＣ"),
    (Preset::NeologdnCompat, "3.14"),
    (Preset::NeologdnCompat, "カレーライス"),
    (Preset::NeologdnCompat, "ｶﾞｷﾞｸﾞｹﾞｺﾞ"),
    (Preset::NeologdnCompat, "ﾊﾟﾋﾟﾌﾟﾍﾟﾎﾟ"),

    // ===== 3. ForSearch =====
    (Preset::ForSearch, "hello world"),
    (Preset::ForSearch, "ＡＢＣ１２３"),
    (Preset::ForSearch, "ｶﾀｶﾅ テスト"),
    (Preset::ForSearch, "㈱髙㌔"),
    (Preset::ForSearch, "絵文字😀テスト"),
    (Preset::ForSearch, "http://example.com/path を参照"),
    (Preset::ForSearch, "mail@example.com まで"),
    (Preset::ForSearch, "@user さん"),
    (Preset::ForSearch, "#タグ付き"),
    (Preset::ForSearch, "“quotes” ‘apos’"),
    (Preset::ForSearch, "連続ーーーー長音"),
    (Preset::ForSearch, "連続!!!!!!"),
    (Preset::ForSearch, "連続あああああ"),
    (Preset::ForSearch, "改行\r\n混在\rCR"),
    (Preset::ForSearch, "  トリム  "),

    // ===== 4. ForDisplay =====
    (Preset::ForDisplay, "ｶﾀｶﾅ"),
    (Preset::ForDisplay, "ＡＢＣ"),
    (Preset::ForDisplay, "  前後  "),
    (Preset::ForDisplay, "a-b‐c"),
    (Preset::ForDisplay, "ーーー"),
    (Preset::ForDisplay, "連続   空白"),
    (Preset::ForDisplay, "日本語 text"),
    (Preset::ForDisplay, "絵文字😀残る"),
    (Preset::ForDisplay, "㈱そのまま"),
    (Preset::ForDisplay, "“quotes”"),

    // ===== 5. ForCompare: 最積極 =====
    (Preset::ForCompare, "hello world"),
    (Preset::ForCompare, "ＡＢＣ１２３"),
    (Preset::ForCompare, "ｶﾀｶﾅ テスト"),
    (Preset::ForCompare, "一千二百三十四"),
    (Preset::ForCompare, "三億五千万"),
    (Preset::ForCompare, "第一章"),
    (Preset::ForCompare, "二〇二五年"),
    (Preset::ForCompare, "一兆円"),
    // High 修正の回帰
    (Preset::ForCompare, "京都"),
    (Preset::ForCompare, "東京"),
    (Preset::ForCompare, "兆し"),
    (Preset::ForCompare, "京王線"),
    (Preset::ForCompare, "万歳"),
    (Preset::ForCompare, "億万長者"),
    // 記号除去
    (Preset::ForCompare, "こんにちは、世界！"),
    (Preset::ForCompare, "A, B, C."),
    (Preset::ForCompare, "これは「テスト」です"),
    (Preset::ForCompare, "（株）テスト"),
    // 絵文字削除
    (Preset::ForCompare, "😀あいう😃"),
    // 空白畳み込み
    (Preset::ForCompare, "  前後  "),
    (Preset::ForCompare, "日本語 と 英語 の 混在"),
    // 長音・チルダ・ハイフン統一
    (Preset::ForCompare, "スーパーーーー"),
    (Preset::ForCompare, "〜〜〜"),
    (Preset::ForCompare, "a-b‐c−d―e—f"),
    (Preset::ForCompare, "ＡＢＣ ＡＢＣ"),
    // 繰り返し畳み込み (repeat_limit = 1 → 同一文字1回のみ)
    (Preset::ForCompare, "あああ"),
    (Preset::ForCompare, "わーーい"),
    (Preset::ForCompare, "!!!"),
    // 互換文字
    (Preset::ForCompare, "㈱髙㌔"),
    (Preset::ForCompare, "①②③"),
    (Preset::ForCompare, "㍻㍼㍽"),
    // 半角カナ
    (Preset::ForCompare, "ﾊﾝｶｸｶﾅ"),
    (Preset::ForCompare, "ｶﾞｷﾞｸﾞ"),
    // URL/email は保護されないので記号除去される
    (Preset::ForCompare, "http://example.com"),
    (Preset::ForCompare, "mail@example.com"),
    // bidi
    (Preset::ForCompare, "abc\u{202E}def"),
    // 空
    (Preset::ForCompare, ""),
    (Preset::ForCompare, "a"),
    // 混在複雑系
    (Preset::ForCompare, "【速報】東京で第一位！！！"),
    (Preset::ForCompare, "2025年1月1日、新年あけましておめでとう🎍"),
    (Preset::ForCompare, "価格: ￥1,200(税込)"),
    (Preset::ForCompare, "Python と Rust で実装"),
    (Preset::ForCompare, "ｸﾞｯﾄﾞ！！ 絵文字😀 混じり"),
    (Preset::ForCompare, "ハイフン-‐−―—の統一"),
    (Preset::ForCompare, "連続    空白の    畳み込み"),

    // ===== 6. NeologdnCompat 追加境界系 =====
    (Preset::NeologdnCompat, "abc\u{00A0}def"), // NBSP
    (Preset::NeologdnCompat, "タブ\t残す"),
    (Preset::NeologdnCompat, "全角\u{3000}スペース"),
    (Preset::NeologdnCompat, "¥1,200"),
    (Preset::NeologdnCompat, "𩸽"), // 非 BMP
    (Preset::NeologdnCompat, "\u{1F600}"), // 絵文字
    (Preset::NeologdnCompat, "〒100-0001"),
    (Preset::NeologdnCompat, "TEL:03-1234-5678"),
    (Preset::NeologdnCompat, "1+1=2"),
    (Preset::NeologdnCompat, "(株)テスト"),

    // ===== 7. ForSearch 追加 =====
    (Preset::ForSearch, "㍻30年"),
    (Preset::ForSearch, "Ⅰ章とⅡ章"),
    (Preset::ForSearch, "ＡＢＣＡＢＣ"),
    (Preset::ForSearch, "CO₂排出"),
    (Preset::ForSearch, "H₂O"),

    // ===== 8. ForCompare 追加長文 =====
    (Preset::ForCompare, "「東京」と『京都』は違う都市です。"),
    (Preset::ForCompare, "2025/01/01"),
    (Preset::ForCompare, "一、二、三"),
    (Preset::ForCompare, "第1回から第10回まで"),
];
