# jpnorm

English / [日本語](README.md)

[![PyPI](https://img.shields.io/pypi/v/jpnorm)](https://pypi.org/project/jpnorm/)
[![Python](https://img.shields.io/pypi/pyversions/jpnorm)](https://pypi.org/project/jpnorm/)
[![CI](https://github.com/YoshitakaOyama/jpnorm/actions/workflows/ci.yml/badge.svg)](https://github.com/YoshitakaOyama/jpnorm/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

**A one-line Japanese text normalization library that adapts to your use case.**
It uses a Rust core with Python bindings. Start with a preset, then tune individual flags when you need more control.

Use it as a neologdn replacement, a preprocessing step for search and RAG, a helper for evaluating LLM/OCR/ASR output, or a way to deduplicate Japanese names and product strings.

```python
import jpnorm

jpnorm.normalize("ﾊﾝｶｸｶﾅ　と  全角  ！！")
# => 'ハンカクカナ と 全角 !!'
```

**Try it in your browser:** <https://yoshitakaoyama.github.io/jpnorm/>. Paste text and compare the output from every preset.

## When to use it

Japanese text often has many written forms for the same meaning: half-width and full-width kana, full-width and half-width alphanumerics, `ー` / `〜` / `-`, `㈱` / `(株)`, `三百二十円` / `320円`, or title abbreviations such as `幽☆遊☆白書` / `幽白`.

| Problem | What happens | What jpnorm does |
|---|---|---|
| Search or RAG misses documents that should match | The indexed text and the query use different spellings | Normalize both with `for_search` before indexing and querying |
| LLM, OCR, or speech output looks correct but fails exact matching | Width, symbol, kanji numeral, or whitespace differences break string comparison | Normalize with `for_compare`, then call `compare()` |
| Customer, product, or content names are hard to deduplicate | Internal abbreviations and spelling variants differ from the canonical name | Use a custom dictionary such as `幽白 -> 幽遊白書` |
| User-generated text looks broken in a UI | Half-width kana, invisible control characters, or mixed forms affect display | Use `for_display` to clean only what is safe for presentation |
| neologdn is useful but too fixed | You may need URL protection, emoji handling, or smaller changes | Start with `neologdn_compat` and adjust flags as needed |

## Installation

```bash
pip install jpnorm
# or
uv add jpnorm
```

jpnorm supports Python 3.10 or later. Wheels are published for Linux (x86_64 / aarch64), macOS (x86_64 / arm64), and Windows (x64), so you normally do not need a Rust toolchain.

If you only want the command-line tool, install it with:

```bash
pipx install jpnorm
```

## Use-case guide

### 1. Search and RAG preprocessing (`for_search`)

Use the same normalizer when you build the index and when you normalize queries. This preset protects URLs and email addresses, removes emoji, and normalizes symbol variants.

```python
from jpnorm import Normalizer

n = Normalizer("for_search")

docs = ["ＰＹＴＨＯＮ入門 〜〜 初心者向け🔰", "python 入門（初心者向け）"]
n.normalize_batch(docs)
# => ['PYTHON入門 〜 初心者向け', 'python 入門(初心者向け)']

n.normalize("詳細は https://example.com/Docs?Q=1 を参照 📎")
# => '詳細は https://example.com/Docs?Q=1 を参照'
```

jpnorm does not change letter case because case handling depends on your application. Add `.lower()` if your search pipeline needs it.

### 2. Evaluating LLM, OCR, and speech output (`for_compare` + `compare`)

Use this when you want to know whether a model output is effectively the same as a reference. The preset removes differences in width, symbols, kanji numerals, and spacing before comparison. Then you can compare with exact match, prefix match, edit distance, or an LLM judge.

```python
from jpnorm import Normalizer, compare

n = Normalizer("for_compare")

n.normalize("合計：￥１，２００（税込）")    # => '合計¥1200税込'
n.normalize("合計:¥1,200(税込)")            # => '合計¥1200税込'
n.normalize("第一章　はじめに")             # => '第1章 はじめに'

compare("ﾃｽﾄ結果：１２３", "テスト結果:123", strategy="exact", normalizer=n).matched
# => True

r = compare("東京都渋谷区", "東京都渋谷区神南1-2-3", strategy="prefix", normalizer=n)
r.matched, r.detail["direction"]
# => (True, 'prediction_is_prefix_of_reference')

r = compare("kitten", "sitting", strategy="edit_distance", threshold=0.5)
r.score   # => 0.571...  (1 - edit distance / length)
```

For semantic matching, use the LLM judge option with `pip install "jpnorm[anthropic]"` or `pip install "jpnorm[openai]"`.

```python
compare(
    "東京タワーの高さは333メートルです",
    "東京タワーは高さ 333m",
    strategy="llm_judge",
    llm_provider="anthropic",        # or "openai"
    llm_model="claude-haiku-4-5",
    threshold=0.8,
)
```

You can also pass your own function with `judge_fn=` to test without calling an API. API keys are read from `ANTHROPIC_API_KEY` or `OPENAI_API_KEY`.

### 3. Name matching and deduplication (`for_compare` + custom dictionaries)

Project names, product names, company names, and content titles often need domain-specific rules. jpnorm lets you map spelling variants to a canonical form in the final normalization step. Dictionary keys can be written in their raw form; jpnorm normalizes them with the same settings before matching.

```python
n = Normalizer("for_compare").with_custom_dict({
    "幽遊白書": ["幽白", "ゆうはく", "幽☆遊☆白書"],
    "株式会社サンプル": ["サンプル社", "(株)サンプル", "㈱サンプル"],
})

n.normalize("㈱サンプル")            # => '株式会社サンプル'
n.normalize("幽☆遊☆白書 第１巻")     # => '幽遊白書 第1巻'

records = ["㈱サンプル", "サンプル社", "株式会社サンプル"]
len({n.normalize(r) for r in records})   # => 1
```

Dictionaries can also be loaded from files. jpnorm detects the format from the extension, or you can pass `format=` explicitly.

```python
n = (
    Normalizer("for_compare")
    .load_custom_dict_file("brands.json")   # {"canonical": ["variant", ...]}
    .load_custom_dict_file("terms.csv")     # variant,canonical
    .load_custom_dict_file("terms.tsv")     # variant<TAB>canonical
)
n.custom_dict_size
n.clear_custom_dict()
```

General Japanese synonym data can be loaded from SudachiDict's `synonyms.txt` (Apache-2.0). jpnorm does not bundle it, so download it only if you need it.

```bash
curl -fSL -o synonyms.txt https://raw.githubusercontent.com/WorksApplications/SudachiDict/develop/src/main/text/synonyms.txt
```

```python
n = Normalizer("for_search").load_sudachi_synonyms("synonyms.txt")
n.normalize("パソコンを買った")   # => 'パーソナルコンピュータを買った'
```

### 4. Cleaning user-generated text for display (`for_display`)

This preset keeps the visual style of the text intact. It converts half-width kana to full-width kana, but keeps emoji, full-width symbols, and spacing.

```python
n = Normalizer("for_display")
n.normalize("ﾊﾝｶｸｶﾅ ＋ 全角 🗼")   # => 'ハンカクカナ ＋ 全角 🗼'
```

If you also want to sanitize text before saving it, add flags for zero-width characters, control characters, bidi control characters, and newline normalization. Removing bidi control characters can also help prevent Trojan Source-style confusion.

```python
n = Normalizer(
    "for_display",
    remove_zero_width=True,
    remove_control=True,
    remove_bidi_control=True,
    normalize_newlines=True,
)
n.normalize("ゼロ幅\u200b文字と\u202e制御\r\n改行")   # => 'ゼロ幅文字と制御\n改行'
```

### 5. Migrating from neologdn (`neologdn_compat`)

`jpnorm.normalize()` without arguments uses this preset. It performs neologdn-like processing in Rust: half-width kana to full-width kana, full-width alphanumerics to half-width, prolonged sound mark folding, whitespace folding, and related cleanup.

```python
import jpnorm

jpnorm.normalize("ﾊﾝｶｸ ﾄ 全角 ＡＢＣ")   # => 'ハンカク ト 全角 ABC'
jpnorm.normalize("あ〜〜〜")              # => 'あ〜'
```

There are three intentional differences from neologdn, tracked by golden tests:

- neologdn removes spaces between Japanese and alphanumeric text; jpnorm keeps one folded space
- neologdn removes `〜` and `~`; jpnorm normalizes them to `〜` and keeps them
- neologdn applies its own conversions for `‘’`, `“”`, and `¥`; jpnorm leaves them unchanged by default (`unify_quotes=True` can normalize quotes)

After migration, you can add only the behavior you need, such as URL protection or emoji removal.

```python
n = Normalizer("neologdn_compat", protect_urls=True, emoji="remove")
```

## Preset quick reference

| Name | When to use it | NFKC | Symbol unification | URL protection | Emoji | Kanji numerals to Arabic digits | Symbol removal |
|---|---|:-:|:-:|:-:|:-:|:-:|:-:|
| `none` | Build your own configuration from flags | | | | Keep | | |
| `for_display` | UI display and post previews | | Prolonged marks only | | Keep | | |
| `neologdn_compat` | neologdn replacement (default) | ✓ | ✓ | | Keep | | |
| `for_search` | Search indexes and RAG | ✓ | ✓ | ✓ | Remove | | |
| `for_compare` | Evaluation, name matching, and deduplication | ✓ | ✓ | | Remove | ✓ | ✓ |

You can inspect all presets with `Normalizer.presets()`. If you are unsure, start with **`for_display` for UI display** and **`for_search` for most other pipelines**, then move toward `for_compare` when you need stricter matching.

## Fine-tuning

Start from a preset and override individual flags with keyword arguments. The keys are the same ones returned by `Normalizer.config`, so `Normalizer(**n.config)` can recreate the same configuration.

```python
n = Normalizer(
    "for_search",
    emoji="keep",
    kana="kata_to_hira",
    url_wrap=("<", ">"),
    repeat_limit=3,
)
n.normalize("スゴーーーーイ😀 https://example.com")
# => 'すごーい😀 <https://example.com>'

n.config["nfkc"]   # => True
```

| Key | Type | Meaning |
|---|---|---|
| `normalize_newlines` | bool | Normalize CRLF / CR / NEL / LS / PS to LF |
| `remove_zero_width` / `remove_control` / `remove_bidi_control` | bool | Remove invisible or control characters |
| `nfkc` | bool | Unicode NFKC, such as full-width alphanumerics to half-width |
| `halfwidth_kana_to_fullwidth` | bool | Convert half-width kana to full-width kana |
| `kana` | `"keep"` / `"hira_to_kata"` / `"kata_to_hira"` | Normalize hiragana and katakana |
| `unify_hyphens` / `unify_tildes` / `unify_prolonged` / `unify_quotes` | bool | Normalize symbol variants |
| `collapse_prolonged_run` | bool | Collapse repeated prolonged sound marks and tildes to one character |
| `repeat_limit` | int / None | Maximum repeated character count; alphanumerics are excluded |
| `collapse_spaces` / `trim` | bool | Fold whitespace and trim edges |
| `expand_cjk_compat` / `remove_cjk_compat` | bool | Expand or remove CJK compatibility characters such as ㈱①㌔ |
| `remove_symbols` | bool | Remove punctuation and symbols |
| `kansuji_to_arabic` / `arabic_to_kansuji` | bool | Convert kanji numerals and Arabic digits; mutually exclusive |
| `canonicalize_numbers` | bool | Convert `1,200` / `1200.00` to `1200` |
| `protect_urls` / `protect_emails` / `protect_mentions` / `protect_hashtags` | bool | Protect spans from later normalization steps |
| `url_wrap` | (str, str) / None | Wrap protected URLs with a prefix and suffix |
| `emoji` | `"keep"` / `"remove"` | Emoji handling |
| `emoji_placeholder` | str / None | Replace emoji with a fixed placeholder |

`normalize` and `normalize_batch` release the GIL while processing, so you can parallelize calls with a thread pool.

## Command line usage

```bash
echo "ﾊﾝｶｸｶﾅ　と  全角  ！！" | jpnorm
# ハンカクカナ と 全角 !!

jpnorm -p for_search --set emoji=keep --set kana=kata_to_hira "スゴーーーイ😀"
# すごーい😀

jpnorm -p for_compare --files --dict brands.json input.txt > normalized.txt
jpnorm --json "ｶﾅ"            # {"input": "ｶﾅ", "output": "カナ"}
jpnorm -p for_compare --show-config   # Show the effective configuration as JSON
jpnorm --list-presets
```

## Performance

Throughput compared with neologdn (C++ implementation) on the same inputs (`scripts/bench-vs-neologdn.py`, Apple Silicon, Python 3.12):

| Input | neologdn | jpnorm `neologdn_compat` | jpnorm `for_search` |
|---|---:|---:|---:|
| tweet (~100B) | 15 MB/s | 42 MB/s (2.9x) | 29 MB/s (2.0x) |
| paragraph (~500B) | 17 MB/s | 70 MB/s (4.1x) | 41 MB/s (2.4x) |
| large (~32KB) | 17 MB/s | 88 MB/s (5.3x) | 45 MB/s (2.7x) |

`for_search` is slower than `neologdn_compat` because it also protects URLs, removes emoji, and expands CJK compatibility characters. `normalize` and `normalize_batch` release the GIL while processing, so thread pools can improve throughput further.

## Comparison with other libraries

| | jpnorm | neologdn | jaconv / mojimoji | `unicodedata.normalize("NFKC")` |
|---|:-:|:-:|:-:|:-:|
| Half-width kana and full-width alphanumeric normalization | ✓ | ✓ | ✓ | ✓ |
| Prolonged marks, hyphens, and tildes | ✓ | ✓ | | |
| Use-case presets and configurable flags | ✓ | | | |
| URL / email / @mention protection | ✓ | | | |
| Emoji removal and replacement | ✓ | | | |
| Kanji numerals to Arabic digits and number canonicalization | ✓ | | | |
| Custom dictionaries and Sudachi synonyms | ✓ | | | |
| Evaluation helper | ✓ | | | |
| Implementation | Rust | C++ | Python / C | C |

jaconv and mojimoji are lightweight and useful when you only need character-type conversion. jpnorm is meant for pipelines that combine several normalization steps depending on the use case.

## Rust usage

The core is available as the `jpnorm-core` crate and can be used without Python. It is not published to crates.io yet, so add it as a git dependency.

```toml
[dependencies]
jpnorm-core = { git = "https://github.com/YoshitakaOyama/jpnorm" }
```

```rust
use jpnorm_core::{EmojiAction, Normalizer, Preset};

let n = Normalizer::builder()
    .preset(Preset::ForSearch)
    .configure(|c| c.emoji_action = EmojiAction::Keep)
    .kata_to_hira()
    .build();
assert_eq!(n.normalize("ｶﾅ😀"), "かな😀");

let r = n.normalize_with_segments("@alice と https://example.com だよ");
println!("{:?}", r.segments);
```

## Development

```bash
git clone https://github.com/YoshitakaOyama/jpnorm.git
cd jpnorm
uv sync --group dev          # Build the Rust extension and install it into .venv
uv run pytest                # Python tests
cargo test --workspace       # Rust tests
cargo clippy --workspace --all-targets -- -D warnings
uv run ruff check . && uv run mypy
cargo bench -p jpnorm-core   # Benchmarks with criterion
uv run --with neologdn scripts/bench-vs-neologdn.py   # Comparison table against neologdn
wasm-pack build crates/jpnorm-wasm --target web --release --out-dir ../../playground/pkg --no-typescript
python -m http.server -d playground 8765   # Open the playground locally
```

When you change Rust source files, run `uv sync` to rebuild the extension. The golden comparison tests against neologdn use `tests/golden/neologdn.jsonl` and can be regenerated with:

```bash
uv run --with neologdn scripts/gen-neologdn-golden.py
```

Releases are automated: update `CHANGELOG.md`, then push a `v*` tag to build wheels, publish to PyPI, and create a GitHub Release.

## License

Dual-licensed under MIT or Apache-2.0. Choose whichever works best for your project.
