from flask import Blueprint, send_file, send_from_directory
from flask.typing import ResponseReturnValue
from werkzeug.exceptions import NotFound

bp = Blueprint("static", __name__)


@bp.route("/")
def root() -> ResponseReturnValue:
    return send_file("static/assets/index.html")


@bp.route("/<path:name>")
def static(name: str) -> ResponseReturnValue:
    try:
        return send_from_directory("static/assets/", name)
    except NotFound:
        try:
            return send_from_directory("static/generated/", name)
        except NotFound:
            return send_file("static/assets/index.html")
