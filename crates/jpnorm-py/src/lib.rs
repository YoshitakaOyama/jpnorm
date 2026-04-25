//! jpnorm の Python バインディング。
//!
//! `import jpnorm` で利用する。

use jpnorm_core::{Normalizer as CoreNormalizer, Preset, SynonymDict};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

/// Python に公開する Normalizer。
#[pyclass(name = "Normalizer", module = "jpnorm._native")]
struct PyNormalizer {
    inner: CoreNormalizer,
}

#[pymethods]
impl PyNormalizer {
    /// 既定の設定(`neologdn_compat`)で生成。
    #[new]
    fn new() -> Self {
        Self {
            inner: CoreNormalizer::default(),
        }
    }

    /// プリセット名から生成する。
    ///
    /// 有効な名前: `"none"`, `"neologdn_compat"`, `"for_search"`, `"for_display"`, `"for_compare"`
    #[staticmethod]
    fn preset(name: &str) -> PyResult<Self> {
        let p = match name {
            "none" => Preset::None,
            "neologdn_compat" => Preset::NeologdnCompat,
            "for_search" => Preset::ForSearch,
            "for_display" => Preset::ForDisplay,
            "for_compare" => Preset::ForCompare,
            other => {
                return Err(PyValueError::new_err(format!(
                    "unknown preset: {other}"
                )))
            }
        };
        Ok(Self {
            inner: CoreNormalizer::preset(p),
        })
    }

    /// カスタム辞書を設定する。
    ///
    /// `mapping` は `{正規形: [表記ゆれ, ...]}` 形式の dict。
    /// 例: `{"幽遊白書": ["幽白", "ゆうはく", "幽☆遊☆白書"]}`
    ///
    /// 与えられた表記ゆれは正規化の最終段で正規形に置換される。
    /// 既存の辞書は上書きされる。
    fn with_custom_dict(&mut self, mapping: &Bound<'_, PyDict>) -> PyResult<()> {
        let mut dict = SynonymDict::new();
        for (k, v) in mapping.iter() {
            let canonical: String = k.extract().map_err(|_| {
                PyValueError::new_err("custom dict keys must be str")
            })?;
            let variants: Bound<'_, PyList> = v.cast_into::<PyList>().map_err(|_| {
                PyValueError::new_err("custom dict values must be list[str]")
            })?;
            for item in variants.iter() {
                let variant: String = item.extract().map_err(|_| {
                    PyValueError::new_err("custom dict variant must be str")
                })?;
                if variant != canonical {
                    dict.insert(variant, canonical.clone());
                }
            }
        }
        self.inner = self.inner.clone().with_synonyms(dict);
        Ok(())
    }

    /// カスタム辞書を JSON 文字列から読み込んで設定する。
    ///
    /// フォーマットは `{"正規形": ["表記ゆれ", ...]}`。
    fn load_custom_dict_json(&mut self, json: &str) -> PyResult<()> {
        let dict = SynonymDict::from_json_grouped(json)
            .map_err(|e| PyValueError::new_err(format!("invalid custom dict json: {e}")))?;
        self.inner = self.inner.clone().with_synonyms(dict);
        Ok(())
    }

    /// テキストを正規化する。
    fn normalize(&self, text: &str) -> String {
        self.inner.normalize(text)
    }

    fn __repr__(&self) -> String {
        format!("Normalizer(config={:?})", self.inner.config())
    }
}

/// トップレベル関数: デフォルト設定で1度だけ正規化する。
#[pyfunction]
fn normalize(text: &str) -> String {
    CoreNormalizer::default().normalize(text)
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyNormalizer>()?;
    m.add_function(wrap_pyfunction!(normalize, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
