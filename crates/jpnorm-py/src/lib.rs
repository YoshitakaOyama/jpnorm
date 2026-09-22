//! jpnorm の Python バインディング。
//!
//! `import jpnorm` で利用する。Python 側の公開 API は `python/jpnorm/__init__.py` と
//! 型スタブ `python/jpnorm/__init__.pyi` を参照。

use std::path::{Path, PathBuf};

use jpnorm_core::{
    Config, ConfigError, ConfigValue, Normalizer as CoreNormalizer, Preset, SynonymDict,
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

/// Python の値を `ConfigValue` に変換する。bool は int より先に判定する (Python の bool は int の派生)。
fn py_to_value(key: &str, v: &Bound<'_, PyAny>) -> PyResult<ConfigValue> {
    if v.is_none() {
        return Ok(ConfigValue::None);
    }
    if let Ok(b) = v.extract::<bool>() {
        return Ok(ConfigValue::Bool(b));
    }
    if let Ok(n) = v.extract::<usize>() {
        return Ok(ConfigValue::Int(n));
    }
    if v.extract::<i64>().is_ok() {
        return Err(PyTypeError::new_err(format!(
            "option {key:?} must be a non-negative int"
        )));
    }
    if let Ok(s) = v.extract::<String>() {
        return Ok(ConfigValue::Str(s));
    }
    if let Ok((a, b)) = v.extract::<(String, String)>() {
        return Ok(ConfigValue::Pair(a, b));
    }
    Err(PyTypeError::new_err(format!(
        "option {key:?} has unsupported type {}",
        v.get_type().name()?
    )))
}

fn value_to_py<'py>(py: Python<'py>, v: ConfigValue) -> PyResult<Bound<'py, PyAny>> {
    Ok(match v {
        ConfigValue::Bool(b) => b.into_pyobject(py)?.to_owned().into_any(),
        ConfigValue::Int(n) => n.into_pyobject(py)?.into_any(),
        ConfigValue::Str(s) => s.into_pyobject(py)?.into_any(),
        ConfigValue::Pair(a, b) => PyTuple::new(py, [a, b])?.into_any(),
        ConfigValue::None => py.None().into_bound(py),
    })
}

fn config_err(e: ConfigError) -> PyErr {
    match e {
        ConfigError::UnknownKey(k) => PyTypeError::new_err(format!(
            "unknown option {k:?}; see Normalizer.config for valid keys"
        )),
        ConfigError::WrongType { .. } => PyTypeError::new_err(e.to_string()),
        _ => PyValueError::new_err(e.to_string()),
    }
}

/// `Config` を kwargs 互換の dict に変換する(`Normalizer(**n.config)` で復元できる)。
fn config_to_dict<'py>(py: Python<'py>, cfg: &Config) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new(py);
    for (k, v) in cfg.entries() {
        d.set_item(k, value_to_py(py, v)?)?;
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
                let value = py_to_value(&key, &v)?;
                config.set(&key, value).map_err(config_err)?;
            }
        }
        config.validate().map_err(config_err)?;
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
