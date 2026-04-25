//! 正規化テストケースの出力をダンプする。
//! 目的: tests/normalize_many.rs の期待値を生成するための確認用。
//!
//! 実行: `cargo run --example dump_normalize_cases -p jpnorm-core`

use jpnorm_core::test_fixtures::NORMALIZE_CASES;
use jpnorm_core::{Normalizer, Preset};

fn main() {
    for (i, (preset, input)) in NORMALIZE_CASES.iter().enumerate() {
        let out = Normalizer::preset(*preset).normalize(input);
        println!("{}\t{:?}\t{:?}\t{:?}", i, preset_name(*preset), input, out);
    }
}

fn preset_name(p: Preset) -> &'static str {
    match p {
        Preset::None => "None",
        Preset::NeologdnCompat => "NeologdnCompat",
        Preset::ForSearch => "ForSearch",
        Preset::ForDisplay => "ForDisplay",
        Preset::ForCompare => "ForCompare",
    }
}
