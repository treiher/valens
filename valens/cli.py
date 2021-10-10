#!/usr/bin/env python

import argparse
import os
import sys
from pathlib import Path
from tempfile import NamedTemporaryFile

from valens import __version__, database as db, demo, web

CONFIG_FILE = Path("config.py")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--version", action="version", version=__version__)

    subparsers = parser.add_subparsers(dest="subcommand")

    parser_create_config = subparsers.add_parser("create_config", help="create config")
    parser_create_config.set_defaults(func=create_config)

    parser_upgrade = subparsers.add_parser("upgrade", help="upgrade database")
    parser_upgrade.set_defaults(func=upgrade)

    parser_run = subparsers.add_parser("run", help="run app on local development server")
    parser_run.set_defaults(func=run)

    parser_demo = subparsers.add_parser(
        "demo", help="run app with random example data (all changes are non-persistent)"
    )
    parser_demo.set_defaults(func=run_demo)

    args = parser.parse_args(sys.argv[1:])

    if not args.subcommand:
        parser.print_usage()
        return 2

    args.func(args)

    return 0


def create_config(_: argparse.Namespace) -> None:
    CONFIG_FILE.write_text(f"SECRET_KEY = {os.urandom(24)!r}\n", encoding="utf-8")


def upgrade(_: argparse.Namespace) -> None:
    db.upgrade_db()


def run(_: argparse.Namespace) -> None:
    web.app.run()


def run_demo(_: argparse.Namespace) -> None:
    with NamedTemporaryFile() as f:
        demo.run(f"sqlite:///{f.name}")
