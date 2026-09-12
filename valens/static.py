from pathlib import Path

from flask import Blueprint, Response, current_app, request, send_from_directory
from flask.typing import ResponseReturnValue
from werkzeug.exceptions import NotFound

bp = Blueprint("static", __name__)

ASSETS_DIR = "static/assets"
GENERATED_DIR = "static/generated"
INDEX = "index.html"


@bp.route("/")
def root() -> ResponseReturnValue:
    return _send(ASSETS_DIR, INDEX)


@bp.route("/<path:name>")
def static(name: str) -> ResponseReturnValue:
    if variant := _send_variant(name):
        return variant
    for directory in (ASSETS_DIR, GENERATED_DIR):
        try:
            return send_from_directory(directory, name)
        except NotFound:
            continue
    return _send(ASSETS_DIR, INDEX)


def _send(directory: str, name: str) -> Response:
    """
    Send a file, preferring its precompressed variant in the generated directory.

    All variants are stored in the generated directory, which also covers the files of the assets
    directory. A file in a subdirectory is sent as is, as its name alone does not identify it.
    """
    return _send_variant(name) or send_from_directory(directory, name)


def _send_variant(name: str) -> Response | None:
    if not _accepts_brotli() or Path(name).parent != Path():
        return None
    variant = f"{name}.br"
    if not (Path(current_app.root_path) / GENERATED_DIR / variant).is_file():
        return None
    # The MIME type of the original file and `Content-Encoding` are derived from the suffixes.
    return send_from_directory(GENERATED_DIR, variant)


def _accepts_brotli() -> bool:
    return request.accept_encodings.quality("br") > 0
