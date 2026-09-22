//! 正規化パイプライン。Builder で構築する。

use std::borrow::Cow;

use crate::config::{Config, Preset};
use crate::l1_char;
use crate::l1_char::case::{self, CaseAction};
use crate::l1_char::spacing::{self, CjkSpacing};
use crate::l2_script::kana::{self, KanaAction};
use crate::l2_script::{era, iteration, loanword, numbers, numerals};
use crate::l3_lexical::SynonymDict;
use crate::l4_extra::emoji::{self, EmojiAction};
use crate::l4_extra::protect::{self, Kind as ProtectKind, ProtectConfig};

/// 正規化結果のセグメント情報。
///
/// 出力テキストがどの種別のセグメント(正規化済みか、保護パススルーか)で
/// 構成されているかを表す。デバッグや検索ハイライトに利用する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    /// 正規化が適用された通常のセグメント。
    Normalized(String),
    /// 保護領域(URL/email/mention/hashtag)で元文字列のまま。
    Protected {
        /// 出力に含まれるテキスト(`url_wrap` 適用後)。
        text: String,
        /// 保護領域の種別。
        kind: ProtectKind,
    },
}

/// `normalize_with_segments()` の戻り値。
#[derive(Debug, Clone)]
pub struct NormalizedText {
    /// 連結済みの最終出力。
    pub text: String,
    /// 出力を構成するセグメントの並び。
    pub segments: Vec<Segment>,
}

/// 宣言的に組み立てる正規化器。
#[derive(Debug, Clone)]
pub struct Normalizer {
    config: Config,
    synonyms: Option<SynonymDict>,
}

impl Normalizer {
    /// プリセットから直接生成。
    pub fn preset(preset: Preset) -> Self {
        Self {
            config: Config::from_preset(preset),
            synonyms: None,
        }
    }

    /// 設定から生成。
    pub fn from_config(config: Config) -> Self {
        Self {
            config,
            synonyms: None,
        }
    }

    /// 同義語辞書を差し込む。
    ///
    /// 辞書は正規化の最終段で適用されるため、キー(表記ゆれ)は正規化後のテキストと
    /// 照合される。利用者が生の表記 (`㈱サンプル` など) をそのまま書けるよう、
    /// ここでキーを同じ設定(辞書なし)で正規化してから登録する。正規化の結果
    /// 空になったキーは捨てる。
    pub fn with_synonyms(mut self, dict: SynonymDict) -> Self {
        let plain = Normalizer {
            config: self.config.clone(),
            synonyms: None,
        };
        let mut normalized = SynonymDict::new();
        for (variant, canonical) in dict.iter() {
            normalized.insert(plain.normalize(variant), canonical);
        }
        self.synonyms = Some(normalized);
        self
    }

    /// Builder を開始。
    pub fn builder() -> NormalizerBuilder {
        NormalizerBuilder::new()
    }

    /// 内部設定への参照。
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// 設定済みの同義語辞書への参照。
    pub fn synonyms(&self) -> Option<&SynonymDict> {
        self.synonyms.as_ref()
    }

