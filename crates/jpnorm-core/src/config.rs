//! 正規化の設定とプリセット。

use crate::l4_extra::emoji::EmojiAction;
use crate::l4_extra::protect::ProtectConfig;

/// 正規化プリセット。
///
/// 実用パターンを名前付きで提供する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    /// 何もしない(ビルダーで個別指定したい場合のベース)。
    None,
    /// neologdn 互換寄りの設定。文字レベルで一般的な正規化を行う。
    NeologdnCompat,
    /// 検索インデックス向け。積極的に揃える。
    ForSearch,
    /// 表示向け。見た目が壊れない範囲で最小限。
    ForDisplay,
    /// 精度比較・重複判定向け。最も積極的に情報を落として等価性を最大化する。
    ForCompare,
}

/// 正規化設定。個別フラグで細かく制御する。
#[derive(Debug, Clone)]
pub struct Config {
    /// 改行コード(CRLF/CR/NEL/LS/PS)を LF に統一する。
    pub normalize_newlines: bool,
    /// ゼロ幅文字・BOM を削除する (ZWJ は絵文字のため除外)。
    pub remove_zero_width: bool,
    /// 制御文字 (C0/C1, DEL) を削除する。`\t` `\n` は残す。
    pub remove_control: bool,
    /// Bidi 制御文字を削除する (Trojan Source 対策)。
    pub remove_bidi_control: bool,
    /// 漢数字をアラビア数字に変換する (例: 一千二百三十四 → 1234)。
    pub kansuji_to_arabic: bool,
    /// アラビア数字を漢数字に変換する。`kansuji_to_arabic` と同時指定は不可。
    pub arabic_to_kansuji: bool,
    /// 数値トークンを正規化する (1,200 / 1200.00 → 1200)。比較用途。
    pub canonicalize_numbers: bool,
    /// Unicode NFKC を適用する。
    pub nfkc: bool,
    /// 半角カナを全角カナへ。
    pub halfwidth_kana_to_fullwidth: bool,
    /// 各種ハイフン/マイナス/ダッシュを `-` に統一する。
    pub unify_hyphens: bool,
    /// 各種チルダ/波ダッシュを `〜` に統一する。
    pub unify_tildes: bool,
    /// 長音符のバリエーションを `ー` に統一する。
    pub unify_prolonged: bool,
    /// 連続した長音符 (`ー`, `〜`) を 1 つに畳み込む(neologdn 互換挙動)。
    pub collapse_prolonged_run: bool,
    /// 同一文字の連続を短縮する際の最大繰り返し数 (None = 無効)。
    pub repeat_limit: Option<usize>,
    /// 空白の畳み込み。日本語間の空白は除去、ASCII間は単一スペース。
    pub collapse_spaces: bool,
    /// 先頭/末尾の空白を削除する。
    pub trim: bool,
    /// 機種依存文字 (㈱①髙㌔ 等) を展開する。
    pub expand_cjk_compat: bool,
    /// 記号(句読点や各種シンボル)を削除する。
    pub remove_symbols: bool,
    /// 機種依存文字を削除する(展開ではなく完全除去)。
    pub remove_cjk_compat: bool,
    /// 曲線引用符を直線引用符に統一する。
    pub unify_quotes: bool,
    /// 保護する領域(URL/email/mention/hashtag)。
    pub protect: ProtectConfig,
    /// 絵文字の処理方法。
    pub emoji_action: EmojiAction,
    /// URL を prefix/suffix で囲む。`protect.urls` 有効時のみ機能する。
    pub url_wrap: Option<(&'static str, &'static str)>,
}

impl Config {
    /// 何も変換しない設定。
    pub const fn none() -> Self {
        Self {
            normalize_newlines: false,
            remove_zero_width: false,
            remove_control: false,
            remove_bidi_control: false,
            kansuji_to_arabic: false,
            arabic_to_kansuji: false,
            canonicalize_numbers: false,
            nfkc: false,
            halfwidth_kana_to_fullwidth: false,
            unify_hyphens: false,
            unify_tildes: false,
            unify_prolonged: false,
            collapse_prolonged_run: false,
            repeat_limit: None,
            collapse_spaces: false,
            trim: false,
            expand_cjk_compat: false,
            remove_symbols: false,
            remove_cjk_compat: false,
            unify_quotes: false,
            protect: ProtectConfig {
                urls: false,
                emails: false,
                mentions: false,
                hashtags: false,
            },
            emoji_action: EmojiAction::Keep,
            url_wrap: None,
        }
    }

