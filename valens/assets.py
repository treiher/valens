from flask import Blueprint, current_app, render_template, send_from_directory
from flask.typing import ResponseReturnValue

bp = Blueprint("assets", __name__, template_folder="templates")


def public_url() -> str:
    return current_app.config["PUBLIC_URL"] if "PUBLIC_URL" in current_app.config else ""


@bp.route("/")
def root() -> ResponseReturnValue:
    return render_template("frontend.html", public_url=public_url())


@bp.route("/manifest.json")
def manifest() -> ResponseReturnValue:
    return render_template("manifest.json", public_url=public_url())


@bp.route("/<path:name>")
def frontend(name: str) -> ResponseReturnValue:
    return send_from_directory("frontend", name)
