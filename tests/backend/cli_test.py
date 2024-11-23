import sys
from pathlib import Path

import pytest

from valens import app, cli, config, database as db, demo


def test_main_noarg(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens"])
    assert cli.main() == 2


def test_main_version(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "--version"])
    with pytest.raises(SystemExit, match="0"):
        cli.main()


def test_main_config(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "config", "-d", str(tmp_path)])
    config_file = tmp_path / "config.py"
    assert cli.main() == 0
    assert "SECRET_KEY" in config_file.read_text()


def test_main_upgrade(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "upgrade"])
    called = []
    monkeypatch.setattr(config, "check_config_file", lambda x: called.append("check_config_file"))
    monkeypatch.setattr(db, "upgrade", lambda: called.append("upgrade"))
    assert cli.main() == 0
    assert called == ["check_config_file", "upgrade"]


def test_main_run(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "run"])
    called = []
    monkeypatch.setattr(config, "check_config_file", lambda x: called.append("check_config_file"))
    monkeypatch.setattr(app, "run", lambda x, y: called.append("run"))
    assert cli.main() == 0
    assert called == ["check_config_file", "run"]


def test_main_run_public(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "run", "--public"])
    called = []
    monkeypatch.setattr(config, "check_config_file", lambda x: called.append("check_config_file"))
    monkeypatch.setattr(app, "run", lambda x, y: called.append("run"))
    assert cli.main() == 0
    assert called == ["check_config_file", "run"]


def test_main_demo(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "demo"])
    demo_called = []
    monkeypatch.setattr(demo, "run", lambda x, y, z: demo_called.append(1))
    assert cli.main() == 0
    assert demo_called


def test_main_demo_public(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "demo", "--public"])
    demo_called = []
    monkeypatch.setattr(demo, "run", lambda x, y, z: demo_called.append(1))
    assert cli.main() == 0
    assert demo_called


def test_main_demo_db_exists(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    db_file = tmp_path / "db"
    db_file.touch()
    monkeypatch.setattr(sys, "argv", ["valens", "demo", "--database", str(db_file)])
    demo_called = []
    monkeypatch.setattr(demo, "run", lambda x, y, z: demo_called.append(1))
    assert cli.main() == 2
    assert not demo_called
