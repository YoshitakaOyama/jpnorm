"""jpnorm — 世界一の日本語正規化ライブラリ / World-class Japanese text normalization.

Python側はRustネイティブ拡張(`jpnorm._native`)を再エクスポートする薄いラッパー。
"""

from jpnorm._native import Normalizer, __version__, normalize
from jpnorm.comparison import (
    ComparisonResult,
    ComparisonStrategy,
    compare,
)

__all__ = [
    "ComparisonResult",
    "ComparisonStrategy",
    "Normalizer",
    "__version__",
    "compare",
    "normalize",
]
