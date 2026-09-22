//! L3: 語彙レベルの正規化。
//!
//! 表記ゆれ・略語・旧字新字・送り仮名ゆれを、キー/正規形 のペア辞書で
//! 解決する基盤を提供する。
//!
//! `SynonymDict` は Aho-Corasick (daachorse) による leftmost-longest 置換を提供する。
//! Sudachi 同義語辞書のローダは `jpnorm-dict` クレートにある(辞書本体はバンドルしない)。

pub mod synonym;

pub use synonym::{SynonymDict, SynonymDictError};
