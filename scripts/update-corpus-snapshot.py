#!/usr/bin/env python3
"""tests/corpus/*.txt を全プリセットで正規化し、スナップショットを書き出す。

実行例:
    uv run python scripts/update-corpus-snapshot.py

出力は tests/corpus/snapshot.jsonl。1 行が入力 1 行に対応し、
`{"file": ..., "input": ..., "<preset>": <出力>, ...}` の JSON になる。
正規化の挙動を変えたら再生成し、差分をレビューしてからコミットする。
"""

from __future__ import annotations

import json
from pathlib import Path

import jpnorm

ROOT = Path(__file__).resolve().parent.parent
CORPUS = ROOT / "tests" / "corpus"
PRESETS = ["neologdn_compat", "for_search", "for_compare", "for_display"]


def build() -> list[dict[str, str]]:
    normalizers = {p: jpnorm.Normalizer(p) for p in PRESETS}
    rows: list[dict[str, str]] = []
    for path in sorted(CORPUS.glob("*.txt")):
        for line in path.read_text(encoding="utf-8").splitlines():
            if not line.strip():
                continue
            row = {"file": path.name, "input": line}
            row.update({p: n.normalize(line) for p, n in normalizers.items()})
            rows.append(row)
    return rows


def main() -> None:
    rows = build()
    with (CORPUS / "snapshot.jsonl").open("w", encoding="utf-8") as f:
        for row in rows:
            f.write(json.dumps(row, ensure_ascii=False) + "\n")
    print(f"{len(rows)} 行を書き出しました")


if __name__ == "__main__":
    main()
