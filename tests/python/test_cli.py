"""Tests for the `jpnorm` command line interface."""

from __future__ import annotations

import io
import json
from pathlib import Path

import pytest

from jpnorm.cli import main


def run(
    argv: list[str], stdin: str = "", capsys: pytest.CaptureFixture[str] | None = None
) -> str:
    assert capsys is not None
    import sys

    old = sys.stdin
    sys.stdin = io.StringIO(stdin)
    try:
        code = main(argv)
    finally:
        sys.stdin = old
    assert code == 0
    return capsys.readouterr().out


def test_cli_normalizes_positional_text(capsys: pytest.CaptureFixture[str]) -> None:
    out = run(["ﾊﾝｶｸｶﾅ　と  全角  ！！"], capsys=capsys)
    assert out == "ハンカクカナ と 全角 !!\n"


def test_cli_reads_stdin_line_by_line(capsys: pytest.CaptureFixture[str]) -> None:
    out = run(["--preset", "for_search"], stdin="ｶﾅ😀\nＡＢＣ\n", capsys=capsys)
    assert out == "カナ\nabc\n"


def test_cli_set_overrides_flags(capsys: pytest.CaptureFixture[str]) -> None:
    out = run(
        [
            "-p",
            "for_search",
            "--set",
            "emoji=keep",
            "--set",
            "kana=kata_to_hira",
            "ｶﾅ😀",
        ],
        capsys=capsys,
    )
    assert out == "かな😀\n"


def test_cli_set_url_wrap_pair(capsys: pytest.CaptureFixture[str]) -> None:
    out = run(["-p", "none", "--set", "url_wrap=<,>", "see https://x.y"], capsys=capsys)
    assert out == "see <https://x.y>\n"


def test_cli_files_and_dict(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    src = tmp_path / "in.txt"
    src.write_text("幽白を読む\nPCを買う\n", encoding="utf-8")
    d = tmp_path / "d.json"
    d.write_text(
        json.dumps({"幽遊白書": ["幽白"], "パソコン": ["PC"]}), encoding="utf-8"
    )
    out = run(["--files", "--dict", str(d), str(src)], capsys=capsys)
    assert out == "幽遊白書を読む\nパソコンを買う\n"


def test_cli_json_output(capsys: pytest.CaptureFixture[str]) -> None:
    out = run(["--json", "ｶﾅ"], capsys=capsys)
    assert json.loads(out) == {"input": "ｶﾅ", "output": "カナ"}


def test_cli_list_presets_and_show_config(capsys: pytest.CaptureFixture[str]) -> None:
    out = run(["--list-presets"], capsys=capsys)
    assert out.splitlines() == [
        "none",
        "neologdn_compat",
        "for_search",
        "for_display",
        "for_compare",
    ]
    out = run(["-p", "for_compare", "--show-config"], capsys=capsys)
    assert json.loads(out)["kansuji_to_arabic"] is True


def test_cli_invalid_option_exits_with_usage_error() -> None:
    with pytest.raises(SystemExit) as e:
        main(["--set", "bogus=1", "x"])
    assert e.value.code == 2
