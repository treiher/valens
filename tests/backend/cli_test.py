import sys
from pathlib import Path
from unittest.mock import MagicMock

import pytest

from valens import app, cli, config, database as db, demo, models


def test_main_noarg(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens"])
    assert cli.main() == 2


def test_main_version(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "--version"])
    with pytest.raises(SystemExit, match="0"):
        cli.main()


def test_main_config(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    monkeypatch.setattr(
        sys, "argv", ["valens", "config", "-d", str(tmp_path), "--database", str(tmp_path / "db")]
    )
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


def test_main_user_noarg(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "user"])
    assert cli.main() == 2


def test_main_user_list(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "user", "list"])
    monkeypatch.setattr(config, "check_config_file", lambda x: None)
    mock_session = MagicMock()
    mock_session.execute.return_value.scalars.return_value.all.return_value = [
        models.User(id=1, name="Alice", sex=models.Sex.FEMALE),
        models.User(id=2, name="Bob", sex=models.Sex.MALE),
    ]
    monkeypatch.setattr(db, "session", mock_session)
    assert cli.main() == 0


def test_main_user_create(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "user", "create", "Alice", "female"])
    monkeypatch.setattr(config, "check_config_file", lambda x: None)
    mock_session = MagicMock()
    mock_session.execute.return_value.scalars.return_value.one_or_none.return_value = None
    monkeypatch.setattr(db, "session", mock_session)
    assert cli.main() == 0
    mock_session.add.assert_called_once()
    created_user = mock_session.add.call_args[0][0]
    assert created_user.name == "Alice"
    assert created_user.sex == models.Sex.FEMALE
    mock_session.commit.assert_called_once()


def test_main_user_create_strips_name(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "user", "create", "  Alice  ", "female"])
    monkeypatch.setattr(config, "check_config_file", lambda x: None)
    mock_session = MagicMock()
    mock_session.execute.return_value.scalars.return_value.one_or_none.return_value = None
    monkeypatch.setattr(db, "session", mock_session)
    assert cli.main() == 0
    created_user = mock_session.add.call_args[0][0]
    assert created_user.name == "Alice"


def test_main_user_create_empty_name(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "user", "create", "   ", "female"])
    monkeypatch.setattr(config, "check_config_file", lambda x: None)
    mock_session = MagicMock()
    monkeypatch.setattr(db, "session", mock_session)
    assert cli.main() == 1
    mock_session.add.assert_not_called()


def test_main_user_create_duplicate(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "user", "create", "Alice", "female"])
    monkeypatch.setattr(config, "check_config_file", lambda x: None)
    mock_session = MagicMock()
    mock_session.execute.return_value.scalars.return_value.one_or_none.return_value = models.User(
        id=1, name="Alice", sex=models.Sex.FEMALE
    )
    monkeypatch.setattr(db, "session", mock_session)
    assert cli.main() == 1
    mock_session.add.assert_not_called()


def test_main_user_update_name(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "user", "update", "Alice", "--name", "Alicia"])
    monkeypatch.setattr(config, "check_config_file", lambda x: None)
    mock_session = MagicMock()
    mock_user = models.User(id=1, name="Alice", sex=models.Sex.FEMALE)
    mock_session.execute.return_value.scalars.return_value.one_or_none.side_effect = [
        mock_user,
        None,
    ]
    monkeypatch.setattr(db, "session", mock_session)
    assert cli.main() == 0
    assert mock_user.name == "Alicia"
    mock_session.commit.assert_called_once()


def test_main_user_update_sex(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "user", "update", "Alice", "--sex", "male"])
    monkeypatch.setattr(config, "check_config_file", lambda x: None)
    mock_session = MagicMock()
    mock_user = models.User(id=1, name="Alice", sex=models.Sex.FEMALE)
    mock_session.execute.return_value.scalars.return_value.one_or_none.return_value = mock_user
    monkeypatch.setattr(db, "session", mock_session)
    assert cli.main() == 0
    assert mock_user.sex == models.Sex.MALE
    mock_session.commit.assert_called_once()


