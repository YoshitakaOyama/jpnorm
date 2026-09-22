# jpnorm

[![PyPI](https://img.shields.io/pypi/v/jpnorm)](https://pypi.org/project/jpnorm/)
[![Python](https://img.shields.io/pypi/pyversions/jpnorm)](https://pypi.org/project/jpnorm/)
[![CI](https://github.com/YoshitakaOyama/jpnorm/actions/workflows/ci.yml/badge.svg)](https://github.com/YoshitakaOyama/jpnorm/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#ライセンス)

**日本語テキストの「表記ゆれ」を、用途に合わせて 1 行で吸収するライブラリ。**
Rust 製コアを Python から使います。neologdn の置き換えから、検索前処理、LLM 出力の評価、
名寄せまで、プリセットを選ぶだけで始められ、必要ならフラグ単位で細かく調整できます。

*Fast, configurable Japanese text normalization. Rust core with Python bindings.
Pick a preset (`neologdn_compat`, `for_search`, `for_compare`, `for_display`) or tune 28 flags;
protects URLs / emails / mentions, handles emoji, converts kanji numerals, applies custom
synonym dictionaries. 3 to 5 times faster than neologdn.*

```python
import jpnorm

jpnorm.normalize("ﾊﾝｶｸｶﾅ　と  全角  ！！")
# => 'ハンカクカナ と 全角 !!'
```

**ブラウザで試す →** <https://yoshitakaoyama.github.io/jpnorm/> (テキストを貼ると全プリセットの結果が並びます)

## こんなときに使います

日本語には「同じ意味なのに文字列としては別物」になる書き方が大量にあります。
半角カナと全角カナ、全角英数と半角英数、`ー` と `〜` と `-`、`㈱` と `(株)`、
`三百二十円` と `320円`、`幽☆遊☆白書` と `幽白`……。
これらを放置すると、次のような問題が起きます。

| 困りごと | 何が起きているか | jpnorm でどうするか |
|---|---|---|
| 検索・RAG で「ヒットするはずの文書」が出てこない | 索引側と検索語で表記が違う | `for_search` で両方を同じ形に揃えてから索引・検索する |
| LLM / OCR / 音声認識の精度を測ると、正解と「実質同じ」なのに不一致になる | 全角半角・記号・漢数字の違いで完全一致が失敗 | `for_compare` で潰してから `compare()` で判定する |
| 顧客名・商品名の名寄せや重複検出で取りこぼす | 社内独自の略称・表記ゆれ | カスタム辞書で `幽白 → 幽遊白書` のように正規形へ寄せる |
| ユーザー投稿を表示したら半角カナやゼロ幅文字で崩れた | 見た目に影響する文字と、見えない制御文字 | `for_display` で見た目を壊さない範囲だけ整える |
| neologdn を使っているが、URL が壊れる・絵文字を消したい・設定を変えたい | neologdn は固定処理で設定できない | `neologdn_compat` から始めて、必要なフラグだけ足し引きする |

## インストール

```bash
pip install jpnorm
# または
uv add jpnorm
```

Python 3.10 以上。Linux (x86_64 / aarch64)・macOS (x86_64 / arm64)・Windows (x64) の
wheel を配布しているので、Rust ツールチェーンは不要です。
コマンドラインだけ使いたい場合は `pipx install jpnorm` で `jpnorm` コマンドが入ります。

## ユースケース別ガイド

### 1. 検索・RAG の前処理 (`for_search`)

索引を作るときと検索するときの両方で同じ Normalizer を通します。
URL やメールアドレスは壊さず、絵文字は落とし、記号のバリエーションを揃えます。

```python
from jpnorm import Normalizer

n = Normalizer("for_search")

docs = ["ＰＹＴＨＯＮ入門 〜〜 初心者向け🔰", "python 入門（初心者向け）"]
n.normalize_batch(docs)
# => ['PYTHON入門 〜 初心者向け', 'python 入門(初心者向け)']

n.normalize("詳細は https://example.com/Docs?Q=1 を参照 📎")
# => '詳細は https://example.com/Docs?Q=1 を参照'   ← URL はそのまま
```

大文字小文字の統一は用途依存なので jpnorm は行いません。必要なら `.lower()` を重ねてください。

### 2. LLM / OCR / 音声認識の出力評価 (`for_compare` + `compare`)

「正解データと実質同じか」を判定したい場面です。全角半角・記号・漢数字・空白の違いを
すべて潰した上で、完全一致・前方一致・編集距離・LLM judge のいずれかで比較します。

```python
from jpnorm import Normalizer, compare

n = Normalizer("for_compare")

n.normalize("合計：￥１，２００（税込）")    # => '合計¥1200税込'
n.normalize("合計:¥1,200(税込)")            # => '合計¥1200税込'   ← 同じ形になる
n.normalize("第一章　はじめに")             # => '第1章 はじめに'

compare("ﾃｽﾄ結果：１２３", "テスト結果:123", strategy="exact", normalizer=n).matched
# => True

r = compare("東京都渋谷区", "東京都渋谷区神南1-2-3", strategy="prefix", normalizer=n)
r.matched, r.detail["direction"]
# => (True, 'prediction_is_prefix_of_reference')

r = compare("kitten", "sitting", strategy="edit_distance", threshold=0.5)
r.score   # => 0.571...  (1 - 編集距離 / 長さ)
```

意味的な一致まで見たい場合は LLM judge が使えます (`pip install "jpnorm[anthropic]"` または `[openai]`)。

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

`judge_fn=` に自前の判定関数を渡せば API を呼ばずにテストできます。
API キーは `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` から読みます。

### 3. 名寄せ・重複検出 (`for_compare` + カスタム辞書)

自社サービス名・作品名・人名など、辞書にしか載っていない表記ゆれは
カスタム辞書で正規形に寄せます。辞書は正規化の最終段で最長一致置換されます。
キーは生の表記のまま書いて構いません (同じ設定で正規化してから照合されます)。

```python
n = Normalizer("for_compare").with_custom_dict({
    "幽遊白書": ["幽白", "ゆうはく", "幽☆遊☆白書"],
    "株式会社サンプル": ["サンプル社", "(株)サンプル", "㈱サンプル"],
})

n.normalize("㈱サンプル")            # => '株式会社サンプル'
n.normalize("幽☆遊☆白書 第１巻")     # => '幽遊白書 第1巻'

# 重複検出はキーを揃えるだけ
records = ["㈱サンプル", "サンプル社", "株式会社サンプル"]
len({n.normalize(r) for r in records})   # => 1
```

辞書はファイルからも読めます (拡張子で形式判定、`format=` で明示も可)。

```python
n = (
    Normalizer("for_compare")
    .load_custom_dict_file("brands.json")   # {"正規形": ["表記ゆれ", ...]}
    .load_custom_dict_file("terms.csv")     # 表記ゆれ,正規形
    .load_custom_dict_file("terms.tsv")     # 表記ゆれ<TAB>正規形
)
n.custom_dict_size      # 登録エントリ数
n.clear_custom_dict()
```

一般語の同義語は [SudachiDict](https://github.com/WorksApplications/SudachiDict) の
`synonyms.txt` (Apache-2.0) をそのまま読み込めます。ライブラリにはバンドルしていないので
必要な場合はダウンロードしてください。

```bash
curl -fSL -o synonyms.txt https://raw.githubusercontent.com/WorksApplications/SudachiDict/develop/src/main/text/synonyms.txt
```

```python
n = Normalizer("for_search").load_sudachi_synonyms("synonyms.txt")
n.normalize("パソコンを買った")   # => 'パーソナルコンピュータを買った'
```

### 4. ユーザー投稿の表示前クリーンアップ (`for_display`)

「見た目を壊さない」が原則です。半角カナだけは全角にし、絵文字・全角記号・空白は残します。

```python
n = Normalizer("for_display")
n.normalize("ﾊﾝｶｸｶﾅ ＋ 全角 🗼")   # => 'ハンカクカナ ＋ 全角 🗼'
```

保存前のサニタイズ (ゼロ幅文字・制御文字・Bidi 制御文字の除去、改行コード統一) を
足したい場合はフラグを重ねます。Bidi 制御文字の除去は Trojan Source 対策にもなります。

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

### 5. neologdn からの移行 (`neologdn_compat`)

引数なしの `jpnorm.normalize()` がこれです。半角カナ→全角、全角英数→半角、
長音符の畳み込み、空白の畳み込みなど neologdn と同等の処理を Rust で行います。

```python
import jpnorm

jpnorm.normalize("ﾊﾝｶｸ ﾄ 全角 ＡＢＣ")   # => 'ハンカク ト 全角 ABC'
jpnorm.normalize("あ〜〜〜")              # => 'あ〜'
```

neologdn と意図的に違う点は次の 3 つです (ゴールデンテストで管理しています)。

- 日本語と英数字の間の空白を neologdn は削除しますが、jpnorm は 1 つに畳むだけで残します
- `〜` `~` を neologdn は削除しますが、jpnorm は `〜` に統一して残します
- `‘’` `“”` `¥` を neologdn は独自変換しますが、jpnorm は既定では触りません (`unify_quotes=True` で統一可)

移行後に「URL は保護したい」「絵文字は消したい」となったら、そこからフラグを足すだけです。

```python
n = Normalizer("neologdn_compat", protect_urls=True, emoji="remove")
```

## プリセット早見表

| 名前 | いつ使うか | NFKC | 記号統一 | URL 保護 | 絵文字 | 漢数字→数字 | 記号除去 |
|---|---|:-:|:-:|:-:|:-:|:-:|:-:|
| `none` | 自分でフラグを積む | | | | 残す | | |
| `for_display` | UI 表示・投稿プレビュー | | 長音のみ | | 残す | | |
| `neologdn_compat` | neologdn 置き換え (既定) | ✓ | ✓ | | 残す | | |
| `for_search` | 検索索引・RAG | ✓ | ✓ | ✓ | 除去 | | |
| `for_compare` | 精度評価・名寄せ・重複検出 | ✓ | ✓ | | 除去 | ✓ | ✓ |

一覧は `Normalizer.presets()` で取得できます。迷ったら、**表示なら `for_display`、
それ以外は `for_search`** から始めて、必要に応じて `for_compare` に寄せるのがおすすめです。

## 細かく調整する

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
| `expand_cjk_compat` / `remove_cjk_compat` | bool | 機種依存文字 (㈱①㌔) の展開 / 除去 |
| `remove_symbols` | bool | 句読点・記号の除去 |
| `kansuji_to_arabic` / `arabic_to_kansuji` | bool | 漢数字⇄算用数字 (排他) |
| `canonicalize_numbers` | bool | `1,200` / `1200.00` → `1200` |
| `protect_urls` / `protect_emails` / `protect_mentions` / `protect_hashtags` | bool | 保護領域 |
| `url_wrap` | (str, str) / None | 保護した URL を prefix/suffix で囲む |
| `emoji` | `"keep"` / `"remove"` | 絵文字の扱い |
| `emoji_placeholder` | str / None | 絵文字を指定文字列に置換 |

`normalize` / `normalize_batch` は処理中に GIL を解放するので、スレッドプールで並列化できます。

## コマンドラインで使う

```bash
echo "ﾊﾝｶｸｶﾅ　と  全角  ！！" | jpnorm
# ハンカクカナ と 全角 !!

jpnorm -p for_search --set emoji=keep --set kana=kata_to_hira "スゴーーーイ😀"
# すごーい😀

jpnorm -p for_compare --files --dict brands.json input.txt > normalized.txt
jpnorm --json "ｶﾅ"            # {"input": "ｶﾅ", "output": "カナ"}
jpnorm -p for_compare --show-config   # 実際に使う設定を JSON で表示
jpnorm --list-presets
```

## パフォーマンス

neologdn (C++ 実装) と同じ入力で比較したスループットです
(`scripts/bench-vs-neologdn.py`、Apple Silicon、Python 3.12)。

| 入力 | neologdn | jpnorm neologdn_compat | jpnorm for_search |
|---|---:|---:|---:|
| tweet (~100B) | 15 MB/s | 42 MB/s (2.9x) | 29 MB/s (2.0x) |
| paragraph (~500B) | 17 MB/s | 70 MB/s (4.1x) | 41 MB/s (2.4x) |
| large (~32KB) | 17 MB/s | 88 MB/s (5.3x) | 45 MB/s (2.7x) |

`for_search` は URL 保護・絵文字除去・機種依存文字展開が加わるぶん `neologdn_compat` より遅くなります。
`normalize` / `normalize_batch` は GIL を解放するので、スレッドプールでさらに並列化できます。

## 他のライブラリとの違い

| | jpnorm | neologdn | jaconv / mojimoji | `unicodedata.normalize("NFKC")` |
|---|:-:|:-:|:-:|:-:|
| 半角カナ・全角英数の統一 | ✓ | ✓ | ✓ | ✓ |
| 長音・ハイフン・チルダの統一 | ✓ | ✓ | | |
| 用途別プリセット / フラグ設定 | ✓ | | | |
| URL / メール / @mention の保護 | ✓ | | | |
| 絵文字の除去・置換 | ✓ | | | |
| 漢数字→算用数字、数値正規化 | ✓ | | | |
| カスタム辞書・Sudachi 同義語 | ✓ | | | |
| 精度比較ユーティリティ | ✓ | | | |
| 実装 | Rust | C++ | Python / C | C |

jaconv や mojimoji は「文字種変換だけしたい」場面では十分軽量です。
jpnorm は複数の処理を用途に合わせて組み合わせたいときに向いています。

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
uv run --with neologdn scripts/bench-vs-neologdn.py   # neologdn との比較表
wasm-pack build crates/jpnorm-wasm --target web --release --out-dir ../../playground/pkg --no-typescript
python -m http.server -d playground 8765   # プレイグラウンドをローカルで開く
```

Rust ソースを変更したら `uv sync` で再ビルドされます。
neologdn とのゴールデン比較テストは `tests/golden/neologdn.jsonl` を使い、
`uv run --with neologdn scripts/gen-neologdn-golden.py` で再生成できます。

リリースは `CHANGELOG.md` を更新し、`v*` タグを push すると wheel のビルド・PyPI 公開・
GitHub Release 作成まで自動で行われます。

## ライセンス

MIT または Apache-2.0 のデュアルライセンス。好きな方を選んでください。
