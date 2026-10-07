"""tests/corpus の実データ風テキストに対する出力のスナップショットテスト。

失敗したら、出力の変化が意図どおりか確認し、
`uv run python scripts/update-corpus-snapshot.py` で再生成する。
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

import jpnorm

CORPUS = Path(__file__).resolve().parent.parent / "corpus"
PRESETS = ["neologdn_compat", "for_search", "for_compare", "for_display"]


def _load() -> list[dict[str, str]]:
    lines = (CORPUS / "snapshot.jsonl").read_text(encoding="utf-8").splitlines()
    return [json.loads(line) for line in lines]


def test_snapshot_covers_every_corpus_line() -> None:
    expected = [
        (path.name, line)
        for path in sorted(CORPUS.glob("*.txt"))
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    assert [(r["file"], r["input"]) for r in _load()] == expected, (
        "コーパスとスナップショットがずれています。"
        "scripts/update-corpus-snapshot.py で再生成してください"
    )


@pytest.mark.parametrize("preset", PRESETS)
def test_corpus_output_matches_snapshot(preset: str) -> None:
    n = jpnorm.Normalizer(preset)
    diffs = [
        f"[{r['file']}] {r['input']!r}\n  期待: {r[preset]!r}\n  実際: {actual!r}"
        for r in _load()
        if (actual := n.normalize(r["input"])) != r[preset]
    ]
    assert not diffs, f"{preset} の出力が {len(diffs)} 行変わりました:\n" + "\n".join(
        diffs
    )
