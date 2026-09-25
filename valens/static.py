from pathlib import Path

from flask import Blueprint, Response, current_app, request, send_from_directory
from flask.typing import ResponseReturnValue
from werkzeug.exceptions import NotFound

from valens import version

bp = Blueprint("static", __name__)

ASSETS_DIR = "static/assets"
GENERATED_DIR = "static/generated"
INDEX = "index.html"


@bp.route("/")
def root() -> ResponseReturnValue:
    return _send_index()


@bp.route("/<path:name>")
def static(name: str) -> ResponseReturnValue:
    if variant := _send_variant(name):
        return variant
    try:
        return send_from_directory(ASSETS_DIR, name)
    except NotFound:
        pass
    try:
        return _send_generated(name)
    except NotFound:
        return _send_index()


def _send_index() -> Response:
    return _send_variant(INDEX) or _send_generated(INDEX)


def _send_variant(name: str) -> Response | None:
    """
    Send the precompressed variant of a file, if it exists and is accepted.

    All variants are stored in the generated directory, which also covers the files of the assets
    directory. A file in a subdirectory has no variant, as its name alone does not identify it.
    """
    if not _accepts_brotli() or Path(name).parent != Path():
        return None
    variant = f"{name}.br"
    if not (Path(current_app.root_path) / GENERATED_DIR / variant).is_file():
        return None
    # The MIME type of the original file and `Content-Encoding` are derived from the suffixes.
    return _send_generated(variant)


def _send_generated(name: str) -> Response:
    """Send a file of the generated directory, marked with the version of its build."""
    response = send_from_directory(GENERATED_DIR, name)
    response.headers["Valens-Version"] = version.get()
    return response


def _accepts_brotli() -> bool:
    return request.accept_encodings.quality("br") > 0
