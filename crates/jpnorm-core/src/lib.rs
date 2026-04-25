//! jpnorm-core — 世界一の日本語正規化ライブラリのコア。
//!
//! ## 使い方
//!
//! ```
//! use jpnorm_core::{Normalizer, Preset};
//!
//! let n = Normalizer::preset(Preset::ForSearch);
//! let out = n.normalize("ﾊﾝｶｸｶﾅ　と  全角  ！！");
//! assert_eq!(out, "ハンカクカナ と 全角 !!");
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod config;
pub mod l1_char;
pub mod l2_script;
pub mod l3_lexical;
pub mod l4_extra;
pub mod pipeline;
pub mod test_fixtures;

pub use config::{Config, Preset};
pub use l3_lexical::{SynonymDict, SynonymDictError};
pub use pipeline::{Normalizer, NormalizerBuilder};
