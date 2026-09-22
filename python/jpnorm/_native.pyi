"""Type stubs for the native extension ``jpnorm._native``."""

from collections.abc import Iterable, Mapping
from os import PathLike
from typing import Any, Literal, TypedDict

from typing_extensions import Self, Unpack

__version__: str

PresetName = Literal[
    "none",
    "neologdn_compat",
    "for_search",
    "for_display",
    "for_compare",
]
KanaOption = Literal["keep", "hira_to_kata", "kata_to_hira"]
EmojiOption = Literal["keep", "remove"]
DictFormat = Literal["json", "csv", "tsv"]

class NormalizerOptions(TypedDict, total=False):
    """`Normalizer(**options)` の個別フラグ。`Normalizer.config` と同じキー。"""

    normalize_newlines: bool
    remove_zero_width: bool
    remove_control: bool
    remove_bidi_control: bool
    kansuji_to_arabic: bool
    arabic_to_kansuji: bool
    canonicalize_numbers: bool
    nfkc: bool
    halfwidth_kana_to_fullwidth: bool
    kana: KanaOption
    unify_hyphens: bool
    unify_tildes: bool
    unify_prolonged: bool
    collapse_prolonged_run: bool
    repeat_limit: int | None
    collapse_spaces: bool
    trim: bool
    expand_cjk_compat: bool
    remove_symbols: bool
    remove_cjk_compat: bool
    unify_quotes: bool
    protect_urls: bool
    protect_emails: bool
    protect_mentions: bool
    protect_hashtags: bool
    emoji: EmojiOption
    emoji_placeholder: str | None
    url_wrap: tuple[str, str] | None

class Normalizer:
    """日本語テキスト正規化器。

    プリセット名をベースに、キーワード引数で個別フラグを上書きして構築する::

        Normalizer("for_search", emoji="keep", kana="kata_to_hira")
    """

    def __init__(
        self,
        preset: PresetName | None = None,
        **options: Unpack[NormalizerOptions],
    ) -> None: ...
    @staticmethod
    def preset(name: PresetName) -> Normalizer: ...
    @staticmethod
    def presets() -> list[str]: ...
    @property
    def config(self) -> dict[str, Any]: ...
    @property
    def custom_dict_size(self) -> int: ...
    def normalize(self, text: str) -> str: ...
    def normalize_batch(self, texts: Iterable[str]) -> list[str]: ...
    def with_custom_dict(self, mapping: Mapping[str, Iterable[str]]) -> Self: ...
    def load_custom_dict_json(self, json: str) -> Self: ...
    def load_custom_dict_file(
        self,
        path: str | PathLike[str],
        format: DictFormat | None = None,
    ) -> Self: ...
    def load_sudachi_synonyms(self, path: str | PathLike[str]) -> Self: ...
    def clear_custom_dict(self) -> Self: ...

def normalize(text: str, preset: PresetName | None = None) -> str:
    """プリセット(既定は ``neologdn_compat``)でテキストを正規化する。"""
    ...

def levenshtein(a: str, b: str) -> int:
    """2 文字列のレーベンシュタイン距離(文字単位)。"""
    ...
