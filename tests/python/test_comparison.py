"""Tests for jpnorm.comparison."""

from __future__ import annotations

import pytest

from jpnorm.comparison import (
    ComparisonResult,
    ComparisonStrategy,
    compare,
)


def test_exact_match_when_strings_are_equal_returns_matched():
    result = compare("abc", "abc", strategy="exact")
    assert result.matched is True
    assert result.score == 1.0
    assert result.strategy == "exact"


def test_exact_match_when_strings_differ_returns_not_matched():
    result = compare("abc", "abd", strategy=ComparisonStrategy.EXACT)
    assert result.matched is False
    assert result.score == 0.0


def test_prefix_match_when_prediction_is_prefix_of_reference():
    result = compare("東京都", "東京都渋谷区", strategy="prefix")
    assert result.matched is True
    assert result.detail["direction"] == "prediction_is_prefix_of_reference"


def test_prefix_match_when_reference_is_prefix_of_prediction():
    result = compare("東京都渋谷区", "東京都", strategy="prefix")
    assert result.matched is True
    assert result.detail["direction"] == "reference_is_prefix_of_prediction"


def test_prefix_match_when_neither_is_prefix():
    result = compare("大阪府", "東京都", strategy="prefix")
    assert result.matched is False
    assert result.detail["direction"] is None


def test_prefix_match_ignores_empty_string():
    # 空文字は技術的には全ての prefix だが match とは扱わない。
    result = compare("", "abc", strategy="prefix")
    assert result.matched is False


def test_edit_distance_computes_correct_score():
    # "kitten" vs "sitting": distance=3, length=7
    result = compare("kitten", "sitting", strategy="edit_distance", threshold=0.5)
    assert result.detail == {"distance": 3, "length": 7}
    assert result.score == pytest.approx(1 - 3 / 7)
    assert result.matched is True


def test_edit_distance_threshold_not_met():
    result = compare("abc", "xyz", strategy="edit_distance", threshold=0.5)
    assert result.score == 0.0
    assert result.matched is False


def test_edit_distance_empty_strings_are_fully_matched():
    result = compare("", "", strategy="edit_distance", threshold=1.0)
    assert result.score == 1.0
    assert result.matched is True
    assert result.detail == {"distance": 0, "length": 0}


def test_edit_distance_identical_strings():
    result = compare("同じ", "同じ", strategy="edit_distance")
    assert result.score == 1.0
    assert result.matched is True


def test_normalizer_applied_before_compare_absorbs_variation():
    from jpnorm import Normalizer

    normalizer = Normalizer.preset("for_compare")
    # 半角カナと全角カナを吸収する想定。
    result = compare(
        "ﾃｽﾄ",
        "テスト",
        strategy="exact",
        normalizer=normalizer,
    )
    assert result.matched is True


def test_llm_judge_with_injected_judge_fn_returning_result():
    def judge_fn(pred: str, ref: str) -> ComparisonResult:
        assert pred == "出力"
        assert ref == "正解"
        return ComparisonResult(
            matched=True,
            score=0.85,
            strategy="llm_judge",
            detail={"reasoning": "意味的に近い"},
        )

    result = compare(
        "出力",
        "正解",
        strategy="llm_judge",
        judge_fn=judge_fn,
    )
    assert result.matched is True
    assert result.score == 0.85
    assert result.detail["reasoning"] == "意味的に近い"


def test_llm_judge_with_injected_judge_fn_returning_bool():
    def judge_fn(pred: str, ref: str) -> bool:
        return True

    result = compare("a", "b", strategy="llm_judge", judge_fn=judge_fn)
    assert result.matched is True
    assert result.score == 1.0
    assert result.strategy == "llm_judge"


def test_unknown_strategy_raises_value_error():
    with pytest.raises(ValueError, match="unknown strategy"):
        compare("a", "b", strategy="fuzzy_magic")


def test_unknown_llm_provider_raises_value_error():
    with pytest.raises(ValueError, match="unknown llm_provider"):
        compare(
            "a",
            "b",
            strategy="llm_judge",
            llm_provider="bogus",
        )
