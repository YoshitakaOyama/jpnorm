"""`jpnorm` コマンド。標準入力またはファイルの各行を正規化して出力する。

例::

    echo "ﾊﾝｶｸｶﾅ　と  全角  ！！" | jpnorm
    jpnorm --preset for_search input.txt
    jpnorm --preset for_compare --set emoji=keep --set kana=kata_to_hira "テキスト"
    jpnorm --list-presets
"""

from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Iterable, Iterator, Sequence
from pathlib import Path
from typing import Any

from jpnorm._native import Normalizer, __version__

_TRUE = {"true", "1", "yes", "on"}
_FALSE = {"false", "0", "no", "off"}


def _parse_option(text: str) -> tuple[str, Any]:
    """`key=value` を Normalizer の kwargs に変換する。

    bool 風の値は bool、整数は int、`none` は None、それ以外は文字列として扱う。
    `url_wrap` は `prefix,suffix` の 2 要素タプルとして解釈する。
    """
    key, sep, raw = text.partition("=")
    key = key.strip()
    if not sep or not key:
        raise argparse.ArgumentTypeError(f"expected key=value, got {text!r}")
    value = raw.strip()
    lowered = value.lower()
    if key == "url_wrap":
        parts = value.split(",", 1)
        if len(parts) != 2:
            raise argparse.ArgumentTypeError("url_wrap expects 'prefix,suffix'")
        return key, (parts[0], parts[1])
    if lowered in _TRUE:
        return key, True
    if lowered in _FALSE:
        return key, False
    if lowered == "none":
        return key, None
    if value.lstrip("-").isdigit():
        return key, int(value)
    return key, value


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="jpnorm",
        description="日本語テキストを正規化する。入力は引数、ファイル、または標準入力。",
    )
    parser.add_argument(
        "inputs",
        nargs="*",
        help="正規化するテキスト (--files ならファイルパス)。省略時は標準入力",
    )
    parser.add_argument(
        "-p",
        "--preset",
        choices=Normalizer.presets(),
        default=None,
        help="プリセット名 (既定: neologdn_compat)",
    )
    parser.add_argument(
        "-s",
        "--set",
        dest="options",
        action="append",
        default=[],
        metavar="KEY=VALUE",
        type=_parse_option,
        help="個別フラグの上書き。複数指定可 (例: --set emoji=keep)",
    )
    parser.add_argument(
        "-d",
        "--dict",
        dest="dicts",
        action="append",
        default=[],
        metavar="PATH",
        type=Path,
        help="カスタム辞書ファイル (.json / .csv / .tsv)。複数指定可",
    )
    parser.add_argument(
        "--sudachi",
        metavar="PATH",
        type=Path,
        default=None,
        help="Sudachi 同義語辞書 synonyms.txt",
    )
    parser.add_argument(
        "-f",
        "--files",
        action="store_true",
        help="位置引数をテキストではなくファイルパスとして扱う",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help='1 行ごとに {"input": ..., "output": ...} の JSON Lines で出力',
    )
    parser.add_argument(
        "--show-config",
        action="store_true",
        help="実際に使う設定を JSON で表示して終了",
    )
    parser.add_argument(
        "--list-presets", action="store_true", help="プリセット一覧を表示"
    )
    parser.add_argument(
        "-V", "--version", action="version", version=f"jpnorm {__version__}"
    )
    return parser


def _iter_lines(paths: Sequence[Path]) -> Iterator[str]:
    for path in paths:
        with path.open(encoding="utf-8") as f:
            for line in f:
                yield line.rstrip("\n")


def _iter_inputs(args: argparse.Namespace) -> Iterable[str]:
    inputs: list[str] = list(args.inputs)
    if args.files:
        return _iter_lines([Path(p) for p in inputs])
    if inputs:
        return inputs
    return (line.rstrip("\n") for line in sys.stdin)


def main(argv: Sequence[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    if args.list_presets:
        for name in Normalizer.presets():
            print(name)
        return 0

    try:
        normalizer = Normalizer(args.preset, **dict(args.options))
        for path in args.dicts:
            normalizer.load_custom_dict_file(path)
        if args.sudachi is not None:
            normalizer.load_sudachi_synonyms(args.sudachi)
    except (TypeError, ValueError, OSError) as e:
        parser.error(str(e))

    if args.show_config:
        print(json.dumps(normalizer.config, ensure_ascii=False, indent=2))
        return 0

    out = sys.stdout
    for text in _iter_inputs(args):
        result = normalizer.normalize(text)
        if args.json:
            out.write(
                json.dumps({"input": text, "output": result}, ensure_ascii=False) + "\n"
            )
        else:
            out.write(result + "\n")
    return 0


if __name__ == "__main__":  # pragma: no cover
    sys.exit(main())
