"""Type stubs for jpnorm."""

from typing import Literal

from jpnorm.comparison import (
    ComparisonResult as ComparisonResult,
    ComparisonStrategy as ComparisonStrategy,
    compare as compare,
)

__version__: str

PresetName = Literal[
    "none",
    "neologdn_compat",
    "for_search",
    "for_display",
    "for_compare",
]

class Normalizer:
    """日本語テキスト正規化器。"""

    def __init__(self) -> None: ...
    @staticmethod
    def preset(name: PresetName) -> "Normalizer": ...
    def normalize(self, text: str) -> str: ...
    def with_custom_dict(self, mapping: dict[str, list[str]]) -> None: ...
    def load_custom_dict_json(self, json: str) -> None: ...

def normalize(text: str) -> str:
    """デフォルト設定でテキストを正規化する。"""
    ...
