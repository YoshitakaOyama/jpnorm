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
#[doc(hidden)]
pub mod test_fixtures;

pub use config::{Config, ConfigError, ConfigValue, ParsePresetError, Preset};
pub use l1_char::case::CaseAction;
pub use l1_char::spacing::CjkSpacing;
pub use l2_script::kana::KanaAction;
pub use l3_lexical::{SynonymDict, SynonymDictError};
pub use l4_extra::emoji::EmojiAction;
pub use l4_extra::protect::{Kind as ProtectKind, ProtectConfig};
pub use pipeline::{NormalizedText, Normalizer, NormalizerBuilder, Segment};
