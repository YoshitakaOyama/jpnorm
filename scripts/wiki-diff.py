#!/usr/bin/env python3
"""Wikipedia の記事で、手元のビルドと PyPI のリリースの出力差分を集計する。

リリース前の確認用。記事本文はライセンスと容量の都合でリポジトリに入れず、
実行時に取得して .cache/wiki/ に保存する。

実行例:
    uv run python scripts/wiki-diff.py                    # 既定の記事、最新リリースと比較
    uv run python scripts/wiki-diff.py --baseline 0.1.1
    uv run python scripts/wiki-diff.py 東京都 円周率 --preset for_compare --top 50

差分は (変換前の断片, 変換後の断片) ごとに件数と文脈を出す。
「行が短くなった」件数が増えていたら、文字が消える方向の変化なので要注意。
"""

from __future__ import annotations

import argparse
import collections
import difflib
import json
import subprocess
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path

import jpnorm

ROOT = Path(__file__).resolve().parent.parent
CACHE = ROOT / ".cache" / "wiki"
PRESETS = ["neologdn_compat", "for_search", "for_compare", "for_display"]
USER_AGENT = "jpnorm-wiki-diff/1.0 (https://github.com/YoshitakaOyama/jpnorm)"

# テーマを散らした既定の記事 (数値・固有名詞・外来語・旧字体・記号が多いもの)
DEFAULT_TITLES = [
    "東京都",
    "夏目漱石",
    "太平洋戦争",
    "ラーメン",
    "Python",
    "円周率",
    "日本国憲法",
    "東海道新幹線",
    "源氏物語",
    "大谷翔平",
    "髙島屋",
    "日本銀行",
    "北海道",
    "トヨタ自動車",
    "織田信長",
    "iPhone",
    "周期表",
    "インフルエンザ",
    "山手線",
    "ビートルズ",
    "鬼滅の刃",
    "将棋",
    "東日本大震災",
    "日本酒",
]

BASELINE_SNIPPET = """
import json, sys, jpnorm
data = json.load(sys.stdin)
out = {p: jpnorm.Normalizer(p).normalize_batch(data["lines"]) for p in data["presets"]}
json.dump({"version": jpnorm.__version__, "out": out}, sys.stdout, ensure_ascii=False)
"""


def fetch(title: str) -> str:
    path = CACHE / f"{title}.txt"
    if path.exists():
        return path.read_text(encoding="utf-8")
    query = urllib.parse.urlencode(
        {
            "action": "query",
            "prop": "extracts",
            "explaintext": 1,
            "titles": title,
            "format": "json",
            "redirects": 1,
        }
    )
    request = urllib.request.Request(
        f"https://ja.wikipedia.org/w/api.php?{query}",
        headers={"User-Agent": USER_AGENT},
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        pages = json.load(response)["query"]["pages"]
    text = next(iter(pages.values())).get("extract", "")
    CACHE.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    time.sleep(0.5)  # API に負荷をかけない
    return text


def run_baseline(version: str | None, lines: list[str], presets: list[str]) -> dict:
    spec = f"jpnorm=={version}" if version else "jpnorm"
    payload = json.dumps({"lines": lines, "presets": presets}, ensure_ascii=False)
    result = subprocess.run(
        [
            "uv",
            "run",
            "--no-project",
            "--isolated",
            "--with",
            spec,
            "python",
            "-c",
            BASELINE_SNIPPET,
        ],
        input=payload,
        capture_output=True,
        text=True,
        check=True,
        cwd="/",
    )
    return json.loads(result.stdout)


def summarize(lines: list[str], old: list[str], new: list[str], top: int) -> None:
    pairs: collections.Counter[tuple[str, str]] = collections.Counter()
    examples: dict[tuple[str, str], tuple[str, str]] = {}
    changed = shorter = 0
    for before, after in zip(old, new, strict=True):
        if before == after:
            continue
        changed += 1
        shorter += len(after) < len(before)
        matcher = difflib.SequenceMatcher(None, before, after, autojunk=False)
        for op, i1, i2, j1, j2 in matcher.get_opcodes():
            if op == "equal":
                continue
            key = (before[i1:i2], after[j1:j2])
            pairs[key] += 1
            examples.setdefault(
                key,
                (before[max(0, i1 - 10) : i2 + 10], after[max(0, j1 - 10) : j2 + 10]),
            )
    print(f"  変化した行: {changed} / {len(lines)} (短くなった行: {shorter})")
    for (src, dst), count in pairs.most_common(top):
        ctx_old, ctx_new = examples[(src, dst)]
        print(
            f"  {count:5d}  {src!r} → {dst!r}\n         {ctx_old!r}\n      => {ctx_new!r}"
        )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("titles", nargs="*", help="記事名 (省略時は既定の記事)")
    parser.add_argument("--baseline", help="比較する PyPI のバージョン (既定: 最新)")
    parser.add_argument("--preset", action="append", choices=PRESETS)
    parser.add_argument("--top", type=int, default=20, help="表示する差分の種類数")
    args = parser.parse_args()

    titles = args.titles or DEFAULT_TITLES
    presets = args.preset or PRESETS
    lines = [
        line for title in titles for line in fetch(title).splitlines() if line.strip()
    ]
    baseline = run_baseline(args.baseline, lines, presets)
    chars = sum(map(len, lines))
    print(f"{len(titles)} 記事 / {len(lines)} 行 / {chars:,} 字")
    print(f"比較: PyPI {baseline['version']} → 手元 {jpnorm.__version__}")
    for preset in presets:
        print(f"\n## {preset}")
        new = jpnorm.Normalizer(preset).normalize_batch(lines)
        summarize(lines, baseline["out"][preset], new, args.top)


if __name__ == "__main__":
    sys.exit(main())
