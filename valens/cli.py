#!/usr/bin/env python

import argparse
import os
import sys
from pathlib import Path
from tempfile import NamedTemporaryFile

from valens import __version__, config, database as db, demo, web

CONFIG_FILE = Path("config.py")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--version", action="version", version=__version__)

    subparsers = parser.add_subparsers(dest="subcommand")

    parser_config = subparsers.add_parser("config", help="create config")
    parser_config.set_defaults(func=create_config)
    parser_config.add_argument(
        "-d",
        dest="directory",
        type=Path,
        default=Path("."),
        help="target directory for the to be created config file",
    )

    parser_upgrade = subparsers.add_parser("upgrade", help="upgrade database")
    parser_upgrade.set_defaults(func=upgrade)

    parser_run = subparsers.add_parser("run", help="run app on local development server")
    parser_run.set_defaults(func=run)
    parser_run.add_argument(
        "--public",
        action="store_true",
        help="make the server publicly available (sould be only used on a trusted network)",
    )

    parser_demo = subparsers.add_parser(
        "demo", help="run app with random example data (all changes are non-persistent)"
    )
    parser_demo.set_defaults(func=run_demo)
    parser_demo.add_argument(
        "--public",
        action="store_true",
        help="make the server publicly available (sould be only used on a trusted network)",
    )

    args = parser.parse_args(sys.argv[1:])

    if not args.subcommand:
        parser.print_usage()
        return 2

    args.func(args)

    return 0


def create_config(args: argparse.Namespace) -> None:
    config_file = args.directory / CONFIG_FILE
    print(f"Creating {config_file}")
    config_file.write_text(
        f"DATABASE = 'sqlite:///{Path.home()}/.local/share/valens/valens.db'\n"
        f"SECRET_KEY = {os.urandom(24)!r}\n",
        encoding="utf-8",
    )


def upgrade(_: argparse.Namespace) -> None:
    config.check_config_file(os.environ.copy())
    db.upgrade_db()


def run(args: argparse.Namespace) -> None:
    config.check_config_file(os.environ.copy())
    web.app.run("0.0.0.0" if args.public else "127.0.0.1")


def run_demo(args: argparse.Namespace) -> None:
    with NamedTemporaryFile() as f:
        demo.run(f"sqlite:///{f.name}", "0.0.0.0" if args.public else "127.0.0.1")
