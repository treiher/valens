import sys
from typing import Any

from valens import cli, database as db


def test_main_noarg(monkeypatch: Any) -> None:
    monkeypatch.setattr(sys, "argv", ["valens"])
    assert cli.main() == 2


def test_main_version(monkeypatch: Any) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "--version"])
    assert cli.main() == 0


def test_main_init(monkeypatch: Any) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "--init"])
    init_called = []
    monkeypatch.setattr(db, "init_db", lambda: init_called.append(True))
    assert cli.main() == 0
    assert init_called
