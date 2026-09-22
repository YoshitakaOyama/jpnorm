//! 正規化の設定とプリセット。

use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

use crate::l1_char::case::CaseAction;
use crate::l1_char::spacing::CjkSpacing;
use crate::l2_script::kana::KanaAction;
use crate::l4_extra::emoji::EmojiAction;
use crate::l4_extra::protect::ProtectConfig;

/// 正規化プリセット。
///
/// 実用パターンを名前付きで提供する。文字列名との相互変換は
/// [`Preset::as_str`] / [`FromStr`] で行える(Python バインディング等で使用)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl Preset {
    /// 全プリセット(定義順)。
    pub const ALL: [Preset; 5] = [
        Preset::None,
        Preset::NeologdnCompat,
        Preset::ForSearch,
        Preset::ForDisplay,
        Preset::ForCompare,
    ];

    /// snake_case のプリセット名。
    pub const fn as_str(self) -> &'static str {
        match self {
            Preset::None => "none",
            Preset::NeologdnCompat => "neologdn_compat",
            Preset::ForSearch => "for_search",
            Preset::ForDisplay => "for_display",
            Preset::ForCompare => "for_compare",
        }
    }
}

impl fmt::Display for Preset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 不明なプリセット名。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsePresetError {
    name: String,
}

impl ParsePresetError {
    /// 与えられた(不明な)名前。
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for ParsePresetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown preset: {:?} (valid: ", self.name)?;
        for (i, p) in Preset::ALL.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            f.write_str(p.as_str())?;
        }
        f.write_str(")")
    }
}

impl std::error::Error for ParsePresetError {}

impl FromStr for Preset {
    type Err = ParsePresetError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Preset::ALL
            .iter()
            .copied()
            .find(|p| p.as_str() == s)
            .ok_or_else(|| ParsePresetError { name: s.to_owned() })
    }
}

/// 正規化設定。個別フラグで細かく制御する。
///
/// プリセット (`Config::for_search()` 等) をベースに、必要なフィールドだけ
/// 書き換えて使うのが基本。フィールドはマイナーバージョンで追加されることがある。
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// ひらがな/カタカナの統一(半角カナ→全角化の後に適用)。
    pub kana: KanaAction,
    /// 旧字体を新字体に統一する (國→国, 體→体)。
    pub kyujitai_to_shinjitai: bool,
    /// 人名・地名で頻出する異体字を代表字に統一する (髙→高, 﨑→崎, 濵→浜)。
    pub unify_itaiji: bool,
    /// 異体字セレクタ (IVS / VS) を除去する。
    pub remove_variation_selectors: bool,
    /// 繰り返し記号 (々ゝゞヽヾ) を展開する (人々→人人, いすゞ→いすず)。
    pub expand_iteration_marks: bool,
    /// カタカナ外来語のゆれを統一する (ヴァ→バ, ウェ→ウエ, ティ→テイ)。
    pub unify_loanword_kana: bool,
    /// 4 文字以上のカタカナ語の末尾長音を落とす (コンピューター→コンピュータ)。
    pub strip_trailing_prolonged: bool,
    /// 英字の大文字小文字の統一。
    pub case: CaseAction,
    /// 日本語と英数字の間の空白の扱い。
    pub cjk_spacing: CjkSpacing,
    /// 元号年を西暦に変換する (令和6年→2024年)。
    pub era_to_western: bool,
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
    pub url_wrap: Option<(Cow<'static, str>, Cow<'static, str>)>,
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
            kana: KanaAction::Keep,
            kyujitai_to_shinjitai: false,
            unify_itaiji: false,
            remove_variation_selectors: false,
            expand_iteration_marks: false,
            unify_loanword_kana: false,
            strip_trailing_prolonged: false,
            case: CaseAction::Keep,
            cjk_spacing: CjkSpacing::Keep,
            era_to_western: false,
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
            protect: ProtectConfig::none(),
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
            kana: KanaAction::Keep,
            kyujitai_to_shinjitai: false,
            unify_itaiji: false,
            remove_variation_selectors: false,
            expand_iteration_marks: false,
            unify_loanword_kana: false,
            strip_trailing_prolonged: false,
            case: CaseAction::Keep,
            cjk_spacing: CjkSpacing::Keep,
            era_to_western: false,
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
            protect: ProtectConfig::none(),
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
            kana: KanaAction::Keep,
            kyujitai_to_shinjitai: true,
            unify_itaiji: true,
            remove_variation_selectors: true,
            expand_iteration_marks: true,
            unify_loanword_kana: true,
            strip_trailing_prolonged: true,
            case: CaseAction::Lower,
            cjk_spacing: CjkSpacing::Keep,
            era_to_western: true,
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
            kana: KanaAction::Keep,
            kyujitai_to_shinjitai: false,
            unify_itaiji: false,
            remove_variation_selectors: false,
            expand_iteration_marks: false,
            unify_loanword_kana: false,
            strip_trailing_prolonged: false,
            case: CaseAction::Keep,
            cjk_spacing: CjkSpacing::Keep,
            era_to_western: false,
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
            protect: ProtectConfig::none(),
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
            kana: KanaAction::Keep,
            kyujitai_to_shinjitai: true,
            unify_itaiji: true,
            remove_variation_selectors: true,
            expand_iteration_marks: true,
            unify_loanword_kana: true,
            strip_trailing_prolonged: true,
            case: CaseAction::Lower,
            cjk_spacing: CjkSpacing::Remove,
            era_to_western: true,
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
            protect: ProtectConfig::none(),
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

