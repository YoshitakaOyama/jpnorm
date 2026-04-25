//! jpnorm-dict — jpnorm 用の辞書ローダ。
//!
//! 現状は Sudachi 同義語辞書 (`synonyms.txt`) のパーサを提供する。
//! 実データは Apache-2.0 ライセンスの
//! <https://github.com/WorksApplications/SudachiDict> から取得して使う想定。

#![forbid(unsafe_code)]

pub mod sudachi;

pub use sudachi::{load_sudachi_synonyms, SudachiParseError};
