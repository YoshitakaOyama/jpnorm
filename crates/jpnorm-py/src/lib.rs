//! jpnorm の Python バインディング。
//!
//! `import jpnorm` で利用する。Python 側の公開 API は `python/jpnorm/__init__.py` と
//! 型スタブ `python/jpnorm/__init__.pyi` を参照。

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use jpnorm_core::{
    Config, EmojiAction, KanaAction, Normalizer as CoreNormalizer, Preset, SynonymDict,
};
use pyo3::exceptions::{PyIOError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString, PyTuple};

/// Python に公開する Normalizer。
#[pyclass(name = "Normalizer", module = "jpnorm._native")]
struct PyNormalizer {
    inner: CoreNormalizer,
}

fn parse_preset(name: &str) -> PyResult<Preset> {
    name.parse::<Preset>()
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

fn extract_bool(key: &str, value: &Bound<'_, PyAny>) -> PyResult<bool> {
    value
        .extract::<bool>()
        .map_err(|_| PyTypeError::new_err(format!("option {key:?} must be bool")))
}

fn extract_str(key: &str, value: &Bound<'_, PyAny>) -> PyResult<String> {
    value
        .extract::<String>()
        .map_err(|_| PyTypeError::new_err(format!("option {key:?} must be str")))
}

/// kwargs の 1 項目を `Config` に反映する。
fn apply_option(cfg: &mut Config, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
    match key {
        "normalize_newlines" => cfg.normalize_newlines = extract_bool(key, value)?,
        "remove_zero_width" => cfg.remove_zero_width = extract_bool(key, value)?,
        "remove_control" => cfg.remove_control = extract_bool(key, value)?,
        "remove_bidi_control" => cfg.remove_bidi_control = extract_bool(key, value)?,
        "kansuji_to_arabic" => cfg.kansuji_to_arabic = extract_bool(key, value)?,
        "arabic_to_kansuji" => cfg.arabic_to_kansuji = extract_bool(key, value)?,
        "canonicalize_numbers" => cfg.canonicalize_numbers = extract_bool(key, value)?,
        "nfkc" => cfg.nfkc = extract_bool(key, value)?,
        "halfwidth_kana_to_fullwidth" => {
            cfg.halfwidth_kana_to_fullwidth = extract_bool(key, value)?
        }
        "kana" => {
            cfg.kana = match extract_str(key, value)?.as_str() {
                "keep" => KanaAction::Keep,
                "hira_to_kata" => KanaAction::HiraToKata,
                "kata_to_hira" => KanaAction::KataToHira,
                other => {
                    return Err(PyValueError::new_err(format!(
                        "option 'kana' must be one of 'keep', 'hira_to_kata', 'kata_to_hira' (got {other:?})"
                    )));
                }
            }
        }
        "unify_hyphens" => cfg.unify_hyphens = extract_bool(key, value)?,
        "unify_tildes" => cfg.unify_tildes = extract_bool(key, value)?,
        "unify_prolonged" => cfg.unify_prolonged = extract_bool(key, value)?,
        "collapse_prolonged_run" => cfg.collapse_prolonged_run = extract_bool(key, value)?,
        "repeat_limit" => {
            cfg.repeat_limit = if value.is_none() {
                None
            } else {
                let n = value.extract::<usize>().map_err(|_| {
                    PyTypeError::new_err("option 'repeat_limit' must be int >= 1 or None")
                })?;
                if n == 0 {
                    return Err(PyValueError::new_err(
                        "option 'repeat_limit' must be >= 1 (use None to disable)",
                    ));
                }
                Some(n)
            }
        }
        "collapse_spaces" => cfg.collapse_spaces = extract_bool(key, value)?,
        "trim" => cfg.trim = extract_bool(key, value)?,
        "expand_cjk_compat" => cfg.expand_cjk_compat = extract_bool(key, value)?,
        "remove_symbols" => cfg.remove_symbols = extract_bool(key, value)?,
        "remove_cjk_compat" => cfg.remove_cjk_compat = extract_bool(key, value)?,
        "unify_quotes" => cfg.unify_quotes = extract_bool(key, value)?,
        "protect_urls" => cfg.protect.urls = extract_bool(key, value)?,
        "protect_emails" => cfg.protect.emails = extract_bool(key, value)?,
        "protect_mentions" => cfg.protect.mentions = extract_bool(key, value)?,
        "protect_hashtags" => cfg.protect.hashtags = extract_bool(key, value)?,
        "emoji" => {
            cfg.emoji_action = match extract_str(key, value)?.as_str() {
                "keep" => EmojiAction::Keep,
                "remove" => EmojiAction::Remove,
                other => {
                    return Err(PyValueError::new_err(format!(
                        "option 'emoji' must be 'keep' or 'remove' (got {other:?}); \
                         use emoji_placeholder=... to replace"
                    )));
                }
            }
        }
        "emoji_placeholder" => {
            cfg.emoji_action = if value.is_none() {
                EmojiAction::Keep
            } else {
                EmojiAction::replace(extract_str(key, value)?)
            }
        }
        "url_wrap" => {
            cfg.url_wrap = if value.is_none() {
                None
            } else {
                let (prefix, suffix) = value.extract::<(String, String)>().map_err(|_| {
                    PyTypeError::new_err(
                        "option 'url_wrap' must be a (prefix, suffix) tuple or None",
                    )
                })?;
                cfg.protect.urls = true;
                Some((Cow::Owned(prefix), Cow::Owned(suffix)))
            }
        }
        other => {
            return Err(PyTypeError::new_err(format!(
                "unknown option {other:?}; see Normalizer.config for valid keys"
            )));
        }
    }
    Ok(())
}

/// `Config` を kwargs 互換の dict に変換する(`Normalizer(**n.config)` で復元できる)。
fn config_to_dict<'py>(py: Python<'py>, cfg: &Config) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new(py);
    d.set_item("normalize_newlines", cfg.normalize_newlines)?;
    d.set_item("remove_zero_width", cfg.remove_zero_width)?;
    d.set_item("remove_control", cfg.remove_control)?;
    d.set_item("remove_bidi_control", cfg.remove_bidi_control)?;
    d.set_item("kansuji_to_arabic", cfg.kansuji_to_arabic)?;
    d.set_item("arabic_to_kansuji", cfg.arabic_to_kansuji)?;
    d.set_item("canonicalize_numbers", cfg.canonicalize_numbers)?;
    d.set_item("nfkc", cfg.nfkc)?;
    d.set_item(
        "halfwidth_kana_to_fullwidth",
        cfg.halfwidth_kana_to_fullwidth,
    )?;
    d.set_item(
        "kana",
        match cfg.kana {
            KanaAction::Keep => "keep",
            KanaAction::HiraToKata => "hira_to_kata",
            KanaAction::KataToHira => "kata_to_hira",
        },
    )?;
    d.set_item("unify_hyphens", cfg.unify_hyphens)?;
    d.set_item("unify_tildes", cfg.unify_tildes)?;
    d.set_item("unify_prolonged", cfg.unify_prolonged)?;
    d.set_item("collapse_prolonged_run", cfg.collapse_prolonged_run)?;
    d.set_item("repeat_limit", cfg.repeat_limit)?;
    d.set_item("collapse_spaces", cfg.collapse_spaces)?;
    d.set_item("trim", cfg.trim)?;
    d.set_item("expand_cjk_compat", cfg.expand_cjk_compat)?;
    d.set_item("remove_symbols", cfg.remove_symbols)?;
    d.set_item("remove_cjk_compat", cfg.remove_cjk_compat)?;
    d.set_item("unify_quotes", cfg.unify_quotes)?;
    d.set_item("protect_urls", cfg.protect.urls)?;
    d.set_item("protect_emails", cfg.protect.emails)?;
    d.set_item("protect_mentions", cfg.protect.mentions)?;
    d.set_item("protect_hashtags", cfg.protect.hashtags)?;
    match &cfg.emoji_action {
        EmojiAction::Keep => {
            d.set_item("emoji", "keep")?;
            d.set_item("emoji_placeholder", py.None())?;
        }
        EmojiAction::Remove => {
            d.set_item("emoji", "remove")?;
            d.set_item("emoji_placeholder", py.None())?;
        }
        EmojiAction::Replace(s) => {
            d.set_item("emoji", "keep")?;
            d.set_item("emoji_placeholder", s.as_ref())?;
        }
    }
    match &cfg.url_wrap {
        Some((p, s)) => d.set_item("url_wrap", PyTuple::new(py, [p.as_ref(), s.as_ref()])?)?,
        None => d.set_item("url_wrap", py.None())?,
    }
    Ok(d)
}

