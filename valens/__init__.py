from datetime import timedelta
from importlib.metadata import PackageNotFoundError, version
from pathlib import Path

from flask import Flask

try:
    __version__ = version("valens")
except PackageNotFoundError:  # pragma: no cover
    pass

app = Flask(__name__)

app.config.from_object("valens.default_config")
app.config.from_envvar("VALENS_CONFIG", silent=True)

app.jinja_env.lstrip_blocks = True
app.jinja_env.trim_blocks = True
