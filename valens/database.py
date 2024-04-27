# ruff: noqa: T201

import sqlite3
from datetime import datetime
from pathlib import Path
from shutil import copy
from time import sleep

from alembic import command, runtime, script
from alembic.config import Config
from flask import current_app, g
from sqlalchemy import Connection, Engine, create_engine, event, inspect, pool
from sqlalchemy.orm import Session, scoped_session, sessionmaker
from werkzeug.local import LocalProxy

from valens import config, models

alembic_cfg = Config()
alembic_cfg.set_main_option("script_location", "valens:migrations")


def db_file() -> Path:
    return Path(current_app.config["DATABASE"].removeprefix("sqlite:///"))


def db_dir() -> Path:
    return db_file().parent


def upgrade_lock_file() -> Path:
    return db_dir() / "valens_upgrade.lock"


def get_engine() -> Engine:
    config.check_app_config()
    db_dir().mkdir(exist_ok=True)
    return create_engine(current_app.config["DATABASE"])


def get_scoped_session() -> scoped_session[Session]:
    return scoped_session(
        sessionmaker(autocommit=False, autoflush=False, bind=get_engine(), future=True)
    )


def get_session() -> Session:
    if "db_session" not in g:
        if not inspect(get_engine()).get_table_names():
            init()
        g.db_session = get_scoped_session()()

    _upgrade(g.db_session.connection())

    return g.db_session


session: Session = LocalProxy(get_session)  # type: ignore[assignment]


def remove_session() -> None:
    get_scoped_session().remove()


def init() -> None:
    print("Creating database")

    models.Base.query = get_scoped_session().query_property()
    models.Base.metadata.create_all(bind=get_engine())

    command.stamp(alembic_cfg, "head")


def upgrade() -> None:
    get_session()


def _upgrade(connection: Connection) -> None:
    current = runtime.migration.MigrationContext.configure(connection).get_current_revision()
    head = script.ScriptDirectory.from_config(alembic_cfg).get_current_head()

    if current != head:
        try:
            upgrade_lock_file().touch(exist_ok=False)

            print(f"Upgrading database from {current} to {head}")

            copy(
                db_file(),
                db_file().with_suffix(
                    f".db.backup_{current}_{datetime.now().isoformat(timespec='seconds')}"
                ),
            )
            command.upgrade(alembic_cfg, "head")

            upgrade_lock_file().unlink()

        except FileExistsError:
            print("Waiting for completion of database upgrade")

            while upgrade_lock_file().exists():
                sleep(1)

        except Exception as e:
            print(f"Database upgrade failed: {e}")

            upgrade_lock_file().unlink()


@event.listens_for(Engine, "connect")
def _set_sqlite_pragma(
    dbapi_connection: sqlite3.Connection, _: pool.base._ConnectionRecord
) -> None:
    if current_app.config["SQLITE_FOREIGN_KEY_SUPPORT"]:
        cursor = dbapi_connection.cursor()
        cursor.execute("PRAGMA foreign_keys=ON")
        cursor.close()
