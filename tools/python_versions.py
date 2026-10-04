#!/usr/bin/env python

"""Print the supported Python versions as GitHub Actions key-value pairs."""

import json
import re
import tomllib
from pathlib import Path

PYPROJECT = Path(__file__).parent.parent / "pyproject.toml"
CLASSIFIER = re.compile(r"Programming Language :: Python :: (\d+\.\d+)")


def main() -> None:
    with PYPROJECT.open("rb") as f:
        classifiers = tomllib.load(f)["project"]["classifiers"]

    versions = [m[1] for c in classifiers if (m := CLASSIFIER.fullmatch(c))]

    if not versions:
        raise RuntimeError(f"no Python version classifiers found in {PYPROJECT}")

    latest = max(versions, key=lambda version: tuple(int(part) for part in version.split(".")))

    print(f"PYTHON_VERSION={latest}")  # noqa: T201
    print(f"PYTHON_VERSIONS={json.dumps(versions)}")  # noqa: T201


if __name__ == "__main__":
    main()
