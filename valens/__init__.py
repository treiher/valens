from datetime import timedelta
from pathlib import Path

from flask import Flask

__version__ = "0.1.0-pre"

app = Flask(__name__)

app.config.from_object("valens.default_config")
app.config.from_envvar("VALENS_CONFIG", silent=True)

app.jinja_env.lstrip_blocks = True
app.jinja_env.trim_blocks = True
