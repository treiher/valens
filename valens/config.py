from __future__ import annotations

import os
from pathlib import Path

from flask import current_app


def check_app_config() -> None:
    for key in ["DATABASE", "SECRET_KEY"]:
        if key not in current_app.config:
            raise RuntimeError(f"'{key}' is not set in app config")


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
        f"DATABASE = 'sqlite:///{database_file}'\nSECRET_KEY = {os.urandom(24)!r}\n",
        encoding="utf-8",
    )
    return config