    /// 複数テキストをまとめて正規化する。
    pub fn normalize_batch<I, S>(&self, inputs: I) -> Vec<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        inputs
            .into_iter()
            .map(|s| self.normalize(s.as_ref()))
            .collect()
    }

    /// 文字列を正規化する。
    pub fn normalize(&self, input: &str) -> String {
        self.normalize_with_segments(input).text
    }

    /// 正規化を実行し、出力と保護セグメントの構成を返す。
    ///
    /// 保護設定 (`config.protect`) が有効な場合は、URL/mail/mention/hashtag を
    /// 検出して該当範囲だけ元のまま残し、それ以外を正規化する。
    pub fn normalize_with_segments(&self, input: &str) -> NormalizedText {
        let c = &self.config;

        // 1) 保護領域スキャン → セグメント分割
        let segs: Vec<Result<&str, (&str, ProtectKind)>> = if c.protect.any() {
            let spans = protect::scan(input, c.protect);
            protect::segment(input, &spans)
        } else {
            vec![Ok(input)]
        };

        // 2) 各セグメントを処理してセグメント列を組み立てる
        //    trim はここでは適用しない: free segment ごとに trim すると保護領域に
        //    隣接する空白まで消えてしまい、URL/email が周辺テキストと結合する。
        let mut segments: Vec<Segment> = Vec::with_capacity(segs.len());
        for seg in segs {
            match seg {
                Ok(plain) => {
                    let normalized = self.apply_to_free(plain);
                    if !normalized.is_empty() {
                        segments.push(Segment::Normalized(normalized));
                    }
                }
                Err((protected, kind)) => {
                    let emitted = if matches!(kind, ProtectKind::Url) {
                        if let Some((prefix, suffix)) = &c.url_wrap {
                            format!("{prefix}{protected}{suffix}")
                        } else {
                            protected.to_owned()
                        }
                    } else {
                        protected.to_owned()
                    };
                    segments.push(Segment::Protected {
                        text: emitted,
                        kind,
                    });
                }
            }
        }

        // 3) 出力全体に対する trim。
        //    先頭/末尾の Normalized セグメントだけを縮め、Protected 領域には触らない。
        if c.trim {
            while let Some(Segment::Normalized(s)) = segments.first() {
                let trimmed = s.trim_start();
                if trimmed.is_empty() {
                    segments.remove(0);
                } else if trimmed.len() != s.len() {
                    let owned = trimmed.to_owned();
                    segments[0] = Segment::Normalized(owned);
                    break;
                } else {
                    break;
                }
            }
            while let Some(Segment::Normalized(s)) = segments.last() {
                let trimmed = s.trim_end();
                if trimmed.is_empty() {
                    segments.pop();
                } else if trimmed.len() != s.len() {
                    let owned = trimmed.to_owned();
                    let last = segments.len() - 1;
                    segments[last] = Segment::Normalized(owned);
                    break;
                } else {
                    break;
                }
            }
        }

        // 4) 連結
        let mut out_text = String::with_capacity(input.len());
        for seg in &segments {
            match seg {
                Segment::Normalized(s) => out_text.push_str(s),
                Segment::Protected { text, .. } => out_text.push_str(text),
            }
        }

        NormalizedText {
            text: out_text,
            segments,
        }
    }

    /// 非保護セグメントに対して一連の正規化を適用する。
    fn apply_to_free(&self, input: &str) -> String {
        let c = &self.config;
        let mut s: String = input.to_owned();

        // 0) クリーンアップ: 不可視・制御・改行。以降の処理が安定するよう最初に掛ける。
        if c.normalize_newlines {
            s = l1_char::cleanup::normalize_newlines(&s);
        }
        if c.remove_bidi_control {
            s = l1_char::cleanup::remove_bidi_control(&s);
        }
        if c.remove_zero_width {
            s = l1_char::cleanup::remove_zero_width(&s);
        }
        if c.remove_control {
            s = l1_char::cleanup::remove_control(&s);
        }

        if c.remove_variation_selectors {
            s = l1_char::kanji::remove_variation_selectors(&s);
        }
        if c.expand_cjk_compat {
            s = l1_char::cjk_compat::expand(&s);
        }
        if c.nfkc {
            s = l1_char::unicode::nfkc(&s);
        }
        // L1 (漢字): NFKC で互換漢字が統合漢字に寄った後に、旧字体・異体字を畳む。
        if c.kyujitai_to_shinjitai {
            s = l1_char::kanji::kyujitai_to_shinjitai(&s);
        }
        if c.unify_itaiji {
            s = l1_char::kanji::unify_itaiji(&s);
        }
        if c.halfwidth_kana_to_fullwidth {
            s = l1_char::width::halfwidth_kana_to_fullwidth(&s);
        }
        // 繰り返し記号は全角化の後・かな種変換の前 (ヽ はカタカナ文脈で判定する)。
        if c.expand_iteration_marks {
            s = iteration::expand_iteration_marks(&s);
        }
        if c.unify_loanword_kana {
            s = loanword::unify_loanword_kana(&s);
        }
        if !matches!(c.kana, KanaAction::Keep) {
            s = kana::process(&s, c.kana);
        }
        if c.unify_quotes {
            s = l1_char::quotes::unify(&s);
        }
        if c.unify_hyphens || c.unify_tildes || c.unify_prolonged {
            s = l1_char::symbols::unify(&s, c.unify_hyphens, c.unify_tildes, c.unify_prolonged);
        }
        if c.collapse_prolonged_run {
            s = l1_char::prolonged::collapse(&s);
        }
        // 長音符の統一・畳み込みの後に末尾長音を落とす (ｰ や ーー を先に揃えておく)。
        if c.strip_trailing_prolonged {
            s = loanword::strip_trailing_prolonged(&s, 4);
        }
        if !matches!(c.emoji_action, EmojiAction::Keep) {
            s = emoji::process(&s, &c.emoji_action);
        }
        // L2: 数値変換。NFKC 済みの半角数字を前提にするため後段で掛ける。
        // remove_symbols より前に動かすことで、漢数字ゼロ 〇 や桁区切りカンマが
        // 記号除去で失われる前に数値として確定させる。
        if c.kansuji_to_arabic {
            s = numerals::kansuji_to_arabic(&s);
        }
        if c.arabic_to_kansuji {
            s = numerals::arabic_to_kansuji(&s);
        }
        // 元号は漢数字→アラビア数字の後 (令和六年 → 令和6年 → 2024年)。
        if c.era_to_western {
            s = era::era_to_western(&s);
        }
        if c.canonicalize_numbers {
            s = numbers::canonicalize(&s);
        }
        if c.remove_cjk_compat {
            s = l1_char::strip::remove_cjk_compat(&s);
        }
        if c.remove_symbols {
            s = l1_char::strip::remove_symbols(&s);
        }
        if let Some(limit) = c.repeat_limit {
            s = l1_char::repeat::shorten(&s, limit);
        }
        if !matches!(c.case, CaseAction::Keep) {
            s = case::process(&s, c.case);
        }
        if c.collapse_spaces {
            s = l1_char::spaces::collapse(&s);
        }
        if !matches!(c.cjk_spacing, CjkSpacing::Keep) {
            s = spacing::process(&s, c.cjk_spacing);
        }
        // trim は normalize_with_segments で出力全体に対して適用する。
        // ここで free segment 単位に trim すると、保護領域 (URL/email等) に
        // 隣接する空白が消えて隣接文字と結合してしまう。
        if let Some(dict) = &self.synonyms {
            s = dict.apply(&s);
        }

        s
    }
}

