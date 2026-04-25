//! `jpnorm-core` のスループット計測ベンチ。
//!
//! 代表的な日本語テキスト(ツイート風・長文・混在記号)に対して
//! 各プリセットを適用した時の処理時間を計測する。

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use jpnorm_core::{Normalizer, Preset};

/// 小さめのサンプル(ツイート風)。
const TWEET: &str = "ｶﾀｶﾅ と  全角 ！！ーーー 〜〜〜 wwwwww https://example.com/path?x=1 @alice #rust_lang 😀";

/// 中くらいのサンプル(段落)。
const PARAGRAPH: &str = "\
ある日ﾊﾝｶｸｶﾅが混ざった文章を見つけた。ＡＢＣ１２３のような全角英数や、\n\
「曲線引用符」と\"直線引用符\"、ハイフン類(‐—−)も入り乱れていた。\n\
ウェーーーイ、ーーーっていう繰り返しや   複数スペース  も頻出。\n\
連絡先は foo.bar@example.co.jp、詳しくは https://example.com/docs?lang=ja を参照。\n\
#jpnorm @dev 🎉🎉🎉";

fn make_large() -> String {
    // ~32KB 程度の日本語混在コーパス
    PARAGRAPH.repeat(64)
}

fn bench_presets(c: &mut Criterion) {
    let large = make_large();
    let samples: &[(&str, &str)] = &[
        ("tweet", TWEET),
        ("paragraph", PARAGRAPH),
        ("large_32k", large.as_str()),
    ];
    let presets: &[(&str, Preset)] = &[
        ("neologdn_compat", Preset::NeologdnCompat),
        ("for_search", Preset::ForSearch),
        ("for_display", Preset::ForDisplay),
        ("for_compare", Preset::ForCompare),
    ];

    let mut g = c.benchmark_group("normalize");
    for (sname, sample) in samples {
        g.throughput(Throughput::Bytes(sample.len() as u64));
        for (pname, preset) in presets {
            let n = Normalizer::preset(*preset);
            let id = BenchmarkId::new(*pname, sname);
            g.bench_with_input(id, sample, |b, text| {
                b.iter(|| {
                    let out = n.normalize(black_box(text));
                    black_box(out);
                });
            });
        }
    }
    g.finish();
}

criterion_group!(benches, bench_presets);
criterion_main!(benches);
