import sqlite3
from pathlib import Path

from alembic import command
from alembic.config import Config
from flask import current_app, g
from sqlalchemy import create_engine, event, inspect, pool
from sqlalchemy.engine import Engine
from sqlalchemy.orm import Session, scoped_session, sessionmaker
from werkzeug.local import LocalProxy

from valens import config, models

alembic_cfg = Config()
alembic_cfg.set_main_option("script_location", "valens:migrations")


@event.listens_for(Engine, "connect")
def _set_sqlite_pragma(
    dbapi_connection: sqlite3.Connection, _: pool.base._ConnectionRecord
) -> None:
    if current_app.config["SQLITE_FOREIGN_KEY_SUPPORT"]:
        cursor = dbapi_connection.cursor()
        cursor.execute("PRAGMA foreign_keys=ON")
        cursor.close()


def get_engine() -> Engine:
    config.check_app_config()
    Path(current_app.config["DATABASE"].split(":")[1]).parent.mkdir(exist_ok=True)
    return create_engine(current_app.config["DATABASE"])


def get_scoped_session() -> scoped_session:
    return scoped_session(
        sessionmaker(autocommit=False, autoflush=False, bind=get_engine(), future=True)
    )


def get_session() -> Session:
    if "db_session" not in g:
        if not inspect(get_engine()).get_table_names():
            init_db()
        g.db_session = get_scoped_session()()

    return g.db_session


session: Session = LocalProxy(get_session)  # type: ignore


def remove_session() -> None:
    get_scoped_session().remove()


def init_db() -> None:
    models.Base.query = get_scoped_session().query_property()
    models.Base.metadata.create_all(bind=get_engine())

    command.stamp(alembic_cfg, "head")


def upgrade_db() -> None:
    init_db()
    command.upgrade(alembic_cfg, "head")