impl Default for Normalizer {
    fn default() -> Self {
        Self::preset(Preset::NeologdnCompat)
    }
}

/// Normalizer の Builder。
///
/// `new()` は何も変換しない状態から始まる。プリセットをベースにしたい場合は
/// [`preset`](Self::preset) を最初に呼び、そこから個別に足し引きする。
/// 「引く」操作(プリセットの一部を無効化する)は [`configure`](Self::configure) で行う。
///
/// ```
/// use jpnorm_core::{Normalizer, Preset};
///
/// let n = Normalizer::builder()
///     .preset(Preset::ForSearch)
///     .configure(|c| c.emoji_action = jpnorm_core::EmojiAction::Keep)
///     .build();
/// assert_eq!(n.normalize("ｶﾅ😀"), "カナ😀");
/// ```
#[derive(Debug, Clone)]
pub struct NormalizerBuilder {
    config: Config,
    synonyms: Option<SynonymDict>,
}

impl Default for NormalizerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl NormalizerBuilder {
    /// 空の Builder(何も変換しない状態)。
    pub fn new() -> Self {
        Self {
            config: Config::none(),
            synonyms: None,
        }
    }

    /// 既存の設定から Builder を開始する。
    pub fn from_config(config: Config) -> Self {
        Self {
            config,
            synonyms: None,
        }
    }

