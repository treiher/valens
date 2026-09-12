#!/usr/bin/env python

"""
Precompress files with Brotli into a destination directory.

The variants are named after the compressed file with a `.br` suffix.
"""

import sys
from pathlib import Path

import brotli

QUALITY = 11


def main() -> int:
    destination = Path(sys.argv[1])

    for argument in sys.argv[2:]:
        path = Path(argument)
        (destination / f"{path.name}.br").write_bytes(
            brotli.compress(path.read_bytes(), quality=QUALITY)
        )

    return 0


if __name__ == "__main__":
    sys.exit(main())
