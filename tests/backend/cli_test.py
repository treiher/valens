import sys
from pathlib import Path

import pytest
from pytest import MonkeyPatch

from valens import app, cli, config, database as db, demo


def test_main_noarg(monkeypatch: MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens"])
    assert cli.main() == 2


def test_main_version(monkeypatch: MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "--version"])
    with pytest.raises(SystemExit, match="0"):
        cli.main()


def test_main_config(monkeypatch: MonkeyPatch, tmp_path: Path) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "config", "-d", str(tmp_path)])
    config_file = tmp_path / "config.py"
    assert cli.main() == 0
    assert "SECRET_KEY" in config_file.read_text()


def test_main_upgrade(monkeypatch: MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "upgrade"])
    called = []
    monkeypatch.setattr(config, "check_config_file", lambda x: called.append("check_config_file"))
    monkeypatch.setattr(db, "upgrade_db", lambda: called.append("upgrade_db"))
    assert cli.main() == 0
    assert called == ["check_config_file", "upgrade_db"]


def test_main_run(monkeypatch: MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "run"])
    called = []
    monkeypatch.setattr(config, "check_config_file", lambda x: called.append("check_config_file"))
    monkeypatch.setattr(app, "run", lambda x, y: called.append("run"))
    assert cli.main() == 0
    assert called == ["check_config_file", "run"]


def test_main_run_public(monkeypatch: MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "run", "--public"])
    called = []
    monkeypatch.setattr(config, "check_config_file", lambda x: called.append("check_config_file"))
    monkeypatch.setattr(app, "run", lambda x, y: called.append("run"))
    assert cli.main() == 0
    assert called == ["check_config_file", "run"]


def test_main_demo(monkeypatch: MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "demo"])
    demo_called = []
    monkeypatch.setattr(demo, "run", lambda x, y, z: demo_called.append(True))
    assert cli.main() == 0
    assert demo_called


def test_main_demo_public(monkeypatch: MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "demo", "--public"])
    demo_called = []
    monkeypatch.setattr(demo, "run", lambda x, y, z: demo_called.append(True))
    assert cli.main() == 0
    assert demo_called
