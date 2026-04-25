//! L3: 語彙レベルの正規化。
//!
//! 表記ゆれ・略語・旧字新字・送り仮名ゆれを、キー/正規形 のペア辞書で
//! 解決する基盤を提供する。
//!
//! M3 では最小限の実装として、`SynonymDict` による longest-match 置換を提供する。
//! 高速化(Aho-Corasick, FST)や Sudachi 同義語辞書のバンドルは次のマイルストーン。

pub mod synonym;

pub use synonym::{SynonymDict, SynonymDictError};
