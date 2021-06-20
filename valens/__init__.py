from datetime import timedelta
from pathlib import Path

from flask import Flask

__version__ = "0.1.0-pre"

app = Flask(__name__)

app.config["DATABASE"] = f"sqlite:///{Path.home()}/.config/valens/valens.db"

app.jinja_env.lstrip_blocks = True
app.jinja_env.trim_blocks = True

app.secret_key = b"Q|6s:@}cC{>v:$,#"
app.permanent_session_lifetime = timedelta(weeks=52)
