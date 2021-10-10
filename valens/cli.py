#!/usr/bin/env python

import argparse
import os
import sys
from pathlib import Path
from tempfile import NamedTemporaryFile
from typing import Union

from valens import __version__, database as db, demo, web

CONFIG_FILE = Path("config.py")


def main() -> Union[int, str]:
    parser = argparse.ArgumentParser()
    parser.add_argument("--version", action="store_true", help="show version")
    parser.add_argument("--create-config", action="store_true", help="create config")
    parser.add_argument("--upgrade", action="store_true", help="upgrade database")
    parser.add_argument("--run", action="store_true", help="run app on local development server")
    parser.add_argument(
        "--demo",
        action="store_true",
        help="run app with random example data (all changes are non-persistent)",
    )

    args = parser.parse_args(sys.argv[1:])

    if args.version:
        print(__version__)
        return 0

    if args.create_config:
        CONFIG_FILE.write_text(f"SECRET_KEY = {os.urandom(24)!r}\n", encoding="utf-8")
        return 0

    if args.upgrade:
        db.upgrade_db()
        return 0

    if args.run:
        web.app.run()
        return 0

    if args.demo:
        with NamedTemporaryFile() as f:
            demo.run(f"sqlite:///{f.name}")
        return 0

    parser.print_usage()
    return 2
