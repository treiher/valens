from flask import Flask

from . import api, assets

app = Flask(__name__)

app.config.from_object("valens.default_config")
app.config.from_envvar("VALENS_CONFIG", silent=True)

app.jinja_env.lstrip_blocks = True
app.jinja_env.trim_blocks = True

app.register_blueprint(assets.bp)
app.register_blueprint(api.bp)