    /// プリセットをベースに適用する(それまでの設定は上書きされる)。
    pub fn preset(mut self, preset: Preset) -> Self {
        self.config = Config::from_preset(preset);
        self
    }

    /// 設定をクロージャで直接編集する。プリセットの一部を無効化する場合などに使う。
    pub fn configure(mut self, f: impl FnOnce(&mut Config)) -> Self {
        f(&mut self.config);
        self
    }

    /// 現在の設定への可変参照。
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// 同義語辞書を設定する。
    pub fn synonyms(mut self, dict: SynonymDict) -> Self {
        self.synonyms = Some(dict);
        self
    }

    /// 改行コードを LF に統一する。
    pub fn normalize_newlines(mut self) -> Self {
        self.config.normalize_newlines = true;
        self
    }

    /// ゼロ幅文字・BOM を削除する。
    pub fn remove_zero_width(mut self) -> Self {
        self.config.remove_zero_width = true;
        self
    }

    /// 制御文字 (C0/C1) を削除する。
    pub fn remove_control(mut self) -> Self {
        self.config.remove_control = true;
        self
    }

    /// Bidi 制御文字を削除する (Trojan Source 対策)。
    pub fn remove_bidi_control(mut self) -> Self {
        self.config.remove_bidi_control = true;
        self
    }

    /// 不可視・制御系のサニタイズをまとめて有効化する。
    pub fn sanitize_invisible(mut self) -> Self {
        self.config.normalize_newlines = true;
        self.config.remove_zero_width = true;
        self.config.remove_control = true;
        self.config.remove_bidi_control = true;
        self
    }

    /// 漢数字をアラビア数字に変換する。
    pub fn kansuji_to_arabic(mut self) -> Self {
        self.config.kansuji_to_arabic = true;
        self.config.arabic_to_kansuji = false;
        self
    }

    /// アラビア数字を漢数字に変換する。
    pub fn arabic_to_kansuji(mut self) -> Self {
        self.config.arabic_to_kansuji = true;
        self.config.kansuji_to_arabic = false;
        self
    }

    /// NFKC を有効化。
    pub fn nfkc(mut self) -> Self {
        self.config.nfkc = true;
        self
    }

    /// 半角カナを全角カナに寄せる。
    pub fn halfwidth_kana_to_fullwidth(mut self) -> Self {
        self.config.halfwidth_kana_to_fullwidth = true;
        self
    }

    /// ひらがなをカタカナに統一する。
    pub fn hira_to_kata(mut self) -> Self {
        self.config.kana = KanaAction::HiraToKata;
        self
    }

    /// カタカナをひらがなに統一する。
    pub fn kata_to_hira(mut self) -> Self {
        self.config.kana = KanaAction::KataToHira;
        self
    }

    /// 旧字体を新字体に統一する。
    pub fn kyujitai_to_shinjitai(mut self) -> Self {
        self.config.kyujitai_to_shinjitai = true;
        self
    }

    /// 人名・地名の異体字を代表字に統一する。
    pub fn unify_itaiji(mut self) -> Self {
        self.config.unify_itaiji = true;
        self
    }

    /// 異体字セレクタを除去する。
    pub fn remove_variation_selectors(mut self) -> Self {
        self.config.remove_variation_selectors = true;
        self
    }

    /// 繰り返し記号 (々ゝゞヽヾ) を展開する。
    pub fn expand_iteration_marks(mut self) -> Self {
        self.config.expand_iteration_marks = true;
        self
    }

    /// カタカナ外来語のゆれを統一する。
    pub fn unify_loanword_kana(mut self) -> Self {
        self.config.unify_loanword_kana = true;
        self
    }

    /// 4 文字以上のカタカナ語の末尾長音を落とす。
    pub fn strip_trailing_prolonged(mut self) -> Self {
        self.config.strip_trailing_prolonged = true;
        self
    }

    /// 英字を小文字に揃える。
    pub fn lowercase(mut self) -> Self {
        self.config.case = CaseAction::Lower;
        self
    }