def test_main_user_update_same_name(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(
        sys, "argv", ["valens", "user", "update", "Alice", "--name", "Alice", "--sex", "male"]
    )
    monkeypatch.setattr(config, "check_config_file", lambda x: None)
    mock_session = MagicMock()
    mock_user = models.User(id=1, name="Alice", sex=models.Sex.FEMALE)
    mock_session.execute.return_value.scalars.return_value.one_or_none.return_value = mock_user
    monkeypatch.setattr(db, "session", mock_session)
    assert cli.main() == 0
    assert mock_user.name == "Alice"
    assert mock_user.sex == models.Sex.MALE
    mock_session.commit.assert_called_once()


def test_main_user_update_strips_name(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "user", "update", "Alice", "--name", "  Alicia  "])
    monkeypatch.setattr(config, "check_config_file", lambda x: None)
    mock_session = MagicMock()
    mock_user = models.User(id=1, name="Alice", sex=models.Sex.FEMALE)
    mock_session.execute.return_value.scalars.return_value.one_or_none.side_effect = [
        mock_user,
        None,
    ]
    monkeypatch.setattr(db, "session", mock_session)
    assert cli.main() == 0
    assert mock_user.name == "Alicia"


def test_main_user_update_empty_name(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "user", "update", "Alice", "--name", "   "])
    monkeypatch.setattr(config, "check_config_file", lambda x: None)
    mock_session = MagicMock()
    monkeypatch.setattr(db, "session", mock_session)
    assert cli.main() == 1
    mock_session.execute.assert_not_called()


def test_main_user_update_no_fields(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "user", "update", "Alice"])
    monkeypatch.setattr(config, "check_config_file", lambda x: None)
    mock_session = MagicMock()
    monkeypatch.setattr(db, "session", mock_session)
    assert cli.main() == 1
    mock_session.execute.assert_not_called()


def test_main_user_update_not_found(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "user", "update", "Alice", "--name", "Alicia"])
    monkeypatch.setattr(config, "check_config_file", lambda x: None)
    mock_session = MagicMock()
    mock_session.execute.return_value.scalars.return_value.one_or_none.return_value = None
    monkeypatch.setattr(db, "session", mock_session)
    assert cli.main() == 1


def test_main_user_update_duplicate(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "user", "update", "Alice", "--name", "Bob"])
    monkeypatch.setattr(config, "check_config_file", lambda x: None)
    mock_session = MagicMock()
    mock_session.execute.return_value.scalars.return_value.one_or_none.side_effect = [
        models.User(id=1, name="Alice", sex=models.Sex.FEMALE),
        models.User(id=2, name="Bob", sex=models.Sex.MALE),
    ]
    monkeypatch.setattr(db, "session", mock_session)
    assert cli.main() == 1
    mock_session.commit.assert_not_called()


def test_main_user_delete(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "user", "delete", "Alice"])
    monkeypatch.setattr(config, "check_config_file", lambda x: None)
    mock_session = MagicMock()
    mock_user = models.User(id=1, name="Alice", sex=models.Sex.FEMALE)
    mock_session.execute.return_value.scalars.return_value.one_or_none.return_value = mock_user
    monkeypatch.setattr(db, "session", mock_session)
    assert cli.main() == 0
    mock_session.delete.assert_called_once_with(mock_user)
    mock_session.commit.assert_called_once()


def test_main_user_delete_not_found(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "argv", ["valens", "user", "delete", "Alice"])
    monkeypatch.setattr(config, "check_config_file", lambda x: None)
    mock_session = MagicMock()
    mock_session.execute.return_value.scalars.return_value.one_or_none.return_value = None
    monkeypatch.setattr(db, "session", mock_session)
    assert cli.main() == 1