    /// neologdn 互換寄りの設定。
    pub const fn neologdn_compat() -> Self {
        Self {
            // neologdn は NFKC 相当で全角英数→半角を行う
            normalize_newlines: true,
            remove_zero_width: true,
            remove_control: true,
            remove_bidi_control: true,
            kansuji_to_arabic: false,
            arabic_to_kansuji: false,
            canonicalize_numbers: false,
            nfkc: true,
            halfwidth_kana_to_fullwidth: true,
            unify_hyphens: true,
            unify_tildes: true,
            unify_prolonged: true,
            // neologdn は通常文字の繰り返し短縮はしない(長音符のみ畳む)
            collapse_prolonged_run: true,
            repeat_limit: None,
            collapse_spaces: true,
            trim: true,
            expand_cjk_compat: false,
            remove_symbols: false,
            remove_cjk_compat: false,
            unify_quotes: false,
            protect: ProtectConfig {
                urls: false,
                emails: false,
                mentions: false,
                hashtags: false,
            },
            emoji_action: EmojiAction::Keep,
            url_wrap: None,
        }
    }

    /// 検索インデックス向け。NFKC まで掛ける。
    pub const fn for_search() -> Self {
        Self {
            normalize_newlines: true,
            remove_zero_width: true,
            remove_control: true,
            remove_bidi_control: true,
            kansuji_to_arabic: false,
            arabic_to_kansuji: false,
            canonicalize_numbers: false,
            nfkc: true,
            halfwidth_kana_to_fullwidth: true,
            unify_hyphens: true,
            unify_tildes: true,
            unify_prolonged: true,
            collapse_prolonged_run: true,
            repeat_limit: Some(2),
            collapse_spaces: true,
            trim: true,
            expand_cjk_compat: true,
            remove_symbols: false,
            remove_cjk_compat: false,
            unify_quotes: true,
            protect: ProtectConfig::all(),
            emoji_action: EmojiAction::Remove,
            url_wrap: None,
        }
    }

    /// 表示向け。見た目に影響しない最小限。
    pub const fn for_display() -> Self {
        Self {
            normalize_newlines: false,
            remove_zero_width: false,
            remove_control: false,
            remove_bidi_control: false,
            kansuji_to_arabic: false,
            arabic_to_kansuji: false,
            canonicalize_numbers: false,
            nfkc: false,
            halfwidth_kana_to_fullwidth: true,
            unify_hyphens: false,
            unify_tildes: false,
            unify_prolonged: true,
            collapse_prolonged_run: false,
            repeat_limit: None,
            collapse_spaces: false,
            trim: true,
            expand_cjk_compat: false,
            remove_symbols: false,
            remove_cjk_compat: false,
            unify_quotes: false,
            protect: ProtectConfig {
                urls: false,
                emails: false,
                mentions: false,
                hashtags: false,
            },
            emoji_action: EmojiAction::Keep,
            url_wrap: None,
        }
    }

    /// 精度比較・重複判定向け。`for_search` をベースに、等価性を最大化するため
    /// さらに情報を落とす(漢数字→アラビア、記号除去、CJK 互換除去、URL 保護解除)。
    pub const fn for_compare() -> Self {
        Self {
            normalize_newlines: true,
            remove_zero_width: true,
            remove_control: true,
            remove_bidi_control: true,
            kansuji_to_arabic: true,
            arabic_to_kansuji: false,
            canonicalize_numbers: true,
            nfkc: true,
            halfwidth_kana_to_fullwidth: true,
            unify_hyphens: true,
            unify_tildes: true,
            unify_prolonged: true,
            collapse_prolonged_run: true,
            repeat_limit: Some(1),
            collapse_spaces: true,
            trim: true,
            expand_cjk_compat: true,
            remove_symbols: true,
            remove_cjk_compat: true,
            unify_quotes: true,
            protect: ProtectConfig {
                urls: false,
                emails: false,
                mentions: false,
                hashtags: false,
            },
            emoji_action: EmojiAction::Remove,
            url_wrap: None,
        }
    }

    /// プリセットから生成する。
    pub const fn from_preset(preset: Preset) -> Self {
        match preset {
            Preset::None => Self::none(),
            Preset::NeologdnCompat => Self::neologdn_compat(),
            Preset::ForSearch => Self::for_search(),
            Preset::ForDisplay => Self::for_display(),
            Preset::ForCompare => Self::for_compare(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::neologdn_compat()
    }
}
