import sys
import tempfile
from typing import Any, Sequence

import matplotlib.pyplot as plt
import pytest

import tests.utils
from valens import cli, config


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
        tests.utils.initialize_data(tmp_dir)
        monkeypatch.setattr(config, "DATA_DIRECTORY", tests.utils.initialize_data(tmp_dir))

        monkeypatch.setattr(sys, "argv", ["valens", *args])
        monkeypatch.setattr(plt, "show", lambda: None)
        assert cli.main() == 0


@pytest.mark.parametrize(
    "args",
    [
        ["-h"],
        ["list", "-h"],
        ["show", "-h"],
    ],
)
def test_main_help(monkeypatch: Any, args: Sequence[str]) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", *args])
    with pytest.raises(SystemExit):
        cli.main()
