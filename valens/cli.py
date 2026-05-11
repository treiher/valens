# ruff: noqa: T201

import argparse
import os
import sys
from pathlib import Path
from tempfile import NamedTemporaryFile

from sqlalchemy import select

from valens import app, config, database as db, demo, version
from valens.models import Sex, User


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
    parser_config.add_argument(
        "--database",
        type=Path,
        default=Path.home() / ".local/share/valens/valens.db",
        help="path to the database file",
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

    parser_user = subparsers.add_parser("user", help="manage users")
    user_subparsers = parser_user.add_subparsers(dest="user_subcommand")

    parser_user_list = user_subparsers.add_parser("list", help="list all users")
    parser_user_list.set_defaults(func=list_users)

    parser_user_create = user_subparsers.add_parser("create", help="create a user")
    parser_user_create.set_defaults(func=create_user)
    parser_user_create.add_argument("name", help="username")
    parser_user_create.add_argument(
        "sex",
        choices=["female", "male"],
        help="biological sex",
    )

    parser_user_update = user_subparsers.add_parser("update", help="update a user")
    parser_user_update.set_defaults(func=update_user)
    parser_user_update.add_argument("name", help="current username")
    parser_user_update.add_argument("--name", dest="new_name", metavar="NAME", help="new username")
    parser_user_update.add_argument(
        "--sex",
        dest="new_sex",
        choices=["female", "male"],
        metavar="SEX",
        help="new biological sex (female or male)",
    )

    parser_user_delete = user_subparsers.add_parser("delete", help="delete a user")
    parser_user_delete.set_defaults(func=delete_user)
    parser_user_delete.add_argument("name", help="username")

    args = parser.parse_args(sys.argv[1:])

    if not args.subcommand:
        parser.print_usage()
        return 2

    if args.subcommand == "user" and not args.user_subcommand:
        parser_user.print_usage()
        return 2

    return args.func(args)


def create_config(args: argparse.Namespace) -> int:
    config_file = config.create_config_file(args.directory, args.database)
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


def list_users(_: argparse.Namespace) -> int:
    with app.app_context():
        config.check_config_file(os.environ.copy())
        users = db.session.execute(select(User)).scalars().all()
        for user in users:
            print(f"{user.id}\t{user.name}\t{user.sex.name.lower()}")

    return 0


def create_user(args: argparse.Namespace) -> int:
    name = args.name.strip()
    if not name:
        print("Username must not be empty", file=sys.stderr)
        return 1

    with app.app_context():
        config.check_config_file(os.environ.copy())
        if db.session.execute(select(User).where(User.name == name)).scalars().one_or_none():
            print(f'User "{name}" already exists', file=sys.stderr)
            return 1
        sex = Sex.FEMALE if args.sex == "female" else Sex.MALE
        db.session.add(User(name=name, sex=sex))
        db.session.commit()
        print(f'Created user "{name}"')

    return 0


def update_user(args: argparse.Namespace) -> int:
    new_name = args.new_name.strip() if args.new_name is not None else None
    if new_name is not None and not new_name:
        print("Username must not be empty", file=sys.stderr)
        return 1
    if new_name is None and args.new_sex is None:
        print("At least one of --name or --sex must be provided", file=sys.stderr)
        return 1

    with app.app_context():
        config.check_config_file(os.environ.copy())
        user = (
            db.session.execute(select(User).where(User.name == args.name)).scalars().one_or_none()
        )
        if user is None:
            print(f'User "{args.name}" not found', file=sys.stderr)
            return 1
        if new_name is not None:
            if new_name != user.name and (
                db.session.execute(select(User).where(User.name == new_name))
                .scalars()
                .one_or_none()
            ):
                print(f'User "{new_name}" already exists', file=sys.stderr)
                return 1
            user.name = new_name
        if args.new_sex is not None:
            user.sex = Sex.FEMALE if args.new_sex == "female" else Sex.MALE
        db.session.commit()
        print(f'Updated user "{args.name}"')

    return 0


def delete_user(args: argparse.Namespace) -> int:
    with app.app_context():
        config.check_config_file(os.environ.copy())
        user = (
            db.session.execute(select(User).where(User.name == args.name)).scalars().one_or_none()
        )
        if user is None:
            print(f'User "{args.name}" not found', file=sys.stderr)
            return 1
        db.session.delete(user)
        db.session.commit()
        print(f'Deleted user "{args.name}"')

    return 0
