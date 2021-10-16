from __future__ import annotations

from pathlib import Path

from valens import app


def check_app_config() -> None:
    for key in ["DATABASE", "SECRET_KEY"]:
        if key not in app.config:
            raise RuntimeError(f"'{key}' is not set in app config")


def check_config_file(environ: dict[str, str]) -> None:
    if "VALENS_CONFIG" not in environ:
        raise RuntimeError("environment variable 'VALENS_CONFIG' is not set")

    config_file = Path(environ["VALENS_CONFIG"])

    if not config_file.exists():
        raise RuntimeError(f"config file '{config_file}' not found")

    check_app_config()
