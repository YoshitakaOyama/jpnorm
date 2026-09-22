"""Type stubs for jpnorm."""

from jpnorm._native import (
    CaseOption as CaseOption,
    CjkSpacingOption as CjkSpacingOption,
    DictFormat as DictFormat,
    EmojiOption as EmojiOption,
    KanaOption as KanaOption,
    Normalizer as Normalizer,
    NormalizerOptions as NormalizerOptions,
    PresetName as PresetName,
    __version__ as __version__,
    levenshtein as levenshtein,
    normalize as normalize,
)
from jpnorm.comparison import (
    ComparisonResult as ComparisonResult,
    ComparisonStrategy as ComparisonStrategy,
    compare as compare,
)
