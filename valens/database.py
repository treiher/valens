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


_engines: dict[str, Engine] = {}
_scoped_sessions: dict[str, scoped_session[Session]] = {}
_checked_databases: set[str] = set()


def get_engine() -> Engine:
    database = current_app.config["DATABASE"]
    if database not in _engines:
        config.check_app_config()
        db_dir().mkdir(exist_ok=True)
        # A changed database URI makes the cached engines and sessions stale; dispose them to
        # release their pooled connections. The file behind a previously checked URI may have
        # been replaced in the meantime, so require a new check as well.
        for engine in _engines.values():
            engine.dispose()
        _engines.clear()
        _scoped_sessions.clear()
        _checked_databases.clear()
        _engines[database] = create_engine(database)
    return _engines[database]


def get_scoped_session() -> scoped_session[Session]:
    database = current_app.config["DATABASE"]
    if database not in _scoped_sessions:
        _scoped_sessions[database] = scoped_session(
            sessionmaker(autocommit=False, autoflush=False, bind=get_engine(), future=True)
        )
    return _scoped_sessions[database]


def get_session() -> Session:
    if "db_session" not in g:
        g.db_session = get_scoped_session()()

    database = current_app.config["DATABASE"]
    if database not in _checked_databases:
        if not inspect(get_engine()).get_table_names():
            init()
        _upgrade(g.db_session.connection())
        _checked_databases.add(database)

    return g.db_session


session: Session = LocalProxy(get_session)  # type: ignore[assignment]


def remove_session(_exception: BaseException | None = None) -> None:
    if "db_session" in g:
        get_scoped_session().remove()
        g.pop("db_session")


def init() -> None:
    print("Creating database")  # noqa: T201

    models.Base.query = get_scoped_session().query_property()
    models.Base.metadata.create_all(bind=get_engine())

    command.stamp(alembic_cfg, "head")


def upgrade() -> None:
    _checked_databases.discard(current_app.config["DATABASE"])
    get_session()


def _upgrade(connection: Connection) -> None:
    current = runtime.migration.MigrationContext.configure(connection).get_current_revision()
    head = script.ScriptDirectory.from_config(alembic_cfg).get_current_head()

    if current != head:
        try:
            upgrade_lock_file().touch(exist_ok=False)

            print(f"Upgrading database from {current} to {head}")  # noqa: T201

            copy(
                db_file(),
                db_file().with_suffix(
                    f".db.backup_{current}_{datetime.now().isoformat(timespec='seconds')}"
                ),
            )
            command.upgrade(alembic_cfg, "head")

            upgrade_lock_file().unlink()

        except FileExistsError:
            print("Waiting for completion of database upgrade")  # noqa: T201

            while upgrade_lock_file().exists():
                sleep(1)

        except Exception as e:
            print(f"Database upgrade failed: {e}")  # noqa: T201

            upgrade_lock_file().unlink()


@event.listens_for(Engine, "connect")
def _set_sqlite_pragma(
    dbapi_connection: sqlite3.Connection, _: pool.base._ConnectionRecord
) -> None:
    if current_app.config["SQLITE_FOREIGN_KEY_SUPPORT"]:
        cursor = dbapi_connection.cursor()
        cursor.execute("PRAGMA foreign_keys=ON")
        cursor.close()
