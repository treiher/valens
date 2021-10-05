#!/usr/bin/env python

import argparse
import os
import sys
from pathlib import Path
from typing import Union

from valens import __version__, database as db

CONFIG_FILE = Path("config.py")


def main() -> Union[int, str]:
    parser = argparse.ArgumentParser()
    parser.add_argument("--version", action="store_true", help="show version")
    parser.add_argument("--create-config", action="store_true", help="create config")
    parser.add_argument("--upgrade", action="store_true", help="upgrade database")

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

    parser.print_usage()
    return 2
