# jpnorm

日本語テキスト正規化ライブラリ。

## できること

- **文字正規化**: NFKC、ハイフン/チルダ/長音符のバリエーション統一、繰り返し短縮、空白畳み込み
- **文字種変換**: 半角カナ⇄全角カナ、ひらがな⇄カタカナ、漢数字⇄算用数字
- **表記ゆれ吸収**: Sudachi 同義語辞書による語彙正規化
- **URL 保護**: 正規化対象から URL を除外
- **プリセット**: 用途別(表示/検索/比較など)の設定プリセット
- **精度比較**: モデル出力と正解データの比較ユーティリティ(完全一致/前方一致/編集距離/LLM judge)

## インストール

```bash
pip install jpnorm
```

## 使い方

```python
import jpnorm

print(jpnorm.normalize("ﾊﾝｶｸｶﾅ　と  全角  ！！"))
# => ハンカクカナ と 全角 !!
```

引数なしの `jpnorm.normalize()` は **`neologdn_compat`** プリセット相当の処理
(半角カナ→全角・空白畳み込み・繰り返し短縮・記号統一など、neologdn と同等)。

用途別のチューニングをしたい場合は `Normalizer.preset(name)` を使います。

## プリセット

| 名前 | 用途 |
|---|---|
| `none` | 何もしない (builder のベース) |
| `for_display` | UI 表示・投稿プレビュー。見た目を壊さない最小限 |
| `neologdn_compat` | 既存 neologdn 置き換え |
| `for_search` | 検索インデックス。URL 等は保護、絵文字除去 |
| `for_compare` | 精度評価・重複判定。漢数字→数字・記号除去まで行い等価性を最大化 |

### `for_search` — 検索インデックス向け

URL/メールアドレスは壊さずに保護、絵文字は除去、空白は最小限。

```python
n = jpnorm.Normalizer.preset("for_search")

n.normalize("ﾊﾝｶｸｶﾅ　と  全角  ！！")
# => 'ハンカクカナ と 全角 !!'

n.normalize("https://example.com/path?q=1 を保護")
# => 'https://example.com/path?q=1 を保護'  ← URL 本体も周辺スペースもそのまま

n.normalize("メールは test@example.com まで")
# => 'メールは test@example.com まで'

n.normalize("東京タワー🗼を見学した🎉")
# => '東京タワーを見学した'  ← 絵文字除去
```

### `for_display` — UI表示・投稿プレビュー向け

「見た目を壊さない」が原則。半角カナだけは全角化するが、絵文字や全角記号はそのまま。

```python
n = jpnorm.Normalizer.preset("for_display")

n.normalize("ﾊﾝｶｸｶﾅ ＋ 全角")
# => 'ハンカクカナ ＋ 全角'  ← 全角プラスは保持

n.normalize("東京タワー🗼 を見学")
# => '東京タワー🗼 を見学'  ← 絵文字も保持
```

### `for_compare` — 精度評価・重複判定向け

漢数字→算用数字、ハイフン/チルダ等の記号も等価判定向けに整理。
モデル評価や重複検出で「実質同じ文字列」を一致させたい場面に。

```python
n = jpnorm.Normalizer.preset("for_compare")

n.normalize("三百二十円")             # => '320円'
n.normalize("第１章")                 # => '第1章'
n.normalize("２０２４年３月２９日")    # => '2024年3月29日'
n.normalize("東京-渋谷")              # => '東京渋谷'
n.normalize("東京〜渋谷")             # => '東京渋谷'
```

### `neologdn_compat` — 既存 neologdn 置き換え

neologdn からの移行用。同等の処理を Rust ネイティブ実装で高速に。

```python
n = jpnorm.Normalizer.preset("neologdn_compat")

n.normalize("Pythonと  Rust")     # => 'Pythonと Rust'
n.normalize("ﾊﾝｶｸ ﾄ 全角")         # => 'ハンカク ト 全角'
n.normalize("あ〜〜〜")            # => 'あ〜'
```

## カスタム辞書

自社サービス名・タレント名・作品タイトルなどの独自表記ゆれを正規化に組み込めます。

```python
n = jpnorm.Normalizer()
n.with_custom_dict({
    "幽遊白書": ["幽白", "ゆうはく", "幽☆遊☆白書"],
    "Python":   ["パイソン", "ぱいそん"],
})

n.normalize("幽☆遊☆白書を読んだ")   # => '幽遊白書を読んだ'
n.normalize("幽白を読んだ")           # => '幽遊白書を読んだ'
n.normalize("ぱいそん最高")           # => 'Python最高'
```

JSON 文字列から読み込む場合は `n.load_custom_dict_json(json_text)` も使えます。

## 精度比較ユーティリティ

モデル出力と正解データを複数戦略で比較できます。戦略は `exact` / `prefix` /
`edit_distance` / `llm_judge` から選択でき、比較前に `Normalizer` を通すことも
可能です。戻り値は `ComparisonResult`(matched, score, strategy, detail)。

```python
from jpnorm import Normalizer, compare

n = Normalizer.preset("for_compare")

# 完全一致 (正規化してから比較)
compare("ﾃｽﾄ", "テスト", strategy="exact", normalizer=n)

# 前方一致 (どちら向きでも可)
compare("東京都", "東京都渋谷区", strategy="prefix")

# 編集距離 (Levenshtein、threshold で matched 判定)
compare("kitten", "sitting", strategy="edit_distance", threshold=0.5)

# LLM judge (Anthropic / OpenAI)
compare(
    "出力テキスト",
    "正解テキスト",
    strategy="llm_judge",
    llm_provider="anthropic",   # or "openai"
    llm_model="claude-haiku-4-5",
    threshold=0.8,
)
```

`llm_judge` 使用時は `anthropic` または `openai` パッケージと、対応する
API キー (`ANTHROPIC_API_KEY` / `OPENAI_API_KEY`) が必要です。

## Sudachi 同義語辞書

表記ゆれ吸収は [SudachiDict](https://github.com/WorksApplications/SudachiDict)
の `synonyms.txt` (Apache-2.0) を利用できます。ライブラリにはバンドルしていないので、
必要な場合はダウンロードしてください:

```bash
curl -fSL -o synonyms.txt https://raw.githubusercontent.com/WorksApplications/SudachiDict/develop/src/main/text/synonyms.txt
```

## 開発

```bash
git clone https://github.com/YoshitakaOyama/jpnorm.git
cd jpnorm
pip install maturin
maturin develop --release
pytest tests/
```

## ライセンス

MIT または Apache-2.0 のデュアルライセンス。好きな方を選んでください。
