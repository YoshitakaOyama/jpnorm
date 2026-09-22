//! 正規化テストケースの出力をダンプする。
//! 目的: tests/normalize_many.rs の期待値を生成するための確認用。
//!
//! 実行: `cargo run --example dump_normalize_cases -p jpnorm-core`

use jpnorm_core::Normalizer;
use jpnorm_core::test_fixtures::NORMALIZE_CASES;

fn main() {
    for (i, (preset, input)) in NORMALIZE_CASES.iter().enumerate() {
        let out = Normalizer::preset(*preset).normalize(input);
        println!("{i}\t{preset}\t{input:?}\t{out:?}");
    }
}
