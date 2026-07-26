#!/usr/bin/env python

"""Check that relative links in Markdown files resolve to existing files and headings."""

import re
import subprocess
import sys
from pathlib import Path

LINK = re.compile(r"\[[^\]]*\]\(\s*([^)\s]+)(?:\s+\"[^\"]*\")?\s*\)")
HEADING = re.compile(r"^(#{1,6})\s+(.*?)\s*#*$")
FENCE = re.compile(r"^\s*(?:```|~~~)")
INLINE_CODE = re.compile(r"`([^`]*)`")
INLINE_LINK = re.compile(r"\[([^\]]*)\]\([^)]*\)")
PUNCTUATION = re.compile(r"[^\w\- ]")
EXTERNAL_SCHEMES = ("http://", "https://", "mailto:")
VENDORED = "third-party"


def main() -> int:
    files = [
        path
        for line in subprocess.run(
            ["git", "ls-files", "*.md"],
            capture_output=True,
            check=True,
            text=True,
        ).stdout.splitlines()
        if VENDORED not in (path := Path(line)).parts
    ]
    anchors: dict[Path, set[str]] = {}
    broken = [error for file in files for error in check_file(file, anchors)]

    for error in broken:
        print(error, file=sys.stderr)  # noqa: T201

    return 1 if broken else 0


def check_file(file: Path, anchors: dict[Path, set[str]]) -> list[str]:
    """Return one message per unresolvable link in `file`."""
    errors = []

    for number, line in enumerate(file.read_text(encoding="utf-8").splitlines(), start=1):
        for target in LINK.findall(line):
            error = check_link(file, target, anchors)
            if error is not None:
                errors.append(f"{file}:{number}: {error}")

    return errors


def check_link(file: Path, target: str, anchors: dict[Path, set[str]]) -> str | None:
    """Return a message if `target` does not resolve, relative to the location of `file`."""
    if target.startswith(EXTERNAL_SCHEMES):
        return None

    path, _, fragment = target.partition("#")
    linked = (file.parent / path).resolve() if path else file.resolve()

    if not linked.exists():
        return f"{target} does not exist"

    if not fragment or linked.suffix != ".md":
        return None

    if linked not in anchors:
        anchors[linked] = headings(linked)

    if fragment not in anchors[linked]:
        return f"{target} has no matching heading"

    return None


def headings(file: Path) -> set[str]:
    """Return the anchors of all headings in `file`, mirroring how GitHub derives them."""
    result = set()
    occurrences: dict[str, int] = {}
    in_fence = False

    for line in file.read_text(encoding="utf-8").splitlines():
        if FENCE.match(line):
            in_fence = not in_fence
            continue

        match = HEADING.match(line) if not in_fence else None

        if match is None:
            continue

        anchor = to_anchor(match.group(2))
        count = occurrences.get(anchor, 0)
        occurrences[anchor] = count + 1
        result.add(anchor if count == 0 else f"{anchor}-{count}")

    return result


def to_anchor(heading: str) -> str:
    text = INLINE_CODE.sub(r"\1", heading)
    text = INLINE_LINK.sub(r"\1", text)
    return PUNCTUATION.sub("", text.lower()).replace(" ", "-")


if __name__ == "__main__":
    sys.exit(main())
