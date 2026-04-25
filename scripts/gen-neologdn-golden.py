#!/usr/bin/env python3
"""neologdn のゴールデン出力を生成する。

実行例:
    /tmp/kanon-venv/bin/python scripts/gen-neologdn-golden.py \
        > tests/golden/neologdn.jsonl

各行は `{"input": ..., "expected": ...}` の JSON。
"""
from __future__ import annotations

import json
import sys

import neologdn

CASES: list[str] = [
    # 基本
    "ﾊﾝｶｸｶﾅ",
    "ｶﾞｷﾞｸﾞ",
    "ﾊﾟﾋﾟﾌﾟ",
    "ＡＢＣ１２３",
    # 記号類
    "ab‐cd—ef−gh",
    "a~b～c",
    "ーーーー",
    # 繰り返し
    "ウェーーーーイ",
    "wwwww",
    "すごーーーーい",
    # 空白
    "日本語 の 文章",
    "hello   world",
    "日本語 text 混在",
    "   trim me   ",
    # 複合
    "ﾊﾝｶｸｶﾅ　と  全角  ！！ーーー",
    "これは ｶﾀｶﾅ と 漢字 の 混在 テスト です〜〜〜",
    "ｵｰﾙｲﾝﾜﾝ ＡＢＣ 〜〜〜 ！！！",
    # 引用符
    "“hello” ‘world’",
    "「日本語」",
    # 英語混在
    "Rust is fast",
    "Python と Rust",
    # 数字・句読点
    "第一章と第二節",
    "2024年の１２月",
    # 記号混在
    "価格 ¥1,200 (税込)",
    # neologdn の挙動を拾いきれない領域も試す
    "URLは http://example.com/path を参照",
    "hello😀world",
]


def main() -> None:
    out = sys.stdout
    for s in CASES:
        rec = {"input": s, "expected": neologdn.normalize(s)}
        out.write(json.dumps(rec, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