/// [`Config::set`] / [`Config::get`] で使う、文字列キー設定の値。
///
/// Python / wasm バインディングや CLI、設定ファイルのように「キー名と値」で
/// 設定を扱いたい場面向け。キー一覧は [`Config::KEYS`]。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigValue {
    /// 真偽値。
    Bool(bool),
    /// 非負整数 (`repeat_limit`)。
    Int(usize),
    /// 文字列 (`kana`, `emoji`, `emoji_placeholder`)。
    Str(String),
    /// 文字列ペア (`url_wrap` の prefix / suffix)。
    Pair(String, String),
    /// 未設定 (`repeat_limit` / `emoji_placeholder` / `url_wrap` の無効化)。
    None,
}

impl From<bool> for ConfigValue {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}

impl From<usize> for ConfigValue {
    fn from(v: usize) -> Self {
        Self::Int(v)
    }
}

impl From<&str> for ConfigValue {
    fn from(v: &str) -> Self {
        Self::Str(v.to_owned())
    }
}

impl From<String> for ConfigValue {
    fn from(v: String) -> Self {
        Self::Str(v)
    }
}

/// [`Config::set`] / [`Config::validate`] のエラー。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConfigError {
    /// 存在しないキー。
    UnknownKey(String),
    /// 値の型が違う。
    WrongType {
        /// キー名。
        key: &'static str,
        /// 期待する型の説明。
        expected: &'static str,
    },
    /// 型は合っているが値が不正、または設定同士が矛盾している。
    InvalidValue {
        /// キー名。
        key: &'static str,
        /// 何が不正か。
        message: String,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownKey(k) => write!(f, "unknown option {k:?}"),
            Self::WrongType { key, expected } => write!(f, "option {key:?} must be {expected}"),
            Self::InvalidValue { key, message } => write!(f, "option {key:?}: {message}"),
        }
    }
}

impl std::error::Error for ConfigError {}

const KANA_CHOICES: [(&str, KanaAction); 3] = [
    ("keep", KanaAction::Keep),
    ("hira_to_kata", KanaAction::HiraToKata),
    ("kata_to_hira", KanaAction::KataToHira),
];

const CASE_CHOICES: [(&str, CaseAction); 3] = [
    ("keep", CaseAction::Keep),
    ("lower", CaseAction::Lower),
    ("upper", CaseAction::Upper),
];

const SPACING_CHOICES: [(&str, CjkSpacing); 3] = [
    ("keep", CjkSpacing::Keep),
    ("remove", CjkSpacing::Remove),
    ("insert", CjkSpacing::Insert),
];

