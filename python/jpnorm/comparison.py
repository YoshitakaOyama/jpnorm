"""予測文字列と正解文字列を複数戦略で比較するユーティリティ。

精度評価ワークフローで「モデル出力 vs 正解データ」を突き合わせる際に使用する。
戦略は `exact` / `prefix` / `edit_distance` / `llm_judge` から選択できる。
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass, field
from enum import Enum
from typing import TYPE_CHECKING, Any, Callable

if TYPE_CHECKING:
    from jpnorm import Normalizer


class ComparisonStrategy(str, Enum):
    """比較戦略。"""

    EXACT = "exact"
    PREFIX = "prefix"
    EDIT_DISTANCE = "edit_distance"
    LLM_JUDGE = "llm_judge"


@dataclass(frozen=True)
class ComparisonResult:
    """比較結果。

    Attributes:
        matched: threshold を満たしたか。
        score: 一致度 0.0–1.0。exact/prefix は 0.0 または 1.0。
        strategy: 使用した戦略名。
        detail: 戦略固有の補助情報。
    """

    matched: bool
    score: float
    strategy: str
    detail: dict[str, Any] = field(default_factory=dict)


JudgeFn = Callable[[str, str], "ComparisonResult | bool"]


def compare(
    prediction: str,
    reference: str,
    strategy: ComparisonStrategy | str = ComparisonStrategy.EXACT,
    *,
    normalizer: Normalizer | None = None,
    threshold: float = 1.0,
    judge_fn: JudgeFn | None = None,
    llm_provider: str | None = None,
    llm_model: str | None = None,
) -> ComparisonResult:
    """予測文字列と正解文字列を指定戦略で比較する。

    Args:
        prediction: モデル出力。
        reference: 正解データ。
        strategy: `ComparisonStrategy` または文字列名。
        normalizer: 指定時は双方に `normalize()` を適用してから比較する。
        threshold: `edit_distance` / `llm_judge` で `matched` 判定に使う閾値。
        judge_fn: `llm_judge` 戦略で外部 LLM の代わりに呼ぶコールバック。
        llm_provider: `"anthropic"` または `"openai"`。`judge_fn` 未指定時のみ使用。
        llm_model: プロバイダのモデル ID。

    Returns:
        `ComparisonResult`。

    Raises:
        ValueError: 不明な戦略またはプロバイダ。
    """
    strategy_enum = _coerce_strategy(strategy)

    if strategy_enum in {
        ComparisonStrategy.EDIT_DISTANCE,
        ComparisonStrategy.LLM_JUDGE,
    }:
        _validate_threshold(threshold)

    if normalizer is not None:
        prediction = normalizer.normalize(prediction)
        reference = normalizer.normalize(reference)

    if strategy_enum is ComparisonStrategy.EXACT:
        return _compare_exact(prediction, reference)
    if strategy_enum is ComparisonStrategy.PREFIX:
        return _compare_prefix(prediction, reference)
    if strategy_enum is ComparisonStrategy.EDIT_DISTANCE:
        return _compare_edit_distance(prediction, reference, threshold)
    if strategy_enum is ComparisonStrategy.LLM_JUDGE:
        return _compare_llm_judge(
            prediction,
            reference,
            threshold=threshold,
            judge_fn=judge_fn,
            llm_provider=llm_provider,
            llm_model=llm_model,
        )
    raise ValueError(f"unknown strategy: {strategy}")


def _validate_threshold(threshold: float) -> float:
    if not 0.0 <= threshold <= 1.0:
        raise ValueError("threshold must be between 0.0 and 1.0")
    return threshold


def _coerce_strategy(
    strategy: ComparisonStrategy | str,
) -> ComparisonStrategy:
    if isinstance(strategy, ComparisonStrategy):
        return strategy
    try:
        return ComparisonStrategy(strategy)
    except ValueError as e:
        valid = ", ".join(s.value for s in ComparisonStrategy)
        raise ValueError(
            f"unknown strategy: {strategy!r} (valid: {valid})"
        ) from e


def _compare_exact(prediction: str, reference: str) -> ComparisonResult:
    matched = prediction == reference
    return ComparisonResult(
        matched=matched,
        score=1.0 if matched else 0.0,
        strategy=ComparisonStrategy.EXACT.value,
    )


def _compare_prefix(prediction: str, reference: str) -> ComparisonResult:
    pred_is_prefix = reference.startswith(prediction) and prediction != ""
    ref_is_prefix = prediction.startswith(reference) and reference != ""
    matched = pred_is_prefix or ref_is_prefix
    direction: str | None
    if pred_is_prefix and ref_is_prefix:
        direction = "equal"
    elif pred_is_prefix:
        direction = "prediction_is_prefix_of_reference"
    elif ref_is_prefix:
        direction = "reference_is_prefix_of_prediction"
    else:
        direction = None
    return ComparisonResult(
        matched=matched,
        score=1.0 if matched else 0.0,
        strategy=ComparisonStrategy.PREFIX.value,
        detail={"direction": direction},
    )


def _compare_edit_distance(
    prediction: str,
    reference: str,
    threshold: float,
) -> ComparisonResult:
    distance = _levenshtein(prediction, reference)
    length = max(len(prediction), len(reference))
    score = 1.0 if length == 0 else 1.0 - distance / length
    return ComparisonResult(
        matched=score >= threshold,
        score=score,
        strategy=ComparisonStrategy.EDIT_DISTANCE.value,
        detail={"distance": distance, "length": length},
    )


def _levenshtein(a: str, b: str) -> int:
    if a == b:
        return 0
    if len(a) == 0:
        return len(b)
    if len(b) == 0:
        return len(a)
    # 2 行 DP。a を行、b を列に取る。
    prev = list(range(len(b) + 1))
    curr = [0] * (len(b) + 1)
    for i, ca in enumerate(a, start=1):
        curr[0] = i
        for j, cb in enumerate(b, start=1):
            cost = 0 if ca == cb else 1
            curr[j] = min(
                prev[j] + 1,       # deletion
                curr[j - 1] + 1,   # insertion
                prev[j - 1] + cost # substitution
            )
        prev, curr = curr, prev
    return prev[len(b)]


def _compare_llm_judge(
    prediction: str,
    reference: str,
    *,
    threshold: float,
    judge_fn: JudgeFn | None,
    llm_provider: str | None,
    llm_model: str | None,
) -> ComparisonResult:
    if judge_fn is not None:
        raw = judge_fn(prediction, reference)
        if isinstance(raw, ComparisonResult):
            return raw
        if isinstance(raw, bool):
            return ComparisonResult(
                matched=raw,
                score=1.0 if raw else 0.0,
                strategy=ComparisonStrategy.LLM_JUDGE.value,
                detail={"source": "judge_fn"},
            )
        raise TypeError(
            "judge_fn must return ComparisonResult or bool, "
            f"got {type(raw).__name__}"
        )

    provider = (llm_provider or "anthropic").lower()
    if provider == "anthropic":
        score, reasoning = _llm_judge_anthropic(
            prediction, reference, llm_model or "claude-haiku-4-5"
        )
    elif provider == "openai":
        score, reasoning = _llm_judge_openai(
            prediction, reference, llm_model or "gpt-4o-mini"
        )
    else:
        raise ValueError(f"unknown llm_provider: {llm_provider!r}")

    return ComparisonResult(
        matched=score >= threshold,
        score=score,
        strategy=ComparisonStrategy.LLM_JUDGE.value,
        detail={"provider": provider, "reasoning": reasoning},
    )


def _build_judge_prompt(prediction: str, reference: str) -> str:
    return (
        "あなたはテキスト評価者です。"
        "『予測』と『正解』が意味的に等価かを 0.0〜1.0 のスコアで判定してください。"
        '出力は JSON のみ: {"score": <float>, "reasoning": <短い理由>}\n\n'
        f"予測: {prediction}\n"
        f"正解: {reference}"
    )


def _parse_judge_json(text: str) -> tuple[float, str]:
    # LLM 出力から JSON 部分だけ抜き出す。
    start = text.find("{")
    end = text.rfind("}")
    if start == -1 or end == -1:
        raise ValueError(f"LLM judge response is not JSON: {text!r}")
    data = json.loads(text[start : end + 1])
    score = float(data.get("score", 0.0))
    reasoning = str(data.get("reasoning", ""))
    return max(0.0, min(1.0, score)), reasoning


def _llm_judge_anthropic(
    prediction: str,
    reference: str,
    model: str,
) -> tuple[float, str]:
    try:
        import anthropic
    except ImportError as e:
        raise ImportError(
            "anthropic package is required for llm_judge with provider='anthropic'. "
            "Install with: pip install anthropic"
        ) from e
    api_key = os.environ.get("ANTHROPIC_API_KEY")
    if not api_key:
        raise RuntimeError("ANTHROPIC_API_KEY is not set")
    client = anthropic.Anthropic(api_key=api_key)
    prompt = _build_judge_prompt(prediction, reference)
    msg = client.messages.create(
        model=model,
        max_tokens=256,
        messages=[{"role": "user", "content": prompt}],
    )
    text = "".join(
        block.text for block in msg.content if getattr(block, "type", "") == "text"
    )
    return _parse_judge_json(text)


def _llm_judge_openai(
    prediction: str,
    reference: str,
    model: str,
) -> tuple[float, str]:
    try:
        import openai
    except ImportError as e:
        raise ImportError(
            "openai package is required for llm_judge with provider='openai'. "
            "Install with: pip install openai"
        ) from e
    api_key = os.environ.get("OPENAI_API_KEY")
    if not api_key:
        raise RuntimeError("OPENAI_API_KEY is not set")
    client = openai.OpenAI(api_key=api_key)
    prompt = _build_judge_prompt(prediction, reference)
    resp = client.chat.completions.create(
        model=model,
        messages=[{"role": "user", "content": prompt}],
        response_format={"type": "json_object"},
    )
    text = resp.choices[0].message.content or ""
    return _parse_judge_json(text)


__all__ = [
    "ComparisonResult",
    "ComparisonStrategy",
    "JudgeFn",
    "compare",
]
