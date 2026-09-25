from importlib.metadata import version
from pathlib import Path

# The version of the built frontend, which differs from the package metadata of an editable install
# after each commit
BUILD_VERSION_FILE = Path(__file__).parent / "static/generated/version"


def get() -> str:
    try:
        build_version = BUILD_VERSION_FILE.read_text().strip()
    except FileNotFoundError:
        build_version = ""
    return build_version or version("valens")
