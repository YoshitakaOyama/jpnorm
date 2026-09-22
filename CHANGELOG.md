# Changelog

All notable changes to this project will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- Python: `Normalizer(preset=None, **options)` で個別フラグをキーワード引数で指定できるようになった。
  有効なキーは `Normalizer.config` と同じで、`Normalizer(**n.config)` で複製できる。
- Python: `Normalizer.presets()`, `Normalizer.config`, `Normalizer.custom_dict_size`。
- Python: `normalize(text, preset=...)`, `Normalizer.normalize_batch(texts)`。
  `normalize` / `normalize_batch` は処理中に GIL を解放する。
- Python: `load_custom_dict_file(path, format=None)` (json / csv / tsv),
  `load_sudachi_synonyms(path)`, `clear_custom_dict()`。
- Python: `levenshtein(a, b)` (Rust 実装)。`compare(strategy="edit_distance")` もこれを使う。
- Python: 型スタブに `py.typed` を追加し、`NormalizerOptions` (TypedDict) を公開。
- Python: `jpnorm[anthropic]` / `jpnorm[openai]` / `jpnorm[llm]` extras。
- Rust: ひらがな⇄カタカナ統一を `Config::kana` (`KanaAction`) としてパイプラインに配線。
  Builder に `hira_to_kata()` / `kata_to_hira()` を追加。
- Rust: `NormalizerBuilder::configure(|c| ...)` / `config_mut()` / `from_config()` /
  `synonyms(dict)` / `protect(ProtectConfig)` / `protect_emails()` / `protect_mentions()` /
  `protect_hashtags()` / `canonicalize_numbers()` / `collapse_prolonged_run()` / `keep_emoji()`。
- Rust: `Preset::ALL`, `Preset::as_str()`, `Display` / `FromStr` (`ParsePresetError`)。
- Rust: `Normalizer::synonyms()`, `Normalizer::normalize_batch()`, `ProtectConfig::none()`, `SynonymDict::iter()`。
- Rust: `EmojiAction`, `KanaAction`, `ProtectConfig`, `ProtectKind`, `Segment`,
  `NormalizedText` をクレートルートから再エクスポート。
- CI: Rust MSRV チェック、`cargo doc` 警告チェック、Python 3.10 / 3.12 / 3.14 + macOS / Windows
  での pytest・ruff・mypy。Dependabot (cargo / uv / actions)。
- Release: タグ push 時に CHANGELOG から本文を抽出して GitHub Release を作成。

### Changed

- **Breaking (Rust)**: `EmojiAction::Replace` と `Config::url_wrap` が `&'static str` ではなく
  `Cow<'static, str>` を取るようになった (`EmojiAction::replace(...)` / `wrap_urls(impl Into<Cow>)`)。
  `EmojiAction` は `Copy` ではなくなり、`emoji::process` は `&EmojiAction` を取る。
- **Breaking (Rust)**: `Config` に `kana` フィールドが追加された。構造体リテラルで構築している
  場合は `..Config::none()` 等を使うこと。`SynonymDictError` は `#[non_exhaustive]` になり
  `InvalidJson` バリアントが追加された。
- Python: `with_custom_dict` / `load_custom_dict_json` は辞書を上書きではなくマージし、
  メソッドチェーンできるよう自身を返す。
- Python: 型エラー (`TypeError`) と値エラー (`ValueError`) を区別するようになった。
- 同義語辞書のキーは `Normalizer` と同じ設定で正規化してから登録されるようになった。
  `for_compare` のような積極的なプリセットでも `㈱サンプル` のような生表記のキーがそのまま使える。
- **Breaking (Python)**: 対応 Python を 3.10 以上に変更 (3.9 は EOL)。
- Rust edition 2024、MSRV 1.85。pyo3 0.29、criterion 0.8。
- `Cargo.lock` をリポジトリに含めるようにした。
- ライセンス表記を PEP 639 の SPDX 形式に変更。

### Fixed

- `load_custom_dict_json` / `SynonymDict::from_json*` が `\uXXXX` エスケープ
  (Python の `json.dumps()` 既定出力) を読めなかった。サロゲートペア・`\b` `\f` `\r` も対応。
- `NormalizerBuilder::default()` が `neologdn_compat` 相当になっていた (`new()` は `none`)。
  どちらも `none` から始まるように統一。
- README で謳っていた「ひらがな⇄カタカナ」変換が実際には配線されておらず利用できなかった。
- 誤ってコミットされていた macOS デバッグシンボル (`*.dSYM`) を削除し、`.gitignore` に追加。
- `cargo fmt` / `clippy` の警告で CI が常に失敗していた。
- `Cargo.toml` と `pyproject.toml` で `description` が食い違っていた。

## [0.0.4] - 2026-04-26

- 保護領域の周辺で空白がトリムされる不具合を修正。

## [0.0.3] - 2026-04-26

- Python 3.14 向け wheel をビルド。

## [0.0.2] - 2026-04-25

- sdist に LICENSE ファイルを含めるようにした。

## [0.0.1] - 2026-04-25

- 初回リリース。

[Unreleased]: https://github.com/YoshitakaOyama/jpnorm/compare/v0.0.4...HEAD
[0.0.4]: https://github.com/YoshitakaOyama/jpnorm/compare/v0.0.3...v0.0.4
[0.0.3]: https://github.com/YoshitakaOyama/jpnorm/compare/v0.0.2...v0.0.3
[0.0.2]: https://github.com/YoshitakaOyama/jpnorm/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/YoshitakaOyama/jpnorm/releases/tag/v0.0.1
