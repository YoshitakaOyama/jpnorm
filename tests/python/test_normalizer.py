"""Tests for the native Normalizer bindings."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

import jpnorm
from jpnorm import Normalizer


def test_module_level_normalize_uses_neologdn_compat():
    assert jpnorm.normalize("ﾊﾝｶｸｶﾅ　と  全角  ！！") == "ハンカクカナ と 全角 !!"


def test_module_level_normalize_accepts_preset():
    assert (
        jpnorm.normalize("東京タワー🗼を見学", preset="for_search")
        == "東京タワーを見学"
    )


def test_presets_lists_all_names():
    names = Normalizer.presets()
    assert names == [
        "none",
        "neologdn_compat",
        "for_search",
        "for_display",
        "for_compare",
    ]
    for name in names:
        assert isinstance(Normalizer.preset(name), Normalizer)  # type: ignore[arg-type]


def test_unknown_preset_lists_valid_names():
    with pytest.raises(ValueError, match="for_search"):
        Normalizer("bogus")  # type: ignore[arg-type]


def test_constructor_preset_equals_static_preset():
    text = "ｶﾅ😀 https://example.com/A"
    expected = Normalizer.preset("for_search").normalize(text)
    assert Normalizer("for_search").normalize(text) == expected


def test_options_override_preset_flags():
    keep = Normalizer("for_search", emoji="keep")
    assert keep.normalize("ｶﾅ😀") == "カナ😀"
    assert Normalizer("for_search").normalize("ｶﾅ😀") == "カナ"


def test_kana_option():
    assert Normalizer("none", kana="hira_to_kata").normalize("ひらがな") == "ヒラガナ"
    assert Normalizer("none", kana="kata_to_hira").normalize("カタカナ") == "かたかな"


def test_emoji_placeholder_option():
    n = Normalizer("none", emoji_placeholder="[e]")
    assert n.normalize("hi 🎉🎉!") == "hi [e]!"


def test_url_wrap_option_enables_protection():
    n = Normalizer("none", url_wrap=("<", ">"))
    assert n.normalize("see https://example.com now") == "see <https://example.com> now"
    assert n.config["protect_urls"] is True


def test_repeat_limit_option_validation():
    with pytest.raises(ValueError, match="repeat_limit"):
        Normalizer("none", repeat_limit=0)
    n = Normalizer("none", repeat_limit=2)
    assert n.normalize("あああああ") == "ああ"
    assert Normalizer("none", repeat_limit=None).normalize("あああ") == "あああ"


def test_unknown_option_raises_type_error():
    with pytest.raises(TypeError, match="unknown option"):
        Normalizer("none", bogus=True)  # type: ignore[call-arg]


def test_wrong_option_type_raises_type_error():
    with pytest.raises(TypeError, match="must be bool"):
        Normalizer("none", nfkc="yes")  # type: ignore[typeddict-item]


def test_conflicting_numeral_options_rejected():
    with pytest.raises(ValueError, match="cannot both"):
        Normalizer("none", kansuji_to_arabic=True, arabic_to_kansuji=True)


def test_config_round_trips_through_constructor():
    n = Normalizer("for_compare", emoji_placeholder="_", url_wrap=("[", "]"))
    clone = Normalizer(**n.config)
    assert clone.config == n.config
    text = "三百二十円 😀 https://x.y/z"
    assert clone.normalize(text) == n.normalize(text)


def test_repr_shows_preset_or_options():
    assert repr(Normalizer("for_search")) == "Normalizer(preset='for_search')"
    r = repr(Normalizer("for_search", nfkc=False))
    assert r.startswith("Normalizer(") and "nfkc=False" in r


def test_normalize_batch_matches_single():
    n = Normalizer("for_search")
    texts = ["ｶﾅ", "ＡＢＣ", "  x  "]
    assert n.normalize_batch(texts) == [n.normalize(t) for t in texts]
    assert n.normalize_batch([]) == []


def test_custom_dict_merges_and_chains():
    n = (
        Normalizer()
        .with_custom_dict({"幽遊白書": ["幽白", "ゆうはく"]})
        .with_custom_dict({"Python": ("パイソン", "ぱいそん")})
    )
    assert n.custom_dict_size == 4
    assert n.normalize("幽白とぱいそん") == "幽遊白書とPython"
    n.clear_custom_dict()
    assert n.custom_dict_size == 0
    assert n.normalize("幽白") == "幽白"


def test_custom_dict_keys_match_raw_notation_under_aggressive_preset():
    n = Normalizer("for_compare").with_custom_dict(
        {"株式会社サンプル": ["サンプル社", "(株)サンプル", "㈱サンプル"]}
    )
    records = ["㈱サンプル", "サンプル社", "株式会社サンプル", "(株)サンプル"]
    assert {n.normalize(r) for r in records} == {"株式会社サンプル"}


def test_custom_dict_rejects_str_value():
    with pytest.raises(TypeError, match="list of str"):
        Normalizer().with_custom_dict({"a": "b"})  # type: ignore[dict-item]


def test_custom_dict_json_accepts_ensure_ascii_output():
    payload = json.dumps({"幽遊白書": ["幽白", "幽☆遊☆白書"]})  # \\uXXXX エスケープ
    n = Normalizer().load_custom_dict_json(payload)
    assert n.normalize("幽☆遊☆白書") == "幽遊白書"


def test_custom_dict_json_error_is_descriptive():
    with pytest.raises(ValueError, match="invalid custom dict json"):
        Normalizer().load_custom_dict_json('{"a": "b"}')


def test_custom_dict_file_formats(tmp_path: Path):
    (tmp_path / "d.json").write_text(json.dumps({"パソコン": ["PC"]}), encoding="utf-8")
    (tmp_path / "d.csv").write_text(
        "# comment\nJR東,東日本旅客鉄道\n", encoding="utf-8"
    )
    (tmp_path / "d.tsv").write_text("受付\t受け付け\n", encoding="utf-8")
    n = (
        Normalizer("none")
        .load_custom_dict_file(tmp_path / "d.json")
        .load_custom_dict_file(str(tmp_path / "d.csv"))
        .load_custom_dict_file(tmp_path / "d.tsv")
    )
    assert n.normalize("PCでJR東の受付") == "パソコンで東日本旅客鉄道の受け付け"
    with pytest.raises(ValueError, match="unknown custom dict format"):
        Normalizer().load_custom_dict_file(tmp_path / "d.xyz")
    with pytest.raises(OSError):
        Normalizer().load_custom_dict_file(tmp_path / "missing.json")


def test_sudachi_synonyms_file(tmp_path: Path):
    fixture = (
        "000001,1,0,1,0,0,0,IT,パーソナルコンピュータ,,\n"
        "000001,1,0,1,2,0,0,IT,パソコン,,\n"
        "000001,1,0,1,2,1,0,IT,PC,,\n"
    )
    path = tmp_path / "synonyms.txt"
    path.write_text(fixture, encoding="utf-8")
    n = Normalizer("none").load_sudachi_synonyms(path)
    assert (
        n.normalize("PCとパソコン") == "パーソナルコンピュータとパーソナルコンピュータ"
    )


def test_levenshtein_native():
    assert jpnorm.levenshtein("kitten", "sitting") == 3
    assert jpnorm.levenshtein("", "") == 0
    assert jpnorm.levenshtein("東京", "東京都") == 1


def test_version_is_exposed():
    assert isinstance(jpnorm.__version__, str) and jpnorm.__version__


# ---- Tier 1: 語彙・表記の深さ ----


def test_kyujitai_and_itaiji_options() -> None:
    n = Normalizer("none", kyujitai_to_shinjitai=True, unify_itaiji=True)
    assert (
        n.normalize("舊字體の國語學と髙橋・﨑山・渡邊")
        == "旧字体の国語学と高橋・崎山・渡辺"
    )
    assert Normalizer("none").normalize("髙橋") == "髙橋"


def test_variation_selectors_removed_in_search_preset() -> None:
    assert Normalizer("for_search").normalize("葛\U000e0100飾") == "葛飾"


def test_iteration_marks_option() -> None:
    n = Normalizer("none", expand_iteration_marks=True)
    assert n.normalize("人々といすゞとこゝろ") == "人人といすずとこころ"


def test_loanword_options() -> None:
    n = Normalizer("none", unify_loanword_kana=True, strip_trailing_prolonged=True)
    assert (
        n.normalize("ヴァイオリンとコンピューターとウェブとキー")
        == "バイオリンとコンピュータとウエブとキー"
    )


def test_case_option() -> None:
    assert (
        Normalizer("none", case="lower").normalize("Python と RUST") == "python と rust"
    )
    assert Normalizer("none", case="upper").normalize("Python") == "PYTHON"
    with pytest.raises(ValueError, match="must be one of"):
        Normalizer("none", case="title")  # type: ignore[typeddict-item]


def test_cjk_spacing_option() -> None:
    assert (
        Normalizer("none", cjk_spacing="remove").normalize("Python と Rust で実装")
        == "PythonとRustで実装"
    )
    assert (
        Normalizer("none", cjk_spacing="insert").normalize("日本語text混在")
        == "日本語 text 混在"
    )


def test_era_option() -> None:
    n = Normalizer("none", era_to_western=True, kansuji_to_arabic=True)
    assert n.normalize("令和六年とH30年度と昭和64年") == "2024年と2018年度と1989年"
    assert (
        Normalizer("none", era_to_western=True).normalize("令和の時代") == "令和の時代"
    )


def test_mixed_numerals_in_compare_preset() -> None:
    n = Normalizer("for_compare")
    assert n.normalize("1万2千円と1.5億と3千円") == "12000円と150000000と3000円"
    assert n.normalize("2千") == "2000"


def test_search_preset_unifies_common_variants() -> None:
    n = Normalizer("for_search")
    pairs = [
        ("ＰＹＴＨＯＮ入門", "python 入門"),
        ("コンピューター", "コンピュータ"),
        ("ヴァイオリン", "バイオリン"),
        ("渡邊", "渡辺"),
        ("佐々木", "佐佐木"),
        ("令和6年度", "2024年度"),
    ]
    for a, b in pairs:
        assert n.normalize(a).replace(" ", "") == n.normalize(b).replace(" ", ""), (
            a,
            b,
        )


def test_config_round_trip_includes_new_keys() -> None:
    n = Normalizer("for_compare")
    cfg = n.config
    for key in ["kyujitai_to_shinjitai", "case", "cjk_spacing", "era_to_western"]:
        assert key in cfg
    assert cfg["case"] == "lower" and cfg["cjk_spacing"] == "remove"
    assert Normalizer(**cfg).config == cfg
