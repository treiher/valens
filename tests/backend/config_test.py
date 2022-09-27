from pathlib import Path

import pytest

from valens import app, config


def test_missing_key() -> None:
    with app.app_context():
        with pytest.raises(RuntimeError, match=r"'DATABASE' is not set in app config"):
            if "DATABASE" in app.config:
                del app.config["DATABASE"]
            config.check_app_config()


def test_config_not_set() -> None:
    with pytest.raises(RuntimeError, match=r"environment variable 'VALENS_CONFIG' is not set"):
        config.check_config_file({})


def test_config_file_not_found(tmp_path: Path) -> None:
    config_file = str(tmp_path / "invalid")
    with pytest.raises(RuntimeError, match=rf"config file '{config_file}' not found"):
        config.check_config_file({"VALENS_CONFIG": config_file})


def test_config(tmp_path: Path) -> None:
    with app.app_context():
        config_file = tmp_path / "config.py"
        config_file.write_text("SECRET_KEY = 'TEST'\nDATABASE = 'TEST'\n", encoding="utf-8")
        app.config["SECRET_KEY"] = "TEST"
        app.config["DATABASE"] = "TEST"
        config.check_config_file({"VALENS_CONFIG": str(config_file)})