fn read_text_file(path: &Path) -> PyResult<String> {
    std::fs::read_to_string(path)
        .map_err(|e| PyIOError::new_err(format!("cannot read {}: {e}", path.display())))
}

#[pymethods]
impl PyNormalizer {
    /// Normalizer を生成する。
    ///
    /// - `preset`: ベースにするプリセット名。省略時は `"neologdn_compat"`。
    /// - `**options`: 個別フラグ。プリセットの設定を上書きする。
    ///   有効なキーは `Normalizer.config` の返す dict のキーと同じ。
    ///
    /// 例: `Normalizer("for_search", emoji="keep", kana="kata_to_hira")`
    #[new]
    #[pyo3(signature = (preset=None, **options))]
    fn new(preset: Option<&str>, options: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let mut config = match preset {
            Some(name) => Config::from_preset(parse_preset(name)?),
            None => Config::default(),
        };
        if let Some(opts) = options {
            for (k, v) in opts.iter() {
                let key: String = k.extract()?;
                apply_option(&mut config, &key, &v)?;
            }
        }
        if config.kansuji_to_arabic && config.arabic_to_kansuji {
            return Err(PyValueError::new_err(
                "kansuji_to_arabic and arabic_to_kansuji cannot both be enabled",
            ));
        }
        Ok(Self {
            inner: CoreNormalizer::from_config(config),
        })
    }

