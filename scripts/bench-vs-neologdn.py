#!/usr/bin/env python3
"""jpnorm と neologdn のスループットを同じ入力で比較する。

実行例:
    uv run --with neologdn scripts/bench-vs-neologdn.py

出力は README に貼れる Markdown 表。
"""

from __future__ import annotations

import platform
import statistics
import time
from collections.abc import Callable

import neologdn

import jpnorm

TWEET = "ｶﾀｶﾅ と  全角 ！！ーーー 〜〜〜 wwwwww https://example.com/path?x=1 @alice #rust_lang 😀"
PARAGRAPH = (
    "ある日ﾊﾝｶｸｶﾅが混ざった文章を見つけた。ＡＢＣ１２３のような全角英数や、\n"
    '「曲線引用符」と"直線引用符"、ハイフン類(‐—−)も入り乱れていた。\n'
    "ウェーーーイ、ーーーっていう繰り返しや   複数スペース  も頻出。\n"
    "連絡先は foo.bar@example.co.jp、詳しくは https://example.com/docs?lang=ja を参照。\n"
    "#jpnorm @dev 🎉🎉🎉"
)
SAMPLES = {
    "tweet (~100B)": TWEET,
    "paragraph (~500B)": PARAGRAPH,
    "large (~32KB)": PARAGRAPH * 64,
}


def bench(fn: Callable[[str], str], text: str, *, seconds: float = 1.0) -> float:
    """`fn(text)` の 1 回あたり秒数 (中央値) を返す。"""
    fn(text)  # warm-up
    # 1 回が短すぎる場合はまとめて計測する
    reps = 1
    while True:
        t0 = time.perf_counter()
        for _ in range(reps):
            fn(text)
        if time.perf_counter() - t0 > 0.02:
            break
        reps *= 4
    samples: list[float] = []
    deadline = time.perf_counter() + seconds
    while time.perf_counter() < deadline:
        t0 = time.perf_counter()
        for _ in range(reps):
            fn(text)
        samples.append((time.perf_counter() - t0) / reps)
    return statistics.median(samples)


def main() -> None:
    n_compat = jpnorm.Normalizer("neologdn_compat")
    n_search = jpnorm.Normalizer("for_search")
    candidates: dict[str, Callable[[str], str]] = {
        "neologdn": neologdn.normalize,
        "jpnorm neologdn_compat": n_compat.normalize,
        "jpnorm for_search": n_search.normalize,
    }
    print(
        f"<!-- {platform.machine()} / Python {platform.python_version()} / jpnorm {jpnorm.__version__} -->"
    )
    print("| 入力 | " + " | ".join(candidates) + " |")
    print("|---|" + "---:|" * len(candidates))
    for label, text in SAMPLES.items():
        cells = []
        base = None
        for fn in candidates.values():
            sec = bench(fn, text)
            mbps = len(text.encode()) / sec / 1e6
            if base is None:
                base = sec
                cells.append(f"{mbps:.0f} MB/s")
            else:
                cells.append(f"{mbps:.0f} MB/s ({base / sec:.1f}x)")
        print(f"| {label} | " + " | ".join(cells) + " |")


if __name__ == "__main__":
    main()
