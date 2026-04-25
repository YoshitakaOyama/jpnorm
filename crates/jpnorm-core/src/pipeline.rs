//! 正規化パイプライン。Builder で構築する。

use crate::config::{Config, Preset};
use crate::l1_char;
use crate::l2_script::{numbers, numerals};
use crate::l3_lexical::SynonymDict;
use crate::l4_extra::emoji::{self, EmojiAction};
use crate::l4_extra::protect::{self, Kind as ProtectKind};

/// 正規化結果のセグメント情報。
///
/// 出力テキストがどの種別のセグメント(正規化済みか、保護パススルーか)で
/// 構成されているかを表す。デバッグや検索ハイライトに利用する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    /// 正規化が適用された通常のセグメント。
    Normalized(String),
    /// 保護領域(URL/email/mention/hashtag)で元文字列のまま。
    Protected { text: String, kind: ProtectKind },
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
    pub fn with_synonyms(mut self, dict: SynonymDict) -> Self {
        self.synonyms = Some(dict);
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

        // 2) 各セグメントを処理して連結
        let mut out_text = String::with_capacity(input.len());
        let mut segments: Vec<Segment> = Vec::with_capacity(segs.len());
        for seg in segs {
            match seg {
                Ok(plain) => {
                    let normalized = self.apply_to_free(plain);
                    if !normalized.is_empty() {
                        out_text.push_str(&normalized);
                        segments.push(Segment::Normalized(normalized));
                    }
                }
                Err((protected, kind)) => {
                    let emitted = if matches!(kind, ProtectKind::Url) {
                        if let Some((prefix, suffix)) = c.url_wrap {
                            format!("{prefix}{protected}{suffix}")
                        } else {
                            protected.to_owned()
                        }
                    } else {
                        protected.to_owned()
                    };
                    out_text.push_str(&emitted);
                    segments.push(Segment::Protected {
                        text: emitted,
                        kind,
                    });
                }
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

        if c.expand_cjk_compat {
            s = l1_char::cjk_compat::expand(&s);
        }
        if c.nfkc {
            s = l1_char::unicode::nfkc(&s);
        }
        if c.halfwidth_kana_to_fullwidth {
            s = l1_char::width::halfwidth_kana_to_fullwidth(&s);
        }
        if c.unify_quotes {
            s = l1_char::quotes::unify(&s);
        }
        if c.unify_hyphens || c.unify_tildes || c.unify_prolonged {
            s = l1_char::symbols::unify(
                &s,
                c.unify_hyphens,
                c.unify_tildes,
                c.unify_prolonged,
            );
        }
        if c.collapse_prolonged_run {
            s = l1_char::prolonged::collapse(&s);
        }
        if !matches!(c.emoji_action, EmojiAction::Keep) {
            s = emoji::process(&s, c.emoji_action);
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
        if c.collapse_spaces {
            s = l1_char::spaces::collapse(&s);
        }
        if c.trim {
            s = s.trim().to_owned();
        }
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
#[derive(Debug, Clone, Default)]
pub struct NormalizerBuilder {
    config: Config,
}

impl NormalizerBuilder {
    /// 空の Builder。
    pub fn new() -> Self {
        Self {
            config: Config::none(),
        }
    }

    /// プリセットをベースに適用する。
    pub fn preset(mut self, preset: Preset) -> Self {
        self.config = Config::from_preset(preset);
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
        self.config.protect = crate::l4_extra::protect::ProtectConfig::all();
        self
    }

    /// URL のみ保護する。
    pub fn protect_urls(mut self) -> Self {
        self.config.protect.urls = true;
        self
    }

    /// URL を `<...>` で囲む。保護も同時に有効化する。
    ///
    /// RFC 3986 Appendix C 準拠の区切りで、Markdown/Slack では自動リンク化される。
    pub fn wrap_urls_angle(mut self) -> Self {
        self.config.protect.urls = true;
        self.config.url_wrap = Some(("<", ">"));
        self
    }

    /// URL を任意の prefix/suffix で囲む。保護も同時に有効化する。
    pub fn wrap_urls(mut self, prefix: &'static str, suffix: &'static str) -> Self {
        self.config.protect.urls = true;
        self.config.url_wrap = Some((prefix, suffix));
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
    pub fn replace_emoji(mut self, placeholder: &'static str) -> Self {
        self.config.emoji_action = EmojiAction::Replace(placeholder);
        self
    }

    /// Normalizer を構築する。
    pub fn build(self) -> Normalizer {
        Normalizer::from_config(self.config)
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
    fn segments_expose_protected_ranges() {
        let n = Normalizer::builder().protect_all().build();
        let r = n.normalize_with_segments("@alice と https://example.com だよ");
        let has_mention = r
            .segments
            .iter()
            .any(|s| matches!(s, Segment::Protected { kind: ProtectKind::Mention, .. }));
        let has_url = r
            .segments
            .iter()
            .any(|s| matches!(s, Segment::Protected { kind: ProtectKind::Url, .. }));
        assert!(has_mention && has_url);
    }
}
