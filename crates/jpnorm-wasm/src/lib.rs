//! jpnorm の WebAssembly バインディング。ブラウザ上のプレイグラウンド用。
//!
//! JavaScript 側の API:
//!
//! - `presets(): string[]`
//! - `presetConfig(name): object` — プリセットの設定を `{key: value}` で返す
//! - `normalize(text, options, dictJson?): string` — `options` は `{preset?, ...flags}`
//! - `version(): string`

use jpnorm_core::{Config, ConfigValue, Normalizer, Preset, SynonymDict};
use serde_json::{Map, Value};
use wasm_bindgen::prelude::*;

/// 利用可能なプリセット名。
#[wasm_bindgen]
pub fn presets() -> Vec<String> {
    Preset::ALL.iter().map(|p| p.as_str().to_owned()).collect()
}

/// プリセットの設定を JS オブジェクトで返す。
#[wasm_bindgen(js_name = presetConfig)]
pub fn preset_config(name: &str) -> Result<JsValue, JsError> {
    let preset: Preset = name.parse().map_err(|e| JsError::new(&format!("{e}")))?;
    let cfg = Config::from_preset(preset);
    // 既定の Serializer は JSON オブジェクトを JS の Map にするので、プレーンオブジェクトにする。
    use serde::Serialize;
    config_to_json(&cfg)
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| JsError::new(&e.to_string()))
}

/// バージョン文字列。
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// テキストを正規化する。
///
/// `options` は `{ preset?: string, ...flags }`。flags のキーは `Config::KEYS` と同じ。
/// `dict_json` は `{"正規形": ["表記ゆれ", ...]}` 形式の JSON 文字列 (省略可)。
#[wasm_bindgen]
pub fn normalize(
    text: &str,
    options: JsValue,
    dict_json: Option<String>,
) -> Result<String, JsError> {
    let opts: Value = if options.is_undefined() || options.is_null() {
        Value::Object(Map::new())
    } else {
        serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))?
    };
    let normalizer = build_normalizer(&opts, dict_json.as_deref())?;
    Ok(normalizer.normalize(text))
}

fn build_normalizer(opts: &Value, dict_json: Option<&str>) -> Result<Normalizer, JsError> {
    let obj = opts
        .as_object()
        .ok_or_else(|| JsError::new("options must be an object"))?;
    let mut cfg = match obj.get("preset").and_then(Value::as_str) {
        Some(name) => Config::from_preset(
            name.parse::<Preset>()
                .map_err(|e| JsError::new(&format!("{e}")))?,
        ),
        None => Config::default(),
    };
    for (k, v) in obj {
        if k == "preset" {
            continue;
        }
        cfg.set(k, json_to_value(v)?)
            .map_err(|e| JsError::new(&e.to_string()))?;
    }
    cfg.validate().map_err(|e| JsError::new(&e.to_string()))?;
    let n = Normalizer::from_config(cfg);
    match dict_json.map(str::trim).filter(|s| !s.is_empty()) {
        Some(json) => {
            let dict = SynonymDict::from_json_grouped(json)
                .map_err(|e| JsError::new(&format!("custom dict: {e}")))?;
            Ok(n.with_synonyms(dict))
        }
        None => Ok(n),
    }
}

fn json_to_value(v: &Value) -> Result<ConfigValue, JsError> {
    Ok(match v {
        Value::Null => ConfigValue::None,
        Value::Bool(b) => ConfigValue::Bool(*b),
        Value::Number(n) => ConfigValue::Int(
            n.as_u64()
                .ok_or_else(|| JsError::new("numeric option must be a non-negative integer"))?
                as usize,
        ),
        Value::String(s) => ConfigValue::Str(s.clone()),
        Value::Array(items) => match items.as_slice() {
            [Value::String(a), Value::String(b)] => ConfigValue::Pair(a.clone(), b.clone()),
            _ => return Err(JsError::new("pair option must be [prefix, suffix]")),
        },
        Value::Object(_) => return Err(JsError::new("option value cannot be an object")),
    })
}

fn config_to_json(cfg: &Config) -> Value {
    let mut m = Map::new();
    for (k, v) in cfg.entries() {
        let j = match v {
            ConfigValue::Bool(b) => Value::Bool(b),
            ConfigValue::Int(n) => Value::from(n as u64),
            ConfigValue::Str(s) => Value::String(s),
            ConfigValue::Pair(a, b) => Value::Array(vec![Value::String(a), Value::String(b)]),
            ConfigValue::None => Value::Null,
        };
        m.insert(k.to_owned(), j);
    }
    Value::Object(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_from_json_options() {
        let opts =
            serde_json::json!({"preset": "for_search", "emoji": "keep", "url_wrap": ["<", ">"]});
        let n = build_normalizer(&opts, Some(r#"{"パソコン": ["PC"]}"#)).unwrap();
        assert_eq!(n.normalize("PC😀 https://x.y"), "パソコン😀 <https://x.y>");
    }

    #[test]
    fn config_json_roundtrip() {
        let cfg = Config::for_compare();
        let j = config_to_json(&cfg);
        let mut back = Config::none();
        for (k, v) in j.as_object().unwrap() {
            back.set(k, json_to_value(v).unwrap()).unwrap();
        }
        assert_eq!(back, cfg);
    }
}
