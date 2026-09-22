"""jpnorm — 日本語テキスト正規化ライブラリ (Rust コア + Python バインディング)。

Python 側はネイティブ拡張 ``jpnorm._native`` を再エクスポートする薄いラッパーと、
精度評価向けの比較ユーティリティ (:mod:`jpnorm.comparison`) からなる。
"""

from jpnorm._native import Normalizer, __version__, levenshtein, normalize
from jpnorm.comparison import ComparisonResult, ComparisonStrategy, compare

__all__ = [
    "ComparisonResult",
    "ComparisonStrategy",
    "Normalizer",
    "__version__",
    "compare",
    "levenshtein",
    "normalize",
]
