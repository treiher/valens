import sys
from pathlib import Path

from pytest import MonkeyPatch

from valens import cli, database as db


def test_main_noarg(monkeypatch: MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens"])
    assert cli.main() == 2


def test_main_version(monkeypatch: MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "--version"])
    assert cli.main() == 0


def test_main_create_config(monkeypatch: MonkeyPatch, tmp_path: Path) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "--create-config"])
    config_file = tmp_path / "config.py"
    monkeypatch.setattr(cli, "CONFIG_FILE", config_file)
    assert cli.main() == 0
    assert "SECRET_KEY" in config_file.read_text()


def test_main_upgrade(monkeypatch: MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "--upgrade"])
    upgrade_called = []
    monkeypatch.setattr(db, "upgrade_db", lambda: upgrade_called.append(True))
    assert cli.main() == 0
    assert upgrade_called