/// 名前 → 列挙値。無ければ選択肢一覧つきのエラー。
fn choice<T: Copy>(
    key: &'static str,
    name: &str,
    choices: &[(&'static str, T)],
) -> Result<T, ConfigError> {
    choices
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, v)| *v)
        .ok_or_else(|| ConfigError::InvalidValue {
            key,
            message: format!(
                "must be one of {} (got {name:?})",
                choices
                    .iter()
                    .map(|(n, _)| *n)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        })
}

/// 列挙値 → 名前。
fn choice_name<T: Copy + PartialEq>(choices: &[(&'static str, T)], value: T) -> &'static str {
    choices
        .iter()
        .find(|(_, v)| *v == value)
        .map_or("", |(n, _)| *n)
}

fn expect_bool(key: &'static str, v: ConfigValue) -> Result<bool, ConfigError> {
    match v {
        ConfigValue::Bool(b) => Ok(b),
        _ => Err(ConfigError::WrongType {
            key,
            expected: "bool",
        }),
    }
}

fn expect_str(key: &'static str, v: ConfigValue) -> Result<String, ConfigError> {
    match v {
        ConfigValue::Str(s) => Ok(s),
        _ => Err(ConfigError::WrongType {
            key,
            expected: "str",
        }),
    }
}

impl Config {
    /// [`set`](Self::set) / [`get`](Self::get) で使える全キー。
    pub const KEYS: &'static [&'static str] = &[
        "normalize_newlines",
        "remove_zero_width",
        "remove_control",
        "remove_bidi_control",
        "kansuji_to_arabic",
        "arabic_to_kansuji",
        "canonicalize_numbers",
        "nfkc",
        "halfwidth_kana_to_fullwidth",
        "kana",
        "kyujitai_to_shinjitai",
        "unify_itaiji",
        "remove_variation_selectors",
        "expand_iteration_marks",
        "unify_loanword_kana",
        "strip_trailing_prolonged",
        "case",
        "cjk_spacing",
        "era_to_western",
        "unify_hyphens",
        "unify_tildes",
        "unify_prolonged",
        "collapse_prolonged_run",
        "repeat_limit",
        "collapse_spaces",
        "trim",
        "expand_cjk_compat",
        "remove_symbols",
        "remove_cjk_compat",
        "unify_quotes",
        "protect_urls",
        "protect_emails",
        "protect_mentions",
        "protect_hashtags",
        "emoji",
        "emoji_placeholder",
        "url_wrap",
    ];

    /// 文字列キーで設定値を書き換える。
    ///
    /// `url_wrap` にペアを設定すると `protect_urls` も有効になる。
    /// `emoji` は `"keep"` / `"remove"`、`emoji_placeholder` は置換文字列 (None で解除)。
    pub fn set(&mut self, key: &str, value: ConfigValue) -> Result<(), ConfigError> {
        // KEYS の &'static str に寄せてエラー型に載せる。
        let Some(&key) = Self::KEYS.iter().find(|k| **k == key) else {
            return Err(ConfigError::UnknownKey(key.to_owned()));
        };
        match key {
            "normalize_newlines" => self.normalize_newlines = expect_bool(key, value)?,
            "remove_zero_width" => self.remove_zero_width = expect_bool(key, value)?,
            "remove_control" => self.remove_control = expect_bool(key, value)?,
            "remove_bidi_control" => self.remove_bidi_control = expect_bool(key, value)?,
            "kansuji_to_arabic" => self.kansuji_to_arabic = expect_bool(key, value)?,
            "arabic_to_kansuji" => self.arabic_to_kansuji = expect_bool(key, value)?,
            "canonicalize_numbers" => self.canonicalize_numbers = expect_bool(key, value)?,
            "nfkc" => self.nfkc = expect_bool(key, value)?,
            "halfwidth_kana_to_fullwidth" => {
                self.halfwidth_kana_to_fullwidth = expect_bool(key, value)?
            }
            "kana" => {
                let s = expect_str(key, value)?;
                self.kana = choice(key, &s, &KANA_CHOICES)?;
            }
            "kyujitai_to_shinjitai" => self.kyujitai_to_shinjitai = expect_bool(key, value)?,
            "unify_itaiji" => self.unify_itaiji = expect_bool(key, value)?,
            "remove_variation_selectors" => {
                self.remove_variation_selectors = expect_bool(key, value)?
            }
            "expand_iteration_marks" => self.expand_iteration_marks = expect_bool(key, value)?,
            "unify_loanword_kana" => self.unify_loanword_kana = expect_bool(key, value)?,
            "strip_trailing_prolonged" => self.strip_trailing_prolonged = expect_bool(key, value)?,
            "case" => {
                let s = expect_str(key, value)?;
                self.case = choice(key, &s, &CASE_CHOICES)?;
            }
            "cjk_spacing" => {
                let s = expect_str(key, value)?;
                self.cjk_spacing = choice(key, &s, &SPACING_CHOICES)?;
            }
            "era_to_western" => self.era_to_western = expect_bool(key, value)?,
            "unify_hyphens" => self.unify_hyphens = expect_bool(key, value)?,
            "unify_tildes" => self.unify_tildes = expect_bool(key, value)?,
            "unify_prolonged" => self.unify_prolonged = expect_bool(key, value)?,
            "collapse_prolonged_run" => self.collapse_prolonged_run = expect_bool(key, value)?,
            "repeat_limit" => {
                self.repeat_limit = match value {
                    ConfigValue::None => None,
                    ConfigValue::Int(0) => {
                        return Err(ConfigError::InvalidValue {
                            key,
                            message: "must be >= 1 (use None to disable)".to_owned(),
                        });
                    }
                    ConfigValue::Int(n) => Some(n),
                    _ => {
                        return Err(ConfigError::WrongType {
                            key,
                            expected: "int >= 1 or None",
                        });
                    }
                }
            }
            "collapse_spaces" => self.collapse_spaces = expect_bool(key, value)?,
            "trim" => self.trim = expect_bool(key, value)?,
            "expand_cjk_compat" => self.expand_cjk_compat = expect_bool(key, value)?,
            "remove_symbols" => self.remove_symbols = expect_bool(key, value)?,
            "remove_cjk_compat" => self.remove_cjk_compat = expect_bool(key, value)?,
            "unify_quotes" => self.unify_quotes = expect_bool(key, value)?,
            "protect_urls" => self.protect.urls = expect_bool(key, value)?,
            "protect_emails" => self.protect.emails = expect_bool(key, value)?,
            "protect_mentions" => self.protect.mentions = expect_bool(key, value)?,
            "protect_hashtags" => self.protect.hashtags = expect_bool(key, value)?,
            "emoji" => {
                let s = expect_str(key, value)?;
                self.emoji_action = match s.as_str() {
                    "keep" => EmojiAction::Keep,
                    "remove" => EmojiAction::Remove,
                    _ => {
                        return Err(ConfigError::InvalidValue {
                            key,
                            message: format!(
                                "must be keep or remove (got {s:?}); use emoji_placeholder to replace"
                            ),
                        });
                    }
                }
            }
            "emoji_placeholder" => {
                self.emoji_action = match value {
                    // None は「置換を解除」。Remove 設定を Keep に戻してしまわないよう、
                    // 置換中のときだけ Keep に戻す (entries() の往復が壊れないように)。
                    ConfigValue::None => match &self.emoji_action {
                        EmojiAction::Replace(_) => EmojiAction::Keep,
                        other => other.clone(),
                    },
                    ConfigValue::Str(s) => EmojiAction::replace(s),
                    _ => {
                        return Err(ConfigError::WrongType {
                            key,
                            expected: "str or None",
                        });
                    }
                }
            }
            "url_wrap" => {
                self.url_wrap = match value {
                    ConfigValue::None => None,
                    ConfigValue::Pair(p, s) => {
                        self.protect.urls = true;
                        Some((Cow::Owned(p), Cow::Owned(s)))
                    }
                    _ => {
                        return Err(ConfigError::WrongType {
                            key,
                            expected: "a (prefix, suffix) pair or None",
                        });
                    }
                }
            }
            _ => unreachable!("key is in KEYS"),
        }
        Ok(())
    }

    /// 文字列キーで設定値を読む。未知のキーは `None`。
    pub fn get(&self, key: &str) -> Option<ConfigValue> {
        use ConfigValue as V;
        Some(match key {
            "normalize_newlines" => V::Bool(self.normalize_newlines),
            "remove_zero_width" => V::Bool(self.remove_zero_width),
            "remove_control" => V::Bool(self.remove_control),
            "remove_bidi_control" => V::Bool(self.remove_bidi_control),
            "kansuji_to_arabic" => V::Bool(self.kansuji_to_arabic),
            "arabic_to_kansuji" => V::Bool(self.arabic_to_kansuji),
            "canonicalize_numbers" => V::Bool(self.canonicalize_numbers),
            "nfkc" => V::Bool(self.nfkc),
            "halfwidth_kana_to_fullwidth" => V::Bool(self.halfwidth_kana_to_fullwidth),
            "kana" => V::Str(choice_name(&KANA_CHOICES, self.kana).to_owned()),
            "kyujitai_to_shinjitai" => V::Bool(self.kyujitai_to_shinjitai),
            "unify_itaiji" => V::Bool(self.unify_itaiji),
            "remove_variation_selectors" => V::Bool(self.remove_variation_selectors),
            "expand_iteration_marks" => V::Bool(self.expand_iteration_marks),
            "unify_loanword_kana" => V::Bool(self.unify_loanword_kana),
            "strip_trailing_prolonged" => V::Bool(self.strip_trailing_prolonged),
            "case" => V::Str(choice_name(&CASE_CHOICES, self.case).to_owned()),
            "cjk_spacing" => V::Str(choice_name(&SPACING_CHOICES, self.cjk_spacing).to_owned()),
            "era_to_western" => V::Bool(self.era_to_western),
            "unify_hyphens" => V::Bool(self.unify_hyphens),
            "unify_tildes" => V::Bool(self.unify_tildes),
            "unify_prolonged" => V::Bool(self.unify_prolonged),
            "collapse_prolonged_run" => V::Bool(self.collapse_prolonged_run),
            "repeat_limit" => self.repeat_limit.map_or(V::None, V::Int),
            "collapse_spaces" => V::Bool(self.collapse_spaces),
            "trim" => V::Bool(self.trim),
            "expand_cjk_compat" => V::Bool(self.expand_cjk_compat),
            "remove_symbols" => V::Bool(self.remove_symbols),
            "remove_cjk_compat" => V::Bool(self.remove_cjk_compat),
            "unify_quotes" => V::Bool(self.unify_quotes),
            "protect_urls" => V::Bool(self.protect.urls),
            "protect_emails" => V::Bool(self.protect.emails),
            "protect_mentions" => V::Bool(self.protect.mentions),
            "protect_hashtags" => V::Bool(self.protect.hashtags),
            "emoji" => V::Str(
                match self.emoji_action {
                    EmojiAction::Remove => "remove",
                    _ => "keep",
                }
                .to_owned(),
            ),
            "emoji_placeholder" => match &self.emoji_action {
                EmojiAction::Replace(s) => V::Str(s.to_string()),
                _ => V::None,
            },
            "url_wrap" => match &self.url_wrap {
                Some((p, s)) => V::Pair(p.to_string(), s.to_string()),
                None => V::None,
            },
            _ => return None,
        })
    }

    /// 全キーと現在値の一覧 ([`KEYS`](Self::KEYS) の順)。
    pub fn entries(&self) -> Vec<(&'static str, ConfigValue)> {
        Self::KEYS
            .iter()
            .map(|k| (*k, self.get(k).expect("key is in KEYS")))
            .collect()
    }

    /// 設定同士の矛盾を検査する。
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.kansuji_to_arabic && self.arabic_to_kansuji {
            return Err(ConfigError::InvalidValue {
                key: "arabic_to_kansuji",
                message: "kansuji_to_arabic and arabic_to_kansuji cannot both be enabled"
                    .to_owned(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get_roundtrip_all_keys() {
        let base = Config::for_compare();
        let mut rebuilt = Config::none();
        for (k, v) in base.entries() {
            rebuilt.set(k, v).unwrap();
        }
        assert_eq!(rebuilt, base);
        assert_eq!(base.entries().len(), Config::KEYS.len());
    }

    #[test]
    fn set_errors() {
        let mut c = Config::none();
        assert!(matches!(
            c.set("bogus", true.into()),
            Err(ConfigError::UnknownKey(_))
        ));
        assert!(matches!(
            c.set("nfkc", "yes".into()),
            Err(ConfigError::WrongType { .. })
        ));
        assert!(matches!(
            c.set("kana", "x".into()),
            Err(ConfigError::InvalidValue { .. })
        ));
        assert!(matches!(
            c.set("repeat_limit", 0usize.into()),
            Err(ConfigError::InvalidValue { .. })
        ));
        c.set("url_wrap", ConfigValue::Pair("<".into(), ">".into()))
            .unwrap();
        assert!(c.protect.urls);
        c.set("emoji_placeholder", "_".into()).unwrap();
        assert_eq!(c.emoji_action, EmojiAction::replace("_"));
        c.set("kansuji_to_arabic", true.into()).unwrap();
        c.set("arabic_to_kansuji", true.into()).unwrap();
        assert!(c.validate().is_err());
    }
}
