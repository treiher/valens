from flask import Flask
from flask_compress import Compress

from . import api, database, static

app = Flask(__name__)

app.config.from_object("valens.default_config")
app.config.from_envvar("VALENS_CONFIG", silent=True)

# Brotli is preferred over gzip, as quality 7 compresses both better and faster than gzip 6. zstd is
# not offered, as its gain over brotli does not pay for a third encoding in the negotiation and a
# third way for caches to fragment. Browsers advertise brotli only over HTTPS, so a plain-HTTP
# deployment receives the gzip result.
app.config["COMPRESS_ALGORITHM"] = ["br", "gzip"]
app.config["COMPRESS_ALGORITHM_STREAMING"] = ["br", "gzip"]
app.config["COMPRESS_BR_LEVEL"] = 7
# Both JavaScript types are listed, as the type guessed for a file depends on the Python version
# and the MIME type database of the system.
app.config["COMPRESS_MIMETYPES"] = [
    "application/javascript",
    "application/json",
    "application/manifest+json",
    "application/wasm",
    "image/svg+xml",
    "image/vnd.microsoft.icon",
    "text/css",
    "text/html",
    "text/javascript",
]
# Static files are sent as streamed responses, for which the etag is only evaluated for the
# endpoints listed here. Without them, every revalidation of a dynamically compressed static file
# answers with the full body instead of `304 Not Modified`.
app.config["COMPRESS_STREAMING_ENDPOINT_CONDITIONAL"] = [
    "static",
    "static.root",
    "static.static",
]

app.jinja_env.lstrip_blocks = True
app.jinja_env.trim_blocks = True

app.register_blueprint(static.bp)
app.register_blueprint(api.bp)

app.teardown_appcontext(database.remove_session)

Compress(app)
