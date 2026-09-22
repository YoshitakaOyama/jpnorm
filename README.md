# jpnorm

[![PyPI](https://img.shields.io/pypi/v/jpnorm)](https://pypi.org/project/jpnorm/)
[![Python](https://img.shields.io/pypi/pyversions/jpnorm)](https://pypi.org/project/jpnorm/)
[![CI](https://github.com/YoshitakaOyama/jpnorm/actions/workflows/ci.yml/badge.svg)](https://github.com/YoshitakaOyama/jpnorm/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#ライセンス)

日本語テキスト正規化ライブラリ。Rust 製のコアを Python から使えます。
neologdn 互換の処理に加えて、用途別プリセット・URL 保護・絵文字処理・
漢数字変換・カスタム辞書による表記ゆれ吸収を、フラグ単位で組み合わせられます。

*Fast, configurable Japanese text normalization. Rust core with Python bindings.*

## できること

- **文字正規化**: NFKC、ハイフン/チルダ/長音符のバリエーション統一、引用符統一、繰り返し短縮、空白畳み込み
- **不可視文字の除去**: ゼロ幅文字・BOM・制御文字・Bidi 制御文字 (Trojan Source 対策)・改行コード統一
- **文字種変換**: 半角カナ→全角カナ、ひらがな⇄カタカナ、漢数字⇄算用数字、機種依存文字 (㈱①㌔) の展開/除去
- **数値正規化**: `1,200` / `1200.00` / `一千二百` を同じ表現に (比較用途)
- **保護領域**: URL / メールアドレス / @mention / #hashtag を正規化から除外 (URL は `<...>` 等で囲むことも可)
- **絵文字**: 保持 / 除去 / プレースホルダ置換
- **表記ゆれ吸収**: カスタム辞書 (dict / JSON / CSV / TSV) と Sudachi 同義語辞書
- **精度比較**: モデル出力と正解データの比較ユーティリティ (完全一致 / 前方一致 / 編集距離 / LLM judge)

## インストール

```bash
pip install jpnorm
# または
uv add jpnorm
```

Python 3.10 以上。Linux (x86_64 / aarch64)・macOS (x86_64 / arm64)・Windows (x64) の
wheel を配布しています。

## 使い方

```python
import jpnorm

jpnorm.normalize("ﾊﾝｶｸｶﾅ　と  全角  ！！")
# => 'ハンカクカナ と 全角 !!'

jpnorm.normalize("東京タワー🗼を見学した🎉", preset="for_search")
# => '東京タワーを見学した'
```

引数なしの `jpnorm.normalize()` は `neologdn_compat` プリセット相当です。
繰り返し使う場合は `Normalizer` を作っておくと辞書などの初期化を共有できます。

```python
from jpnorm import Normalizer

n = Normalizer("for_search")
n.normalize("https://example.com/path?q=1 を保護")
# => 'https://example.com/path?q=1 を保護'

n.normalize_batch(["ｶﾅ", "ＡＢＣ"])
# => ['カナ', 'ABC']
```

`normalize` / `normalize_batch` は処理中に GIL を解放するので、スレッドプールで並列化できます。

## プリセット

| 名前 | 用途 |
|---|---|
| `none` | 何もしない (個別フラグを積み上げるベース) |
| `neologdn_compat` | 既存の neologdn を置き換える。既定値 |
| `for_display` | UI 表示・投稿プレビュー。見た目を壊さない最小限 |
| `for_search` | 検索インデックス。URL 等は保護、絵文字除去、記号統一 |
| `for_compare` | 精度評価・重複判定。漢数字→数字・記号除去まで行い等価性を最大化 |

一覧は `Normalizer.presets()` で取得できます。

```python
Normalizer("for_display").normalize("ﾊﾝｶｸｶﾅ ＋ 全角 🗼")   # => 'ハンカクカナ ＋ 全角 🗼'
Normalizer("for_compare").normalize("三百二十円")           # => '320円'
Normalizer("for_compare").normalize("２０２４年３月２９日")  # => '2024年3月29日'
Normalizer("neologdn_compat").normalize("あ〜〜〜")          # => 'あ〜'
```

## カスタマイズ

プリセットをベースに、キーワード引数で個別フラグを上書きできます。
キーは `Normalizer.config` が返す dict と同じで、`Normalizer(**n.config)` で複製できます。

```python
n = Normalizer(
    "for_search",
    emoji="keep",              # 絵文字を残す
    kana="kata_to_hira",       # カタカナをひらがなに統一
    url_wrap=("<", ">"),       # URL を <...> で囲む (Slack/Markdown の自動リンク)
    repeat_limit=3,            # 同一文字の連続を 3 つまでに短縮
)
n.normalize("スゴーーーーイ😀 https://example.com")
# => 'すごーい😀 <https://example.com>'

n.config["nfkc"]   # => True
```

| キー | 型 | 内容 |
|---|---|---|
| `normalize_newlines` | bool | CRLF / CR / NEL / LS / PS を LF に統一 |
| `remove_zero_width` / `remove_control` / `remove_bidi_control` | bool | 不可視・制御文字の除去 |
| `nfkc` | bool | Unicode NFKC (全角英数→半角など) |
| `halfwidth_kana_to_fullwidth` | bool | 半角カナ→全角カナ |
| `kana` | `"keep"` / `"hira_to_kata"` / `"kata_to_hira"` | ひらがな⇄カタカナ統一 |
| `unify_hyphens` / `unify_tildes` / `unify_prolonged` / `unify_quotes` | bool | 記号バリエーションの統一 |
| `collapse_prolonged_run` | bool | 連続する長音符・チルダを 1 つに |
| `repeat_limit` | int / None | 同一文字の最大連続数 (英数字は対象外) |
| `collapse_spaces` / `trim` | bool | 空白の畳み込み・前後トリム |
| `expand_cjk_compat` / `remove_cjk_compat` | bool | 機種依存文字の展開 / 除去 |
| `remove_symbols` | bool | 句読点・記号の除去 |
| `kansuji_to_arabic` / `arabic_to_kansuji` | bool | 漢数字⇄算用数字 (排他) |
| `canonicalize_numbers` | bool | `1,200` / `1200.00` → `1200` |
| `protect_urls` / `protect_emails` / `protect_mentions` / `protect_hashtags` | bool | 保護領域 |
| `url_wrap` | (str, str) / None | 保護した URL を prefix/suffix で囲む |
| `emoji` | `"keep"` / `"remove"` | 絵文字の扱い |
| `emoji_placeholder` | str / None | 絵文字を指定文字列に置換 |

## カスタム辞書

自社サービス名・タレント名・作品タイトルなどの独自表記ゆれを正規化に組み込めます。
辞書は正規化の最終段で最長一致置換されます。複数回呼ぶとマージされます。

```python
n = Normalizer().with_custom_dict({
    "幽遊白書": ["幽白", "ゆうはく", "幽☆遊☆白書"],
    "Python":   ["パイソン", "ぱいそん"],
})

n.normalize("幽☆遊☆白書を読んだ")   # => '幽遊白書を読んだ'
n.normalize("ぱいそん最高")           # => 'Python最高'
```

ファイルからも読み込めます。拡張子で形式を判定します (`format=` で明示も可)。

```python
n = (
    Normalizer("for_search")
    .load_custom_dict_file("brands.json")   # {"正規形": ["表記ゆれ", ...]}
    .load_custom_dict_file("terms.csv")     # 表記ゆれ,正規形
    .load_custom_dict_file("terms.tsv")     # 表記ゆれ<TAB>正規形
)
n.load_custom_dict_json(json.dumps({...}))  # 文字列から
n.custom_dict_size                          # 登録エントリ数
n.clear_custom_dict()
```

### Sudachi 同義語辞書

[SudachiDict](https://github.com/WorksApplications/SudachiDict) の `synonyms.txt`
(Apache-2.0) をそのまま読み込めます。ライブラリにはバンドルしていないので、
必要な場合はダウンロードしてください。

```bash
curl -fSL -o synonyms.txt https://raw.githubusercontent.com/WorksApplications/SudachiDict/develop/src/main/text/synonyms.txt
```

```python
n = Normalizer("for_search").load_sudachi_synonyms("synonyms.txt")
n.normalize("パソコンを買った")   # => 'パーソナルコンピュータを買った'
```

## 精度比較ユーティリティ

モデル出力と正解データを複数戦略で比較できます。戦略は `exact` / `prefix` /
`edit_distance` / `llm_judge` から選択でき、比較前に `Normalizer` を通すことも
可能です。戻り値は `ComparisonResult` (`matched`, `score`, `strategy`, `detail`)。

```python
from jpnorm import Normalizer, compare

n = Normalizer("for_compare")

compare("ﾃｽﾄ", "テスト", strategy="exact", normalizer=n)            # 正規化してから完全一致
compare("東京都", "東京都渋谷区", strategy="prefix")                 # 前方一致 (どちら向きでも可)
compare("kitten", "sitting", strategy="edit_distance", threshold=0.5)  # 編集距離 (Rust 実装)

# LLM judge (Anthropic / OpenAI)。pip install "jpnorm[anthropic]" などで SDK を入れておく
compare(
    "出力テキスト", "正解テキスト",
    strategy="llm_judge",
    llm_provider="anthropic",   # or "openai"
    llm_model="claude-haiku-4-5",
    threshold=0.8,
)
```

`llm_judge` は `judge_fn=` で任意の判定関数に差し替えられるので、テストでは
API を呼ばずに済みます。API キーは `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` から読みます。

## Rust から使う

コアは `jpnorm-core` クレートとして独立しており、Python 無しでも使えます
(現時点では crates.io 未公開のため git 依存で指定してください)。

```toml
[dependencies]
jpnorm-core = { git = "https://github.com/YoshitakaOyama/jpnorm" }
```

```rust
use jpnorm_core::{EmojiAction, Normalizer, Preset};

let n = Normalizer::builder()
    .preset(Preset::ForSearch)
    .configure(|c| c.emoji_action = EmojiAction::Keep)   // プリセットの一部を打ち消す
    .kata_to_hira()
    .build();
assert_eq!(n.normalize("ｶﾅ😀"), "かな😀");

// 保護領域の位置も取れる (検索ハイライト等に)
let r = n.normalize_with_segments("@alice と https://example.com だよ");
println!("{:?}", r.segments);
```

## 開発

```bash
git clone https://github.com/YoshitakaOyama/jpnorm.git
cd jpnorm
uv sync --group dev          # Rust 拡張をビルドして .venv に入れる
uv run pytest                # Python テスト
cargo test --workspace       # Rust テスト
cargo clippy --workspace --all-targets -- -D warnings
uv run ruff check . && uv run mypy
cargo bench -p jpnorm-core   # ベンチマーク (criterion)
```

Rust ソースを変更したら `uv sync` で再ビルドされます。
neologdn とのゴールデン比較テストは `tests/golden/neologdn.jsonl` を使い、
`uv run --with neologdn scripts/gen-neologdn-golden.py` で再生成できます。

リリースは `CHANGELOG.md` を更新し、`v*` タグを push すると wheel のビルド・PyPI 公開・
GitHub Release 作成まで自動で行われます。

## ライセンス

MIT または Apache-2.0 のデュアルライセンス。好きな方を選んでください。
