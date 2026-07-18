import logging
from pathlib import Path

import pytest

from valens import app, config


def test_missing_key() -> None:
    with app.app_context():
        if "DATABASE" in app.config:
            del app.config["DATABASE"]
        with pytest.raises(RuntimeError, match=r"'DATABASE' is not set in app config"):
            config.check_app_config()


def test_username_login_disabled_without_public_url() -> None:
    with app.app_context():
        app.config["DATABASE"] = "TEST"
        app.config["SECRET_KEY"] = "TEST"
        app.config["USERNAME_LOGIN_ENABLED"] = False
        app.config.pop("PUBLIC_URL", None)
        try:
            with pytest.raises(
                RuntimeError,
                match=r"'PUBLIC_URL' must be set in app config if username login is disabled",
            ):
                config.check_app_config()
        finally:
            app.config["USERNAME_LOGIN_ENABLED"] = True


def test_username_login_disabled_with_public_url() -> None:
    with app.app_context():
        app.config["DATABASE"] = "TEST"
        app.config["SECRET_KEY"] = "TEST"
        app.config["USERNAME_LOGIN_ENABLED"] = False
        app.config["PUBLIC_URL"] = "https://valens.example.com"
        try:
            config.check_app_config()
        finally:
            app.config["USERNAME_LOGIN_ENABLED"] = True
            del app.config["PUBLIC_URL"]


def test_public_url_not_set_warning(caplog: pytest.LogCaptureFixture) -> None:
    with app.app_context():
        app.config["DATABASE"] = "TEST"
        app.config["SECRET_KEY"] = "TEST"
        app.config.pop("PUBLIC_URL", None)

        with caplog.at_level(logging.WARNING):
            config.check_app_config()

    assert "'PUBLIC_URL' is not set in app config, passkey login is unavailable" in caplog.text


@pytest.mark.parametrize("public_url", ["valens.example.com", "ftp://valens.example.com", ""])
def test_invalid_public_url(public_url: str) -> None:
    with app.app_context():
        app.config["DATABASE"] = "TEST"
        app.config["SECRET_KEY"] = "TEST"
        app.config["PUBLIC_URL"] = public_url
        try:
            with pytest.raises(RuntimeError, match=r"'PUBLIC_URL' must be an HTTP or HTTPS URL"):
                config.check_app_config()
        finally:
            del app.config["PUBLIC_URL"]


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