    /// 英字を大文字に揃える。
    pub fn uppercase(mut self) -> Self {
        self.config.case = CaseAction::Upper;
        self
    }

    /// 日本語と英数字の間の空白の扱いを指定する。
    pub fn cjk_spacing(mut self, mode: CjkSpacing) -> Self {
        self.config.cjk_spacing = mode;
        self
    }

    /// 元号年を西暦に変換する。
    pub fn era_to_western(mut self) -> Self {
        self.config.era_to_western = true;
        self
    }

    /// 数値トークンを正規化する (1,200 / 1200.00 → 1200)。
    pub fn canonicalize_numbers(mut self) -> Self {
        self.config.canonicalize_numbers = true;
        self
    }

    /// 連続した長音符・チルダを 1 つに畳み込む。
    pub fn collapse_prolonged_run(mut self) -> Self {
        self.config.collapse_prolonged_run = true;
        self
    }

    /// ハイフン統一。
    pub fn unify_hyphens(mut self) -> Self {
        self.config.unify_hyphens = true;
        self
    }

    /// チルダ統一。
    pub fn unify_tildes(mut self) -> Self {
        self.config.unify_tildes = true;
        self
    }

    /// 長音符統一。
    pub fn unify_prolonged(mut self) -> Self {
        self.config.unify_prolonged = true;
        self
    }

    /// 繰り返し短縮を有効化。`limit` は残す最大連続数。
    pub fn repeat(mut self, limit: usize) -> Self {
        self.config.repeat_limit = Some(limit);
        self
    }

    /// 空白畳み込みを有効化。
    pub fn collapse_spaces(mut self) -> Self {
        self.config.collapse_spaces = true;
        self
    }

    /// 前後トリムを有効化。
    pub fn trim(mut self) -> Self {
        self.config.trim = true;
        self
    }

    /// 機種依存文字展開を有効化。
    pub fn expand_cjk_compat(mut self) -> Self {
        self.config.expand_cjk_compat = true;
        self
    }

    /// 引用符統一を有効化。
    pub fn unify_quotes(mut self) -> Self {
        self.config.unify_quotes = true;
        self
    }

    /// URL/email/mention/hashtag を保護する。
    pub fn protect_all(mut self) -> Self {
        self.config.protect = ProtectConfig::all();
        self
    }

    /// 保護対象をまとめて指定する。
    pub fn protect(mut self, protect: ProtectConfig) -> Self {
        self.config.protect = protect;
        self
    }

    /// URL を保護する。
    pub fn protect_urls(mut self) -> Self {
        self.config.protect.urls = true;
        self
    }

    /// メールアドレスを保護する。
    pub fn protect_emails(mut self) -> Self {
        self.config.protect.emails = true;
        self
    }

    /// `@mention` を保護する。
    pub fn protect_mentions(mut self) -> Self {
        self.config.protect.mentions = true;
        self
    }

    /// `#hashtag` を保護する。
    pub fn protect_hashtags(mut self) -> Self {
        self.config.protect.hashtags = true;
        self
    }

    /// URL を `<...>` で囲む。保護も同時に有効化する。
    ///
    /// RFC 3986 Appendix C 準拠の区切りで、Markdown/Slack では自動リンク化される。
    pub fn wrap_urls_angle(self) -> Self {
        self.wrap_urls("<", ">")
    }

    /// URL を任意の prefix/suffix で囲む。保護も同時に有効化する。
    pub fn wrap_urls(
        mut self,
        prefix: impl Into<Cow<'static, str>>,
        suffix: impl Into<Cow<'static, str>>,
    ) -> Self {
        self.config.protect.urls = true;
        self.config.url_wrap = Some((prefix.into(), suffix.into()));
        self
    }

    /// 絵文字を削除する。
    pub fn remove_emoji(mut self) -> Self {
        self.config.emoji_action = EmojiAction::Remove;
        self
    }

    /// 記号(句読点・各種シンボル)を削除する。
    pub fn remove_symbols(mut self) -> Self {
        self.config.remove_symbols = true;
        self
    }