    /// プリセット名から生成する(`Normalizer(name)` と同じ)。
    #[staticmethod]
    fn preset(name: &str) -> PyResult<Self> {
        Ok(Self {
            inner: CoreNormalizer::preset(parse_preset(name)?),
        })
    }

    /// 利用可能なプリセット名の一覧。
    #[staticmethod]
    fn presets() -> Vec<&'static str> {
        Preset::ALL.iter().map(|p| p.as_str()).collect()
    }

    /// 現在の設定を kwargs 互換の dict で返す。`Normalizer(**n.config)` で複製できる。
    #[getter]
    fn config<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        config_to_dict(py, self.inner.config())
    }

    /// 登録済みカスタム辞書のエントリ数。
    #[getter]
    fn custom_dict_size(&self) -> usize {
        self.inner.synonyms().map_or(0, SynonymDict::len)
    }

    /// テキストを正規化する。長いテキストの処理中は GIL を解放する。
    fn normalize(&self, py: Python<'_>, text: &str) -> String {
        py.detach(|| self.inner.normalize(text))
    }

    /// 複数テキストをまとめて正規化する。処理中は GIL を解放する。
    fn normalize_batch(&self, py: Python<'_>, texts: Vec<String>) -> Vec<String> {
        py.detach(|| self.inner.normalize_batch(&texts))
    }

    /// カスタム辞書を追加する。既存のエントリにマージされる。
    ///
    /// `mapping` は `{正規形: [表記ゆれ, ...]}` 形式の dict。
    /// 例: `{"幽遊白書": ["幽白", "ゆうはく", "幽☆遊☆白書"]}`
    ///
    /// 与えられた表記ゆれは正規化の最終段で正規形に置換される。
    /// メソッドチェーンできるよう自身を返す。
    fn with_custom_dict<'py>(
        mut slf: PyRefMut<'py, Self>,
        mapping: &Bound<'py, PyDict>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let mut dict = SynonymDict::new();
        for (k, v) in mapping.iter() {
            let canonical: String = k
                .extract()
                .map_err(|_| PyTypeError::new_err("custom dict keys must be str"))?;
            if v.is_instance_of::<PyString>() {
                return Err(PyTypeError::new_err(format!(
                    "custom dict value for {canonical:?} must be a list of str, not str"
                )));
            }
            let variants = v.try_iter().map_err(|_| {
                PyTypeError::new_err(format!(
                    "custom dict value for {canonical:?} must be an iterable of str"
                ))
            })?;
            for item in variants {
                let variant: String = item?
                    .extract()
                    .map_err(|_| PyTypeError::new_err("custom dict variants must be str"))?;
                if variant != canonical {
                    dict.insert(variant, canonical.clone());
                }
            }
        }
        slf.merge_synonyms(dict);
        Ok(slf)
    }

    /// カスタム辞書を JSON 文字列から読み込んで追加する。
    ///
    /// フォーマットは `{"正規形": ["表記ゆれ", ...]}`。`json.dumps()` の既定出力
    /// (`\uXXXX` エスケープ)もそのまま読める。
    fn load_custom_dict_json<'py>(
        mut slf: PyRefMut<'py, Self>,
        json: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let dict = SynonymDict::from_json_grouped(json)
            .map_err(|e| PyValueError::new_err(format!("invalid custom dict json: {e}")))?;
        slf.merge_synonyms(dict);
        Ok(slf)
    }

    /// カスタム辞書をファイルから読み込んで追加する。
    ///
    /// `format` 省略時は拡張子で判定する:
    /// - `.json`: `{"正規形": ["表記ゆれ", ...]}`
    /// - `.csv`: `表記ゆれ,正規形` (1行1件、`#` 行はコメント)
    /// - `.tsv` / `.txt`: `表記ゆれ<TAB>正規形`
    #[pyo3(signature = (path, format=None))]
    fn load_custom_dict_file<'py>(
        mut slf: PyRefMut<'py, Self>,
        path: PathBuf,
        format: Option<&str>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let fmt = match format {
            Some(f) => f.to_ascii_lowercase(),
            None => path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
                .unwrap_or_default(),
        };
        let loader: fn(&str) -> Result<SynonymDict, _> = match fmt.as_str() {
            "json" => SynonymDict::from_json_grouped,
            "csv" => |t| SynonymDict::from_delimited(t, ','),
            "tsv" | "txt" => |t| SynonymDict::from_delimited(t, '\t'),
            other => {
                return Err(PyValueError::new_err(format!(
                    "unknown custom dict format {other:?} (expected json, csv, tsv)"
                )));
            }
        };
        let text = read_text_file(&path)?;
        let dict = loader(&text).map_err(|e| {
            PyValueError::new_err(format!("invalid custom dict {}: {e}", path.display()))
        })?;
        slf.merge_synonyms(dict);
        Ok(slf)
    }

    /// Sudachi 同義語辞書 (`synonyms.txt`) を読み込んで追加する。
    ///
    /// 辞書本体はバンドルしていないので、SudachiDict から別途ダウンロードすること。
    fn load_sudachi_synonyms<'py>(
        mut slf: PyRefMut<'py, Self>,
        path: PathBuf,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let text = read_text_file(&path)?;
        let dict = jpnorm_dict::load_sudachi_synonyms(&text).map_err(|e| {
            PyValueError::new_err(format!("invalid sudachi synonyms {}: {e}", path.display()))
        })?;
        slf.merge_synonyms(dict);
        Ok(slf)
    }

    /// カスタム辞書をすべて削除する。
    fn clear_custom_dict<'py>(mut slf: PyRefMut<'py, Self>) -> PyRefMut<'py, Self> {
        let config = slf.inner.config().clone();
        slf.inner = CoreNormalizer::from_config(config);
        slf
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let cfg = self.inner.config();
        let name = Preset::ALL
            .iter()
            .find(|p| Config::from_preset(**p) == *cfg)
            .map(|p| format!("preset='{}'", p.as_str()));
        let body = match name {
            Some(n) => n,
            None => {
                let d = config_to_dict(py, cfg)?;
                let mut parts = Vec::new();
                for (k, v) in d.iter() {
                    parts.push(format!("{}={}", k, v.repr()?));
                }
                parts.join(", ")
            }
        };
        let dict = self.custom_dict_size();
        if dict > 0 {
            Ok(format!("Normalizer({body}, custom_dict_size={dict})"))
        } else {
            Ok(format!("Normalizer({body})"))
        }
    }
}

