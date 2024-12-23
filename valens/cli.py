# ruff: noqa: T201

import argparse
import os
import sys
from pathlib import Path
from tempfile import NamedTemporaryFile

from valens import app, config, database as db, demo, version


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--version", action="version", version=version.get())

    subparsers = parser.add_subparsers(dest="subcommand")

    parser_config = subparsers.add_parser("config", help="create config")
    parser_config.set_defaults(func=create_config)
    parser_config.add_argument(
        "-d",
        dest="directory",
        type=Path,
        default=Path(),
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
    parser_run.add_argument(
        "--port",
        metavar="NUMBER",
        type=int,
        default=5000,
        help="port to bind to",
    )

    parser_demo = subparsers.add_parser(
        "demo", help="run app with random example data (all changes are non-persistent)"
    )
    parser_demo.set_defaults(func=run_demo)
    parser_demo.add_argument(
        "--database",
        type=Path,
        help="path to the database file that will be created",
    )
    parser_demo.add_argument(
        "--public",
        action="store_true",
        help="make the server publicly available (should only be used on a trusted network)",
    )
    parser_demo.add_argument(
        "--port",
        metavar="NUMBER",
        type=int,
        default=5000,
        help="port to bind to",
    )

    args = parser.parse_args(sys.argv[1:])

    if not args.subcommand:
        parser.print_usage()
        return 2

    return args.func(args)


def create_config(args: argparse.Namespace) -> int:
    config_file = config.create_config_file(
        args.directory, Path.home() / ".local/share/valens/valens.db"
    )
    print(f"Created {config_file}")
    return 0


def upgrade(_: argparse.Namespace) -> int:
    with app.app_context():
        config.check_config_file(os.environ.copy())
        db.upgrade()
    return 0


def run(args: argparse.Namespace) -> int:
    with app.app_context():
        config.check_config_file(os.environ.copy())
        app.run("0.0.0.0" if args.public else "127.0.0.1", args.port)
    return 0


def run_demo(args: argparse.Namespace) -> int:
    if isinstance(args.database, Path) and args.database.exists():
        print(f'Database "{args.database}" already exists, exiting.', file=sys.stderr)
        return 2

    with NamedTemporaryFile() as f:
        demo.run(
            f"sqlite:///{args.database or f.name}",
            "0.0.0.0" if args.public else "127.0.0.1",
            args.port,
        )
    return 0
