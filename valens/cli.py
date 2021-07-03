#!/usr/bin/env python

import argparse
import sys
from typing import Union

from valens import __version__, database as db


def main() -> Union[int, str]:
    parser = argparse.ArgumentParser()
    parser.add_argument("--version", action="store_true", help="show version")
    parser.add_argument("--init", action="store_true", help="initialize database")
    parser.add_argument("--upgrade", action="store_true", help="upgrade database")

    args = parser.parse_args(sys.argv[1:])

    if args.version:
        print(__version__)
        return 0

    if args.init:
        db.init_db()
        return 0

    if args.upgrade:
        db.upgrade_db()
        return 0

    parser.print_usage()
    return 2
