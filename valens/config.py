from __future__ import annotations

import os
from pathlib import Path
from urllib.parse import urlsplit

from flask import current_app


def check_app_config() -> None:
    for key in ["DATABASE", "SECRET_KEY"]:
        if key not in current_app.config:
            raise RuntimeError(f"'{key}' is not set in app config")

    # WebAuthn requires the relying party ID and expected origin, which are derived from
    # `PUBLIC_URL`. Without username login, passkeys are the only way to log in.
    if not current_app.config["USERNAME_LOGIN_ENABLED"] and "PUBLIC_URL" not in current_app.config:
        raise RuntimeError("'PUBLIC_URL' must be set in app config if username login is disabled")

    if "PUBLIC_URL" in current_app.config:
        parts = urlsplit(current_app.config["PUBLIC_URL"])
        if parts.scheme not in ("http", "https") or not parts.hostname:
            raise RuntimeError("'PUBLIC_URL' must be an HTTP or HTTPS URL")
    else:
        current_app.logger.warning(
            "'PUBLIC_URL' is not set in app config, passkey login is unavailable"
        )


def check_config_file(environ: dict[str, str]) -> None:
    if "VALENS_CONFIG" not in environ:
        raise RuntimeError("environment variable 'VALENS_CONFIG' is not set")

    config_file = Path(environ["VALENS_CONFIG"])

    if not config_file.exists():
        raise RuntimeError(f"config file '{config_file}' not found")

    check_app_config()


def create_config_file(config_directory: Path, database_file: Path) -> Path:
    config = config_directory / "config.py"
    config.write_text(
        f"DATABASE = 'sqlite:///{database_file}'\n"
        f"SECRET_KEY = {os.urandom(24)!r}\n"
        "# The URL under which the app is reachable by its users. It must match the address\n"
        "# entered in the browser, otherwise passkeys and one-time login links do not work.\n"
        f"PUBLIC_URL = 'http://localhost:5000'\n",
        encoding="utf-8",
    )
    return config