impl PyNormalizer {
    fn merge_synonyms(&mut self, new: SynonymDict) {
        let mut merged = self.inner.synonyms().cloned().unwrap_or_default();
        merged.extend(new);
        self.inner = self.inner.clone().with_synonyms(merged);
    }
}

/// トップレベル関数: プリセット(既定は `neologdn_compat`)でテキストを正規化する。
#[pyfunction]
#[pyo3(signature = (text, preset=None))]
fn normalize(py: Python<'_>, text: &str, preset: Option<&str>) -> PyResult<String> {
    let n = match preset {
        Some(name) => CoreNormalizer::preset(parse_preset(name)?),
        None => CoreNormalizer::default(),
    };
    Ok(py.detach(|| n.normalize(text)))
}

/// 2 文字列のレーベンシュタイン距離(文字単位)。
#[pyfunction]
fn levenshtein(py: Python<'_>, a: &str, b: &str) -> usize {
    py.detach(|| levenshtein_impl(a, b))
}

fn levenshtein_impl(a: &str, b: &str) -> usize {
    if a == b {
        return 0;
    }
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

#[pymodule(gil_used = false)]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyNormalizer>()?;
    m.add_function(wrap_pyfunction!(normalize, m)?)?;
    m.add_function(wrap_pyfunction!(levenshtein, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::levenshtein_impl;

    #[test]
    fn levenshtein_basic() {
        assert_eq!(levenshtein_impl("kitten", "sitting"), 3);
        assert_eq!(levenshtein_impl("", "abc"), 3);
        assert_eq!(levenshtein_impl("同じ", "同じ"), 0);
        assert_eq!(levenshtein_impl("東京", "東京都"), 1);
    }
}
