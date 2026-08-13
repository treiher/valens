#!/usr/bin/env python

"""
Check the links in the README of a distribution.

Building a distribution turns the relative links in the README into absolute links to the released
revision. Only these links are requested, as unrelated links to other projects can break at any
time and are irrelevant for a release.
"""

import re
import sys
import time
import urllib.error
import urllib.request
import zipfile
from pathlib import Path

LINK = re.compile(r"\[.+?\]\(\s*([^)\s]+)(?:\s+\"[^\"]*\")?\s*\)")
REPOSITORY_URLS = (
    "https://github.com/treiher/valens/",
    "https://raw.githubusercontent.com/treiher/valens/",
)
SCHEMES = ("http://", "https://", "mailto:")
ATTEMPTS = 5
DELAY = 5
TIMEOUT = 10


def main() -> int:
    if len(sys.argv) != 2:
        print(f"usage: {Path(sys.argv[0]).name} WHEEL", file=sys.stderr)  # noqa: T201
        return 1

    wheel = Path(sys.argv[1])

    try:
        description = readme(wheel)
    except (OSError, zipfile.BadZipFile, LookupError) as exception:
        print(f"{wheel} could not be read: {exception}", file=sys.stderr)  # noqa: T201
        return 1

    errors = check_links(description)

    for error in errors:
        print(error, file=sys.stderr)  # noqa: T201

    return 1 if errors else 0


def readme(wheel: Path) -> str:
    """Return the description stored in the metadata of `wheel`."""
    with zipfile.ZipFile(wheel) as archive:
        name = next((n for n in archive.namelist() if n.endswith(".dist-info/METADATA")), None)

        if name is None:
            raise LookupError("no metadata found")

        metadata = archive.read(name).decode("utf-8")

    # The description is the message body, which is separated from the headers by an empty line.
    _, _, description = metadata.partition("\n\n")

    return description


def check_links(readme: str) -> list[str]:
    """Return one message per link in `readme` that is relative or unreachable."""
    targets = sorted(set(LINK.findall(readme)))

    if not targets:
        return ["README contains no links"]

    errors = []

    for target in targets:
        if not target.startswith(SCHEMES):
            errors.append(f"{target} was not replaced by an absolute link")
        elif target.startswith(REPOSITORY_URLS) and (error := unreachable(target)):
            errors.append(f"{target} is not reachable: {error}")

    return errors


def unreachable(url: str) -> str:
    """Return a message if `url` cannot be requested successfully, an empty string otherwise."""
    request = urllib.request.Request(url, method="HEAD")
    result = ""

    # A revision pushed shortly before may not be served immediately.
    for attempt in range(ATTEMPTS):
        if attempt:
            time.sleep(DELAY)

        try:
            with urllib.request.urlopen(request, timeout=TIMEOUT):
                return ""
        except (urllib.error.URLError, TimeoutError) as error:
            result = str(error)

    return result


if __name__ == "__main__":
    sys.exit(main())