    /// 機種依存文字を削除する(展開ではなく完全除去)。
    pub fn remove_cjk_compat(mut self) -> Self {
        self.config.remove_cjk_compat = true;
        self
    }

    /// 絵文字を指定プレースホルダに置換する。
    pub fn replace_emoji(mut self, placeholder: impl Into<Cow<'static, str>>) -> Self {
        self.config.emoji_action = EmojiAction::replace(placeholder);
        self
    }

    /// 絵文字をそのまま残す(プリセットの除去設定を打ち消す)。
    pub fn keep_emoji(mut self) -> Self {
        self.config.emoji_action = EmojiAction::Keep;
        self
    }

    /// Normalizer を構築する。
    pub fn build(self) -> Normalizer {
        let n = Normalizer::from_config(self.config);
        match self.synonyms {
            Some(dict) => n.with_synonyms(dict),
            None => n,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idempotent_for_search() {
        let n = Normalizer::preset(Preset::ForSearch);
        let once = n.normalize("ﾊﾝｶｸｶﾅ　と  全角  ！！ーーー");
        let twice = n.normalize(&once);
        assert_eq!(once, twice, "normalize must be idempotent");
    }

    #[test]
    fn builder_matches_preset() {
        let a = Normalizer::preset(Preset::ForSearch).normalize("テスト〜〜〜");
        let b = Normalizer::builder()
            .preset(Preset::ForSearch)
            .build()
            .normalize("テスト〜〜〜");
        assert_eq!(a, b);
    }

    #[test]
    fn url_is_preserved() {
        let n = Normalizer::builder()
            .halfwidth_kana_to_fullwidth()
            .protect_urls()
            .build();
        let out = n.normalize("ｶﾅ と https://example.com/PATH を見て");
        // URL はそのまま、前後は正規化
        assert!(out.contains("https://example.com/PATH"));
        assert!(out.contains("カナ"));
    }

    #[test]
    fn sanitize_invisible_all() {
        let n = Normalizer::builder().sanitize_invisible().build();
        let s = "a\u{200B}b\r\nc\u{202E}d\x07e";
        assert_eq!(n.normalize(s), "ab\ncde");
    }

    #[test]
    fn kansuji_conversion() {
        let n = Normalizer::builder().kansuji_to_arabic().build();
        assert_eq!(n.normalize("第一千二百三十四章"), "第1234章");
    }

    #[test]
    fn url_wrap_angle_default() {
        let n = Normalizer::builder().wrap_urls_angle().build();
        let out = n.normalize("見て https://example.com/a を");
        assert!(out.contains("<https://example.com/a>"));
    }

    #[test]
    fn url_wrap_custom() {
        let n = Normalizer::builder().wrap_urls("`", "`").build();
        let out = n.normalize("https://example.com");
        assert_eq!(out, "`https://example.com`");
    }

    #[test]
    fn remove_symbols_flag() {
        let n = Normalizer::builder().remove_symbols().build();
        assert_eq!(n.normalize("hello, world!「テスト」"), "hello worldテスト");
        // フラグ未指定時は削除しない
        let keep = Normalizer::builder().build();
        assert_eq!(keep.normalize("hello!"), "hello!");
    }

    #[test]
    fn remove_cjk_compat_flag() {
        let n = Normalizer::builder().remove_cjk_compat().build();
        assert_eq!(n.normalize("①②㈱kanon㌔"), "kanon");
    }

    #[test]
    fn emoji_remove() {
        let n = Normalizer::builder().remove_emoji().build();
        assert_eq!(n.normalize("hi 😀!"), "hi !");
    }

    #[test]
    fn builder_default_equals_new() {
        assert_eq!(
            NormalizerBuilder::default().build().config(),
            NormalizerBuilder::new().build().config()
        );
    }

    #[test]
    fn configure_can_disable_preset_flags() {
        let n = Normalizer::builder()
            .preset(Preset::ForSearch)
            .configure(|c| c.emoji_action = EmojiAction::Keep)
            .build();
        assert_eq!(n.normalize("ｶﾅ😀"), "カナ😀");
    }

    #[test]
    fn kana_unification() {
        let n = Normalizer::builder()
            .halfwidth_kana_to_fullwidth()
            .kata_to_hira()
            .build();
        assert_eq!(n.normalize("ﾃｽﾄとテスト"), "てすととてすと");
        let n = Normalizer::builder().hira_to_kata().build();
        assert_eq!(n.normalize("ひらがな"), "ヒラガナ");
    }

    #[test]
    fn builder_synonyms_applied() {
        let mut d = SynonymDict::new();
        d.insert("PC", "パソコン");
        let n = Normalizer::builder().synonyms(d).build();
        assert_eq!(n.normalize("PCを買う"), "パソコンを買う");
    }

    #[test]
    fn synonym_keys_are_normalized_with_same_config() {
        // for_compare は ㈱ を展開し記号を除去するので、生表記のキーはそのままでは
        // 正規化後のテキストに一致しない。キー側も正規化されることを確認する。
        let mut d = SynonymDict::new();
        d.insert("㈱サンプル", "株式会社サンプル");
        d.insert("幽☆遊☆白書", "幽遊白書");
        d.insert("☆", "無視される"); // 正規化で空になるキー
        let n = Normalizer::builder()
            .preset(Preset::ForCompare)
            .synonyms(d)
            .build();
        assert_eq!(n.normalize("㈱サンプル"), "株式会社サンプル");
        assert_eq!(n.normalize("(株)サンプル"), "株式会社サンプル");
        assert_eq!(n.normalize("幽☆遊☆白書"), "幽遊白書");
        assert_eq!(n.synonyms().unwrap().len(), 2);
    }

    #[test]
    fn url_wrap_owned_strings() {
        let prefix = String::from("[");
        let n = Normalizer::builder().wrap_urls(prefix, "]").build();
        assert_eq!(n.normalize("https://example.com"), "[https://example.com]");
    }

    #[test]
    fn preset_roundtrip_via_str() {
        for p in Preset::ALL {
            assert_eq!(p.as_str().parse::<Preset>().unwrap(), p);
        }
        assert!("bogus".parse::<Preset>().is_err());
    }

    #[test]
    fn tier1_flags_via_builder() {
        let n = Normalizer::builder()
            .kyujitai_to_shinjitai()
            .unify_itaiji()
            .expand_iteration_marks()
            .unify_loanword_kana()
            .strip_trailing_prolonged()
            .lowercase()
            .cjk_spacing(CjkSpacing::Remove)
            .kansuji_to_arabic()
            .era_to_western()
            .build();
        assert_eq!(n.normalize("髙橋さんの舊車"), "高橋さんの旧車");
        assert_eq!(
            n.normalize("人々のヴァイオリンとサーバー"),
            "人人のバイオリンとサーバ"
        );
        assert_eq!(n.normalize("Python と Rust"), "pythonとrust");
        assert_eq!(n.normalize("令和六年に1万2千円"), "2024年に12000円");
    }

    #[test]
    fn for_search_lowercases_and_unifies_loanwords() {
        let n = Normalizer::preset(Preset::ForSearch);
        assert_eq!(n.normalize("ＰＹＴＨＯＮ入門"), "python入門");
        assert_eq!(
            n.normalize("コンピューターのウェブサーバー"),
            "コンピュータのウエブサーバ"
        );
        assert_eq!(n.normalize("佐々木と渡邊"), "佐佐木と渡辺");
    }

    #[test]
    fn segments_expose_protected_ranges() {
        let n = Normalizer::builder().protect_all().build();
        let r = n.normalize_with_segments("@alice と https://example.com だよ");
        let has_mention = r.segments.iter().any(|s| {
            matches!(
                s,
                Segment::Protected {
                    kind: ProtectKind::Mention,
                    ..
                }
            )
        });
        let has_url = r.segments.iter().any(|s| {
            matches!(
                s,
                Segment::Protected {
                    kind: ProtectKind::Url,
                    ..
                }
            )
        });
        assert!(has_mention && has_url);
    }
}
