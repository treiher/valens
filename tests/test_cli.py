import sys
import tempfile
from typing import Any, Sequence

import matplotlib.pyplot as plt
import pytest  # type: ignore

from tests import utils
from valens import cli, storage


def test_main_noarg(monkeypatch: Any) -> None:
    monkeypatch.setattr(sys, "argv", ["valens"])
    assert cli.main() == 2


@pytest.mark.parametrize(
    "args",
    [
        ["list"],
        ["list", "--short"],
        ["list", "--last"],
        ["show", "bw"],
        ["show", "ex", "foo"],
        ["show", "wo"],
    ],
)
def test_main(monkeypatch: Any, args: Sequence[str]) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        utils.initialize_data(tmp_dir)
        monkeypatch.setattr(storage.utils, "parse_config", lambda: utils.config(tmp_dir))

        monkeypatch.setattr(sys, "argv", ["valens", *args])
        monkeypatch.setattr(plt, "show", lambda: None)
        assert cli.main() == 0


@pytest.mark.parametrize(
    "args", [["-h"], ["list", "-h"], ["show", "-h"]],
)
def test_main_help(monkeypatch: Any, args: Sequence[str]) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", *args])
    with pytest.raises(SystemExit):
        cli.main()


def test_format_set() -> None:
    nan = float("nan")
    assert cli.format_set((0, 10, nan, nan, nan)) == "10"
    assert cli.format_set((0, nan, 60, nan, nan)) == "60s"
    assert cli.format_set((0, nan, nan, 20, nan)) == "20.0kg"
    assert cli.format_set((0, nan, nan, nan, 8)) == "@8"
    assert cli.format_set((0, 10, nan, 20, nan)) == "10x20.0kg"
    assert cli.format_set((0, 10, nan, nan, 8)) == "10@8"
    assert cli.format_set((0, 10, nan, 20, 8)) == "10x20.0kg@8"
    assert cli.format_set((0, 10, 60, 20, 8)) == "10x60sx20.0kg@8"
